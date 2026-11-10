#![forbid(unsafe_code)]
//! Lock-Free In-Memory Native Dual-Deck Audio Transport Core (`DeckA` + `DeckB`).
//!
//! Built with `#![forbid(unsafe_code)]`, zero `Mutex`, zero heap allocation, and zero I/O inside
//! [`DualDeckEngine::render_interleaved_stereo`], avoiding OS soundcard contention with VirtualDJ.

mod decoder;
#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

pub use decoder::{
    load_deck_buffer_from_bytes, load_deck_buffer_from_file, HotCueSampleMarker,
    PreallocatedDeckBuffer, HOT_CUE_LABELS,
};

/// Fixed SPSC ring-buffer slot count (`64` non-blocking atomic command words).
pub const COMMAND_QUEUE_CAPACITY: usize = 64;
/// Maximum lossless 54-bit sample frame index packable into a `64`-bit `TransportCommand` word.
pub const MAX_SEEK_FRAME: u64 = (1u64 << 54) - 1;

const RING_MASK: usize = COMMAND_QUEUE_CAPACITY - 1;
const PAYLOAD_MASK: u64 = MAX_SEEK_FRAME;
const DECK_SHIFT: u32 = 54;
const OPCODE_SHIFT: u32 = 55;
const OCCUPIED_BIT: u64 = 1u64 << 63;

/// Target virtual DJ deck identifier (`DeckA` or `DeckB`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckId {
    DeckA = 0,
    DeckB = 1,
}

/// Real-time transport command losslessly packable into a single non-zero `u64` atomic word.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransportCommand {
    Play(DeckId),
    Pause(DeckId),
    SeekToSample(DeckId, u64),
    JumpToHotCue(DeckId, u8),
    SetTempoRatio(DeckId, f32),
    SetGain(DeckId, f32),
}

/// Lock-free Single-Producer Single-Consumer (SPSC) transport command ring buffer (`[AtomicU64; 64]`).
pub struct CommandQueue {
    slots: [AtomicU64; COMMAND_QUEUE_CAPACITY],
    write_idx: AtomicUsize,
    read_idx: AtomicUsize,
}

impl CommandQueue {
    /// Construct an empty lock-free 64-slot SPSC `CommandQueue`.
    pub const fn new() -> Self {
        Self {
            slots: [const { AtomicU64::new(0) }; COMMAND_QUEUE_CAPACITY],
            write_idx: AtomicUsize::new(0),
            read_idx: AtomicUsize::new(0),
        }
    }

    /// Push a [`TransportCommand`] without blocking or allocating. Returns `false` if full.
    pub fn try_push(&self, cmd: TransportCommand) -> bool {
        let w = self.write_idx.load(Ordering::Acquire);
        let slot = &self.slots[w & RING_MASK];
        if slot.load(Ordering::Acquire) != 0 {
            return false;
        }
        slot.store(pack_command(cmd), Ordering::Release);
        self.write_idx.store(w.wrapping_add(1), Ordering::Release);
        true
    }

    /// Pop the next [`TransportCommand`] in FIFO order without blocking or allocating.
    pub fn try_pop(&self) -> Option<TransportCommand> {
        let r = self.read_idx.load(Ordering::Acquire);
        let slot = &self.slots[r & RING_MASK];
        let word = slot.load(Ordering::Acquire);
        if word == 0 {
            return None;
        }
        slot.store(0, Ordering::Release);
        self.read_idx.store(r.wrapping_add(1), Ordering::Release);
        unpack_command(word)
    }
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self::new()
    }
}

fn pack_command(cmd: TransportCommand) -> u64 {
    let (deck, opcode, payload) = match cmd {
        TransportCommand::Play(d) => (d, 1u64, 0),
        TransportCommand::Pause(d) => (d, 2u64, 0),
        TransportCommand::SeekToSample(d, frame) => (d, 3u64, frame.min(MAX_SEEK_FRAME)),
        TransportCommand::JumpToHotCue(d, cue) => (d, 4u64, u64::from(cue)),
        TransportCommand::SetTempoRatio(d, ratio) => (d, 5u64, u64::from(ratio.to_bits())),
        TransportCommand::SetGain(d, gain) => (d, 6u64, u64::from(gain.to_bits())),
    };
    let deck_bit = deck as u64;
    OCCUPIED_BIT | (opcode << OPCODE_SHIFT) | (deck_bit << DECK_SHIFT) | (payload & PAYLOAD_MASK)
}

