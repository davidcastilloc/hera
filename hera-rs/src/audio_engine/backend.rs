//! Hexagonal `trait AudioBackend`, transport snapshot DTOs, and `HeadlessClockAudioBackend`.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::dsp_chain::AudioTelemetryFrame;
use super::hub::{create_controller_and_worker, AudioControllerHub, RealtimeRenderWorker};
use super::{load_deck_buffer_from_bytes, load_deck_buffer_from_file, DeckId, TransportCommand};

/// Active OS or deterministic fallback audio transport backend kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioBackendKind {
    CpalSharedDevice,
    HeadlessClockFallback,
}

impl AudioBackendKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CpalSharedDevice => "cpal_shared",
            Self::HeadlessClockFallback => "headless_clock",
        }
    }
}

/// Snapshot of active player transport state exposed to Tauri IPC and tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTransportSnapshot {
    pub backend_kind: String,
    pub sample_rate: u32,
    pub track_loaded: bool,
    pub loaded_path: Option<String>,
    pub is_playing: bool,
    pub ended: bool,
    pub current_time_sec: f64,
    pub duration_sec: f64,
    pub playhead_frame: u64,
    pub total_frames: u64,
    pub volume: f32,
    pub tempo_ratio: f32,
    pub hot_cue_frames: [u64; 4],
    pub hot_cue_seconds: [f64; 4],
}

/// Hexagonal port driving HERA's native lock-free `DualDeckEngine` + DSP chain.
pub trait AudioBackend: Send + Sync {
    fn backend_kind(&self) -> AudioBackendKind;
    fn output_sample_rate(&self) -> u32;
    fn load_file(&self, deck: DeckId, path: &Path) -> Result<AudioTransportSnapshot, String>;
    fn load_bytes(
        &self,
        deck: DeckId,
        bytes: &[u8],
        ext_hint: Option<&str>,
    ) -> Result<AudioTransportSnapshot, String>;
    fn send_command(&self, cmd: TransportCommand) -> AudioTransportSnapshot;
    fn set_master_volume(&self, volume: f32) -> AudioTransportSnapshot;
    fn snapshot(&self) -> AudioTransportSnapshot;
    fn poll_latest_telemetry(&self) -> Option<AudioTelemetryFrame>;
}

/// Pure-Rust deterministic clock/render backend for headless environments and shadow tests.
pub struct HeadlessClockAudioBackend {
    hub: Arc<AudioControllerHub>,
    manual_worker: Option<Mutex<RealtimeRenderWorker>>,
    shutdown: Arc<AtomicBool>,
    ticker_thread: Option<JoinHandle<()>>,
}

impl HeadlessClockAudioBackend {
    /// Spawn a real-time headless ticker (`10 ms` quantum) at `sample_rate`.
    pub fn new(sample_rate: u32) -> Self {
        let (hub, mut worker) =
            create_controller_and_worker(AudioBackendKind::HeadlessClockFallback, sample_rate);
        let hub = Arc::new(hub);
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = Arc::clone(&shutdown);
        let quantum_frames = ((sample_rate.max(8_000) / 100) as usize).max(128);
        let ticker_thread = thread::Builder::new()
            .name("hera-headless-audio".to_string())
            .spawn(move || {
                let mut scratch = vec![0.0_f32; quantum_frames * 2];
                while !shutdown_flag.load(Ordering::Acquire) {
                    worker.render_stereo_block(&mut scratch);
                    thread::sleep(Duration::from_millis(10));
                }
            })
            .ok();
        Self {
            hub,
            manual_worker: None,
            shutdown,
            ticker_thread,
        }
    }

    /// Construct a manual step-driven headless backend for deterministic zero-sleep unit tests.
    pub fn new_deterministic(sample_rate: u32) -> Self {
        let (hub, worker) =
            create_controller_and_worker(AudioBackendKind::HeadlessClockFallback, sample_rate);
        Self {
            hub: Arc::new(hub),
            manual_worker: Some(Mutex::new(worker)),
            shutdown: Arc::new(AtomicBool::new(false)),
            ticker_thread: None,
        }
    }

    /// Deterministically render exactly `frames` stereo frames (for manual test instances).
    pub fn render_deterministic_frames(&self, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0_f32; frames * 2];
        if let Some(worker_mtx) = &self.manual_worker {
            if let Ok(mut w) = worker_mtx.lock() {
                w.render_stereo_block(&mut out);
            }
        }
        out
    }
}

impl Default for HeadlessClockAudioBackend {
    fn default() -> Self {
        Self::new(48_000)
    }
}

impl Drop for HeadlessClockAudioBackend {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        if let Some(handle) = self.ticker_thread.take() {
            let _ = handle.join();
        }
    }
}

impl AudioBackend for HeadlessClockAudioBackend {
    fn backend_kind(&self) -> AudioBackendKind {
        AudioBackendKind::HeadlessClockFallback
    }
    fn output_sample_rate(&self) -> u32 {
        self.hub.atomics.sample_rate
    }
    fn load_file(&self, deck: DeckId, path: &Path) -> Result<AudioTransportSnapshot, String> {
        let raw = load_deck_buffer_from_file(path)?;
        self.hub
            .install_buffer(deck, raw, Some(path.to_string_lossy().to_string()))
    }
    fn load_bytes(
        &self,
        deck: DeckId,
        bytes: &[u8],
        ext_hint: Option<&str>,
    ) -> Result<AudioTransportSnapshot, String> {
        let raw = load_deck_buffer_from_bytes(bytes, ext_hint)?;
        self.hub.install_buffer(deck, raw, None)
    }
    fn send_command(&self, cmd: TransportCommand) -> AudioTransportSnapshot {
        self.hub.send_command(cmd)
    }
    fn set_master_volume(&self, volume: f32) -> AudioTransportSnapshot {
        self.hub.set_master_volume(volume)
    }
    fn snapshot(&self) -> AudioTransportSnapshot {
        self.hub.snapshot()
    }
    fn poll_latest_telemetry(&self) -> Option<AudioTelemetryFrame> {
        self.hub.poll_latest_telemetry()
    }
}
