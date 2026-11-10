//! Lock-free Single-Producer Single-Consumer (SPSC) 64-bit packed `CommandQueue`.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

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

/// Lock-free consumer, mutex-serialized producer transport command ring buffer (`[AtomicU64; 64]`).
pub struct CommandQueue {
    slots: [AtomicU64; COMMAND_QUEUE_CAPACITY],
    write_idx: AtomicUsize,
    read_idx: AtomicUsize,
    push_lock: std::sync::Mutex<()>,
}

impl CommandQueue {
    /// Construct an empty 64-slot `CommandQueue`.
    pub const fn new() -> Self {
        Self {
            slots: [const { AtomicU64::new(0) }; COMMAND_QUEUE_CAPACITY],
            write_idx: AtomicUsize::new(0),
            read_idx: AtomicUsize::new(0),
            push_lock: std::sync::Mutex::new(()),
        }
    }

    /// Push a [`TransportCommand`]. Multiple producers are serialized without blocking the audio reader. Returns `false` if full.
    pub fn try_push(&self, cmd: TransportCommand) -> bool {
        let Ok(_guard) = self.push_lock.lock() else {
            return false;
        };
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
