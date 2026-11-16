//! Prefix-first (`< 50 ms`) + seamless background upgrade loader for `AudioControllerHub`.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::structure::analyze_structure_and_style;

use super::backend::AudioTransportSnapshot;
use super::dsp_chain::{resample_deck_buffer_fast_prefix, resample_deck_buffer_to_rate};
use super::hub::AudioControllerHub;
#[cfg(test)]
pub(crate) use super::prefix_decode::clamp_reserve_frames;
use super::prefix_decode::decode_audio_chunk;
pub(crate) use super::prefix_decode::MAX_FULL_IN_MEMORY_DECODE_SECS;
use super::{DeckId, PreallocatedDeckBuffer};

pub(crate) const PREFIX_DECODE_SECS: f64 = 2.5;

/// Immediately decodes up to 2.5s of audio (`< 50 ms`) so deck playback and Hot Cue seek start
/// without blocking on full-track decoding/resampling; spawns a background worker for long tracks
/// that upgrades the playing deck buffer seamlessly (capped at `15 min` to prevent OOM on mixes).
pub(crate) fn load_file_prefix_then_full(
    hub: &Arc<AudioControllerHub>,
    deck: DeckId,
    path: &Path,
    duration_hint_sec: Option<f64>,
    known_cues_sec: Option<[f64; 4]>,
) -> Result<AudioTransportSnapshot, String> {
    let prefix = decode_audio_chunk(path, Some(PREFIX_DECODE_SECS), None)?;
    let target_sr = hub.atomics.sample_rate.max(8_000);
    let decoded_sec =
        (prefix.interleaved_stereo.len() / 2) as f64 / f64::from(prefix.sample_rate.max(1));
    let logical_dur_sec = if prefix.eof_reached {
        decoded_sec
    } else {
        prefix
            .container_duration_sec
            .filter(|d| d.is_finite() && *d > decoded_sec)
            .or_else(|| duration_hint_sec.filter(|d| d.is_finite() && *d > decoded_sec))
            .unwrap_or(decoded_sec)
    };

    let prefix_cues_sec = if let Some(cues) = known_cues_sec {
        sanitize_cue_seconds(cues)
    } else if prefix.eof_reached {
        compute_cues_seconds(&prefix.interleaved_stereo, prefix.sample_rate)
    } else {
        [0.02, 0.25, 0.50, 0.80].map(|ratio| logical_dur_sec * ratio)
    };

    let raw_prefix = PreallocatedDeckBuffer::from_stereo_frames(
        prefix.interleaved_stereo,
        prefix.sample_rate,
        [0; 4],
    )?;
    let mut resampled_prefix = resample_deck_buffer_fast_prefix(&raw_prefix, target_sr)?;
    let logical_total_frames = ((logical_dur_sec * f64::from(target_sr)).round() as u64)
        .max(resampled_prefix.total_frames());
    apply_cue_seconds(&mut resampled_prefix, prefix_cues_sec, logical_total_frames);

    let source_path = Some(path.to_string_lossy().to_string());
    if prefix.eof_reached {
        return Ok(hub.install_resampled_buffer(deck, Arc::new(resampled_prefix), source_path));
    }

    let cancel = Arc::new(AtomicBool::new(false));
    let (snap, generation) = hub.install_prefix_buffer(
        deck,
        Arc::new(resampled_prefix),
        logical_total_frames,
        source_path,
        Arc::clone(&cancel),
    );

    let hub_bg = Arc::clone(hub);
    let path_bg = path.to_path_buf();
    let _ = thread::Builder::new()
        .name("hera-deck-full-decode".to_string())
        .spawn(move || {
            if cancel.load(Ordering::Acquire) {
                return;
            }
            let Ok(full) = decode_audio_chunk(
                &path_bg,
                Some(MAX_FULL_IN_MEMORY_DECODE_SECS),
                Some(&cancel),
            ) else {
                return;
            };
            if cancel.load(Ordering::Acquire)
                || hub_bg.atomics.load_generation.load(Ordering::Acquire) != generation
            {
                return;
            }
            let cue_secs = match known_cues_sec {
                Some(cues) => sanitize_cue_seconds(cues),
                None => compute_cues_seconds(&full.interleaved_stereo, full.sample_rate),
            };
            let Ok(raw_full) = PreallocatedDeckBuffer::from_stereo_frames(
                full.interleaved_stereo,
                full.sample_rate,
                [0; 4],
            ) else {
                return;
            };
            let Ok(mut resampled_full) = resample_deck_buffer_to_rate(&raw_full, target_sr) else {
                return;
            };
            if cancel.load(Ordering::Acquire)
                || hub_bg.atomics.load_generation.load(Ordering::Acquire) != generation
            {
                return;
            }
            let full_frames = resampled_full.total_frames();
            apply_cue_seconds(&mut resampled_full, cue_secs, full_frames);
            hub_bg.upgrade_buffer_if_generation(
                deck,
                generation,
                Arc::new(resampled_full),
                &cancel,
            );
        });

    Ok(snap)
}

fn sanitize_cue_seconds(cues: [f64; 4]) -> [f64; 4] {
    cues.map(|s| if s.is_finite() { s.max(0.0) } else { 0.0 })
}

fn apply_cue_seconds(
    buf: &mut PreallocatedDeckBuffer,
    cue_secs: [f64; 4],
    logical_total_frames: u64,
) {
    let max_frame = logical_total_frames
        .max(buf.total_frames())
        .saturating_sub(1);
    let sr = f64::from(buf.sample_rate.max(1));
    for (slot, sec) in buf.hot_cues.iter_mut().zip(cue_secs.iter()) {
        slot.frame_index = ((*sec).max(0.0) * sr).round().min(max_frame as f64) as u64;
    }
}

fn compute_cues_seconds(interleaved_stereo: &[f32], sample_rate: u32) -> [f64; 4] {
    let total_frames = interleaved_stereo.len() / 2;
    if total_frames < 4 {
        return [0.0; 4];
    }
    let mono: Vec<f32> = (0..total_frames)
        .map(|f| 0.5 * (interleaved_stereo[f * 2] + interleaved_stereo[f * 2 + 1]))
        .collect();
    let full_dur_sec = (total_frames as f32) / (sample_rate.max(1) as f32);
    let profile = analyze_structure_and_style(&mono, sample_rate.max(1), Some(full_dur_sec), None);
    [
        f64::from(profile.first_beat_seconds.max(0.0)),
        f64::from(profile.phrase_in_seconds.max(0.0)),
        f64::from(profile.main_drop_seconds.max(0.0)),
        f64::from(profile.mix_out_seconds.max(0.0)),
    ]
}
