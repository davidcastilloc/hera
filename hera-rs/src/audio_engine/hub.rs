//! Shared lock-free atomic controller hub & real-time render worker for `AudioBackend` adapters.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rtrb::{Consumer, Producer, RingBuffer};

use super::backend::{AudioBackendKind, AudioTransportSnapshot};
use super::buffer_gc::{create_retired_buffer_channel, drain_retired_buffers};
use super::dsp_chain::{
    create_telemetry_ring, resample_deck_buffer_to_rate, AudioTelemetryFrame, MasterDspBus,
    RealTimeSpectrumAnalyzer, TELEMETRY_RING_CAPACITY,
};
pub(crate) use super::render_worker::{DeckLoadSwap, RealtimeRenderWorker};
use super::{CommandQueue, DeckId, DualDeckEngine, PreallocatedDeckBuffer, TransportCommand};

#[derive(Default)]
pub(super) struct LoadedDeckMeta {
    pub(super) path: Option<String>,
    pub(super) buffer: Option<Arc<PreallocatedDeckBuffer>>,
}

struct TelemetryReaderState {
    consumer: Consumer<AudioTelemetryFrame>,
    unread_latest: Option<AudioTelemetryFrame>,
}

pub(crate) struct SharedAudioAtomics {
    pub sample_rate: u32,
    pub command_queue: CommandQueue,
    pub command_epoch: AtomicU64,
    pub load_generation: AtomicU64,
    pub stream_complete: AtomicBool,
    pub buffered_frames: AtomicU64,
    pub played_frames: AtomicU64,
    pub playhead_frame: AtomicU64,
    pub total_frames: AtomicU64,
    pub is_playing: AtomicBool,
    pub is_buffering: AtomicBool,
    pub force_full_decode: AtomicBool,
    pub ended: AtomicBool,
    pub master_volume_bits: AtomicU32,
    pub tempo_ratio_bits: AtomicU32,
}

/// Non-realtime controller hub paired 1:1 with a [`RealtimeRenderWorker`] on the audio thread.
pub(crate) struct AudioControllerHub {
    kind: AudioBackendKind,
    pub(crate) atomics: Arc<SharedAudioAtomics>,
    pub(super) deck_load_tx: Mutex<Producer<DeckLoadSwap>>,
    pub(super) deck_a_meta: Mutex<LoadedDeckMeta>,
    active_cancel: Mutex<Option<Arc<AtomicBool>>>,
    telemetry_rx: Mutex<TelemetryReaderState>,
    retired_buf_rx: Mutex<Consumer<Arc<PreallocatedDeckBuffer>>>,
}

pub(crate) fn create_controller_and_worker(
    kind: AudioBackendKind,
    sample_rate: u32,
) -> (AudioControllerHub, RealtimeRenderWorker) {
    let sr = sample_rate.max(8_000);
    let (deck_load_tx, deck_load_rx) = RingBuffer::new(16);
    let (telemetry_tx, telemetry_rx) = create_telemetry_ring(TELEMETRY_RING_CAPACITY);
    let (retired_buf_tx, retired_buf_rx) = create_retired_buffer_channel();
    let atomics = Arc::new(SharedAudioAtomics {
        sample_rate: sr,
        command_queue: CommandQueue::new(),
        command_epoch: AtomicU64::new(0),
        load_generation: AtomicU64::new(0),
        stream_complete: AtomicBool::new(true),
        buffered_frames: AtomicU64::new(0),
        played_frames: AtomicU64::new(0),
        playhead_frame: AtomicU64::new(0),
        total_frames: AtomicU64::new(0),
        is_playing: AtomicBool::new(false),
        is_buffering: AtomicBool::new(false),
        force_full_decode: AtomicBool::new(false),
        ended: AtomicBool::new(false),
        master_volume_bits: AtomicU32::new(1.0_f32.to_bits()),
        tempo_ratio_bits: AtomicU32::new(1.0_f32.to_bits()),
    });
    let (deck_a_meta, active_cancel) = (Mutex::new(LoadedDeckMeta::default()), Mutex::new(None));
    let hub = AudioControllerHub {
        kind,
        atomics: Arc::clone(&atomics),
        deck_load_tx: Mutex::new(deck_load_tx),
        deck_a_meta,
        active_cancel,
        telemetry_rx: Mutex::new(TelemetryReaderState {
            consumer: telemetry_rx,
            unread_latest: None,
        }),
        retired_buf_rx,
    };
    let worker = RealtimeRenderWorker {
        atomics,
        deck_load_rx,
        engine: DualDeckEngine::new(),
        master_bus: MasterDspBus::new(sr),
        analyzer: RealTimeSpectrumAnalyzer::new(sr),
        telemetry_tx,
        retired_buf_tx,
    };
    (hub, worker)
}