fn unpack_command(word: u64) -> Option<TransportCommand> {
    if word == 0 {
        return None;
    }
    let deck = if ((word >> DECK_SHIFT) & 1) == 0 {
        DeckId::DeckA
    } else {
        DeckId::DeckB
    };
    let payload = word & PAYLOAD_MASK;
    let bits_f32 = f32::from_bits((payload & 0xFFFF_FFFF) as u32);
    match (word >> OPCODE_SHIFT) & 0xFF {
        1 => Some(TransportCommand::Play(deck)),
        2 => Some(TransportCommand::Pause(deck)),
        3 => Some(TransportCommand::SeekToSample(deck, payload)),
        4 => Some(TransportCommand::JumpToHotCue(deck, (payload & 0xFF) as u8)),
        5 => Some(TransportCommand::SetTempoRatio(deck, bits_f32)),
        6 => Some(TransportCommand::SetGain(deck, bits_f32)),
        _ => None,
    }
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
            mix_deck_into_output(deck, active_slice, frames);
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

#[inline]
fn accumulate_stereo(out: &mut [f32], out_frame: usize, l: f32, r: f32, gain: f32) {
    let base = out_frame * 2;
    if l.is_finite() {
        out[base] += l * gain;
    }
    if r.is_finite() {
        out[base + 1] += r * gain;
    }
}

fn mix_deck_into_output(deck: &mut DeckState, out_stereo: &mut [f32], frames: usize) {
    if !deck.playing {
        return;
    }
    let Some(buf) = deck.buffer.as_deref().filter(|b| b.total_frames() > 0) else {
        deck.playing = false;
        return;
    };
    let total_frames = buf.total_frames();
    let samples = buf.interleaved_stereo();
    let gain = deck.gain.clamp(0.0, 2.0);
    let ratio = deck.tempo_ratio.clamp(0.5, 2.0);

    if (ratio - 1.0).abs() <= f32::EPSILON && deck.playhead_frames.fract() == 0.0 {
        let mut idx = deck.playhead_frames.max(0.0) as u64;
        for out_frame in 0..frames {
            if idx >= total_frames {
                deck.playhead_frames = total_frames as f64;
                deck.playing = false;
                return;
            }
            let base = (idx as usize) * 2;
            let (l, r) = (samples[base], samples[base + 1]);
            accumulate_stereo(out_stereo, out_frame, l, r, gain);
            idx += 1;
        }
        deck.playhead_frames = idx as f64;
        deck.playing = idx < total_frames;
        return;
    }

    let step = f64::from(ratio);
    let total_f64 = total_frames as f64;
    for out_frame in 0..frames {
        let idx0 = if deck.playhead_frames.is_finite() && deck.playhead_frames < total_f64 {
            deck.playhead_frames.max(0.0).floor() as u64
        } else {
            total_frames
        };
        if idx0 >= total_frames {
            deck.playhead_frames = total_f64;
            deck.playing = false;
            return;
        }
        let frac = (deck.playhead_frames - (idx0 as f64)) as f32;
        let idx1 = (idx0 + 1).min(total_frames - 1);
        let (b0, b1) = ((idx0 as usize) * 2, (idx1 as usize) * 2);
        let l = samples[b0] + (samples[b1] - samples[b0]) * frac;
        let r = samples[b0 + 1] + (samples[b1 + 1] - samples[b0 + 1]) * frac;
        accumulate_stereo(out_stereo, out_frame, l, r, gain);
        deck.playhead_frames += step;
    }

    if deck.playhead_frames >= total_f64 {
        deck.playhead_frames = total_f64;
        deck.playing = false;
    }
}
