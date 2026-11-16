//! Time-coherence offset histogram matcher (`Wang 2003` Shazam alignment).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::constellation::LandmarkHash;

const HISTOGRAM_BIN_MS: i64 = 64;
const MIN_ALIGNED_LANDMARKS: usize = 6;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShazamMatch {
    pub track_id: String,
    pub aligned_landmarks: usize,
    pub confidence: f32,
    pub estimated_offset_ms: i32,
}

#[derive(Default, Clone, Copy)]
struct OffsetBinStats {
    count: usize,
    sum_delta_ms: i64,
}

/// Aligns query `LandmarkHash` entries against candidate `(hash_u64, track_id, db_time_offset_ms)`
/// index hits using a `64 ms` time-coherence histogram with adjacent-bin straddle coalescing.
pub fn match_landmarks_against_index(
    query: &[LandmarkHash],
    index_hits: &[(u64, String, i64)],
) -> Option<ShazamMatch> {
    if query.is_empty() || index_hits.is_empty() {
        return None;
    }

    let mut query_by_hash: HashMap<u64, Vec<u32>> = HashMap::with_capacity(query.len());
    for lm in query {
        if lm.hash_u64 == 0 {
            continue;
        }
        let times = query_by_hash.entry(lm.hash_u64).or_default();
        if !times.contains(&lm.anchor_time_ms) {
            times.push(lm.anchor_time_ms);
        }
    }

    // Per `track_id`, build histogram of `delta_bin = (db_offset_ms - query_anchor_ms) / 64`
    let mut per_track_bins: HashMap<&str, HashMap<i64, OffsetBinStats>> = HashMap::new();

    for (hash, track_id, db_time_ms) in index_hits {
        let Some(q_times) = query_by_hash.get(hash) else {
            continue;
        };
        let track_hist = per_track_bins.entry(track_id.as_str()).or_default();
        for &q_ms in q_times {
            let delta_ms = *db_time_ms - i64::from(q_ms);
            let bin = delta_ms.div_euclid(HISTOGRAM_BIN_MS);
            let entry = track_hist.entry(bin).or_default();
            entry.count += 1;
            entry.sum_delta_ms += delta_ms;
        }
    }

    let mut best_track: Option<(&str, usize, i32)> = None;
    let mut runner_up_count: usize = 0;

    // Sort track keys deterministically so ties never depend on HashMap seed
    let mut tracks: Vec<(&str, &HashMap<i64, OffsetBinStats>)> =
        per_track_bins.iter().map(|(&k, v)| (k, v)).collect();
    tracks.sort_by(|a, b| a.0.cmp(b.0));

    for (track_id, hist) in tracks {
        let mut track_best_count = 0_usize;
        let mut track_best_sum_ms = 0_i64;
        let mut track_best_bin = i64::MAX;

        for (&bin, stats) in hist {
            // Coalesce `(bin, bin + 1)` so an offset straddling a 64ms bin boundary never splits votes
            let next_stats = hist.get(&(bin + 1)).copied().unwrap_or_default();
            let combined_count = stats.count + next_stats.count;
            let combined_sum = stats.sum_delta_ms + next_stats.sum_delta_ms;

            if combined_count > track_best_count
                || (combined_count == track_best_count && bin < track_best_bin)
            {
                track_best_count = combined_count;
                track_best_sum_ms = combined_sum;
                track_best_bin = bin;
            }
        }

        if track_best_count == 0 {
            continue;
        }

        let est_offset_ms = ((track_best_sum_ms as f64) / (track_best_count as f64)).round() as i32;

        match best_track {
            None => {
                best_track = Some((track_id, track_best_count, est_offset_ms));
            }
            Some((_, curr_best, _)) if track_best_count > curr_best => {
                runner_up_count = curr_best;
                best_track = Some((track_id, track_best_count, est_offset_ms));
            }
            Some(_) => {
                runner_up_count = runner_up_count.max(track_best_count);
            }
        }
    }

    let (winner_id, aligned, estimated_offset_ms) = best_track?;
    if aligned < MIN_ALIGNED_LANDMARKS {
        return None;
    }

    // Require clear separation margin over runner-up track to eliminate false positives
    if runner_up_count > 0
        && (aligned < runner_up_count + 4 || (aligned as f32) < (runner_up_count as f32) * 1.75)
    {
        return None;
    }

    let norm_denom = ((query.len() as f32) * 0.25).max(MIN_ALIGNED_LANDMARKS as f32);
    let confidence = ((aligned as f32) / norm_denom).clamp(0.15, 1.0);

    Some(ShazamMatch {
        track_id: winner_id.to_string(),
        aligned_landmarks: aligned,
        confidence,
        estimated_offset_ms,
    })
}
