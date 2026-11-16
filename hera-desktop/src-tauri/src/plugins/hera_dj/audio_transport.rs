//! Tauri v2 command adapter bridging `Player` transport calls to `hera-node-rs::audio_engine::AudioBackend`.

use std::path::PathBuf;
use std::sync::Arc;

use hera_ipc::audio_types::{NativeAudioTelemetryFrame, NativePlayerSnapshot};
use hera_node_rs::audio_engine::{
    AudioBackend, AudioTelemetryFrame, AudioTransportSnapshot, DeckId, TransportCommand,
};

/// Managed Tauri plugin state holding the shared OS `cpal` or headless clock audio backend.
pub struct NativeAudioState(pub Arc<dyn AudioBackend>);

pub(crate) fn map_telemetry_frame(frame: AudioTelemetryFrame) -> NativeAudioTelemetryFrame {
    NativeAudioTelemetryFrame {
        sequence: frame.sequence,
        sample_rate: frame.sample_rate,
        current_time_sec: frame.current_time_sec,
        duration_sec: frame.duration_sec,
        is_playing: frame.is_playing,
        rms_left: frame.rms_left,
        rms_right: frame.rms_right,
        peak_db: frame.peak_db,
        sub_bass: frame.sub_bass,
        low_mid: frame.low_mid,
        high_mid: frame.high_mid,
        air_presence: frame.air_presence,
        spectrum_bins: frame.spectrum_bins.to_vec(),
    }
}

pub(crate) fn map_player_snapshot(
    snap: AudioTransportSnapshot,
    telemetry: Option<AudioTelemetryFrame>,
) -> NativePlayerSnapshot {
    NativePlayerSnapshot {
        backend_kind: snap.backend_kind,
        sample_rate: snap.sample_rate,
        track_loaded: snap.track_loaded,
        loaded_path: snap.loaded_path,
        is_playing: snap.is_playing,
        ended: snap.ended,
        current_time_sec: snap.current_time_sec,
        duration_sec: snap.duration_sec,
        playhead_frame: snap.playhead_frame,
        total_frames: snap.total_frames,
        volume: snap.volume,
        playback_rate: snap.tempo_ratio,
        hot_cue_frames: snap.hot_cue_frames.to_vec(),
        hot_cue_seconds: snap.hot_cue_seconds.to_vec(),
        latest_telemetry: telemetry.map(map_telemetry_frame),
    }
}

/// Converts per-track LUFS ReplayGain in dB (clamped to `-12.0..=6.0 dB`) into linear deck gain,
/// defaulting to `1.0` unity when absent or non-finite so prior track gains never leak across loads.
pub(crate) fn replay_gain_db_to_linear(replay_gain_db: Option<f32>) -> f32 {
    match replay_gain_db {
        Some(db) if db.is_finite() => 10.0_f32.powf(db.clamp(-12.0, 6.0) / 20.0),
        _ => 1.0_f32,
    }
}

/// Maps the first 4 finite non-negative cues onto the 4 hardware deck pads when `cues.len() >= 4`.
pub(crate) fn extract_deck_pad_cues(hot_cues_sec: Option<Vec<f64>>) -> Option<[f64; 4]> {
    hot_cues_sec.and_then(|cues| {
        if cues.len() >= 4 && cues[..4].iter().all(|v| v.is_finite() && *v >= 0.0) {
            Some([cues[0], cues[1], cues[2], cues[3]])
        } else {
            None
        }
    })
}

#[tauri::command]
pub async fn native_audio_load_track(
    state: tauri::State<'_, NativeAudioState>,
    file_path: String,
    duration_hint_sec: Option<f64>,
    hot_cues_sec: Option<Vec<f64>>,
    replay_gain_db: Option<f32>,
) -> Result<NativePlayerSnapshot, String> {
    let backend = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        let known_cues = extract_deck_pad_cues(hot_cues_sec);
        let linear_gain = replay_gain_db_to_linear(replay_gain_db);
        let _ = backend.load_file_fast(
            DeckId::DeckA,
            &PathBuf::from(&file_path),
            duration_hint_sec,
            known_cues,
        )?;
        let mut snap = backend.send_command(TransportCommand::SetGain(DeckId::DeckA, linear_gain));
        if snap.duration_sec <= 0.0
            && let Some(hint) = duration_hint_sec.filter(|d| d.is_finite() && *d > 0.0)
        {
            snap.duration_sec = hint;
        }
        let telemetry = backend.poll_latest_telemetry();
        Ok(map_player_snapshot(snap, telemetry))
    })
    .await
    .map_err(|e| format!("native_audio_load_track join error: {e}"))?
}

#[tauri::command]
pub async fn native_audio_play(
    state: tauri::State<'_, NativeAudioState>,
) -> Result<NativePlayerSnapshot, String> {
    let snap = state.0.send_command(TransportCommand::Play(DeckId::DeckA));
    let telemetry = state.0.poll_latest_telemetry();
    Ok(map_player_snapshot(snap, telemetry))
}

#[tauri::command]
pub async fn native_audio_pause(
    state: tauri::State<'_, NativeAudioState>,
) -> Result<NativePlayerSnapshot, String> {
    let snap = state.0.send_command(TransportCommand::Pause(DeckId::DeckA));
    let telemetry = state.0.poll_latest_telemetry();
    Ok(map_player_snapshot(snap, telemetry))
}

#[tauri::command]
pub async fn native_audio_seek(
    state: tauri::State<'_, NativeAudioState>,
    position_sec: f64,
) -> Result<NativePlayerSnapshot, String> {
    let safe_sec = if position_sec.is_finite() {
        position_sec.max(0.0)
    } else {
        0.0
    };
    let sr = f64::from(state.0.output_sample_rate().max(1));
    let target_frame = (safe_sec * sr).round() as u64;
    let snap = state
        .0
        .send_command(TransportCommand::SeekToSample(DeckId::DeckA, target_frame));
    let telemetry = state.0.poll_latest_telemetry();
    Ok(map_player_snapshot(snap, telemetry))
}

#[tauri::command]
pub async fn native_audio_set_volume(
    state: tauri::State<'_, NativeAudioState>,
    volume: f32,
) -> Result<NativePlayerSnapshot, String> {
    let snap = state.0.set_master_volume(volume);
    let telemetry = state.0.poll_latest_telemetry();
    Ok(map_player_snapshot(snap, telemetry))
}

#[tauri::command]
pub async fn native_audio_set_playback_rate(
    state: tauri::State<'_, NativeAudioState>,
    rate: f32,
) -> Result<NativePlayerSnapshot, String> {
    let safe_rate = if rate.is_finite() {
        rate.clamp(0.5, 2.0)
    } else {
        1.0
    };
    let snap = state
        .0
        .send_command(TransportCommand::SetTempoRatio(DeckId::DeckA, safe_rate));
    let telemetry = state.0.poll_latest_telemetry();
    Ok(map_player_snapshot(snap, telemetry))
}

#[tauri::command]
pub async fn native_audio_get_state(
    state: tauri::State<'_, NativeAudioState>,
) -> Result<NativePlayerSnapshot, String> {
    let snap = state.0.snapshot();
    // Do not drain `rtrb` telemetry here so the 120 Hz Channel stream receives every frame.
    Ok(map_player_snapshot(snap, None))
}
