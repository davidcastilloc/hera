//! Shared lock-free atomic controller hub & real-time render worker for `AudioBackend` adapters.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rtrb::{Consumer, Producer, RingBuffer};

use super::backend::{AudioBackendKind, AudioTransportSnapshot};
use super::dsp_chain::{
    create_telemetry_ring, resample_deck_buffer_to_rate, AudioTelemetryFrame, MasterDspBus,
    RealTimeSpectrumAnalyzer, TELEMETRY_RING_CAPACITY,
};
use super::{CommandQueue, DeckId, DualDeckEngine, PreallocatedDeckBuffer, TransportCommand};

#[derive(Default)]
struct LoadedDeckMeta {
    path: Option<String>,
    buffer: Option<Arc<PreallocatedDeckBuffer>>,
}

struct TelemetryReaderState {
    consumer: Consumer<AudioTelemetryFrame>,
    unread_latest: Option<AudioTelemetryFrame>,
}

pub(crate) struct SharedAudioAtomics {
    pub sample_rate: u32,
    pub command_queue: CommandQueue,
    pub command_epoch: AtomicU64,
    pub playhead_frame: AtomicU64,
    pub total_frames: AtomicU64,
    pub is_playing: AtomicBool,
    pub ended: AtomicBool,
    pub master_volume_bits: AtomicU32,
    pub tempo_ratio_bits: AtomicU32,
}

/// Non-realtime controller hub paired 1:1 with a [`RealtimeRenderWorker`] on the audio thread.
pub(crate) struct AudioControllerHub {
    kind: AudioBackendKind,
    pub(crate) atomics: Arc<SharedAudioAtomics>,
    deck_load_tx: Mutex<Producer<(DeckId, Arc<PreallocatedDeckBuffer>)>>,
    deck_a_meta: Mutex<LoadedDeckMeta>,
    telemetry_rx: Mutex<TelemetryReaderState>,
}

/// Real-time render worker owned exclusively by the `cpal` callback or headless clock thread.
pub(crate) struct RealtimeRenderWorker {
    atomics: Arc<SharedAudioAtomics>,
    deck_load_rx: Consumer<(DeckId, Arc<PreallocatedDeckBuffer>)>,
    engine: DualDeckEngine,
    master_bus: MasterDspBus,
    analyzer: RealTimeSpectrumAnalyzer,
    telemetry_tx: Producer<AudioTelemetryFrame>,
}

pub(crate) fn create_controller_and_worker(
    kind: AudioBackendKind,
    sample_rate: u32,
) -> (AudioControllerHub, RealtimeRenderWorker) {
    let sr = sample_rate.max(8_000);
    let (deck_load_tx, deck_load_rx) = RingBuffer::new(8);
    let (telemetry_tx, telemetry_rx) = create_telemetry_ring(TELEMETRY_RING_CAPACITY);
    let atomics = Arc::new(SharedAudioAtomics {
        sample_rate: sr,
        command_queue: CommandQueue::new(),
        command_epoch: AtomicU64::new(0),
        playhead_frame: AtomicU64::new(0),
        total_frames: AtomicU64::new(0),
        is_playing: AtomicBool::new(false),
        ended: AtomicBool::new(false),
        master_volume_bits: AtomicU32::new(1.0_f32.to_bits()),
        tempo_ratio_bits: AtomicU32::new(1.0_f32.to_bits()),
    });
    let hub = AudioControllerHub {
        kind,
        atomics: Arc::clone(&atomics),
        deck_load_tx: Mutex::new(deck_load_tx),
        deck_a_meta: Mutex::new(LoadedDeckMeta::default()),
        telemetry_rx: Mutex::new(TelemetryReaderState {
            consumer: telemetry_rx,
            unread_latest: None,
        }),
    };
    let worker = RealtimeRenderWorker {
        atomics,
        deck_load_rx,
        engine: DualDeckEngine::new(),
        master_bus: MasterDspBus::new(sr),
        analyzer: RealTimeSpectrumAnalyzer::new(sr),
        telemetry_tx,
    };
    (hub, worker)
}

impl AudioControllerHub {
    pub(crate) fn install_buffer(
        &self,
        deck: DeckId,
        raw: PreallocatedDeckBuffer,
        source_path: Option<String>,
    ) -> Result<AudioTransportSnapshot, String> {
        let resampled = Arc::new(resample_deck_buffer_to_rate(&raw, self.atomics.sample_rate)?);
        if deck == DeckId::DeckA {
            let total = resampled.total_frames();
            if let Ok(mut meta) = self.deck_a_meta.lock() {
                meta.path = source_path;
                meta.buffer = Some(Arc::clone(&resampled));
            }
            self.atomics.command_epoch.fetch_add(1, Ordering::AcqRel);
            self.atomics.total_frames.store(total, Ordering::Release);
            self.atomics.playhead_frame.store(0, Ordering::Release);
            self.atomics.is_playing.store(false, Ordering::Release);
            self.atomics.ended.store(false, Ordering::Release);
        }
        if let Ok(mut tx) = self.deck_load_tx.lock() {
            let _ = tx.push((deck, resampled));
        }
        Ok(self.snapshot())
    }

