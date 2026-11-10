#![forbid(unsafe_code)]
//! Lock-Free In-Memory Native Dual-Deck Audio Transport Core (`DeckA` + `DeckB`) & Pro-DJ DSP.
//!
//! Built with `#![forbid(unsafe_code)]`, zero `Mutex`, zero heap allocation, and zero I/O inside
//! [`DualDeckEngine::render_interleaved_stereo`], avoiding OS soundcard contention with VirtualDJ.

#[cfg(feature = "native")]
pub mod backend;
mod command_queue;
#[cfg(feature = "audio-device")]
pub mod cpal_backend;
mod decoder;
#[cfg(feature = "native")]
pub mod dsp_chain;
#[cfg(all(test, feature = "native"))]
mod dsp_tests;
#[cfg(feature = "native")]
mod hub;
mod mixer;
#[cfg(test)]
mod tests;

use std::sync::Arc;

#[cfg(feature = "native")]
pub use backend::{
    AudioBackend, AudioBackendKind, AudioTransportSnapshot, HeadlessClockAudioBackend,
};
pub use command_queue::{
    CommandQueue, DeckId, TransportCommand, COMMAND_QUEUE_CAPACITY, MAX_SEEK_FRAME,
};
#[cfg(feature = "audio-device")]
pub use cpal_backend::{create_default_audio_backend, CpalSharedAudioBackend};
pub use decoder::{
    load_deck_buffer_from_bytes, load_deck_buffer_from_file, HotCueSampleMarker,
    PreallocatedDeckBuffer, HOT_CUE_LABELS,
};
#[cfg(feature = "native")]
pub use dsp_chain::{
    create_telemetry_ring, resample_deck_buffer_to_rate, AudioTelemetryFrame, MasterDspBus,
    RealTimeSpectrumAnalyzer, SPECTRUM_BIN_COUNT, TELEMETRY_RING_CAPACITY,
};

/// Fallback default backend factory when `native` is enabled without `audio-device`.
#[cfg(all(feature = "native", not(feature = "audio-device")))]
pub fn create_default_audio_backend() -> Arc<dyn AudioBackend> {
    Arc::new(HeadlessClockAudioBackend::default())
}

/// Per-deck playback state owned exclusively by [`DualDeckEngine`].
#[derive(Debug, Clone)]
pub struct DeckState {
    buffer: Option<Arc<PreallocatedDeckBuffer>>,
    playing: bool,
    playhead_frames: f64,
    tempo_ratio: f32,
    gain: f32,
    pub cue_monitor_enabled: bool,
}

impl Default for DeckState {
    fn default() -> Self {
        Self {
            buffer: None,
            playing: false,
            playhead_frames: 0.0,
            tempo_ratio: 1.0,
            gain: 1.0,
            cue_monitor_enabled: false,
        }
    }
}

/// Pure-Rust lock-free Dual-Deck (`DeckA` + `DeckB`) stereo transport and mixer engine.
#[derive(Debug, Default)]
pub struct DualDeckEngine {
    decks: [DeckState; 2],
}

impl DualDeckEngine {
    /// Create a new stopped `DualDeckEngine` with unity tempo (`1.0`) and unity gain (`1.0`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-load a decoded [`PreallocatedDeckBuffer`] onto `deck` (called off the audio thread).
    pub fn load_deck(&mut self, deck: DeckId, buffer: Arc<PreallocatedDeckBuffer>) {
        let state = &mut self.decks[deck as usize];
        state.buffer = Some(buffer);
        state.playhead_frames = 0.0;
        state.playing = false;
    }

    /// Enable or disable headphone cue monitoring flag on `deck`.
    pub fn set_cue_monitor(&mut self, deck: DeckId, enabled: bool) {
        self.decks[deck as usize].cue_monitor_enabled = enabled;
    }

    /// Return current stereo frame playhead index for `deck`.
    pub fn deck_playhead_frame(&self, deck: DeckId) -> u64 {
        self.decks[deck as usize].playhead_frames.max(0.0) as u64
    }

    /// Return `true` if `deck` is actively playing.
    pub fn is_deck_playing(&self, deck: DeckId) -> bool {
        self.decks[deck as usize].playing
    }

    /// Return current clamped tempo ratio (`0.5..=2.0`) for `deck`.
    pub fn deck_tempo_ratio(&self, deck: DeckId) -> f32 {
        self.decks[deck as usize].tempo_ratio
    }

    /// Return current clamped linear channel gain (`0.0..=2.0`) for `deck`.
    pub fn deck_gain(&self, deck: DeckId) -> f32 {
        self.decks[deck as usize].gain
    }

    /// Real-time stereo render callback: drains up to 64 lock-free commands, mixes `DeckA + DeckB`,
    /// soft-clamps output to `-1.0..=1.0` with zero NaNs, and returns rendered stereo frame count.
    pub fn render_interleaved_stereo(
        &mut self,
        queue: &CommandQueue,
        out_stereo: &mut [f32],
    ) -> usize {
        for _ in 0..COMMAND_QUEUE_CAPACITY {
            let Some(cmd) = queue.try_pop() else {
                break;
            };
            self.apply_command(cmd);
        }

        out_stereo.fill(0.0);
        let frames = out_stereo.len() / 2;
        if frames == 0 {
            return 0;
        }

        let active_slice = &mut out_stereo[..frames * 2];
        for deck in &mut self.decks {
            mixer::mix_deck_into_output(deck, active_slice, frames);
        }
        for s in active_slice.iter_mut() {
            *s = if s.is_finite() {
                s.clamp(-1.0, 1.0)
            } else {
                0.0
            };
        }
        frames
    }

    fn apply_command(&mut self, cmd: TransportCommand) {
        match cmd {
            TransportCommand::Play(deck) => {
                let d = &mut self.decks[deck as usize];
                if d.buffer.is_some() {
                    d.playing = true;
                }
            }
            TransportCommand::Pause(deck) => self.decks[deck as usize].playing = false,
            TransportCommand::SeekToSample(deck, target_frame) => {
                let d = &mut self.decks[deck as usize];
                if let Some(buf) = d.buffer.as_deref() {
                    let clamped = target_frame.min(buf.total_frames());
                    d.playhead_frames = clamped as f64;
                    if clamped >= buf.total_frames() {
                        d.playing = false;
                    }
                } else {
                    d.playhead_frames = 0.0;
                }
            }
            TransportCommand::JumpToHotCue(deck, cue_num) if (1..=4).contains(&cue_num) => {
                let d = &mut self.decks[deck as usize];
                if let Some(buf) = d.buffer.as_deref() {
                    let target = buf.hot_cues[(cue_num - 1) as usize]
                        .frame_index
                        .min(buf.total_frames());
                    d.playhead_frames = target as f64;
                }
            }
            TransportCommand::SetTempoRatio(deck, ratio) if ratio.is_finite() => {
                self.decks[deck as usize].tempo_ratio = ratio.clamp(0.5, 2.0);
            }
            TransportCommand::SetGain(deck, gain) if gain.is_finite() => {
                self.decks[deck as usize].gain = gain.clamp(0.0, 2.0);
            }
            _ => {}
        }
    }
}
