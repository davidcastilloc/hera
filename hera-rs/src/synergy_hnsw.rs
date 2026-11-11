use crate::synergy::{parse_camelot, CrateTrackCandidate, SetSpineConfig};
use crate::synergy_hnsw_cache::query_or_build_cached_hnsw;
use std::collections::HashSet;

/// Minimum candidate pool size before two-stage HNSW approximate nearest neighbor
/// pre-filtering activates ahead of the $O(N^2)$ 2-Opt TSP solver.
/// Crates with `<= 64` candidates (including the 6-track Golden Master snapshot)
/// bypass pre-filtering and run exact full-matrix 2-Opt TSP untouched.
pub const HNSW_PREFILTER_THRESHOLD: usize = 64;

/// Default maximum candidate pool retained by the HNSW stage when `optimize_crate_2opt`
/// is invoked on a large library (`> HNSW_PREFILTER_THRESHOLD`).
pub const HNSW_DEFAULT_STAGE1_POOL: usize = 64;

/// Produces a deterministic L2-normalized 64-D vector for a `CrateTrackCandidate`.
/// If `track.embedding_64d` has 64 non-uniform dimensions, it is L2-normalized and
/// enriched with circular Camelot/BPM/energy/punch harmonic coordinates; otherwise a
/// deterministic 64-D acoustic & harmonic embedding is synthesized from the track's metadata.
pub fn synthesize_or_normalize_64d_embedding(track: &CrateTrackCandidate) -> [f32; 64] {
    let mut vec64 = [0.0_f32; 64];

    let has_distinct_embedding = if track.embedding_64d.len() == 64 {
        let first = track.embedding_64d[0];
        track
            .embedding_64d
            .iter()
            .any(|&v| (v - first).abs() > 1e-5)
    } else {
        false
    };

    let (cam_num, _cam_letter) = parse_camelot(&track.camelot_key).unwrap_or((8, 'A'));
    let angle = ((cam_num - 1) as f32) * (std::f32::consts::TAU / 12.0);
    let bpm_norm = ((track.bpm - 90.0) / 80.0).clamp(0.0, 1.5);
    let energy = track.energy_score.clamp(0.0, 1.0);

    if has_distinct_embedding {
        for (dst, &src) in vec64.iter_mut().zip(track.embedding_64d.iter()) {
            *dst = if src.is_finite() { src } else { 0.0 };
        }
        l2_normalize_64d(&mut vec64);

        // Blend circular Camelot, BPM, and energy coordinates into dims 0..4
        let basis = [angle.cos(), angle.sin(), bpm_norm, energy];
        for i in 0..4 {
            vec64[i] = vec64[i] * 0.5 + basis[i] * 0.5;
        }
    } else {
        let (_, cam_letter) = parse_camelot(&track.camelot_key).unwrap_or((8, 'A'));
        let mode_sign = if cam_letter == 'B' { 1.0_f32 } else { -1.0_f32 };
        let punch = track.punch_index.clamp(0.0, 1.0);

        // Harmonic circle & tempo/energy basis in dimensions 0..8
        vec64[0] = angle.cos();
        vec64[1] = angle.sin();
        vec64[2] = mode_sign * 0.5;
        vec64[3] = bpm_norm;
        vec64[4] = energy;
        vec64[5] = punch;
        vec64[6] = if track.has_intro_vocals { 0.4 } else { -0.4 };
        vec64[7] = if track.has_outro_vocals { 0.4 } else { -0.4 };

        // Deterministic spectral-style Fourier harmonics across dimensions 8..64
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in track.top_subgenre.bytes().chain(track.groove_type.bytes()) {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        for (i, v) in vec64.iter_mut().enumerate().skip(8) {
            let k = (i - 7) as f32;
            let phase = ((hash.rotate_left((i % 63) as u32) & 0xFFFF) as f32) / 65535.0;
            *v = (angle * (1.0 + (k % 3.0)) + phase * std::f32::consts::PI).cos() * 0.35
                + (bpm_norm * k * 0.25).sin() * 0.25
                + energy * 0.20
                + punch * 0.15;
        }
    }

    l2_normalize_64d(&mut vec64);
    vec64
}

fn l2_normalize_64d(v: &mut [f32; 64]) {
    let norm_sq: f32 = v.iter().map(|x| x * x).sum();
    let norm = norm_sq.sqrt();
    if norm > 1e-8 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    } else {
        v[0] = 1.0;
    }
}

/// Stage 1 HNSW Candidate Retrieval without spine awareness (delegates to multi-probe centroid).
pub fn prefilter_candidates_hnsw(
    candidates: &[CrateTrackCandidate],
    max_candidates: usize,
) -> Vec<CrateTrackCandidate> {
    prefilter_candidates_hnsw_for_spine(candidates, max_candidates, &SetSpineConfig::default())
}