impl AudioControllerHub {
    fn swap_active_cancel(&self, next: Option<Arc<AtomicBool>>) {
        if let Ok(mut slot) = self.active_cancel.lock() {
            if let Some(prev) = slot.take() {
                prev.store(true, Ordering::Release);
            }
            *slot = next;
        }
    }

    fn commit_load(
        &self,
        deck: DeckId,
        buffer: Arc<PreallocatedDeckBuffer>,
        logical_total_frames: Option<u64>,
        source_path: Option<String>,
        cancel: Option<Arc<AtomicBool>>,
    ) -> (AudioTransportSnapshot, u64) {
        self.swap_active_cancel(cancel);
        let complete = logical_total_frames.is_none();
        let buf_frames = buffer.total_frames();
        let total = logical_total_frames.unwrap_or(buf_frames).max(buf_frames);
        let generation = {
            let mut meta_guard = (deck == DeckId::DeckA)
                .then(|| self.deck_a_meta.lock().ok())
                .flatten();
            let next_gen = self.atomics.load_generation.fetch_add(1, Ordering::AcqRel) + 1;
            if let Some(ref mut meta) = meta_guard {
                meta.path = source_path;
                meta.buffer = Some(Arc::clone(&buffer));
            }
            if deck == DeckId::DeckA {
                let a = &self.atomics;
                a.command_epoch.fetch_add(1, Ordering::AcqRel);
                a.stream_complete.store(complete, Ordering::Release);
                a.buffered_frames.store(buf_frames, Ordering::Release);
                a.played_frames.store(0, Ordering::Release);
                a.total_frames.store(total, Ordering::Release);
                a.playhead_frame.store(0, Ordering::Release);
                a.is_playing.store(false, Ordering::Release);
                a.is_buffering.store(false, Ordering::Release);
                a.force_full_decode.store(false, Ordering::Release);
                a.ended.store(false, Ordering::Release);
            }
            let swap = if logical_total_frames.is_some() {
                DeckLoadSwap::Prefix {
                    deck,
                    buffer,
                    logical_total_frames: total,
                }
            } else {
                DeckLoadSwap::Reset { deck, buffer }
            };
            if let Ok(mut tx) = self.deck_load_tx.lock() {
                let _ = tx.push(swap);
            }
            next_gen
        };
        (self.snapshot(), generation)
    }

    pub(crate) fn install_buffer(
        &self,
        deck: DeckId,
        raw: PreallocatedDeckBuffer,
        source_path: Option<String>,
    ) -> Result<AudioTransportSnapshot, String> {
        let resampled = Arc::new(resample_deck_buffer_to_rate(
            &raw,
            self.atomics.sample_rate,
        )?);
        Ok(self.install_resampled_buffer(deck, resampled, source_path))
    }

    pub(crate) fn install_resampled_buffer(
        &self,
        deck: DeckId,
        resampled: Arc<PreallocatedDeckBuffer>,
        source_path: Option<String>,
    ) -> AudioTransportSnapshot {
        self.commit_load(deck, resampled, None, source_path, None).0
    }

    pub(crate) fn install_prefix_buffer(
        &self,
        deck: DeckId,
        prefix: Arc<PreallocatedDeckBuffer>,
        logical_total_frames: u64,
        source_path: Option<String>,
        cancel: Arc<AtomicBool>,
    ) -> (AudioTransportSnapshot, u64) {
        self.commit_load(
            deck,
            prefix,
            Some(logical_total_frames),
            source_path,
            Some(cancel),
        )
    }

