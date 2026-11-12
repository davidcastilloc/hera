//! `120 Hz` (`8 ms`) Tauri v2 `tauri::ipc::Channel<NativeAudioTelemetryFrame>` streaming bridge
//! from `hera-node-rs::audio_engine::AudioBackend` (`rtrb` SPSC ring buffer) to the WebView.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use hera_ipc::audio_types::{NativeAudioTelemetryFrame, NativePlayerSnapshot};
use hera_node_rs::audio_engine::AudioTransportSnapshot;

use super::audio_transport::{NativeAudioState, map_player_snapshot, map_telemetry_frame};

static TELEMETRY_SUB_GEN: AtomicU64 = AtomicU64::new(0);
static SYNTH_FRAME_SEQ: AtomicU64 = AtomicU64::new(1);

/// Synthesizes a deterministic 32-bin `NativeAudioTelemetryFrame` from an `AudioTransportSnapshot`
/// when the lock-free `rtrb` telemetry ring buffer has no newly rendered callback block queued yet
/// (e.g. `HeadlessClockAudioBackend` or between callback boundaries).
pub(crate) fn synthesize_telemetry_frame_from_snapshot(
    snap: &AudioTransportSnapshot,
    sequence: u64,
) -> NativeAudioTelemetryFrame {
    let active = snap.is_playing && snap.track_loaded;
    let phase = snap.current_time_sec * 8.0 + (sequence as f64) * 0.15;
    let mut bins = Vec::with_capacity(32);

    for idx in 0..32usize {
        if !active {
            bins.push(0.0);
            continue;
        }
        let ratio = (idx as f64) / 31.0;
        let tilt = 1.0 - ratio * 0.58;
        let wave = ((phase + (idx as f64) * 0.42).sin() * 0.5 + 0.5) as f32;
        let sub_pulse = if idx < 5 {
            ((snap.current_time_sec * 4.2).cos().abs() as f32) * 0.28
        } else {
            0.0
        };
        let mag = ((0.18 + wave * 0.56 + sub_pulse) * (tilt as f32) * snap.volume.clamp(0.05, 1.0))
            .clamp(0.0, 1.0);
        bins.push(mag);
    }

    let avg = |slice: &[f32]| -> f32 {
        if slice.is_empty() {
            0.0
        } else {
            slice.iter().copied().sum::<f32>() / (slice.len() as f32)
        }
    };

    let sub_bass = avg(&bins[0..4]);
    let low_mid = avg(&bins[4..12]);
    let high_mid = avg(&bins[12..22]);
    let air_presence = avg(&bins[22..32]);
    let rms = if active {
        ((sub_bass * 0.4 + low_mid * 0.35 + high_mid * 0.25) * 0.78).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let peak_db = if rms > 1.0e-5 {
        (20.0 * rms.log10()).clamp(-96.0, 0.0)
    } else {
        -96.0
    };

    NativeAudioTelemetryFrame {
        sequence,
        sample_rate: snap.sample_rate,
        current_time_sec: snap.current_time_sec,
        duration_sec: snap.duration_sec,
        is_playing: snap.is_playing,
        rms_left: rms,
        rms_right: (rms * 0.97).clamp(0.0, 1.0),
        peak_db,
        sub_bass,
        low_mid,
        high_mid,
        air_presence,
        spectrum_bins: bins,
    }
}

/// Resolve the telemetry frame for a 120Hz tick: uses fresh telemetry if available,
/// decays the last real frame if on a hardware backend, or synthesizes from snapshot on headless fallback.
pub(crate) fn resolve_tick_telemetry_frame(
    raw_opt: Option<hera_node_rs::audio_engine::AudioTelemetryFrame>,
    snap: &AudioTransportSnapshot,
    last_real_frame: &mut Option<NativeAudioTelemetryFrame>,
    seq_counter: &AtomicU64,
) -> Option<NativeAudioTelemetryFrame> {
    if let Some(raw) = raw_opt {
        let mapped = map_telemetry_frame(raw);
        *last_real_frame = Some(mapped.clone());
        return Some(mapped);
    }
    if !snap.is_playing {
        *last_real_frame = None;
        return None;
    }
    if snap.backend_kind != "headless_clock" {
        if let Some(last) = last_real_frame {
            last.rms_left *= 0.96;
            last.rms_right *= 0.96;
            for bin in last.spectrum_bins.iter_mut() {
                *bin *= 0.96;
            }
            last.current_time_sec = snap.current_time_sec;
            last.is_playing = snap.is_playing;
            return Some(last.clone());
        }
    }
    let seq = seq_counter.fetch_add(1, Ordering::Relaxed);
    Some(synthesize_telemetry_frame_from_snapshot(snap, seq))
}

/// Subscribes a Tauri v2 `Channel<NativeAudioTelemetryFrame>` to `120 Hz` (`8 ms`) audio DSP
/// telemetry frames. Starting a new subscription atomically cancels any prior subscription loop.
#[tauri::command]
pub async fn subscribe_audio_telemetry(
    state: tauri::State<'_, NativeAudioState>,
    on_frame: tauri::ipc::Channel<NativeAudioTelemetryFrame>,
) -> Result<NativePlayerSnapshot, String> {
    let backend = Arc::clone(&state.0);
    let sub_id = TELEMETRY_SUB_GEN.fetch_add(1, Ordering::SeqCst) + 1;

    let initial_snap = backend.snapshot();
    let initial_raw = backend.poll_latest_telemetry();
    let mut last_real_frame = initial_raw.map(map_telemetry_frame);
    let initial_frame = last_real_frame.clone().unwrap_or_else(|| {
        let seq = SYNTH_FRAME_SEQ.fetch_add(1, Ordering::Relaxed);
        synthesize_telemetry_frame_from_snapshot(&initial_snap, seq)
    });
    let _ = on_frame.send(initial_frame);

    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(8));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        while TELEMETRY_SUB_GEN.load(Ordering::Relaxed) == sub_id {
            ticker.tick().await;
            if TELEMETRY_SUB_GEN.load(Ordering::Relaxed) != sub_id {
                break;
            }

            let raw_opt = backend.poll_latest_telemetry();
            let snap = backend.snapshot();
            if let Some(frame) =
                resolve_tick_telemetry_frame(raw_opt, &snap, &mut last_real_frame, &SYNTH_FRAME_SEQ)
            {
                if on_frame.send(frame).is_err() {
                    break;
                }
            }
        }
    });

    Ok(map_player_snapshot(initial_snap, initial_raw))
}

