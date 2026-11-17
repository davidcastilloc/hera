//! In-memory OMKF cross-folder deduplicated search & harmonic candidate ranking.

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use hera_ipc::dj_types::{FolderCrateDetail, FolderCrateSummary, OmkfFsTrackDto};
use hera_node_rs::genre_intelligence::normalize_ascii_lower;
use hera_node_rs::synergy::{compute_bpm_synergy, compute_camelot_synergy};

use super::omkf_fs_catalog::CatalogInner;
use super::omkf_fs_hydrate::CatalogTrackRecord;

fn collapse_repeated_chars(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let (mut prev, mut run) = ('\0', 0_usize);
    for ch in s.chars() {
        if ch == prev && ch.is_ascii_alphabetic() {
            run += 1;
            if run >= 2 {
                continue;
            }
        } else {
            prev = ch;
            run = 0;
        }
        out.push(ch);
    }
    out
}

fn clean_title_slug(raw: &str) -> String {
    let mut s = raw.to_string();
    for noise in [
        "(official video)",
        "[official video]",
        "(video oficial)",
        "[video oficial]",
        "(official music video)",
        "(official audio)",
        "[official audio]",
        "(lyric video)",
        "(lyrics)",
        "[hd]",
        "(hd)",
        "[4k]",
        "(4k)",
    ] {
        let low = s.to_ascii_lowercase();
        if let Some(pos) = low.find(noise) {
            s.replace_range(pos..pos + noise.len(), " ");
        }
    }
    collapse_repeated_chars(&normalize_ascii_lower(&s))
}

fn base_meta_slug(rec: &CatalogTrackRecord) -> String {
    let art = collapse_repeated_chars(&normalize_ascii_lower(&rec.dto.artist));
    let tit = clean_title_slug(&rec.dto.title);
    if !art.is_empty() || !tit.is_empty() {
        format!("meta:{art}::{tit}")
    } else {
        format!("file:{}", clean_title_slug(&rec.dto.file_name))
    }
}

pub(crate) fn folder_priority_score(path_str: &str, scan_level: i8) -> i32 {
    let p = path_str.replace('\\', "/").to_ascii_lowercase();
    let mut score = i32::from(scan_level.max(0)) * 20;
    if p.contains("recycle") || p.contains("trash") || p.contains("quarantine") {
        score -= 1000;
    } else if p.contains("backup")
        || p.contains("respaldo")
        || p.contains("archivo")
        || p.contains("/old/")
        || p.ends_with("/old")
        || p.contains("_old")
    {
        score -= 500;
    } else if p.contains("_usb") || p.contains("usb_pioneer") || p.contains("/export") {
        score -= 200;
    }
    if p.contains("_repertorio_dj") || p.contains("repertorio") {
        score += 300;
    }
    score
}

fn build_effective_dedup_keys(
    tracks_by_id: &HashMap<String, CatalogTrackRecord>,
) -> HashMap<String, String> {
    let mut dur_buckets_by_base: HashMap<String, HashSet<u32>> = HashMap::new();
    for rec in tracks_by_id.values() {
        if rec.dto.duration_sec > 0 {
            dur_buckets_by_base
                .entry(base_meta_slug(rec))
                .or_default()
                .insert((rec.dto.duration_sec + 5) / 10);
        }
    }
    let meta_dur_key = |rec: &CatalogTrackRecord| -> String {
        let base = base_meta_slug(rec);
        let bucket = if rec.dto.duration_sec > 0 {
            (rec.dto.duration_sec + 5) / 10
        } else {
            match dur_buckets_by_base.get(&base) {
                Some(set) if set.len() == 1 => *set.iter().next().unwrap_or(&0),
                _ => 0,
            }
        };
        format!("{base}::{bucket}")
    };
    let mut hash_rep: HashMap<[u8; 16], (String, i32)> = HashMap::new();
    for rec in tracks_by_id.values() {
        if rec.partial_hash_64kb != [0; 16] {
            let k = meta_dur_key(rec);
            let prio = folder_priority_score(&rec.dto.path, rec.dto.scan_level);
            match hash_rep.get_mut(&rec.partial_hash_64kb) {
                Some((best_k, best_p)) if prio > *best_p || (prio == *best_p && k < *best_k) => {
                    (*best_k, *best_p) = (k, prio);
                }
                Some(_) => {}
                None => {
                    hash_rep.insert(rec.partial_hash_64kb, (k, prio));
                }
            }
        }
    }
    tracks_by_id
        .iter()
        .map(|(id, rec)| {
            let key = if rec.partial_hash_64kb != [0; 16] {
                hash_rep
                    .get(&rec.partial_hash_64kb)
                    .map(|(k, _)| k.clone())
                    .unwrap_or_else(|| meta_dur_key(rec))
            } else {
                meta_dur_key(rec)
            };
            (id.clone(), key)
        })
        .collect()
}

pub(crate) fn assign_global_copy_counts(tracks_by_id: &mut HashMap<String, CatalogTrackRecord>) {
    let eff = build_effective_dedup_keys(tracks_by_id);
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for key in eff.values() {
        *counts.entry(key.as_str()).or_default() += 1;
    }
    for (id, rec) in tracks_by_id.iter_mut() {
        let cnt = eff
            .get(id)
            .and_then(|k| counts.get(k.as_str()).copied())
            .unwrap_or(1);
        rec.dto.copy_count = cnt.max(1);
    }
}