    pub(crate) fn send_command(&self, cmd: TransportCommand) -> AudioTransportSnapshot {
        self.atomics.command_epoch.fetch_add(1, Ordering::AcqRel);
        let total = self.atomics.total_frames.load(Ordering::Acquire);
        let buffered = self.atomics.buffered_frames.load(Ordering::Acquire);
        let complete = self.atomics.stream_complete.load(Ordering::Acquire);
        let mark_seek_target = |target: u64| {
            let c = target.min(total);
            self.atomics.playhead_frame.store(c, Ordering::Release);
            let need_buf = !complete && c >= buffered;
            if need_buf {
                self.atomics
                    .force_full_decode
                    .store(true, Ordering::Release);
            }
            let buf_now = need_buf && self.atomics.is_playing.load(Ordering::Acquire);
            self.atomics.is_buffering.store(buf_now, Ordering::Release);
            if c < total {
                self.atomics.ended.store(false, Ordering::Release);
            }
        };
        match cmd {
            TransportCommand::Play(DeckId::DeckA) if total > 0 => {
                if self.atomics.playhead_frame.load(Ordering::Acquire) >= total {
                    self.atomics.playhead_frame.store(0, Ordering::Release);
                    let reset = TransportCommand::SeekToSample(DeckId::DeckA, 0);
                    let _ = self.atomics.command_queue.try_push(reset);
                }
                self.atomics.ended.store(false, Ordering::Release);
                self.atomics.is_playing.store(true, Ordering::Release);
            }
            TransportCommand::Pause(DeckId::DeckA) => {
                self.atomics.is_playing.store(false, Ordering::Release);
                self.atomics.is_buffering.store(false, Ordering::Release);
            }
            TransportCommand::SeekToSample(DeckId::DeckA, frame) => mark_seek_target(frame),
            TransportCommand::JumpToHotCue(DeckId::DeckA, cue) if (1..=4).contains(&cue) => {
                let buf = self.deck_a_meta.lock().ok().and_then(|m| m.buffer.clone());
                if let Some(b) = buf {
                    mark_seek_target(b.hot_cues[(cue - 1) as usize].frame_index);
                }
            }
            TransportCommand::SetTempoRatio(DeckId::DeckA, ratio) if ratio.is_finite() => {
                let bits = ratio.clamp(0.5, 2.0).to_bits();
                self.atomics.tempo_ratio_bits.store(bits, Ordering::Release);
            }
            _ => {}
        }
        let _ = self.atomics.command_queue.try_push(cmd);
        self.snapshot()
    }

    pub(crate) fn set_master_volume(&self, volume: f32) -> AudioTransportSnapshot {
        let clamped = if volume.is_finite() {
            volume.clamp(0.0, 2.0)
        } else {
            0.0
        };
        self.atomics
            .master_volume_bits
            .store(clamped.to_bits(), Ordering::Release);
        self.snapshot()
    }

    #[cfg(test)]
    pub(crate) fn deck_buffer(&self, deck: DeckId) -> Option<Arc<PreallocatedDeckBuffer>> {
        (deck == DeckId::DeckA)
            .then(|| self.deck_a_meta.lock().ok()?.buffer.clone())
            .flatten()
    }

    pub(crate) fn drain_retired_buffers(&self) -> usize {
        drain_retired_buffers(&self.retired_buf_rx)
    }

    pub(crate) fn snapshot(&self) -> AudioTransportSnapshot {
        let _ = self.drain_retired_buffers();
        self.drain_telemetry_into_slot();
        let a = &self.atomics;
        let sr = a.sample_rate;
        let sr_f64 = f64::from(sr.max(1));
        let total_frames = a.total_frames.load(Ordering::Acquire);
        let playhead_frame = a.playhead_frame.load(Ordering::Acquire).min(total_frames);
        let (loaded_path, hot_cue_frames) = self
            .deck_a_meta
            .lock()
            .ok()
            .map(|m| {
                let cues = m
                    .buffer
                    .as_deref()
                    .map_or([0; 4], |b| b.hot_cues.map(|c| c.frame_index));
                (m.path.clone(), cues)
            })
            .unwrap_or((None, [0; 4]));
        AudioTransportSnapshot {
            backend_kind: self.kind.as_str().to_string(),
            sample_rate: sr,
            track_loaded: total_frames > 0,
            loaded_path,
            is_playing: self.atomics.is_playing.load(Ordering::Acquire),
            is_buffering: self.atomics.is_buffering.load(Ordering::Acquire),
            ended: self.atomics.ended.load(Ordering::Acquire),
            current_time_sec: (playhead_frame as f64) / sr_f64,
            duration_sec: (total_frames as f64) / sr_f64,
            playhead_frame,
            total_frames,
            volume: f32::from_bits(self.atomics.master_volume_bits.load(Ordering::Acquire)),
            tempo_ratio: f32::from_bits(self.atomics.tempo_ratio_bits.load(Ordering::Acquire)),
            hot_cue_frames,
            hot_cue_seconds: hot_cue_frames.map(|f| (f as f64) / sr_f64),
        }
    }

    pub(crate) fn poll_latest_telemetry(&self) -> Option<AudioTelemetryFrame> {
        let _ = self.drain_retired_buffers();
        self.drain_telemetry_into_slot();
        self.telemetry_rx
            .lock()
            .ok()
            .and_then(|mut rx| rx.unread_latest.take())
    }

    fn drain_telemetry_into_slot(&self) {
        if let Ok(mut rx) = self.telemetry_rx.lock() {
            while let Ok(frame) = rx.consumer.pop() {
                rx.unread_latest = Some(frame);
            }
        }
    }
}
