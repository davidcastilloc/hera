//! Two-stage audition window (`65.0s`) and `>= 40.0s` played-audio commit gate for `AudioControllerHub`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::dsp_chain::resample_deck_buffer_fast_prefix;
use super::hub::AudioControllerHub;
use super::prefix_decode::{decode_audio_chunk, MAX_FULL_IN_MEMORY_DECODE_SECS};
use super::prefix_loader::{
    apply_cue_seconds, sanitize_cue_seconds, AUDITION_WINDOW_SECS, FULL_DECODE_COMMIT_PLAYED_SECS,
};
use super::{DeckId, PreallocatedDeckBuffer};

pub(crate) fn is_stale_generation(
    hub: &AudioControllerHub,
    expected_gen: u64,
    cancel: &AtomicBool,
) -> bool {
    cancel.load(Ordering::Acquire)
        || hub.atomics.load_generation.load(Ordering::Acquire) != expected_gen
}

/// Blocks the background deck loader until:
/// - `cancel` or a newer `load_generation` supersedes `expected_gen` -> returns `false`, or
/// - `force_full_decode` is set by an out-of-window seek/cue jump -> returns `true`, or
/// - cumulative rendered `played_frames >= threshold_frames` (`40.0s`) -> returns `true`.
pub(crate) fn wait_for_commit_or_cancel(
    hub: &AudioControllerHub,
    expected_gen: u64,
    cancel: &AtomicBool,
    threshold_frames: u64,
) -> bool {
    loop {
        if is_stale_generation(hub, expected_gen, cancel) {
            return false;
        }
        if hub.atomics.force_full_decode.load(Ordering::Acquire)
            || hub.atomics.played_frames.load(Ordering::Acquire) >= threshold_frames
        {
            return true;
        }
        thread::sleep(Duration::from_millis(8));
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_two_stage_deck_loader(
    hub: Arc<AudioControllerHub>,
    deck: DeckId,
    path_bg: PathBuf,
    target_sr: u32,
    logical_dur_sec: f64,
    logical_total_frames: u64,
    known_cues_sec: Option<[f64; 4]>,
    generation: u64,
    cancel: Arc<AtomicBool>,
) {
    let _ = thread::Builder::new()
        .name("hera-deck-loader".to_string())
        .spawn(move || {
            if is_stale_generation(&hub, generation, &cancel) {
                return;
            }

            // Stage 2A: Lightweight 65.0s linear-resampled audition window (zero rubato, zero rustfft).
            if let Ok(aud) = decode_audio_chunk(&path_bg, Some(AUDITION_WINDOW_SECS), Some(&cancel))
            {
                if !is_stale_generation(&hub, generation, &cancel) {
                    if let Ok(raw_aud) = PreallocatedDeckBuffer::from_stereo_frames(
                        aud.interleaved_stereo,
                        aud.sample_rate,
                        [0; 4],
                    ) {
                        if let Ok(mut resampled_aud) =
                            resample_deck_buffer_fast_prefix(&raw_aud, target_sr)
                        {
                            if !is_stale_generation(&hub, generation, &cancel) {
                                let aud_cues =
                                    known_cues_sec.map(sanitize_cue_seconds).unwrap_or_else(|| {
                                        [0.02, 0.25, 0.50, 0.80].map(|r| logical_dur_sec * r)
                                    });
                                let total_for_cues = if aud.eof_reached {
                                    resampled_aud.total_frames()
                                } else {
                                    logical_total_frames.max(resampled_aud.total_frames())
                                };
                                apply_cue_seconds(&mut resampled_aud, aud_cues, total_for_cues);
                                hub.upgrade_audition_buffer_if_generation(
                                    deck,
                                    generation,
                                    Arc::new(resampled_aud),
                                    aud.eof_reached,
                                    &cancel,
                                );
                                if aud.eof_reached {
                                    return;
                                }
                            }
                        }
                    }
                }
            }

            // Stage 2B: Wait until >= 40.0s of real audio has played OR an out-of-window seek occurs.
            let commit_frames =
                (FULL_DECODE_COMMIT_PLAYED_SECS * f64::from(target_sr)).round() as u64;
            if !wait_for_commit_or_cancel(&hub, generation, &cancel, commit_frames) {
                return;
            }

            // Stage 3: Full-track decode (<= 900s) + fast linear resample (zero rustfft, zero rubato).
            let Ok(full) = decode_audio_chunk(
                &path_bg,
                Some(MAX_FULL_IN_MEMORY_DECODE_SECS),
                Some(&cancel),
            ) else {
                return;
            };
            if is_stale_generation(&hub, generation, &cancel) {
                return;
            }
            let full_dur_sec =
                (full.interleaved_stereo.len() / 2) as f64 / f64::from(full.sample_rate.max(1));
            let raw_cues = known_cues_sec
                .map(sanitize_cue_seconds)
                .unwrap_or_else(|| [0.02, 0.25, 0.50, 0.80].map(|r| full_dur_sec * r));
            let Ok(raw_full) = PreallocatedDeckBuffer::from_stereo_frames(
                full.interleaved_stereo,
                full.sample_rate,
                [0; 4],
            ) else {
                return;
            };
            let Ok(mut resampled_full) = resample_deck_buffer_fast_prefix(&raw_full, target_sr)
            else {
                return;
            };
            if is_stale_generation(&hub, generation, &cancel) {
                return;
            }
            let full_frames = resampled_full.total_frames();
            apply_cue_seconds(&mut resampled_full, raw_cues, full_frames);
            hub.upgrade_buffer_if_generation(deck, generation, Arc::new(resampled_full), &cancel);
        });
}

impl AudioControllerHub {
    pub(crate) fn upgrade_audition_buffer_if_generation(
        &self,
        deck: DeckId,
        expected_gen: u64,
        audition_buf: Arc<PreallocatedDeckBuffer>,
        eof_reached: bool,
        cancel: &AtomicBool,
    ) -> bool {
        if is_stale_generation(self, expected_gen, cancel) {
            return false;
        }
        let mut meta_guard = (deck == DeckId::DeckA)
            .then(|| self.deck_a_meta.lock().ok())
            .flatten();
        if is_stale_generation(self, expected_gen, cancel) {
            return false;
        }
        if let Some(ref mut meta) = meta_guard {
            let n = audition_buf.total_frames();
            meta.buffer = Some(Arc::clone(&audition_buf));
            self.atomics.buffered_frames.store(n, Ordering::Release);
            if eof_reached {
                self.atomics.total_frames.store(n, Ordering::Release);
                self.atomics.stream_complete.store(true, Ordering::Release);
            }
            if self.atomics.playhead_frame.load(Ordering::Acquire) < n {
                self.atomics.is_buffering.store(false, Ordering::Release);
            }
        }
        if let Ok(mut tx) = self.deck_load_tx.lock() {
            let _ = tx.push(super::render_worker::DeckLoadSwap::AuditionWindow {
                deck,
                buffer: audition_buf,
                stream_complete: eof_reached,
            });
        }
        true
    }

    pub(crate) fn upgrade_buffer_if_generation(
        &self,
        deck: DeckId,
        expected_gen: u64,
        full_resampled: Arc<PreallocatedDeckBuffer>,
        cancel: &AtomicBool,
    ) -> bool {
        if is_stale_generation(self, expected_gen, cancel) {
            return false;
        }
        let mut meta_guard = (deck == DeckId::DeckA)
            .then(|| self.deck_a_meta.lock().ok())
            .flatten();
        if is_stale_generation(self, expected_gen, cancel) {
            return false;
        }
        if let Some(ref mut meta) = meta_guard {
            let n = full_resampled.total_frames();
            meta.buffer = Some(Arc::clone(&full_resampled));
            self.atomics.buffered_frames.store(n, Ordering::Release);
            self.atomics.total_frames.store(n, Ordering::Release);
            self.atomics.stream_complete.store(true, Ordering::Release);
            self.atomics.is_buffering.store(false, Ordering::Release);
        }
        if let Ok(mut tx) = self.deck_load_tx.lock() {
            let _ = tx.push(super::render_worker::DeckLoadSwap::SeamlessUpgrade {
                deck,
                buffer: full_resampled,
            });
        }
        true
    }
}