    pub(crate) fn send_command(&self, cmd: TransportCommand) -> AudioTransportSnapshot {
        self.atomics.command_epoch.fetch_add(1, Ordering::AcqRel);
        let total = self.atomics.total_frames.load(Ordering::Acquire);
        match cmd {
            TransportCommand::Play(DeckId::DeckA) if total > 0 => {
                if self.atomics.playhead_frame.load(Ordering::Acquire) >= total {
                    self.atomics.playhead_frame.store(0, Ordering::Release);
                    let _ = self
                        .atomics
                        .command_queue
                        .try_push(TransportCommand::SeekToSample(DeckId::DeckA, 0));
                }
                self.atomics.ended.store(false, Ordering::Release);
                self.atomics.is_playing.store(true, Ordering::Release);
            }
            TransportCommand::Pause(DeckId::DeckA) => {
                self.atomics.is_playing.store(false, Ordering::Release);
            }
            TransportCommand::SeekToSample(DeckId::DeckA, frame) => {
                let clamped = frame.min(total);
                self.atomics.playhead_frame.store(clamped, Ordering::Release);
                if clamped < total {
                    self.atomics.ended.store(false, Ordering::Release);
                }
            }
            TransportCommand::JumpToHotCue(DeckId::DeckA, cue) if (1..=4).contains(&cue) => {
                if let Ok(meta) = self.deck_a_meta.lock() {
                    if let Some(buf) = meta.buffer.as_deref() {
                        let target = buf.hot_cues[(cue - 1) as usize].frame_index.min(total);
                        self.atomics.playhead_frame.store(target, Ordering::Release);
                        self.atomics.ended.store(false, Ordering::Release);
                    }
                }
            }
            TransportCommand::SetTempoRatio(DeckId::DeckA, ratio) if ratio.is_finite() => {
                let clamped = ratio.clamp(0.5, 2.0);
                self.atomics
                    .tempo_ratio_bits
                    .store(clamped.to_bits(), Ordering::Release);
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

    pub(crate) fn snapshot(&self) -> AudioTransportSnapshot {
        self.drain_telemetry_into_slot();
        let sr = self.atomics.sample_rate;
        let sr_f64 = f64::from(sr.max(1));
        let total_frames = self.atomics.total_frames.load(Ordering::Acquire);
        let playhead_frame = self
            .atomics
            .playhead_frame
            .load(Ordering::Acquire)
            .min(total_frames);
        let (loaded_path, hot_cue_frames) = match self.deck_a_meta.lock() {
            Ok(meta) => {
                let cues = meta
                    .buffer
                    .as_deref()
                    .map(|b| b.hot_cues.map(|c| c.frame_index))
                    .unwrap_or([0; 4]);
                (meta.path.clone(), cues)
            }
            Err(_) => (None, [0; 4]),
        };
        AudioTransportSnapshot {
            backend_kind: self.kind.as_str().to_string(),
            sample_rate: sr,
            track_loaded: total_frames > 0,
            loaded_path,
            is_playing: self.atomics.is_playing.load(Ordering::Acquire),
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

impl RealtimeRenderWorker {
    /// Lock-free real-time stereo block renderer (`DualDeckEngine` + `fundsp` + `realfft` + `rtrb`).
    pub(crate) fn render_stereo_block(&mut self, out_stereo: &mut [f32]) {
        let epoch_before = self.atomics.command_epoch.load(Ordering::Acquire);
        while let Ok((deck, buf)) = self.deck_load_rx.pop() {
            self.engine.load_deck(deck, buf);
        }
        let started_or_was_playing = self.engine.is_deck_playing(DeckId::DeckA)
            || self.atomics.is_playing.load(Ordering::Acquire);
        self.engine
            .render_interleaved_stereo(&self.atomics.command_queue, out_stereo);
        let is_playing = self.engine.is_deck_playing(DeckId::DeckA);
        let playhead = self.engine.deck_playhead_frame(DeckId::DeckA);
        let total = self.atomics.total_frames.load(Ordering::Acquire);
        let master_vol = f32::from_bits(self.atomics.master_volume_bits.load(Ordering::Acquire));

        if started_or_was_playing || is_playing {
            self.master_bus
                .process_interleaved_in_place(out_stereo, master_vol);
            let sr_f64 = f64::from(self.atomics.sample_rate.max(1));
            let frame = self.analyzer.analyze_block(
                out_stereo,
                (playhead as f64) / sr_f64,
                (total as f64) / sr_f64,
                is_playing,
            );
            let _ = self.telemetry_tx.push(frame);
        }

        if self.atomics.command_epoch.load(Ordering::Acquire) == epoch_before {
            self.atomics.playhead_frame.store(playhead, Ordering::Release);
            self.atomics.is_playing.store(is_playing, Ordering::Release);
            if started_or_was_playing && !is_playing && total > 0 && playhead >= total {
                self.atomics.ended.store(true, Ordering::Release);
            }
        }
    }
}
