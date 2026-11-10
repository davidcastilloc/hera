use crate::synergy::{parse_camelot, CrateTrackCandidate};

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

    if has_distinct_embedding {
        for (dst, &src) in vec64.iter_mut().zip(track.embedding_64d.iter()) {
            *dst = if src.is_finite() { src } else { 0.0 };
        }
    } else {
        let (cam_num, cam_letter) = parse_camelot(&track.camelot_key).unwrap_or((8, 'A'));
        let angle = ((cam_num - 1) as f32) * (std::f32::consts::TAU / 12.0);
        let mode_sign = if cam_letter == 'B' { 1.0_f32 } else { -1.0_f32 };
        let bpm_norm = ((track.bpm - 90.0) / 80.0).clamp(0.0, 1.5);
        let energy = track.energy_score.clamp(0.0, 1.0);
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
        for b in track
            .top_subgenre
            .bytes()
            .chain(track.groove_type.bytes())
        {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        for i in 8..64 {
            let k = (i - 7) as f32;
            let phase = ((hash.rotate_left((i % 63) as u32) & 0xFFFF) as f32) / 65535.0;
            vec64[i] = (angle * (1.0 + (k % 3.0)) + phase * std::f32::consts::PI).cos() * 0.35
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

/// Stage 1 HNSW Candidate Retrieval ($O(\log N)$ via `usearch::Index` on native builds).
///
/// - When `candidates.len() <= max_candidates` or `candidates.len() <= HNSW_PREFILTER_THRESHOLD`,
///   returns `candidates.to_vec()` verbatim so small crates and Golden Master snapshots
///   preserve 100% bit-for-bit parity.
/// - When `candidates.len() > max_candidates` and `candidates.len() > HNSW_PREFILTER_THRESHOLD`,
///   indexes all 64-D candidate vectors into a `usearch::Index` (`MetricKind::Cos`, `ScalarKind::F32`)
///   and retrieves the top `max_candidates` nearest neighbors to the harmonic/energy centroid.
pub fn prefilter_candidates_hnsw(
    candidates: &[CrateTrackCandidate],
    max_candidates: usize,
) -> Vec<CrateTrackCandidate> {
    let cap = max_candidates.max(1);
    if candidates.len() <= cap || candidates.len() <= HNSW_PREFILTER_THRESHOLD {
        return candidates.to_vec();
    }

    let embeddings: Vec<[f32; 64]> = candidates
        .iter()
        .map(synthesize_or_normalize_64d_embedding)
        .collect();

    let mut centroid = [0.0_f32; 64];
    for emb in &embeddings {
        for (c, &v) in centroid.iter_mut().zip(emb.iter()) {
            *c += v;
        }
    }
    l2_normalize_64d(&mut centroid);

    #[cfg(feature = "native")]
    if let Ok(selected_indices) = query_usearch_hnsw_indices(&embeddings, &centroid, cap) {
        if !selected_indices.is_empty() {
            return selected_indices
                .into_iter()
                .map(|idx| candidates[idx].clone())
                .collect();
        }
    }

    // Deterministic fallback for non-native (WASM) targets
    let mut scored: Vec<(usize, f32)> = embeddings
        .iter()
        .enumerate()
        .map(|(idx, emb)| {
            let dot: f32 = emb.iter().zip(centroid.iter()).map(|(a, b)| a * b).sum();
            (idx, dot)
        })
        .collect();
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    let mut indices: Vec<usize> = scored.into_iter().take(cap).map(|(idx, _)| idx).collect();
    indices.sort_unstable();
    indices
        .into_iter()
        .map(|idx| candidates[idx].clone())
        .collect()
}

#[cfg(feature = "native")]
fn query_usearch_hnsw_indices(
    embeddings: &[[f32; 64]],
    query: &[f32; 64],
    max_candidates: usize,
) -> Result<Vec<usize>, String> {
    use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

    let options = IndexOptions {
        dimensions: 64,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        connectivity: 16,
        expansion_add: 64,
        expansion_search: 32,
        multi: false,
    };

    let index = Index::new(&options).map_err(|e| format!("usearch Index::new failed: {e}"))?;
    index
        .reserve(embeddings.len())
        .map_err(|e| format!("usearch Index::reserve failed: {e}"))?;

    for (idx, emb) in embeddings.iter().enumerate() {
        index
            .add(idx as u64, emb)
            .map_err(|e| format!("usearch Index::add failed at {idx}: {e}"))?;
    }

    let matches = index
        .search(query, max_candidates)
        .map_err(|e| format!("usearch Index::search failed: {e}"))?;

    let mut selected: Vec<usize> = matches
        .keys
        .into_iter()
        .filter_map(|k| {
            let idx = k as usize;
            (idx < embeddings.len()).then_some(idx)
        })
        .collect();
    selected.sort_unstable();
    selected.dedup();
    Ok(selected)
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

        let crate_res = optimize_crate_2opt("HNSW 200 Scale Set", "steady_climb", library, Some(12));
        let elapsed = start.elapsed();

        assert_eq!(crate_res.total_tracks, 12);
        assert!(crate_res.mean_synergy_score >= 0.75);
        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "Expected 200-track HNSW + 2-Opt in < 100ms, took {elapsed:?}"
        );
    }
}
