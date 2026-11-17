//! Filesystem-native `.hera/crate.md` rules, reversible organization & `Track` projection.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use hera_ipc::dj_types::{
    OmkfFsTrackDto, OrganizeFolderRequest, OrganizeFolderResponse, PlannedFileMove,
};
use hera_node_rs::genre_intelligence::{classify_track_genre_and_era, era_decade_label};
use hera_node_rs::scanner::omkf_capsule as omkf;
use uuid::Uuid;

use super::fast_library_scan::{parse_crate_md_sections, write_crate_md_sections};
pub use super::omkf_fs_capsule_upsert::upsert_scanner_outcomes_to_omkf;
use super::omkf_fs_capsule_upsert::{load_capsule_entry_map, rewrite_dir_capsule_with_map};
use super::omkf_fs_catalog::OmkfFsCatalog;
use super::omkf_fs_hydrate::CatalogTrackRecord;
use super::track::{HotCuePin, Track};

pub(crate) fn norm_cmp_key(path: &Path) -> String {
    let canon = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let raw = canon.to_string_lossy();
    raw.strip_prefix(r"\\?\")
        .unwrap_or(&raw)
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

pub(crate) fn deduplicate_roots(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut indexed: Vec<(usize, PathBuf, String)> = roots
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_dir())
        .map(|(i, p)| {
            let raw = p.to_string_lossy();
            let clean = raw
                .strip_prefix(r"\\?\")
                .map_or_else(|| p.clone(), PathBuf::from);
            let key = norm_cmp_key(&clean);
            (i, clean, key)
        })
        .collect();
    indexed.sort_by(|a, b| a.2.len().cmp(&b.2.len()).then_with(|| a.0.cmp(&b.0)));
    let mut kept: Vec<(usize, PathBuf, String)> = Vec::new();
    for (idx, path, key) in indexed {
        if !kept
            .iter()
            .any(|(_, _, anc)| key == *anc || key.starts_with(&format!("{anc}/")))
        {
            kept.push((idx, path, key));
        }
    }
    kept.sort_by_key(|(idx, _, _)| *idx);
    kept.into_iter().map(|(_, p, _)| p).collect()
}

impl CatalogTrackRecord {
    pub fn to_track(&self) -> Track {
        let d = &self.dto;
        let colors = ["#22c55e", "#38bdf8", "#ef4444", "#f59e0b"];
        let pins: Vec<HotCuePin> = d
            .hot_cues
            .iter()
            .map(|c| HotCuePin {
                num: c.slot,
                name: c.label.clone(),
                seconds: c.seconds,
                color: colors[usize::from(c.slot.saturating_sub(1)) % 4].to_string(),
                role: c.kind.clone(),
                origin: Some("omkf".to_string()),
            })
            .collect();
        let artists = if d.artist.is_empty() {
            vec![]
        } else {
            vec![d.artist.clone()]
        };
        Track {
            id: d.id.clone(),
            path: d.path.clone(),
            title: d.title.clone(),
            album: d.album.clone(),
            album_artist: d.artist.clone(),
            artists,
            genres: self.genre_family.iter().cloned().collect(),
            year: (self.era_year > 0).then_some(self.era_year),
            duration: d.duration_sec,
            track_no: None,
            track_of: None,
            disk_no: None,
            disk_of: None,
            is_compilation: false,
            bpm: d.bpm,
            camelot: d.camelot.clone(),
            energy: d.energy,
            hera_seal: None,
            is_video: Some(d.is_video),
            scan_level: Some(i64::from(d.scan_level)),
            is_transition_candidate: Some(false),
            is_fake_320kbps: Some(d.is_fake_320kbps),
            waveform_peaks_blob: None,
            is_mix_over_10m: Some(d.duration_sec > 600),
            has_audio_skips: Some(d.has_audio_skips),
            is_variable_bpm: Some(d.is_variable_bpm),
            entry_bpm: self.entry_bpm,
            exit_bpm: self.exit_bpm,
            hot_cues: (!pins.is_empty()).then_some(pins),
            replay_gain_db: d.replay_gain_db,
            loudness_lufs: self.lufs,
            has_intro_vocals: Some(d.has_intro_vocals),
            has_outro_vocals: Some(d.has_outro_vocals),
            is_own_production: Some(false),
            has_bpm_trust_diff: Some(false),
            waveform_peaks_b64: d.waveform_peaks_b64.clone(),
            embedding_64d_b64: None,
            inferred_genre_family: self.genre_family.clone(),
            inferred_era_year: (self.era_year > 0).then(|| i64::from(self.era_year)),
            partial_hash_64kb: None,
            file_size_bytes: None,
            legacy_bpm: None,
            bitrate_kbps: None,
            sample_rate_hz: None,
        }
    }
}

pub fn save_folder_rules_fs(
    catalog: &OmkfFsCatalog,
    roots: &[PathBuf],
    crate_id: &str,
    rules_markdown: &str,
) -> Result<bool, String> {
    let dir = catalog
        .folder_path_for_crate_id(crate_id, roots)
        .ok_or_else(|| "Carpeta no encontrada".to_string())?;
    if !dir.is_dir() {
        return Err("Carpeta desconectada en disco".to_string());
    }
    let raw = std::fs::read_to_string(dir.join(".hera/crate.md")).unwrap_or_default();
    let (_, _, ledger) = parse_crate_md_sections(&raw);
    write_crate_md_sections(&dir, rules_markdown, &ledger)?;
    catalog.invalidate_folder_path(&dir);
    Ok(true)
}

fn bucket_for_fs_track(strategy: &str, t: &OmkfFsTrackDto) -> (String, String) {
    let bad = t.scan_level < 0 || t.has_audio_skips || t.is_fake_320kbps;
    match strategy {
        "quarantine_issues" => {
            if bad {
                (
                    "_Quarantine_Audit".into(),
                    "Pista con fallo o transcodificada".into(),
                )
            } else {
                (String::new(), String::new())
            }
        }
        "by_genre_era" => {
            let prof = classify_track_genre_and_era(&t.artist, &t.title, &[], &t.path, None, None);
            let dec = era_decade_label(prof.inferred_era_year);
            (
                format!("{}_{dec}", prof.family_name).replace(['/', ' '], "_"),
                "Agrupación Genre × Era".into(),
            )
        }
        _ => {
            if t.duration_sec > 600 {
                return ("_Mixes_10m_Plus".into(), "Sesión > 10 min".into());
            }
            let bpm = t.bpm.unwrap_or(124.0);
            let z = if bpm < 100.0 {
                "_Sub_100"
            } else if bpm < 118.0 {
                "_100_118"
            } else if bpm < 128.0 {
                "_118_128"
            } else if bpm < 140.0 {
                "_128_140"
            } else {
                "_140_Plus"
            };
            (z.into(), format!("Zona tempo {bpm:.0} BPM"))
        }
    }
}

pub fn organize_folder_fs(
    catalog: &OmkfFsCatalog,
    roots: &[PathBuf],
    req: OrganizeFolderRequest,
) -> Result<OrganizeFolderResponse, String> {
    let base_dir = catalog
        .folder_path_for_crate_id(&req.crate_id, roots)
        .ok_or_else(|| "Crate de carpeta no encontrado".to_string())?;
    if !base_dir.is_dir() {
        return Err("Carpeta desconectada en disco".to_string());
    }
    let md_raw = std::fs::read_to_string(base_dir.join(".hera/crate.md")).unwrap_or_default();
    let (_, rules_md, mut ledger) = parse_crate_md_sections(&md_raw);
    let (mut moves, mut rolled_back) = (Vec::new(), false);

    if let Some(ref rtx) = req.rollback_tx_id {
        let prefix = format!("- tx:{rtx} | ");
        for line in ledger.iter().filter(|l| l.starts_with(&prefix)) {
            let parts: Vec<&str> = line[prefix.len()..].split(" | ").collect();
            if parts.len() == 3 {
                moves.push(PlannedFileMove {
                    track_id: parts[0].into(),
                    source_path: parts[2].into(),
                    target_path: parts[1].into(),
                    reason: format!("Rollback {rtx}"),
                    bucket_label: "Rollback".into(),
                });
            }
        }
        if !req.dry_run && !moves.is_empty() {
            ledger.retain(|l| !l.starts_with(&prefix));
            rolled_back = true;
        }
    } else {
        let (detail_opt, _) = catalog.read_folder(&req.crate_id, roots);
        for t in detail_opt.map(|d| d.tracks).unwrap_or_default() {
            let (bucket, reason) = bucket_for_fs_track(&req.strategy, &t);
            if bucket.is_empty() {
                continue;
            }
            let dst_s = base_dir
                .join(&bucket)
                .join(&t.file_name)
                .to_string_lossy()
                .into_owned();
            if dst_s != t.path {
                moves.push(PlannedFileMove {
                    track_id: t.id,
                    source_path: t.path,
                    target_path: dst_s,
                    reason,
                    bucket_label: bucket,
                });
            }
        }
    }

    let tx_id = format!("tx-{}", &Uuid::new_v4().to_string()[..8]);
    let mut applied = 0_usize;
    if !req.dry_run && !moves.is_empty() {
        let mut dir_maps: HashMap<PathBuf, HashMap<String, omkf::OmkfTrackEntry>> = HashMap::new();
        for m in &moves {
            let (s, d) = (Path::new(&m.source_path), Path::new(&m.target_path));
            let (Some(src_dir), Some(dst_dir), Some(fname)) = (
                s.parent(),
                d.parent(),
                s.file_name().and_then(|n| n.to_str()),
            ) else {
                continue;
            };
            std::fs::create_dir_all(dst_dir).map_err(|e| e.to_string())?;
            if s.exists() && !d.exists() {
                let fkey = fname.to_ascii_lowercase();
                let carried = dir_maps
                    .entry(src_dir.to_path_buf())
                    .or_insert_with(|| load_capsule_entry_map(src_dir))
                    .remove(&fkey);
                std::fs::rename(s, d).map_err(|e| e.to_string())?;
                if let Some(entry) = carried {
                    dir_maps
                        .entry(dst_dir.to_path_buf())
                        .or_insert_with(|| load_capsule_entry_map(dst_dir))
                        .insert(fkey, entry);
                }
                if !rolled_back {
                    ledger.push(format!(
                        "- tx:{tx_id} | {} | {} | {}",
                        m.track_id, m.source_path, m.target_path
                    ));
                }
                applied += 1;
            }
        }
        for (dir, map) in dir_maps {
            let _ = rewrite_dir_capsule_with_map(&dir, map);
        }
        write_crate_md_sections(&base_dir, &rules_md, &ledger)?;
        catalog.mark_stale();
    }

    Ok(OrganizeFolderResponse {
        crate_id: req.crate_id,
        dry_run: req.dry_run,
        tx_id,
        moves,
        applied_count: applied,
        rolled_back,
    })
}
