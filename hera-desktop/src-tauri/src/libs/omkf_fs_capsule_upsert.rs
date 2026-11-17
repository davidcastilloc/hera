//! OMKF `.hera/tracks.omkf` filesystem capsule upsert, root deduplication & DJ filename hints.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use hera_node_rs::genre_intelligence::BoothGenreFamily;
use hera_node_rs::scanner::omkf_capsule as omkf;
use hera_node_rs::synergy::parse_camelot;

use super::db_scanner_queries::ScannerRowUpdate;
use super::fast_library_scan::{LoadedFolderOmkfRow, pack_omkf_header_and_entries};
use super::omkf_fs_catalog::OmkfFsCatalog;
use super::omkf_sync::{compute_folder_crate_id, hide_hera_dir_if_windows, scan_dir_entry_seals};
use super::track::HotCuePin;

pub(crate) use super::omkf_fs_organize::{deduplicate_roots, norm_cmp_key};

pub(crate) fn parse_filename_dj_hints(stem: &str) -> (String, String, Option<f64>, Option<String>) {
    let (mut bpm_hint, mut cam_hint) = (None, None);
    let parse_bpm = |s: &str| s.parse::<f64>().ok().filter(|b| (45.0..=220.0).contains(b));
    let tokens: Vec<&str> = stem
        .split([' ', '[', ']', '(', ')', '-', '_', ',', ';', '|'])
        .filter(|t| !t.is_empty())
        .collect();
    for (i, &tok) in tokens.iter().enumerate() {
        let low = tok.to_ascii_lowercase();
        if bpm_hint.is_none() {
            bpm_hint = low
                .strip_suffix("bpm")
                .filter(|s| !s.is_empty())
                .and_then(parse_bpm)
                .or_else(|| {
                    (low == "bpm" && i > 0)
                        .then(|| parse_bpm(tokens[i - 1]))
                        .flatten()
                });
        }
        if cam_hint.is_none()
            && (2..=3).contains(&tok.len())
            && let Some((n, ab)) = parse_camelot(tok)
        {
            cam_hint = Some(format!("{n}{ab}"));
        }
    }
    let cleaned = strip_dj_meta_brackets(stem);
    let trimmed = cleaned.trim().trim_matches('-').trim();
    for sep in [" - ", " – ", " — "] {
        if let Some((l, r)) = trimmed.split_once(sep) {
            let lt = l.trim();
            if lt.len() <= 3
                && lt.chars().all(|c| c.is_ascii_digit())
                && let Some((a2, t2)) = r.split_once(sep)
            {
                return (a2.trim().into(), t2.trim().into(), bpm_hint, cam_hint);
            }
            return (lt.into(), r.trim().into(), bpm_hint, cam_hint);
        }
    }
    (String::new(), trimmed.into(), bpm_hint, cam_hint)
}

fn strip_dj_meta_brackets(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        let close = match ch {
            '[' => Some(']'),
            '(' => Some(')'),
            _ => None,
        };
        if let Some(cl) = close
            && let Some(rel_end) = s[i + ch.len_utf8()..].find(cl)
        {
            let inner = &s[i + ch.len_utf8()..i + ch.len_utf8() + rel_end];
            let has_meta = inner.to_ascii_lowercase().contains("bpm")
                || inner
                    .split([' ', '-', '_', ',', '/'])
                    .any(|t| (2..=3).contains(&t.len()) && parse_camelot(t).is_some());
            if has_meta {
                let target = i + ch.len_utf8() + rel_end;
                while chars.peek().is_some_and(|&(idx, _)| idx <= target) {
                    chars.next();
                }
                continue;
            }
        }
        out.push(ch);
    }
    out
}

