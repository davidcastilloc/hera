//! Typed IPC DTOs for HERA Desktop's Native Pro-DJ Rust Audio Engine (`ts-rs`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Real-time audio spectrum & loudness frame exported from Rust DSP (`realfft` + `rtrb`) to TypeScript.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct NativeAudioTelemetryFrame {
    #[ts(type = "number")]
    pub sequence: u64,
    pub sample_rate: u32,
    pub current_time_sec: f64,
    pub duration_sec: f64,
    pub is_playing: bool,
    pub rms_left: f32,
    pub rms_right: f32,
    pub peak_db: f32,
    pub sub_bass: f32,
    pub low_mid: f32,
    pub high_mid: f32,
    pub air_presence: f32,
    pub spectrum_bins: Vec<f32>,
}

/// Complete transport state snapshot returned by `plugin:hera-dj|native_audio_*` commands.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct NativePlayerSnapshot {
    pub backend_kind: String,
    pub sample_rate: u32,
    pub track_loaded: bool,
    #[ts(optional = nullable)]
    pub loaded_path: Option<String>,
    pub is_playing: bool,
    pub ended: bool,
    pub current_time_sec: f64,
    pub duration_sec: f64,
    #[ts(type = "number")]
    pub playhead_frame: u64,
    #[ts(type = "number")]
    pub total_frames: u64,
    pub volume: f32,
    pub playback_rate: f32,
    #[ts(type = "Array<number>")]
    pub hot_cue_frames: Vec<u64>,
    pub hot_cue_seconds: Vec<f64>,
    #[ts(optional = nullable)]
    pub latest_telemetry: Option<NativeAudioTelemetryFrame>,
}
