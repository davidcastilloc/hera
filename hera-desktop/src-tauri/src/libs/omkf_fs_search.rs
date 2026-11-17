//! In-memory OMKF cross-folder deduplicated search & harmonic candidate ranking.

use std::collections::HashMap;
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

pub(crate) fn canonical_dedup_key(rec: &CatalogTrackRecord) -> String {
    let art = collapse_repeated_chars(&normalize_ascii_lower(&rec.dto.artist));
    let tit = clean_title_slug(&rec.dto.title);
    if !art.is_empty() || !tit.is_empty() {
        return format!("meta:{art}::{tit}");
    }
    if rec.partial_hash_64kb != [0; 16] {
        return format!("hash:{:02x?}", rec.partial_hash_64kb);
    }
    format!("file:{}", clean_title_slug(&rec.dto.file_name))
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

pub(crate) fn assign_global_copy_counts(tracks_by_id: &mut HashMap<String, CatalogTrackRecord>) {
    let mut hash_alias: HashMap<[u8; 16], String> = HashMap::new();
    for rec in tracks_by_id.values() {
        if rec.partial_hash_64kb != [0; 16] {
            hash_alias
                .entry(rec.partial_hash_64kb)
                .or_insert_with(|| canonical_dedup_key(rec));
        }
    }
    let mut counts: HashMap<String, usize> = HashMap::new();
    for rec in tracks_by_id.values() {
        let key = if rec.partial_hash_64kb != [0; 16] {
            hash_alias
                .get(&rec.partial_hash_64kb)
                .cloned()
                .unwrap_or_else(|| canonical_dedup_key(rec))
        } else {
            canonical_dedup_key(rec)
        };
        *counts.entry(key).or_default() += 1;
    }
    for rec in tracks_by_id.values_mut() {
        let key = if rec.partial_hash_64kb != [0; 16] {
            hash_alias
                .get(&rec.partial_hash_64kb)
                .cloned()
                .unwrap_or_else(|| canonical_dedup_key(rec))
        } else {
            canonical_dedup_key(rec)
        };
        rec.dto.copy_count = counts.get(&key).copied().unwrap_or(1).max(1);
    }
}

fn deduplicate_records<'a>(
    iter: impl Iterator<Item = &'a CatalogTrackRecord>,
) -> Vec<&'a CatalogTrackRecord> {
    let mut groups: HashMap<String, (&'a CatalogTrackRecord, i32)> = HashMap::new();
    for rec in iter {
        let key = canonical_dedup_key(rec);
        let prio = folder_priority_score(&rec.dto.path, rec.dto.scan_level);
        match groups.get_mut(&key) {
            Some((best, best_prio)) => {
                if prio > *best_prio || (prio == *best_prio && rec.dto.path < best.dto.path) {
                    *best = rec;
                    *best_prio = prio;
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
    let mut winners = deduplicate_records(matched);
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
        inner
            .tracks_by_id
            .values()
            .filter(|r| r.dto.scan_level >= 0 && !r.dto.has_audio_skips),
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
            if cam_score < 0.75 || bpm_score < 0.60 {
                return None;
            }
            let bonus = if folder_priority_score(&r.dto.path, r.dto.scan_level) >= 0 {
                0.08
            } else {
                -0.15
            };
            Some((0.55 * cam_score + 0.45 * bpm_score + bonus, r))
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
