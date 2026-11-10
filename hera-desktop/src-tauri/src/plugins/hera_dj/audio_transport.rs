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

#[tauri::command]
pub async fn native_audio_load_track(
    state: tauri::State<'_, NativeAudioState>,
    file_path: String,
    duration_hint_sec: Option<f64>,
) -> Result<NativePlayerSnapshot, String> {
    let backend = Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        let mut snap = backend.load_file(DeckId::DeckA, &PathBuf::from(&file_path))?;
        if snap.duration_sec <= 0.0 {
            if let Some(hint) = duration_hint_sec.filter(|d| d.is_finite() && *d > 0.0) {
                snap.duration_sec = hint;
            }
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
    let telemetry = state.0.poll_latest_telemetry();
    Ok(map_player_snapshot(snap, telemetry))
}
