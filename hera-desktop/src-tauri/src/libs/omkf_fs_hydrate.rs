//! Live directory + `.hera/tracks.omkf` track record hydration.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use hera_ipc::dj_types::{FolderCrateSummary, OmkfFsHotCueDto, OmkfFsTrackDto};
use hera_node_rs::genre_intelligence::BoothGenreFamily;
use hera_node_rs::scanner::omkf_capsule as omkf;
use uuid::Uuid;

use super::database::SUPPORTED_TRACKS_EXTENSIONS;
use super::fast_library_scan::{LoadedFolderOmkfRow, parse_crate_md_sections};
use super::omkf_fs_capsule_upsert::{norm_cmp_key, parse_filename_dj_hints};
use super::omkf_sync::DiskEntrySeal;
use super::track::get_track_id_for_path;
use super::utils::is_file_valid;
use crate::plugins::hera_dj::workability::is_junk_audio_path;

pub(crate) fn retain_non_sql_capsule_entries(
    folder: &Path,
    disk: &[DiskEntrySeal],
    loaded: &mut Vec<LoadedFolderOmkfRow>,
) {
    let Ok(existing) = omkf::read_omkf_capsule(&folder.join(".hera/tracks.omkf")) else {
        return;
    };
    let sql_names: HashSet<String> = loaded
        .iter()
        .map(|r| r.entry.filename.to_ascii_lowercase())
        .collect();
    let mut prev: HashMap<String, omkf::OmkfTrackEntry> = existing
        .tracks
        .into_iter()
        .map(|e| (e.filename.to_ascii_lowercase(), e))
        .collect();
    for d in disk {
        let key = d.filename.to_ascii_lowercase();
        if !sql_names.contains(&key)
            && let Some(mut entry) = prev.remove(&key)
        {
            entry.filename = d.filename.clone();
            (entry.slot.file_size_bytes, entry.slot.mtime_ns) = (d.size, d.mtime_ns);
            let camelot = omkf::code_to_camelot(entry.slot.camelot_code);
            let fam = BoothGenreFamily::from_code(entry.slot.genre_family_code);
            let genre_family = (fam != BoothGenreFamily::Unknown).then(|| fam.as_str().to_string());
            let lufs = f64::from(entry.slot.lufs_x100) / 100.0;
            let is_video = (entry.slot.flags_bits & omkf::OMKF_FLAG_IS_VIDEO) != 0;
            let is_full = entry.slot.scan_level >= 2;
            loaded.push(LoadedFolderOmkfRow {
                entry,
                camelot,
                genre_family,
                lufs,
                is_video,
                is_full,
            });
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CatalogTrackRecord {
    pub dto: OmkfFsTrackDto,
    pub partial_hash_64kb: [u8; 16],
    pub folder_display_name: String,
    pub genre_family: Option<String>,
    pub era_year: u16,
    pub lufs: Option<f64>,
    pub entry_bpm: Option<f64>,
    pub exit_bpm: Option<f64>,
}

pub(crate) struct HydratedDir {
    pub summary: FolderCrateSummary,
    pub rules_md: String,
    pub ledger: Vec<String>,
    pub from_omkf: bool,
    pub records: Vec<CatalogTrackRecord>,
    pub unanalyzed: Vec<(String, String)>,
}

pub(crate) fn hydrate_directory(
    dir: &Path,
    crate_id: &str,
    parent_crate_id: Option<String>,
    rel_path: String,
    depth: usize,
    subfolder_count: usize,
) -> HydratedDir {
    let disp = dir
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Carpeta")
        .to_string();
    let cap_opt = omkf::read_omkf_capsule(&dir.join(".hera/tracks.omkf")).ok();
    let md_raw = std::fs::read_to_string(dir.join(".hera/crate.md")).unwrap_or_default();
    let (_, rules_md, ledger) = parse_crate_md_sections(&md_raw);
    let mut by_name: HashMap<String, &omkf::OmkfTrackEntry> = HashMap::new();
    if let Some(ref cap) = cap_opt {
        for e in &cap.tracks {
            by_name.insert(e.filename.to_ascii_lowercase(), e);
        }
    }
    let mut files: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let valid = p.is_file()
                && !is_junk_audio_path(&p)
                && is_file_valid(&p, &SUPPORTED_TRACKS_EXTENSIONS);
            valid.then(|| (e.file_name().to_string_lossy().into_owned(), p))
        })
        .collect();
    files.sort_by_key(|a| a.0.to_ascii_lowercase());

    let (mut records, mut unanalyzed) = (Vec::with_capacity(files.len()), Vec::new());
    let (mut vid_n, mut full_n, mut bmin, mut bmax) = (0, 0, f64::INFINITY, 0.0_f64);
    let (mut cam_h, mut gen_h) = (
        HashMap::<String, usize>::new(),
        HashMap::<String, usize>::new(),
    );
    let (mut ymin, mut ymax, mut lufs_sum, mut lufs_n) = (u16::MAX, 0_u16, 0.0_f64, 0_usize);

    for (fname, fpath) in files {
        let tid = get_track_id_for_path(&fpath).unwrap_or_else(|_| {
            Uuid::new_v3(&Uuid::NAMESPACE_OID, norm_cmp_key(&fpath).as_bytes()).to_string()
        });
        let stem = fpath.file_stem().and_then(|s| s.to_str()).unwrap_or(&fname);
        let fb = parse_filename_dj_hints(stem);
        let ext_vid = fpath
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "mp4" | "m4v" | "mov" | "webm" | "mkv"
                )
            });
        let rec = match by_name.get(&fname.to_ascii_lowercase()) {
            Some(e) => build_slot_record(crate_id, &disp, &fname, &fpath, tid, e, fb, ext_vid),
            None => build_live_record(crate_id, &disp, &fname, &fpath, tid, stem, fb, ext_vid),
        };
        if rec.dto.scan_level == 0 {
            unanalyzed.push((rec.dto.id.clone(), rec.dto.path.clone()));
        }
        vid_n += usize::from(rec.dto.is_video);
        full_n += usize::from(rec.dto.scan_level >= 2 || rec.dto.hot_cues.len() >= 4);
        if let Some(b) = rec.dto.bpm.filter(|&b| b > 0.0) {
            (bmin, bmax) = (bmin.min(b), bmax.max(b));
        }
        if let Some(ref c) = rec.dto.camelot {
            *cam_h.entry(c.clone()).or_default() += 1;
        }
        if let Some(ref g) = rec.genre_family {
            *gen_h.entry(g.clone()).or_default() += 1;
        }
        if rec.era_year > 0 {
            (ymin, ymax) = (ymin.min(rec.era_year), ymax.max(rec.era_year));
        }
        if let Some(l) = rec.lufs {
            (lufs_sum, lufs_n) = (lufs_sum + l, lufs_n + 1);
        }
        records.push(rec);
    }
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64);
    let summary = FolderCrateSummary {
        crate_id: crate_id.to_string(),
        folder_path: dir.to_string_lossy().into_owned(),
        display_name: disp,
        tree_seal_64: cap_opt
            .as_ref()
            .map_or_else(|| "0".into(), |c| format!("{:016x}", c.header.tree_seal_64)),
        track_count: records.len(),
        video_count: vid_n,
        full_count: full_n,
        bpm_min: bmin.is_finite().then_some(bmin),
        bpm_max: (bmax > 0.0).then_some(bmax),
        dominant_camelot: cam_h.into_iter().max_by_key(|x| x.1).map(|x| x.0),
        dominant_genre: gen_h.into_iter().max_by_key(|x| x.1).map(|x| x.0),
        era_span: (ymin <= ymax).then(|| format!("{ymin}-{ymax}")),
        lufs_avg: (lufs_n > 0).then(|| lufs_sum / lufs_n as f64),
        has_custom_rules: !rules_md.trim().is_empty(),
        updated_at_unix_ms: Some(now_ms),
        parent_crate_id,
        rel_path,
        depth,
        subfolder_count,
    };
    HydratedDir {
        summary,
        rules_md,
        ledger,
        from_omkf: cap_opt.is_some(),
        records,
        unanalyzed,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_live_record(
    cid: &str,
    disp: &str,
    fname: &str,
    fpath: &Path,
    tid: String,
    stem: &str,
    fb: (String, String, Option<f64>, Option<String>),
    ext_vid: bool,
) -> CatalogTrackRecord {
    CatalogTrackRecord {
        dto: OmkfFsTrackDto {
            id: tid,
            crate_id: cid.to_string(),
            path: fpath.to_string_lossy().into_owned(),
            file_name: fname.to_string(),
            title: if fb.1.is_empty() {
                stem.to_string()
            } else {
                fb.1
            },
            artist: fb.0,
            album: disp.to_string(),
            duration_sec: 0,
            bpm: fb.2,
            camelot: fb.3,
            energy: None,
            hot_cues: Vec::new(),
            scan_level: 0,
            is_video: ext_vid,
            is_fake_320kbps: false,
            has_audio_skips: false,
            is_variable_bpm: false,
            has_intro_vocals: false,
            has_outro_vocals: false,
            replay_gain_db: None,
            waveform_peaks_b64: None,
            copy_count: 1,
        },
        partial_hash_64kb: [0; 16],
        folder_display_name: disp.to_string(),
        genre_family: None,
        era_year: 0,
        lufs: None,
        entry_bpm: None,
        exit_bpm: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_slot_record(
    cid: &str,
    disp: &str,
    fname: &str,
    fpath: &Path,
    tid: String,
    e: &omkf::OmkfTrackEntry,
    fb: (String, String, Option<f64>, Option<String>),
    ext_vid: bool,
) -> CatalogTrackRecord {
    let s = &e.slot;
    let labels = [
        ("1:Intro", "intro"),
        ("2:Breakdown", "breakdown"),
        ("3:Drop", "drop"),
        ("4:Outro", "outro"),
    ];
    let cues: Vec<OmkfFsHotCueDto> = s
        .hot_cues_ms
        .iter()
        .enumerate()
        .filter(|(_, ms)| **ms > 0)
        .map(|(i, ms)| {
            let (lbl, kind) = labels.get(i).copied().unwrap_or(("Phrase", "phrase"));
            OmkfFsHotCueDto {
                slot: (i + 1) as u8,
                label: lbl.into(),
                seconds: f64::from(*ms) / 1000.0,
                kind: kind.into(),
            }
        })
        .collect();
    let lufs = (s.lufs_x100 > -6900).then(|| f64::from(s.lufs_x100) / 100.0);
    let fam = BoothGenreFamily::from_code(s.genre_family_code);
    let fl = |b: u16| (s.flags_bits & b) != 0;
    CatalogTrackRecord {
        dto: OmkfFsTrackDto {
            id: tid,
            crate_id: cid.to_string(),
            path: fpath.to_string_lossy().into_owned(),
            file_name: fname.to_string(),
            title: if e.title.trim().is_empty() {
                fb.1
            } else {
                e.title.clone()
            },
            artist: if e.artist.trim().is_empty() {
                fb.0
            } else {
                e.artist.clone()
            },
            album: disp.to_string(),
            duration_sec: u32::from(s.duration_sec_u16),
            bpm: (s.bpm_x100 > 0)
                .then(|| f64::from(s.bpm_x100) / 100.0)
                .or(fb.2),
            camelot: omkf::code_to_camelot(s.camelot_code).or(fb.3),
            energy: lufs.map(|l| (((l + 24.0) / 18.0).clamp(0.25, 0.99) * 100.0).round() / 100.0),
            hot_cues: cues,
            scan_level: s.scan_level,
            is_video: ext_vid || fl(omkf::OMKF_FLAG_IS_VIDEO),
            is_fake_320kbps: fl(omkf::OMKF_FLAG_IS_FAKE_320KBPS),
            has_audio_skips: fl(omkf::OMKF_FLAG_HAS_AUDIO_SKIPS),
            is_variable_bpm: fl(omkf::OMKF_FLAG_IS_VARIABLE_BPM),
            has_intro_vocals: fl(omkf::OMKF_FLAG_HAS_INTRO_VOCALS),
            has_outro_vocals: fl(omkf::OMKF_FLAG_HAS_OUTRO_VOCALS),
            replay_gain_db: (s.replay_gain_x100 != 0)
                .then(|| f64::from(s.replay_gain_x100) / 100.0),
            waveform_peaks_b64: s
                .rgb_peaks_256
                .iter()
                .any(|&v| v != 0)
                .then(|| B64.encode(s.rgb_peaks_256)),
            copy_count: 1,
        },
        partial_hash_64kb: s.partial_hash_64kb,
        folder_display_name: disp.to_string(),
        genre_family: (fam != BoothGenreFamily::Unknown).then(|| fam.as_str().to_string()),
        era_year: s.inferred_era_year,
        lufs,
        entry_bpm: (s.entry_bpm_x100 > 0).then(|| f64::from(s.entry_bpm_x100) / 100.0),
        exit_bpm: (s.exit_bpm_x100 > 0).then(|| f64::from(s.exit_bpm_x100) / 100.0),
    }
}