#[cfg(test)]
mod tests {
    use super::synthesize_telemetry_frame_from_snapshot;
    use hera_node_rs::audio_engine::AudioTransportSnapshot;

    #[test]
    fn test_synthesize_telemetry_frame_32_bins_playing_and_paused() {
        let playing_snap = AudioTransportSnapshot {
            backend_kind: "headless_clock".to_string(),
            sample_rate: 48_000,
            track_loaded: true,
            loaded_path: Some("track.wav".to_string()),
            is_playing: true,
            ended: false,
            current_time_sec: 12.5,
            duration_sec: 180.0,
            playhead_frame: 600_000,
            total_frames: 8_640_000,
            volume: 0.9,
            tempo_ratio: 1.0,
            hot_cue_frames: [0, 96_000, 192_000, 288_000],
            hot_cue_seconds: [0.0, 2.0, 4.0, 6.0],
        };
        let frame = synthesize_telemetry_frame_from_snapshot(&playing_snap, 42);
        assert_eq!(frame.sequence, 42);
        assert_eq!(frame.spectrum_bins.len(), 32);
        assert!(frame.is_playing);
        assert!(frame.sub_bass > 0.0);
        assert!(frame.peak_db > -96.0 && frame.peak_db <= 0.0);

        let mut paused_snap = playing_snap;
        paused_snap.is_playing = false;
        let idle = synthesize_telemetry_frame_from_snapshot(&paused_snap, 43);
        assert!(!idle.is_playing);
        assert_eq!(idle.peak_db, -96.0);
    }

    #[test]
    fn test_resolve_tick_telemetry_frame_decays_hardware_frame() {
        use super::resolve_tick_telemetry_frame;
        use std::sync::atomic::AtomicU64;
        let snap = AudioTransportSnapshot {
            backend_kind: "cpal_shared".to_string(),
            sample_rate: 48_000,
            track_loaded: true,
            loaded_path: Some("track.wav".to_string()),
            is_playing: true,
            ended: false,
            current_time_sec: 10.0,
            duration_sec: 180.0,
            playhead_frame: 480_000,
            total_frames: 8_640_000,
            volume: 1.0,
            tempo_ratio: 1.0,
            hot_cue_frames: [0; 4],
            hot_cue_seconds: [0.0; 4],
        };
        let synth = super::synthesize_telemetry_frame_from_snapshot(&snap, 1);
        let orig_rms = synth.rms_left;
        let mut last_frame = Some(synth);
        let seq = AtomicU64::new(10);

        let decayed = resolve_tick_telemetry_frame(None, &snap, &mut last_frame, &seq).unwrap();
        assert!((decayed.rms_left - orig_rms * 0.96).abs() < 1e-4);
        assert_eq!(seq.load(std::sync::atomic::Ordering::Relaxed), 10);
    }
}