fn deduplicate_records<'a>(
    tracks_by_id: &HashMap<String, CatalogTrackRecord>,
    iter: impl Iterator<Item = &'a CatalogTrackRecord>,
) -> Vec<&'a CatalogTrackRecord> {
    let eff = build_effective_dedup_keys(tracks_by_id);
    let mut groups: HashMap<String, (&'a CatalogTrackRecord, i32)> = HashMap::new();
    for rec in iter {
        let key = eff
            .get(&rec.dto.id)
            .cloned()
            .unwrap_or_else(|| base_meta_slug(rec));
        let prio = folder_priority_score(&rec.dto.path, rec.dto.scan_level);
        match groups.get_mut(&key) {
            Some((best, best_prio)) => {
                if prio > *best_prio || (prio == *best_prio && rec.dto.path < best.dto.path) {
                    (*best, *best_prio) = (rec, prio);
                }
            }
            None => {
                groups.insert(key, (rec, prio));
            }
        }
    }
    groups.into_values().map(|(r, _)| r).collect()
}

fn synthetic_detail(
    crate_id: String,
    name: &str,
    tracks: Vec<OmkfFsTrackDto>,
) -> FolderCrateDetail {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64);
    let track_ids = tracks.iter().map(|t| t.id.clone()).collect();
    let video_count = tracks.iter().filter(|t| t.is_video).count();
    let full_count = tracks.iter().filter(|t| t.scan_level >= 2).count();
    FolderCrateDetail {
        summary: FolderCrateSummary {
            crate_id,
            folder_path: String::new(),
            display_name: name.to_string(),
            tree_seal_64: "0".to_string(),
            track_count: tracks.len(),
            video_count,
            full_count,
            bpm_min: None,
            bpm_max: None,
            dominant_camelot: None,
            dominant_genre: None,
            era_span: None,
            lufs_avg: None,
            has_custom_rules: false,
            updated_at_unix_ms: Some(now_ms),
            parent_crate_id: None,
            rel_path: String::new(),
            depth: 0,
            subfolder_count: 0,
        },
        rules_markdown: String::new(),
        ledger_entries: Vec::new(),
        track_ids,
        hydrated_from_omkf: true,
        disk_online: true,
        subfolders: Vec::new(),
        tracks,
    }
}

pub(crate) fn search_catalog_deduplicated(
    inner: &CatalogInner,
    query: &str,
    limit: usize,
) -> FolderCrateDetail {
    let q_norm = normalize_ascii_lower(query);
    let tokens: Vec<&str> = q_norm.split_whitespace().collect();
    let matched = inner.tracks_by_id.values().filter(|r| {
        if r.dto.scan_level < 0 {
            return false;
        }
        if tokens.is_empty() {
            return true;
        }
        let hay = normalize_ascii_lower(&format!(
            "{} {} {} {} {} {}",
            r.dto.artist,
            r.dto.title,
            r.dto.file_name,
            r.folder_display_name,
            r.dto.camelot.as_deref().unwrap_or(""),
            r.genre_family.as_deref().unwrap_or("")
        ));
        tokens.iter().all(|tok| hay.contains(tok))
    });
    let mut winners = deduplicate_records(&inner.tracks_by_id, matched);
    winners.sort_by(|a, b| {
        let pa = folder_priority_score(&a.dto.path, a.dto.scan_level);
        let pb = folder_priority_score(&b.dto.path, b.dto.scan_level);
        pb.cmp(&pa)
            .then_with(|| {
                a.dto
                    .artist
                    .to_ascii_lowercase()
                    .cmp(&b.dto.artist.to_ascii_lowercase())
            })
            .then_with(|| {
                a.dto
                    .title
                    .to_ascii_lowercase()
                    .cmp(&b.dto.title.to_ascii_lowercase())
            })
    });
    winners.truncate(limit);
    let dtos = winners.into_iter().map(|r| r.dto.clone()).collect();
    synthetic_detail(
        format!("__search__:{query}"),
        "Resultados en carpetas",
        dtos,
    )
}

pub(crate) fn query_harmonic_candidates(
    inner: &CatalogInner,
    spec: &str,
    limit: usize,
) -> Option<FolderCrateDetail> {
    let (bpm_s, cam_s) = spec.split_once(':').unwrap_or((spec, ""));
    let target_bpm = bpm_s.trim().parse::<f32>().unwrap_or(0.0);
    let target_cam = cam_s.trim();
    let deduped = deduplicate_records(
        &inner.tracks_by_id,
        inner.tracks_by_id.values().filter(|r| {
            r.dto.scan_level >= 0
                && !r.dto.has_audio_skips
                && folder_priority_score(&r.dto.path, r.dto.scan_level) >= 0
        }),
    );
    let mut scored: Vec<(f32, &CatalogTrackRecord)> = deduped
        .into_iter()
        .filter_map(|r| {
            let cand_bpm = r.dto.bpm.unwrap_or(0.0) as f32;
            if target_bpm > 0.0 && cand_bpm <= 0.0 {
                return None;
            }
            let cam_score = if target_cam.is_empty() {
                0.85
            } else {
                compute_camelot_synergy(target_cam, r.dto.camelot.as_deref()?).0
            };
            let bpm_score = if target_bpm > 0.0 {
                compute_bpm_synergy(target_bpm, cand_bpm).0
            } else {
                0.85
            };
            (cam_score >= 0.75 && bpm_score >= 0.60)
                .then_some((0.55 * cam_score + 0.45 * bpm_score, r))
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    let dtos = scored.into_iter().map(|(_, r)| r.dto.clone()).collect();
    Some(synthetic_detail(
        format!("__harmonic__:{spec}"),
        "Candidatos armónicos",
        dtos,
    ))
}