pub(crate) fn load_capsule_entry_map(dir: &Path) -> HashMap<String, omkf::OmkfTrackEntry> {
    omkf::read_omkf_capsule(&dir.join(".hera/tracks.omkf"))
        .map(|c| {
            c.tracks
                .into_iter()
                .map(|e| (e.filename.to_ascii_lowercase(), e))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn rewrite_dir_capsule_with_map(
    dir: &Path,
    mut entries_map: HashMap<String, omkf::OmkfTrackEntry>,
) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    let cid = compute_folder_crate_id(dir);
    let (dir_ns, disk, tree_seal) = scan_dir_entry_seals(dir);
    let mut loaded = Vec::new();
    for d in &disk {
        let Some(mut entry) = entries_map.remove(&d.filename.to_ascii_lowercase()) else {
            continue;
        };
        entry.filename = d.filename.clone();
        (entry.slot.file_size_bytes, entry.slot.mtime_ns) = (d.size, d.mtime_ns);
        let fam = BoothGenreFamily::from_code(entry.slot.genre_family_code);
        let g = (fam != BoothGenreFamily::Unknown).then(|| fam.as_str().to_string());
        let lufs = f64::from(entry.slot.lufs_x100) / 100.0;
        let is_vid = (entry.slot.flags_bits & omkf::OMKF_FLAG_IS_VIDEO) != 0;
        let is_full = entry.slot.scan_level >= 2;
        let camelot = omkf::code_to_camelot(entry.slot.camelot_code);
        loaded.push(LoadedFolderOmkfRow {
            entry,
            camelot,
            genre_family: g,
            lufs,
            is_video: is_vid,
            is_full,
        });
    }
    if loaded.is_empty() && !dir.join(".hera/tracks.omkf").exists() {
        return Ok(());
    }
    let (hdr, entries, md, _) = pack_omkf_header_and_entries(&cid, tree_seal, dir_ns, loaded);
    let hera_dir = dir.join(".hera");
    omkf::write_omkf_capsule_atomic(&hera_dir, &hdr, &entries, &md)?;
    hide_hera_dir_if_windows(&hera_dir);
    Ok(())
}

fn partial_hash_16b(path: &Path) -> [u8; 16] {
    let mut buf = vec![0_u8; 65_536];
    let n = std::fs::File::open(path)
        .and_then(|mut f| f.read(&mut buf))
        .unwrap_or(0);
    if n == 0 {
        return [0; 16];
    }
    let (mut h1, mut h2) = (0xcbf2_9ce4_8422_2325_u64, 0x8422_2325_cbf2_9ce4_u64);
    for &b in &buf[..n] {
        h1 = (h1 ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        h2 = (h2.rotate_left(5) ^ u64::from(b)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    std::array::from_fn(|i| {
        if i < 8 {
            h1.to_le_bytes()[i]
        } else {
            h2.to_le_bytes()[i - 8]
        }
    })
}

pub type CapsuleOutcomeItem = (String, ScannerRowUpdate, Option<Vec<HotCuePin>>);

pub fn upsert_scanner_outcomes_to_omkf(
    catalog: &OmkfFsCatalog,
    batch: &[CapsuleOutcomeItem],
) -> Vec<String> {
    let mut by_dir: HashMap<PathBuf, Vec<&CapsuleOutcomeItem>> = HashMap::new();
    for item in batch {
        if let Some(parent) = Path::new(&item.0).parent().filter(|p| p.is_dir()) {
            by_dir.entry(parent.to_path_buf()).or_default().push(item);
        }
    }
    let mut touched = HashSet::new();
    for (dir, items) in by_dir {
        let mut map = load_capsule_entry_map(&dir);
        for (path_s, upd, pins_opt) in items {
            let p = Path::new(path_s);
            let Some(fname) = p.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(fname);
            let (fb_art, fb_tit, _, _) = parse_filename_dj_hints(stem);
            let meta = std::fs::metadata(p).ok();
            let entry =
                map.entry(fname.to_ascii_lowercase())
                    .or_insert_with(|| omkf::OmkfTrackEntry {
                        slot: omkf::OmkfTrackSlot::default(),
                        filename: fname.to_string(),
                        artist: fb_art,
                        title: if fb_tit.is_empty() {
                            stem.to_string()
                        } else {
                            fb_tit
                        },
                    });
            apply_outcome_to_slot(p, &mut entry.slot, meta.as_ref(), upd, pins_opt.as_deref());
        }
        if rewrite_dir_capsule_with_map(&dir, map).is_ok() {
            catalog.invalidate_folder_path(&dir);
            touched.insert(compute_folder_crate_id(&dir));
        }
    }
    let mut out: Vec<String> = touched.into_iter().collect();
    out.sort();
    out
}

fn apply_outcome_to_slot(
    p: &Path,
    s: &mut omkf::OmkfTrackSlot,
    meta: Option<&std::fs::Metadata>,
    upd: &ScannerRowUpdate,
    pins_opt: Option<&[HotCuePin]>,
) {
    s.file_size_bytes = meta.map_or(s.file_size_bytes, std::fs::Metadata::len);
    s.mtime_ns = meta
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(s.mtime_ns, |d| d.as_nanos() as i64);
    if s.partial_hash_64kb == [0; 16] {
        s.partial_hash_64kb = partial_hash_16b(p);
    }
    s.scan_level = if upd.scan_level < 0 {
        -1
    } else {
        s.scan_level.max(upd.scan_level as i8)
    };
    s.duration_sec_u16 = upd
        .duration_sec
        .map_or(s.duration_sec_u16, |d| d.min(65535) as u16);
    s.bpm_x100 = upd
        .bpm
        .filter(|&b| b > 0.0)
        .map_or(s.bpm_x100, |b| (b * 100.0).round() as u16);
    s.camelot_code = upd
        .camelot
        .as_deref()
        .map_or(s.camelot_code, omkf::camelot_to_code);
    s.entry_bpm_x100 = upd
        .entry_bpm
        .filter(|&b| b > 0.0)
        .map_or(s.entry_bpm_x100, |b| (b * 100.0).round() as u16);
    s.exit_bpm_x100 = upd
        .exit_bpm
        .filter(|&b| b > 0.0)
        .map_or(s.exit_bpm_x100, |b| (b * 100.0).round() as u16);
    s.lufs_x100 = upd
        .loudness_lufs
        .map_or(s.lufs_x100, |l| (l * 100.0).round() as i16);
    s.replay_gain_x100 = upd
        .replay_gain_db
        .map_or(s.replay_gain_x100, |r| (r * 100.0).round() as i16);
    let is_vid = p.extension().and_then(|e| e.to_str()).is_some_and(|ext| {
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "mp4" | "m4v" | "mov" | "webm" | "mkv"
        )
    });
    for (cond, bit) in [
        (upd.is_transition_candidate, omkf::OMKF_FLAG_IS_TRANSITION),
        (upd.is_fake_320kbps, omkf::OMKF_FLAG_IS_FAKE_320KBPS),
        (
            upd.has_audio_skips == Some(true),
            omkf::OMKF_FLAG_HAS_AUDIO_SKIPS,
        ),
        (
            upd.has_intro_vocals == Some(true),
            omkf::OMKF_FLAG_HAS_INTRO_VOCALS,
        ),
        (
            upd.has_outro_vocals == Some(true),
            omkf::OMKF_FLAG_HAS_OUTRO_VOCALS,
        ),
        (is_vid, omkf::OMKF_FLAG_IS_VIDEO),
    ] {
        if cond {
            s.flags_bits |= bit;
        }
    }
    for pin in pins_opt
        .iter()
        .flat_map(|v| v.iter())
        .filter(|x| (1..=8).contains(&x.num))
    {
        s.hot_cues_ms[usize::from(pin.num - 1)] = (pin.seconds.max(0.0) * 1000.0).round() as u32;
    }
    if let Some(raw) = upd
        .embedding_64d_b64
        .as_ref()
        .and_then(|b| B64.decode(b).ok())
    {
        raw.into_iter()
            .take(64)
            .enumerate()
            .for_each(|(i, b)| s.embedding_64d[i] = b as i8);
    }
    if let Some(raw) = upd
        .waveform_peaks_b64
        .as_ref()
        .and_then(|b| B64.decode(b).ok())
    {
        raw.into_iter()
            .take(768)
            .enumerate()
            .for_each(|(i, b)| s.rgb_peaks_256[i] = b);
    }
}
