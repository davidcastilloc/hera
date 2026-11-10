//! In-memory cache for `usearch::Index` over DJ library embeddings, keyed by 64-bit FNV-1a fingerprint.
//! Ensures candidate libraries (`> 64` tracks) reuse existing HNSW graph structures across multiple crate queries.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[cfg(feature = "native")]
use usearch::Index;

static CACHE_HITS: AtomicUsize = AtomicUsize::new(0);

/// Increments and returns the atomic HNSW cache hits counter.
pub fn record_cache_hit() -> usize {
    CACHE_HITS.fetch_add(1, Ordering::Relaxed) + 1
}

/// Returns the current count of HNSW cache hits.
pub fn cache_hits_count() -> usize {
    CACHE_HITS.load(Ordering::Relaxed)
}

/// Resets the cache hits counter (primarily for deterministic unit testing).
pub fn reset_cache_hits() {
    CACHE_HITS.store(0, Ordering::Relaxed);
}

/// Compute 64-bit FNV-1a fingerprint over a slice of 64-D vectors.
pub fn compute_embeddings_fingerprint(embeddings: &[[f32; 64]]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for emb in embeddings {
        for &val in emb.iter() {
            let bits = val.to_bits();
            for b in bits.to_le_bytes() {
                hash ^= b as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
    }
    hash
}

#[cfg(feature = "native")]
pub struct CachedCatalogHnsw {
    pub fingerprint: u64,
    pub count: usize,
    pub index: Index,
}

#[cfg(feature = "native")]
static GLOBAL_HNSW_CACHE: Mutex<Option<CachedCatalogHnsw>> = Mutex::new(None);

#[cfg(feature = "native")]
pub fn query_or_build_cached_hnsw(
    embeddings: &[[f32; 64]],
    probes: &[[f32; 64]],
    per_probe_k: usize,
) -> Result<Vec<usize>, String> {
    use usearch::{IndexOptions, MetricKind, ScalarKind};

    if embeddings.is_empty() || probes.is_empty() {
        return Ok(Vec::new());
    }

    let fp = compute_embeddings_fingerprint(embeddings);
    let mut lock = GLOBAL_HNSW_CACHE
        .lock()
        .map_err(|e| format!("Mutex poisoned: {e}"))?;

    let is_hit = match lock.as_ref() {
        Some(cached) => cached.fingerprint == fp && cached.count == embeddings.len(),
        None => false,
    };

    if is_hit {
        record_cache_hit();
    } else {
        let options = IndexOptions {
            dimensions: 64,
            metric: MetricKind::Cos,
            quantization: ScalarKind::F32,
            connectivity: 16,
            expansion_add: 64,
            expansion_search: 32,
            multi: false,
        };
        let index = Index::new(&options).map_err(|e| format!("Index::new: {e}"))?;
        index
            .reserve(embeddings.len())
            .map_err(|e| format!("Index::reserve: {e}"))?;

        for (idx, emb) in embeddings.iter().enumerate() {
            index
                .add(idx as u64, emb)
                .map_err(|e| format!("Index::add at {idx}: {e}"))?;
        }

        *lock = Some(CachedCatalogHnsw {
            fingerprint: fp,
            count: embeddings.len(),
            index,
        });
    }

    let cached = lock.as_ref().ok_or_else(|| "Index uninitialized".to_string())?;
    let mut probe_results: Vec<Vec<usize>> = Vec::with_capacity(probes.len());

    for probe in probes {
        let matches = cached
            .index
            .search(probe, per_probe_k)
            .map_err(|e| format!("Index::search: {e}"))?;

        let mut list = Vec::new();
        for k in matches.keys {
            let idx = k as usize;
            if idx < embeddings.len() {
                list.push(idx);
            }
        }
        probe_results.push(list);
    }

    // Interleave probe results in round-robin probe-rank order and deduplicate
    let mut selected = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let max_len = probe_results.iter().map(|r| r.len()).max().unwrap_or(0);
    for rank in 0..max_len {
        for probe_list in &probe_results {
            if rank < probe_list.len() {
                let idx = probe_list[rank];
                if seen.insert(idx) {
                    selected.push(idx);
                }
            }
        }
    }

    Ok(selected)
}

#[cfg(not(feature = "native"))]
static CACHED_FALLBACK_FP: Mutex<Option<u64>> = Mutex::new(None);

#[cfg(not(feature = "native"))]
pub fn query_or_build_cached_hnsw(
    embeddings: &[[f32; 64]],
    probes: &[[f32; 64]],
    per_probe_k: usize,
) -> Result<Vec<usize>, String> {
    if embeddings.is_empty() || probes.is_empty() {
        return Ok(Vec::new());
    }

    let fp = compute_embeddings_fingerprint(embeddings);
    if let Ok(mut lock) = CACHED_FALLBACK_FP.lock() {
        if *lock == Some(fp) {
            record_cache_hit();
        } else {
            *lock = Some(fp);
        }
    }

    let mut probe_results: Vec<Vec<usize>> = Vec::with_capacity(probes.len());
    for probe in probes {
        let mut scored: Vec<(usize, f32)> = embeddings
            .iter()
            .enumerate()
            .map(|(idx, emb)| {
                let dot: f32 = emb.iter().zip(probe.iter()).map(|(a, b)| a * b).sum();
                (idx, dot)
            })
            .collect();
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        let list: Vec<usize> = scored.into_iter().take(per_probe_k).map(|(idx, _)| idx).collect();
        probe_results.push(list);
    }

    // Interleave probe results in round-robin probe-rank order and deduplicate
    let mut selected = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let max_len = probe_results.iter().map(|r| r.len()).max().unwrap_or(0);
    for rank in 0..max_len {
        for probe_list in &probe_results {
            if rank < probe_list.len() {
                let idx = probe_list[rank];
                if seen.insert(idx) {
                    selected.push(idx);
                }
            }
        }
    }

    Ok(selected)
}