/// Stage 1 HNSW Multi-Probe Candidate Retrieval with Set Spine Anchor Preservation.
///
/// - When `candidates.len() <= max_candidates` or `candidates.len() <= HNSW_PREFILTER_THRESHOLD`,
///   returns `candidates.to_vec()` verbatim so small crates and Golden Master snapshots
///   preserve 100% bit-for-bit parity.
/// - When `candidates.len() > max_candidates` and `candidates.len() > HNSW_PREFILTER_THRESHOLD`,
///   enriches embeddings, queries 3 diverse probes (opener 15th-pct BPM, bridge 50th-pct BPM, peak 85th-pct BPM),
///   and unconditionally retains every `SpineAnchor.track_id` in `spine.anchors`.
pub fn prefilter_candidates_hnsw_for_spine(
    candidates: &[CrateTrackCandidate],
    max_candidates: usize,
    spine: &SetSpineConfig,
) -> Vec<CrateTrackCandidate> {
    let cap = max_candidates.max(1);
    if candidates.len() <= cap || candidates.len() <= HNSW_PREFILTER_THRESHOLD {
        return candidates.to_vec();
    }

    let embeddings: Vec<[f32; 64]> = candidates
        .iter()
        .map(synthesize_or_normalize_64d_embedding)
        .collect();

    // Collect all locked spine anchor indices to unconditionally retain them
    let mut anchor_indices = HashSet::new();
    let mut final_indices: Vec<usize> = Vec::with_capacity(cap);
    for anchor in &spine.anchors {
        if let Some(pos) = candidates
            .iter()
            .position(|c| c.track_id == anchor.track_id)
        {
            if anchor_indices.insert(pos) {
                final_indices.push(pos);
            }
        }
    }

    // Determine 15th, 50th, and 85th percentile BPM indices for 3 diverse probes
    let mut bpm_sorted: Vec<(usize, f32)> = candidates
        .iter()
        .enumerate()
        .map(|(i, c)| (i, c.bpm))
        .collect();
    bpm_sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let n = bpm_sorted.len();
    let idx_opener = bpm_sorted[(n * 15 / 100).min(n - 1)].0;
    let idx_bridge = bpm_sorted[(n * 50 / 100).min(n - 1)].0;
    let idx_peak = bpm_sorted[(n * 85 / 100).min(n - 1)].0;

    let probe_opener = embeddings[idx_opener];
    let probe_bridge = embeddings[idx_bridge];
    let probe_peak = embeddings[idx_peak];
    let probes = [probe_opener, probe_bridge, probe_peak];

    let per_probe_k = (cap / probes.len()).max(4) + 4;
    let ranked_indices =
        query_or_build_cached_hnsw(&embeddings, &probes, per_probe_k).unwrap_or_default();

    // Fill remaining slots up to cap from interleaved nearest-neighbor probe order
    for idx in ranked_indices {
        if final_indices.len() >= cap {
            break;
        }
        if !anchor_indices.contains(&idx) {
            anchor_indices.insert(idx);
            final_indices.push(idx);
        }
    }

    // If still below cap and library has more tracks, fill remaining
    if final_indices.len() < cap {
        for i in 0..candidates.len() {
            if final_indices.len() >= cap {
                break;
            }
            if !anchor_indices.contains(&i) {
                anchor_indices.insert(i);
                final_indices.push(i);
            }
        }
    }

    final_indices.sort_unstable();
    final_indices
        .into_iter()
        .map(|idx| candidates[idx].clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synergy::optimize_crate_2opt;

    fn build_synthetic_library(count: usize) -> Vec<CrateTrackCandidate> {
        let keys = ["8A", "9A", "10A", "8B", "9B", "11A", "7A", "12A"];
        let subgenres = ["Tech House", "Afro House", "Melodic Techno", "Deep House"];
        (0..count)
            .map(|i| {
                let bpm = 120.0 + ((i % 12) as f32) * 0.8;
                let energy = (0.45 + ((i % 20) as f32) * 0.025).clamp(0.40, 0.98);
                let mut track = CrateTrackCandidate {
                    track_id: format!("hnsw_t_{i:04}"),
                    title: format!("HNSW Club Track {i}"),
                    artist: format!("DJ Node {}", i % 16),
                    file_path: format!("/library/hnsw_{i:04}.flac"),
                    duration_seconds: 210.0,
                    bpm,
                    camelot_key: keys[i % keys.len()].to_string(),
                    energy_score: energy,
                    punch_index: (energy * 0.92).clamp(0.35, 0.95),
                    groove_type: "four_on_floor".to_string(),
                    top_subgenre: subgenres[i % subgenres.len()].to_string(),
                    has_intro_vocals: i % 7 == 0,
                    has_outro_vocals: i % 11 == 0,
                    embedding_64d: vec![],
                    hot_cues: vec![],
                };
                track.embedding_64d = synthesize_or_normalize_64d_embedding(&track).to_vec();
                track
            })
            .collect()
    }

    #[test]
    fn test_usearch_hnsw_200_tracks_prefilter_and_2opt_under_100ms() {
        let library = build_synthetic_library(200);
        let start = std::time::Instant::now();
        let prefiltered = prefilter_candidates_hnsw(&library, 48);
        assert_eq!(prefiltered.len(), 48);

        let crate_res =
            optimize_crate_2opt("HNSW 200 Scale Set", "steady_climb", library, Some(12));
        let elapsed = start.elapsed();

        assert_eq!(crate_res.total_tracks, 12);
        assert!(crate_res.mean_synergy_score >= 0.75);
        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "Expected 200-track HNSW + 2-Opt in < 100ms, took {elapsed:?}"
        );
    }
}
