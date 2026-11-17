//! Real-time audio-callback render worker owned exclusively by the `cpal` or headless thread.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use rtrb::{Consumer, Producer};

use super::dsp_chain::{AudioTelemetryFrame, MasterDspBus, RealTimeSpectrumAnalyzer};
use super::hub::SharedAudioAtomics;
use super::{DeckId, DualDeckEngine, PreallocatedDeckBuffer};

pub(crate) enum DeckLoadSwap {
    Reset {
        deck: DeckId,
        buffer: Arc<PreallocatedDeckBuffer>,
    },
    Prefix {
        deck: DeckId,
        buffer: Arc<PreallocatedDeckBuffer>,
        logical_total_frames: u64,
    },
    AuditionWindow {
        deck: DeckId,
        buffer: Arc<PreallocatedDeckBuffer>,
        stream_complete: bool,
    },
    SeamlessUpgrade {
        deck: DeckId,
        buffer: Arc<PreallocatedDeckBuffer>,
    },
}

/// Real-time render worker owned exclusively by the `cpal` callback or headless clock thread.
pub(crate) struct RealtimeRenderWorker {
    pub(crate) atomics: Arc<SharedAudioAtomics>,
    pub(crate) deck_load_rx: Consumer<DeckLoadSwap>,
    pub(crate) engine: DualDeckEngine,
    pub(crate) master_bus: MasterDspBus,
    pub(crate) analyzer: RealTimeSpectrumAnalyzer,
    pub(crate) telemetry_tx: Producer<AudioTelemetryFrame>,
    pub(crate) retired_buf_tx: Producer<Arc<PreallocatedDeckBuffer>>,
}

impl RealtimeRenderWorker {
    /// Lock-free real-time stereo block renderer (`DualDeckEngine` + `fundsp` + `realfft` + `rtrb`).
    pub(crate) fn render_stereo_block(&mut self, out_stereo: &mut [f32]) {
        let epoch_before = self.atomics.command_epoch.load(Ordering::Acquire);
        while let Ok(swap) = self.deck_load_rx.pop() {
            let old = match swap {
                DeckLoadSwap::Reset { deck, buffer } => self.engine.replace_deck(deck, buffer),
                DeckLoadSwap::Prefix {
                    deck,
                    buffer,
                    logical_total_frames,
                } => self
                    .engine
                    .replace_deck_prefix(deck, buffer, logical_total_frames),
                DeckLoadSwap::AuditionWindow {
                    deck,
                    buffer,
                    stream_complete,
                } => self
                    .engine
                    .upgrade_deck_audition_window(deck, buffer, stream_complete),
                DeckLoadSwap::SeamlessUpgrade { deck, buffer } => {
                    self.engine.upgrade_deck_seamless(deck, buffer)
                }
            };
            if let Some(retired) = old {
                let _ = self.retired_buf_tx.push(retired);
            }
        }
        let any_deck_started_or_playing = self.engine.is_deck_playing(DeckId::DeckA)
            || self.engine.is_deck_playing(DeckId::DeckB)
            || self.atomics.is_playing.load(Ordering::Acquire);
        self.engine
            .render_interleaved_stereo(&self.atomics.command_queue, out_stereo);
        let is_playing_a = self.engine.is_deck_playing(DeckId::DeckA);
        let is_buffering_a = self.engine.is_deck_buffering(DeckId::DeckA);
        let played_a = self.engine.deck_played_frames(DeckId::DeckA);
        let is_playing_any = is_playing_a || self.engine.is_deck_playing(DeckId::DeckB);
        let playhead = self.engine.deck_playhead_frame(DeckId::DeckA);
        let total = self.atomics.total_frames.load(Ordering::Acquire);
        let master_vol = f32::from_bits(self.atomics.master_volume_bits.load(Ordering::Acquire));

        if any_deck_started_or_playing || is_playing_any {
            self.master_bus
                .process_interleaved_in_place(out_stereo, master_vol);
            let sr_f64 = f64::from(self.atomics.sample_rate.max(1));
            let mut frame = self.analyzer.analyze_block(
                out_stereo,
                (playhead as f64) / sr_f64,
                (total as f64) / sr_f64,
                is_playing_any,
            );
            frame.is_buffering = is_buffering_a;
            let _ = self.telemetry_tx.push(frame);
        }

        if self.atomics.command_epoch.load(Ordering::Acquire) == epoch_before {
            self.atomics
                .playhead_frame
                .store(playhead, Ordering::Release);
            self.atomics
                .played_frames
                .store(played_a, Ordering::Release);
            self.atomics
                .is_playing
                .store(is_playing_a, Ordering::Release);
            self.atomics
                .is_buffering
                .store(is_buffering_a, Ordering::Release);
            if any_deck_started_or_playing && !is_playing_a && total > 0 && playhead >= total {
                self.atomics.ended.store(true, Ordering::Release);
            }
        }
    }
}
