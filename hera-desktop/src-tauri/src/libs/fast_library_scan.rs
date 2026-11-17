use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use hera_ipc::dj_types::{
    OrganizeFolderRequest, OrganizeFolderResponse, PlannedFileMove, PreGigAuditReport,
    PreGigTrackIssue,
};
use hera_node_rs::curation_rules::{DjCurationRuleSet, GoldenTransitionPair, TrackCurationHint};
use hera_node_rs::forensics::decode_audio_file_partial;
use hera_node_rs::genre_intelligence::{
    BoothGenreFamily, classify_track_genre_and_era, era_decade_label,
};
use hera_node_rs::scanner::omkf_capsule::{
    OMKF_FLAG_HAS_AUDIO_SKIPS, OMKF_FLAG_HAS_BPM_TRUST_DIFF, OMKF_FLAG_HAS_INTRO_VOCALS,
    OMKF_FLAG_HAS_OUTRO_VOCALS, OMKF_FLAG_IS_FAKE_320KBPS, OMKF_FLAG_IS_MIX_OVER_10M,
    OMKF_FLAG_IS_OWN_PRODUCTION, OMKF_FLAG_IS_TRANSITION, OMKF_FLAG_IS_VARIABLE_BPM,
    OMKF_FLAG_IS_VIDEO, OmkfCrateMdMetadata, OmkfHeader, OmkfTrackEntry, OmkfTrackSlot,
    camelot_code_mask_bit, camelot_to_code, compute_centroid_top24_i8,
};
use hera_node_rs::shazam_id::{extract_constellation_landmarks, match_landmarks_against_index};
use hera_node_rs::synergy::{CrateTrackCandidate, SetSpineConfig, optimize_crate_with_dj_rules};
use log::info;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{QueryBuilder, Row, Sqlite, SqliteConnection, sqlite::SqliteRow};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::sync_channel;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Runtime};
use uuid::Uuid;

use crate::libs::database::{DB, SUPPORTED_PLAYLISTS_EXTENSIONS, SUPPORTED_TRACKS_EXTENSIONS};
use crate::libs::db_agent_queries::{AgentTrackFilter, query_agent_tracks_filtered};
use crate::libs::error::AnyResult;
use crate::libs::events::IPCEvent;
use crate::libs::omkf_sync::{
    DiskEntrySeal, assign_crate_ids_and_mark_dirty, compute_folder_crate_id,
    flush_dirty_omkf_capsules, folder_matches_omkf_capsule, hydrate_folder_from_omkf_zero_dsp,
    write_folder_omkf_from_db,
};
use crate::libs::playlist::Playlist;
use crate::libs::track::{HotCuePin, Track, get_indexed_track_fast};
use crate::plugins::db::{DBState, ScanProgress};
use crate::plugins::hera_dj::workability::{
    ExternalDjSoftwareIndex, is_junk_audio_path, is_scan_cancelled, reset_scan_flag,
};

fn is_supported_audio(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    let ext_lower = ext.to_ascii_lowercase();
    SUPPORTED_TRACKS_EXTENSIONS
        .iter()
        .any(|&e| e.eq_ignore_ascii_case(&ext_lower))
}

fn is_supported_playlist(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    let ext_lower = ext.to_ascii_lowercase();
    SUPPORTED_PLAYLISTS_EXTENSIONS
        .iter()
        .any(|&e| e.eq_ignore_ascii_case(&ext_lower))
}

const CUE_COLORS: [&str; 8] = [
    "#22c55e", "#38bdf8", "#ef4444", "#f59e0b", "#a855f7", "#06b6d4", "#ec4899", "#eab308",
];

#[cfg(test)]
pub fn import_tracks_with_index(
    paths: &[PathBuf],
    software_index: &ExternalDjSoftwareIndex,
) -> Vec<Track> {
    process_batch_parallel(paths, software_index)
}

fn process_batch_parallel(
    paths: &[PathBuf],
    software_index: &ExternalDjSoftwareIndex,
) -> Vec<Track> {
    paths
        .par_iter()
        .filter_map(|path| {
            let mut track = get_indexed_track_fast(path).ok()?;

            if let Some(cached) = software_index.lookup(path) {
                if (track.album_artist.is_empty() || track.album_artist == "Unknown Artist")
                    && let Some(ref a) = cached.author
                {
                    track.album_artist = a.clone();
                    track.artists = vec![a.clone()];
                }
                if (track.title.is_empty() || track.title == "Unknown")
                    && let Some(ref t) = cached.title
                {
                    track.title = t.clone();
                }
                if track.duration == 0
                    && let Some(dur) = cached.duration_sec
                {
                    track.duration = dur.round() as u32;
                }
                if let Some(bpm) = cached.bpm {
                    track.bpm = Some(bpm);
                    track.legacy_bpm = Some(bpm);
                }
                if let Some(br) = cached.bitrate_kbps {
                    track.bitrate_kbps = Some(i64::from(br));
                }
                if let Some(ref key) = cached.key {
                    track.camelot = Some(key.clone());
                }

                if !cached.cues.is_empty() {
                    let pins: Vec<HotCuePin> = cached
                        .cues
                        .iter()
                        .take(8)
                        .enumerate()
                        .map(|(idx, cue)| {
                            let num = if (1..=8).contains(&cue.num) {
                                cue.num
                            } else {
                                (idx + 1) as u8
                            };
                            let color_idx =
                                usize::from(num.saturating_sub(1)).min(CUE_COLORS.len() - 1);
                            let color = CUE_COLORS[color_idx];
                            let role = match num {
                                1 => "intro",
                                2 => "breakdown",
                                3 => "drop",
                                4 => "outro",
                                5 => "phrase",
                                6 => "bridge",
                                7 => "climax",
                                _ => "outro",
                            };
                            HotCuePin {
                                num,
                                name: if cue.name.is_empty() {
                                    format!("{num}:Cue")
                                } else {
                                    cue.name.clone()
                                },
                                seconds: ((cue.seconds * 1000.0).round()) / 1000.0,
                                color: color.to_string(),
                                role: role.to_string(),
                                origin: Some("dj_manual".to_string()),
                            }
                        })
                        .collect();

                    let cues_count = pins.len();
                    track.hot_cues = Some(pins);

                    if track.bpm.is_some() && track.camelot.is_some() && cues_count >= 4 {
                        track.scan_level = Some(2);
                    } else if track.bpm.is_some() && track.camelot.is_some() {
                        track.scan_level = Some(1);
                    } else {
                        track.scan_level = Some(0);
                    }
                } else if track.bpm.is_some() && track.camelot.is_some() {
                    track.scan_level = Some(1);
                } else {
                    track.scan_level = Some(0);
                }
            }

            // Duration filter: ONLY drop tracks when track.duration > 0 && track.duration < 25!
            // NEVER drop track.duration == 0 videos/tagless files during Level 0.
            if track.duration > 0 && track.duration < 25 {
                return None;
            }

            Some(track)
        })
        .collect()
}

pub struct ScanExecutionResult {
    pub scanned_tracks_count: usize,
    pub track_failures: usize,
    pub found_playlists: Vec<PathBuf>,
}

pub async fn execute_streaming_library_scan<R: Runtime>(
    app_handle: &AppHandle<R>,
    db_state: &DBState,
    import_paths: Vec<PathBuf>,
    existing_paths: HashSet<PathBuf>,
    refresh: bool,
) -> AnyResult<ScanExecutionResult> {
    reset_scan_flag();
    let software_index = ExternalDjSoftwareIndex::build_fast_index();

    let (path_tx, path_rx) = sync_channel::<PathBuf>(1024);
    let (playlist_tx, playlist_rx) = sync_channel::<PathBuf>(512);
    let (omkf_dir_tx, omkf_dir_rx) = sync_channel::<PathBuf>(256);
    let total_discovered = Arc::new(AtomicUsize::new(0));

    let walk_paths = import_paths.clone();
    let walk_existing = Arc::new(existing_paths);
    let walk_discovered = Arc::clone(&total_discovered);

    std::thread::spawn(move || {
        let mut zero_import_dirs = HashSet::<PathBuf>::new();
        for root in &walk_paths {
            if is_scan_cancelled() {
                break;
            }
            for entry in walkdir::WalkDir::new(root)
                .follow_links(true)
                .into_iter()
                .filter_entry(|e| e.file_name() != ".hera" && !is_junk_audio_path(e.path()))
                .filter_map(|e| e.ok())
            {
                if is_scan_cancelled() {
                    break;
                }
                let path = entry.path();
                if entry.file_type().is_dir() && !refresh && folder_matches_omkf_capsule(path) {
                    let has_new =
                        std::fs::read_dir(path)
                            .into_iter()
                            .flatten()
                            .flatten()
                            .any(|e| {
                                let p = e.path();
                                p.is_file() && is_supported_audio(&p) && !walk_existing.contains(&p)
                            });
                    if has_new && zero_import_dirs.insert(path.to_path_buf()) {
                        let _ = omkf_dir_tx.send(path.to_path_buf());
                    }
                    continue;
                }
                if entry.file_type().is_file() {
                    if is_supported_playlist(path) {
                        let _ = playlist_tx.send(path.to_path_buf());
                    } else if is_supported_audio(path)
                        && (refresh || !walk_existing.contains(path))
                        && !path.parent().is_some_and(|p| zero_import_dirs.contains(p))
                    {
                        walk_discovered.fetch_add(1, Ordering::Relaxed);
                        if path_tx.send(path.to_path_buf()).is_err() {
                            return;
                        }
                    }
                }
            }
        }
    });

    let mut total_scanned = 0usize;
    let mut total_cached_fast = 0usize;
    let mut total_cached_full = 0usize;
    let mut total_raw_candidates = 0usize;
    let mut current_batch = Vec::with_capacity(250);
    let mut target_batch_size = 60usize;
    let start_time = Instant::now();

    while let Ok(path) = path_rx.recv() {
        if is_scan_cancelled() {
            info!("Fast library scan cancelled by user (< 100ms abort)");
            break;
        }

        current_batch.push(path);
        total_raw_candidates += 1;

        if current_batch.len() >= target_batch_size {
            let (count, fast_c, full_c) = commit_batch(
                &current_batch,
                &software_index,
                db_state,
                refresh,
                app_handle,
            )
            .await?;
            total_scanned += count;
            total_cached_fast += fast_c;
            total_cached_full += full_c;
            current_batch.clear();
            target_batch_size = 250;

            let est_total = total_discovered.load(Ordering::Relaxed).max(total_scanned);
            let _ = app_handle.emit(
                IPCEvent::LibraryScanProgress.as_ref(),
                ScanProgress {
                    current: total_scanned,
                    total: est_total,
                },
            );

            let elapsed = start_time.elapsed().as_secs_f64().max(0.001);
            let tracks_per_sec = (total_scanned as f64) / elapsed;
            let _ = app_handle.emit(
                "hera:scanner-telemetry",
                json!({
                    "phase": "indexing",
                    "indexedDone": total_scanned,
                    "fastDone": total_cached_fast,
                    "fullDone": total_cached_full,
                    "total": est_total,
                    "tracksPerSec": (tracks_per_sec * 10.0).round() / 10.0,
                    "etaSec": 0,
                    "governorActive": false
                }),
            );
        }
    }

    if !current_batch.is_empty() && !is_scan_cancelled() {
        let (count, fast_c, full_c) = commit_batch(
            &current_batch,
            &software_index,
            db_state,
            refresh,
            app_handle,
        )
        .await?;
        total_scanned += count;
        total_cached_fast += fast_c;
        total_cached_full += full_c;
        current_batch.clear();
    }

    let omkf_dirs: Vec<PathBuf> = omkf_dir_rx.try_iter().collect();
    if !omkf_dirs.is_empty() && !is_scan_cancelled() {
        let mut guard = db_state.get_lock().await;
        for dir in omkf_dirs {
            if let Ok(n) = hydrate_folder_from_omkf_zero_dsp(&mut guard.connection, &dir).await {
                total_scanned += n;
                total_cached_fast += n;
                total_cached_full += n;
            }
        }
        let _ = flush_dirty_omkf_capsules(&mut guard.connection).await;
    } else if total_scanned > 0 {
        let mut guard = db_state.get_lock().await;
        let _ = flush_dirty_omkf_capsules(&mut guard.connection).await;
    }

    let final_total = total_scanned;
    let _ = app_handle.emit(
        IPCEvent::LibraryScanProgress.as_ref(),
        ScanProgress {
            current: final_total,
            total: final_total,
        },
    );

    let elapsed = start_time.elapsed().as_secs_f64().max(0.001);
    let tracks_per_sec = (final_total as f64) / elapsed;
    let next_phase = if total_cached_fast < final_total && !is_scan_cancelled() {
        "analyzing"
    } else {
        "idle"
    };
    let _ = app_handle.emit(
        "hera:scanner-telemetry",
        json!({
            "phase": next_phase,
            "indexedDone": final_total,
            "fastDone": total_cached_fast,
            "fullDone": total_cached_full,
            "total": final_total,
            "tracksPerSec": (tracks_per_sec * 10.0).round() / 10.0,
            "etaSec": 0,
            "governorActive": false
        }),
    );

    let mut found_playlists = Vec::new();
    while let Ok(pl) = playlist_rx.try_recv() {
        found_playlists.push(pl);
    }

    let failures = total_raw_candidates.saturating_sub(total_scanned);

    Ok(ScanExecutionResult {
        scanned_tracks_count: total_scanned,
        track_failures: failures,
        found_playlists,
    })
}

async fn commit_batch<R: Runtime>(
    batch_paths: &[PathBuf],
    index: &ExternalDjSoftwareIndex,
    db: &DBState,
    refresh: bool,
    app: &AppHandle<R>,
) -> AnyResult<(usize, usize, usize)> {
    if batch_paths.is_empty() {
        return Ok((0, 0, 0));
    }
    let batch = process_batch_parallel(batch_paths, index);
    let batch_len = batch.len();
    let fast_cached = batch
        .iter()
        .filter(|t| t.scan_level.unwrap_or(0) >= 1)
        .count();
    let full_cached = batch
        .iter()
        .filter(|t| t.scan_level.unwrap_or(0) >= 2)
        .count();
    if !batch.is_empty() {
        let mut guard = db.get_lock().await;
        guard.upsert_scanned_tracks_batch(&batch, refresh).await?;
        assign_crate_ids_and_mark_dirty(&mut guard.connection, &batch).await;
        drop(guard);
        let _ = app.emit("hera:scanner-tracks-added", &batch);
    }
    Ok((batch_len, fast_cached, full_cached))
}

pub(crate) struct LoadedFolderOmkfRow {
    pub entry: OmkfTrackEntry,
    pub camelot: Option<String>,
    pub genre_family: Option<String>,
    pub lufs: f64,
    pub is_video: bool,
    pub is_full: bool,
}

pub(crate) async fn sql_upsert_hydrated_omkf_entry_tx(
    tx: &mut SqliteConnection,
    cid: &str,
    tid: &str,
    path: &Path,
    e: &OmkfTrackEntry,
    cam: &str,
    genre_s: Option<&str>,
) -> Result<(), String> {
    let s = &e.slot;
    let pins: Vec<HotCuePin> = s
        .hot_cues_ms
        .iter()
        .enumerate()
        .filter(|(_, ms)| **ms > 0)
        .map(|(i, ms)| HotCuePin {
            num: (i + 1) as u8,
            name: format!("Pad {}", i + 1),
            seconds: f64::from(*ms) / 1000.0,
            color: CUE_COLORS[i % CUE_COLORS.len()].to_string(),
            role: "cue".to_string(),
            origin: Some("omkf".to_string()),
        })
        .collect();
    let emb = s
        .embedding_64d
        .iter()
        .any(|&v| v != 0)
        .then(|| B64.encode(s.embedding_64d.map(|v| v as u8)));
    let rgb = s
        .rgb_peaks_256
        .iter()
        .any(|&v| v != 0)
        .then(|| B64.encode(s.rgb_peaks_256));
    let fl = |bit: u16| i64::from((s.flags_bits & bit) != 0);
    let q = "INSERT INTO tracks (id,path,title,album,album_artist,artists,genres,duration,bpm,camelot,hot_cues,scan_level,crate_id,is_video,is_transition_candidate,is_variable_bpm,is_fake_320kbps,has_audio_skips,has_intro_vocals,has_outro_vocals,is_own_production,is_mix_over_10m,has_bpm_trust_diff,entry_bpm,exit_bpm,loudness_lufs,replay_gain_db,sub_phase_corr,embedding_64d_b64,waveform_peaks_b64,inferred_genre_family,genre_confidence,inferred_era_year,bitrate_kbps,sample_rate_hz)
             VALUES (?1,?2,?3,'Single / Cabina',?4,?5,'[]',?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33)
             ON CONFLICT(path) DO UPDATE SET crate_id=excluded.crate_id, scan_level=MAX(tracks.scan_level, excluded.scan_level), bpm=excluded.bpm, camelot=excluded.camelot, hot_cues=excluded.hot_cues, embedding_64d_b64=COALESCE(excluded.embedding_64d_b64, tracks.embedding_64d_b64), waveform_peaks_b64=COALESCE(excluded.waveform_peaks_b64, tracks.waveform_peaks_b64)";
    sqlx::query(q)
        .bind(tid)
        .bind(path.to_string_lossy().to_string())
        .bind(&e.title)
        .bind(&e.artist)
        .bind(serde_json::to_string(&[&e.artist]).unwrap_or_else(|_| "[]".into()))
        .bind(i64::from(s.duration_sec_u16))
        .bind(f64::from(s.bpm_x100) / 100.0)
        .bind(cam)
        .bind(serde_json::to_string(&pins).unwrap_or_else(|_| "[]".into()))
        .bind(i64::from(s.scan_level))
        .bind(cid)
        .bind(fl(OMKF_FLAG_IS_VIDEO))
        .bind(fl(OMKF_FLAG_IS_TRANSITION))
        .bind(fl(OMKF_FLAG_IS_VARIABLE_BPM))
        .bind(fl(OMKF_FLAG_IS_FAKE_320KBPS))
        .bind(fl(OMKF_FLAG_HAS_AUDIO_SKIPS))
        .bind(fl(OMKF_FLAG_HAS_INTRO_VOCALS))
        .bind(fl(OMKF_FLAG_HAS_OUTRO_VOCALS))
        .bind(fl(OMKF_FLAG_IS_OWN_PRODUCTION))
        .bind(fl(OMKF_FLAG_IS_MIX_OVER_10M))
        .bind(fl(OMKF_FLAG_HAS_BPM_TRUST_DIFF))
        .bind((s.entry_bpm_x100 > 0).then(|| f64::from(s.entry_bpm_x100) / 100.0))
        .bind((s.exit_bpm_x100 > 0).then(|| f64::from(s.exit_bpm_x100) / 100.0))
        .bind(f64::from(s.lufs_x100) / 100.0)
        .bind(f64::from(s.replay_gain_x100) / 100.0)
        .bind(f64::from(s.sub_phase_x1000) / 1000.0)
        .bind(emb)
        .bind(rgb)
        .bind(genre_s)
        .bind(f64::from(s.genre_confidence_pct) / 100.0)
        .bind((s.inferred_era_year > 0).then(|| i64::from(s.inferred_era_year)))
        .bind((s.bitrate_kbps > 0).then(|| i64::from(s.bitrate_kbps)))
        .bind((s.sample_rate_div100 > 0).then(|| i64::from(s.sample_rate_div100) * 100))
        .execute(tx)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub(crate) async fn sql_load_folder_omkf_entries(
    conn: &mut SqliteConnection,
    cid: &str,
    folder: &Path,
    disk: &[DiskEntrySeal],
) -> Result<Vec<LoadedFolderOmkfRow>, String> {
    let by_path: HashMap<String, &DiskEntrySeal> = disk
        .iter()
        .map(|d| (d.path.to_string_lossy().to_string(), d))
        .collect();
    let rows = sqlx::query("SELECT * FROM tracks WHERE crate_id = ?1 OR path LIKE ?2")
        .bind(cid)
        .bind(format!("{}%", folder.to_string_lossy()))
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        let rpath: String = r.try_get("path").unwrap_or_default();
        let Some(d) = by_path.get(&rpath).copied().or_else(|| {
            Path::new(&rpath)
                .parent()
                .filter(|p| *p == folder)
                .and_then(|_| {
                    disk.iter()
                        .find(|x| x.path.file_name() == Path::new(&rpath).file_name())
                })
        }) else {
            continue;
        };
        let gi = |k: &str| r.try_get::<i64, _>(k).unwrap_or(0);
        let gf = |k: &str| r.try_get::<Option<f64>, _>(k).ok().flatten();
        let gs = |k: &str| r.try_get::<Option<String>, _>(k).ok().flatten();
        let scan_lvl = gi("scan_level");
        let mut s = OmkfTrackSlot {
            file_size_bytes: d.size,
            mtime_ns: d.mtime_ns,
            scan_level: scan_lvl.clamp(-1, 2) as i8,
            duration_sec_u16: gi("duration").clamp(0, 65535) as u16,
            ..Default::default()
        };
        let cam = gs("camelot");
        s.bpm_x100 = (gf("bpm").unwrap_or(0.0).max(0.0) * 100.0).round() as u16;
        s.entry_bpm_x100 = gf("entry_bpm").map_or(0, |b| (b * 100.0) as u16);
        s.exit_bpm_x100 = gf("exit_bpm").map_or(0, |b| (b * 100.0) as u16);
        s.camelot_code = camelot_to_code(cam.as_deref().unwrap_or(""));
        let flag_cols = [
            ("is_video", OMKF_FLAG_IS_VIDEO),
            ("is_transition_candidate", OMKF_FLAG_IS_TRANSITION),
            ("is_variable_bpm", OMKF_FLAG_IS_VARIABLE_BPM),
            ("is_fake_320kbps", OMKF_FLAG_IS_FAKE_320KBPS),
            ("has_audio_skips", OMKF_FLAG_HAS_AUDIO_SKIPS),
            ("has_intro_vocals", OMKF_FLAG_HAS_INTRO_VOCALS),
            ("has_outro_vocals", OMKF_FLAG_HAS_OUTRO_VOCALS),
            ("is_own_production", OMKF_FLAG_IS_OWN_PRODUCTION),
            ("is_mix_over_10m", OMKF_FLAG_IS_MIX_OVER_10M),
            ("has_bpm_trust_diff", OMKF_FLAG_HAS_BPM_TRUST_DIFF),
        ];
        s.flags_bits = flag_cols
            .into_iter()
            .fold(0, |a, (c, f)| if gi(c) != 0 { a | f } else { a });
        let lufs = gf("loudness_lufs").unwrap_or(-14.0);
        s.lufs_x100 = (gf("loudness_lufs").unwrap_or(-70.0) * 100.0) as i16;
        s.replay_gain_x100 = (gf("replay_gain_db").unwrap_or(0.0) * 100.0) as i16;
        s.sub_phase_x1000 = (gf("sub_phase_corr").unwrap_or(1.0) * 1000.0) as i16;
        let cues_str: String = r.try_get("hot_cues").unwrap_or_default();
        for p in serde_json::from_str::<Vec<HotCuePin>>(&cues_str).unwrap_or_default() {
            if (1..=8).contains(&p.num) {
                s.hot_cues_ms[usize::from(p.num - 1)] =
                    (p.seconds.max(0.0) * 1000.0).round() as u32;
            }
        }
        if let Some(raw) = gs("embedding_64d_b64").and_then(|b| B64.decode(b).ok()) {
            for (i, b) in raw.into_iter().take(64).enumerate() {
                s.embedding_64d[i] = b as i8;
            }
        }
        if let Some(raw) = gs("waveform_peaks_b64").and_then(|b| B64.decode(b).ok()) {
            for (i, b) in raw.into_iter().take(768).enumerate() {
                s.rgb_peaks_256[i] = b;
            }
        }
        let gen_fam = gs("inferred_genre_family");
        let gname = gen_fam.as_deref().unwrap_or("");
        s.genre_family_code = (0..=14_u8)
            .find(|&c| BoothGenreFamily::from_code(c).as_str() == gname)
            .unwrap_or(0);
        s.inferred_era_year = r
            .try_get::<Option<i64>, _>("inferred_era_year")
            .ok()
            .flatten()
            .filter(|&y| y > 0)
            .map_or(0, |y| y as u16);
        s.genre_confidence_pct =
            (gf("genre_confidence").unwrap_or(0.0) * 100.0).clamp(0.0, 100.0) as u8;
        s.bitrate_kbps = r
            .try_get::<Option<i64>, _>("bitrate_kbps")
            .ok()
            .flatten()
            .unwrap_or(0)
            .clamp(0, 65535) as u16;
        s.sample_rate_div100 = (r
            .try_get::<Option<i64>, _>("sample_rate_hz")
            .ok()
            .flatten()
            .unwrap_or(0)
            / 100)
            .clamp(0, 65535) as u16;
        let tid: String = r.try_get("id").unwrap_or_default();
        let _ = sqlx::query("UPDATE tracks SET crate_id = ? WHERE id = ? AND crate_id IS NULL")
            .bind(cid)
            .bind(&tid)
            .execute(&mut *conn)
            .await;
        let entry = OmkfTrackEntry {
            slot: s,
            filename: d.filename.clone(),
            artist: r.try_get("album_artist").unwrap_or_default(),
            title: r.try_get("title").unwrap_or_default(),
        };
        out.push(LoadedFolderOmkfRow {
            entry,
            camelot: cam,
            genre_family: gen_fam,
            lufs,
            is_video: gi("is_video") != 0,
            is_full: scan_lvl >= 2,
        });
    }
    Ok(out)
}

pub(crate) async fn sql_upsert_folder_crate(
    conn: &mut SqliteConnection,
    cid: &str,
    folder: &Path,
    hdr: &OmkfHeader,
    md: &OmkfCrateMdMetadata,
    full_n: i64,
    updated_ms: i64,
) -> Result<(), String> {
    let disp = folder
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Carpeta")
        .to_string();
    let cent_b64 = B64.encode(hdr.centroid_top24_i8.map(|v| v as u8));
    let bmin = (hdr.bpm_min_x100 > 0).then(|| f64::from(hdr.bpm_min_x100) / 100.0);
    let bmax = (hdr.bpm_max_x100 > 0).then(|| f64::from(hdr.bpm_max_x100) / 100.0);
    let sql = "INSERT INTO folder_crates (crate_id,folder_path,display_name,tree_seal_64,dir_mtime_ns,track_count,video_count,full_count,bpm_min,bpm_max,dominant_camelot,camelot_mask,dominant_genre,era_span,lufs_avg,centroid_64d_b64,updated_at_unix_ms)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
               ON CONFLICT(crate_id) DO UPDATE SET folder_path=excluded.folder_path,display_name=excluded.display_name,tree_seal_64=excluded.tree_seal_64,dir_mtime_ns=excluded.dir_mtime_ns,track_count=excluded.track_count,video_count=excluded.video_count,full_count=excluded.full_count,bpm_min=excluded.bpm_min,bpm_max=excluded.bpm_max,dominant_camelot=excluded.dominant_camelot,camelot_mask=excluded.camelot_mask,dominant_genre=excluded.dominant_genre,era_span=excluded.era_span,lufs_avg=excluded.lufs_avg,centroid_64d_b64=excluded.centroid_64d_b64,updated_at_unix_ms=excluded.updated_at_unix_ms";
    sqlx::query(sql)
        .bind(cid)
        .bind(folder.to_string_lossy().to_string())
        .bind(disp)
        .bind(format!("{:016x}", hdr.tree_seal_64))
        .bind(hdr.dir_mtime_ns)
        .bind(i64::from(hdr.track_count))
        .bind(i64::from(md.video_count))
        .bind(full_n)
        .bind(bmin)
        .bind(bmax)
        .bind(&md.dominant_camelot)
        .bind(i64::from(hdr.camelot_mask))
        .bind(&md.dominant_genre)
        .bind(&md.era_span)
        .bind(f64::from(md.lufs_avg))
        .bind(cent_b64)
        .bind(updated_ms)
        .execute(conn)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn summarize_omkf_crate_md(
    cid: &str,
    seal: u64,
    bmin: u16,
    bmax: u16,
    items: &[(&str, &str, u16, f64, bool, bool)],
) -> (OmkfCrateMdMetadata, i64) {
    let (mut vid, mut full, mut lufs, mut ymin, mut ymax) =
        (0_u32, 0_i64, 0.0_f64, u16::MAX, 0_u16);
    let (mut cam_h, mut gen_h) = (HashMap::<&str, usize>::new(), HashMap::<&str, usize>::new());
    for &(c, g, era, l, v, f) in items {
        if v {
            vid += 1;
        }
        if f {
            full += 1;
        }
        lufs += l;
        if era > 0 {
            ymin = ymin.min(era);
            ymax = ymax.max(era);
        }
        if !c.is_empty() {
            *cam_h.entry(c).or_default() += 1;
        }
        if !g.is_empty() {
            *gen_h.entry(g).or_default() += 1;
        }
    }
    let mode = |h: HashMap<&str, usize>| {
        h.into_iter()
            .max_by_key(|x| x.1)
            .map(|x| x.0.to_string())
            .unwrap_or_default()
    };
    let bpm_span = if bmin > 0 && bmin <= bmax {
        format!("{}-{}", bmin / 100, bmax / 100)
    } else {
        String::new()
    };
    let era_span = if ymin <= ymax {
        format!("{ymin}-{ymax}")
    } else {
        String::new()
    };
    let lufs_avg = (lufs / items.len().max(1) as f64) as f32;
    let md = OmkfCrateMdMetadata {
        crate_id: cid.into(),
        tree_seal_64: seal,
        track_count: items.len() as u32,
        video_count: vid,
        bpm_span,
        dominant_camelot: mode(cam_h),
        dominant_genre: mode(gen_h),
        era_span,
        lufs_avg,
        health: "ok".into(),
    };
    (md, full)
}

pub(crate) fn pack_omkf_header_and_entries(
    cid: &str,
    tree_seal: u64,
    dir_ns: i64,
    loaded: Vec<LoadedFolderOmkfRow>,
) -> (OmkfHeader, Vec<OmkfTrackEntry>, OmkfCrateMdMetadata, i64) {
    let (mut entries, mut slots, mut mask, mut bmin, mut bmax) =
        (Vec::new(), Vec::new(), 0_u32, u16::MAX, 0_u16);
    for item in &loaded {
        let s = &item.entry.slot;
        if s.bpm_x100 > 0 {
            bmin = bmin.min(s.bpm_x100);
            bmax = bmax.max(s.bpm_x100);
        }
        mask |= camelot_code_mask_bit(s.camelot_code);
        slots.push(s.clone());
        entries.push(item.entry.clone());
    }
    let stat_rows: Vec<_> = loaded
        .iter()
        .map(|x| {
            (
                x.camelot.as_deref().unwrap_or(""),
                x.genre_family.as_deref().unwrap_or(""),
                x.entry.slot.inferred_era_year,
                x.lufs,
                x.is_video,
                x.is_full,
            )
        })
        .collect();
    let hdr = OmkfHeader {
        tree_seal_64: tree_seal,
        dir_mtime_ns: dir_ns,
        track_count: entries.len() as u32,
        strtab_offset: 64,
        bpm_min_x100: if bmin <= bmax { bmin } else { 0 },
        bpm_max_x100: bmax,
        camelot_mask: mask,
        centroid_top24_i8: compute_centroid_top24_i8(&slots),
    };
    let (md, full_n) = summarize_omkf_crate_md(
        cid,
        tree_seal,
        hdr.bpm_min_x100,
        hdr.bpm_max_x100,
        &stat_rows,
    );
    (hdr, entries, md, full_n)
}

#[allow(dead_code)]
pub(crate) fn parse_crate_md_sections(raw: &str) -> (String, String, Vec<String>) {
    let body = raw
        .trim_start()
        .strip_prefix("---")
        .and_then(|s| s.trim_start_matches(['\r', '\n']).split_once("\n---"))
        .map_or(raw, |(_, rest)| rest.trim_start_matches(['\r', '\n']));
    let front = if raw.trim_start().starts_with("---") && body.len() < raw.len() {
        raw[..raw.len() - body.len()].to_string()
    } else {
        String::new()
    };
    let (mut rules, mut ledger, mut in_ledger) = (Vec::new(), Vec::new(), false);
    for line in body.lines() {
        let t = line.trim();
        if t.starts_with("# Ledger Reversible") {
            in_ledger = true;
            continue;
        }
        if t.starts_with("# Intención y Reglas de Cabina") {
            in_ledger = false;
            continue;
        }
        if in_ledger && !t.is_empty() {
            ledger.push(t.to_string());
        } else if !in_ledger {
            rules.push(line);
        }
    }
    (front, rules.join("\n").trim().to_string(), ledger)
}

#[allow(dead_code)]
pub(crate) fn write_crate_md_sections(
    dir: &Path,
    rules: &str,
    ledger: &[String],
) -> Result<(), String> {
    let hera = dir.join(".hera");
    std::fs::create_dir_all(&hera).map_err(|e| e.to_string())?;
    let md_path = hera.join("crate.md");
    let existing = std::fs::read_to_string(&md_path).unwrap_or_default();
    let (front, _, _) = parse_crate_md_sections(&existing);
    let mut out = front;
    out.push_str("# Intención y Reglas de Cabina\n");
    if !rules.trim().is_empty() {
        out.push_str(rules.trim());
        out.push('\n');
    }
    out.push_str("\n# Ledger Reversible\n");
    for l in ledger {
        out.push_str(l);
        out.push('\n');
    }
    let tmp = hera.join("crate.md.tmp");
    std::fs::write(&tmp, out)
        .and_then(|_| std::fs::rename(&tmp, &md_path))
        .map_err(|e| e.to_string())?;
    crate::libs::omkf_sync::hide_hera_dir_if_windows(&hera);
    Ok(())
}

fn target_organize_bucket(strategy: &str, r: &SqliteRow) -> (String, String) {
    let dur = r.try_get::<i64, _>("duration").unwrap_or(0);
    let bpm = r
        .try_get::<Option<f64>, _>("bpm")
        .ok()
        .flatten()
        .unwrap_or(124.0);
    let scan_lvl = r.try_get::<i64, _>("scan_level").unwrap_or(0);
    let bad = scan_lvl < 0
        || r.try_get::<i64, _>("has_audio_skips").unwrap_or(0) != 0
        || r.try_get::<i64, _>("is_fake_320kbps").unwrap_or(0) != 0;
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
            let title: String = r.try_get("title").unwrap_or_default();
            let artist: String = r.try_get("album_artist").unwrap_or_default();
            let path: String = r.try_get("path").unwrap_or_default();
            let fam = r
                .try_get::<Option<String>, _>("inferred_genre_family")
                .ok()
                .flatten();
            let era = r
                .try_get::<Option<i64>, _>("inferred_era_year")
                .ok()
                .flatten()
                .map(|y| y as u16);
            let prof = classify_track_genre_and_era(&artist, &title, &[], &path, era, None);
            let g = fam.filter(|s| !s.is_empty()).unwrap_or(prof.family_name);
            let dec = era_decade_label(era.unwrap_or(prof.inferred_era_year));
            (
                format!("{g}_{dec}").replace(['/', ' '], "_"),
                "Agrupación Genre × Era".into(),
            )
        }
        _ => {
            if dur > 600 {
                return ("_Mixes_10m_Plus".into(), "Sesión > 10 min".into());
            }
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

#[allow(dead_code)]
pub(crate) async fn run_omkf_organize_folder(
    conn: &mut SqliteConnection,
    req: OrganizeFolderRequest,
) -> Result<OrganizeFolderResponse, String> {
    let fp: String = sqlx::query_scalar("SELECT folder_path FROM folder_crates WHERE crate_id = ?")
        .bind(&req.crate_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Crate de carpeta no encontrado".to_string())?;
    let base_dir = PathBuf::from(&fp);
    let md_raw = std::fs::read_to_string(base_dir.join(".hera/crate.md")).unwrap_or_default();
    let (_, rules_md, mut ledger) = parse_crate_md_sections(&md_raw);
    let mut moves = Vec::new();
    let mut rolled_back = false;
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
        rolled_back = !req.dry_run && !moves.is_empty();
    } else {
        let rows = sqlx::query("SELECT id,path,title,album_artist,duration,bpm,scan_level,has_audio_skips,is_fake_320kbps,inferred_genre_family,inferred_era_year FROM tracks WHERE crate_id = ?")
            .bind(&req.crate_id)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| e.to_string())?;
        for r in &rows {
            let tid: String = r.try_get("id").unwrap_or_default();
            let src: String = r.try_get("path").unwrap_or_default();
            let (bucket, reason) = target_organize_bucket(&req.strategy, r);
            if bucket.is_empty() {
                continue;
            }
            let Some(fname) = Path::new(&src).file_name() else {
                continue;
            };
            let dst_s = base_dir
                .join(&bucket)
                .join(fname)
                .to_string_lossy()
                .to_string();
            if dst_s != src {
                moves.push(PlannedFileMove {
                    track_id: tid,
                    source_path: src,
                    target_path: dst_s,
                    reason,
                    bucket_label: bucket,
                });
            }
        }
    }
    let tx_id = format!("tx-{}", &Uuid::new_v4().to_string()[..8]);
    let mut applied = 0_usize;
    if !req.dry_run {
        if !base_dir.is_dir() {
            return Err("Carpeta desconectada en disco".into());
        }
        let mut touch_dirs = vec![base_dir.clone()];
        for m in &moves {
            let (s, d) = (Path::new(&m.source_path), Path::new(&m.target_path));
            let Some(parent) = d.parent() else { continue };
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            if s.exists() {
                std::fs::rename(s, d).map_err(|e| e.to_string())?;
                let new_cid = compute_folder_crate_id(parent);
                let _ = sqlx::query("UPDATE tracks SET path = ?, crate_id = ? WHERE id = ?")
                    .bind(&m.target_path)
                    .bind(new_cid)
                    .bind(&m.track_id)
                    .execute(&mut *conn)
                    .await;
                ledger.push(format!(
                    "- tx:{tx_id} | {} | {} | {}",
                    m.track_id, m.source_path, m.target_path
                ));
                touch_dirs.push(parent.to_path_buf());
                applied += 1;
            }
        }
        write_crate_md_sections(&base_dir, &rules_md, &ledger)?;
        for d in touch_dirs {
            let _ = write_folder_omkf_from_db(conn, &d).await;
        }
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

#[cfg(test)]
mod omkf_organize_tests {
    use super::*;
    use sqlx::{Connection, SqliteConnection};

    #[tokio::test]
    async fn test_organize_preserves_uuid_and_rolls_back() {
        let dir = std::env::temp_dir().join(format!("hera_org_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav = dir.join("fast.wav");
        std::fs::write(&wav, [0_u8; 512]).unwrap();
        let mut c = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/migrations")
            .run_direct(&mut c)
            .await
            .unwrap();
        let cid = compute_folder_crate_id(&dir);
        let src_s = wav.to_string_lossy().to_string();
        sqlx::query("INSERT INTO tracks (id,path,title,album,album_artist,artists,genres,duration,bpm,camelot,hot_cues,scan_level,crate_id) VALUES ('u1',?1,'Fast','EP','DJ','[]','[]',200,132.0,'8A','[]',2,?2)")
            .bind(&src_s).bind(&cid).execute(&mut c).await.unwrap();
        sqlx::query("INSERT INTO folder_crates (crate_id,folder_path,display_name,tree_seal_64,dir_mtime_ns,updated_at_unix_ms) VALUES (?1,?2,'Org','0',0,0)")
            .bind(&cid).bind(dir.to_string_lossy().to_string()).execute(&mut c).await.unwrap();
        let res = run_omkf_organize_folder(
            &mut c,
            OrganizeFolderRequest {
                crate_id: cid.clone(),
                strategy: "by_tempo_zone".into(),
                dry_run: false,
                rollback_tx_id: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(res.applied_count, 1);
        let moved = PathBuf::from(&res.moves[0].target_path);
        assert!(moved.is_file() && !wav.exists());
        let id_still: String = sqlx::query_scalar("SELECT id FROM tracks WHERE path = ?")
            .bind(&res.moves[0].target_path)
            .fetch_one(&mut c)
            .await
            .unwrap();
        assert_eq!(id_still, "u1");
        let rb = run_omkf_organize_folder(
            &mut c,
            OrganizeFolderRequest {
                crate_id: cid,
                strategy: "by_tempo_zone".into(),
                dry_run: false,
                rollback_tx_id: Some(res.tx_id),
            },
        )
        .await
        .unwrap();
        assert!(rb.rolled_back && wav.is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_pre_gig_audit_and_shazam_db() {
        let dir = std::env::temp_dir().join(format!("hera_pg_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let (p_ok, p_vid) = (dir.join("ok.wav"), dir.join("vid.mp4"));
        std::fs::write(&p_ok, [0_u8; 64]).unwrap();
        std::fs::write(&p_vid, [0_u8; 64]).unwrap();
        let mut c = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/migrations")
            .run_direct(&mut c)
            .await
            .unwrap();
        sqlx::query("INSERT INTO tracks (id,path,title,album,album_artist,artists,genres,duration,bpm,camelot,hot_cues,scan_level,sample_rate_hz) VALUES ('t1',?1,'Ok','EP','DJ','[]','[]',200,126.0,'8A','[]',2,44100)").bind(p_ok.to_string_lossy().to_string()).execute(&mut c).await.unwrap();
        sqlx::query("INSERT INTO tracks (id,path,title,album,album_artist,artists,genres,duration,bpm,camelot,hot_cues,scan_level,is_video,sample_rate_hz) VALUES ('t2',?1,'Vid','EP','DJ2','[]','[]',200,127.0,'8A','[]',2,1,48000)").bind(p_vid.to_string_lossy().to_string()).execute(&mut c).await.unwrap();
        let audit = compute_pre_gig_audit(&mut c, "denon_cd_mp3", None)
            .await
            .unwrap();
        assert!(!audit.passed && audit.ready_count == 1 && audit.blocker_count == 2);
        let shaz = run_shazam_identify(&mut c, "t1", None)
            .await
            .unwrap()
            .unwrap();
        assert!(!shaz.matched);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamIdentifyResult {
    pub matched: bool,
    pub track_id: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub confidence: f32,
    pub source: String,
}

#[allow(dead_code)]
pub(crate) async fn compute_pre_gig_audit(
    conn: &mut SqliteConnection,
    target: &str,
    track_ids: Option<&[String]>,
) -> Result<PreGigAuditReport, String> {
    let mut qb: QueryBuilder<'_, Sqlite> = QueryBuilder::new(
        "SELECT id,title,path,scan_level,has_audio_skips,is_video,is_fake_320kbps,sample_rate_hz FROM tracks",
    );
    if let Some(ids) = track_ids.filter(|x| !x.is_empty()) {
        qb.push(" WHERE id IN (");
        let mut sep = qb.separated(",");
        for id in ids {
            sep.push_bind(id);
        }
        qb.push(")");
    } else {
        qb.push(" LIMIT 2500");
    }
    let rows = qb
        .build()
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;
    let (mut issues, mut warn_n, mut block_n, mut flagged) =
        (Vec::new(), 0_usize, 0_usize, HashSet::new());
    let cdj = target == "usb_cdj" || target == "denon_cd_mp3";
    for r in &rows {
        let (tid, title, path): (String, String, String) = (
            r.try_get("id").unwrap_or_default(),
            r.try_get("title").unwrap_or_default(),
            r.try_get("path").unwrap_or_default(),
        );
        let (lvl, skips, vid, fake) = (
            r.try_get::<i64, _>("scan_level").unwrap_or(0),
            r.try_get::<i64, _>("has_audio_skips").unwrap_or(0) != 0,
            r.try_get::<i64, _>("is_video").unwrap_or(0) != 0,
            r.try_get::<i64, _>("is_fake_320kbps").unwrap_or(0) != 0,
        );
        let sr = r.try_get::<Option<i64>, _>("sample_rate_hz").ok().flatten();
        let mut push = |c: &str, sev: &str, d: &str| {
            if sev == "error" {
                block_n += 1;
            } else {
                warn_n += 1;
            }
            flagged.insert(tid.clone());
            issues.push(PreGigTrackIssue {
                track_id: tid.clone(),
                title: title.clone(),
                file_path: path.clone(),
                issue_code: c.into(),
                severity: sev.into(),
                detail: d.into(),
            });
        };
        if lvl < 0 {
            push("corrupt_file", "error", "Archivo corrupto o en cuarentena");
            continue;
        }
        if !path.is_empty() && !Path::new(&path).exists() {
            push("missing_file", "error", "Archivo faltante en disco/USB");
        }
        if skips {
            push("cd_skip_glitch", "error", "Salto o micro-corte detectado");
        }
        if cdj && vid {
            push("video_on_cdj", "error", "Video incompatible con CDJ/Denon");
        }
        if target == "denon_cd_mp3" && sr.is_some_and(|h| h != 44100) {
            push(
                "sample_rate_incompatible",
                "error",
                &format!("Sample rate {} Hz != 44.1 kHz", sr.unwrap_or(0)),
            );
        }
        if fake {
            push(
                "fake_320kbps",
                "warning",
                "Posible falso 320 kbps (<16 kHz)",
            );
        }
    }
    Ok(PreGigAuditReport {
        target_hardware: target.into(),
        total_tracks: rows.len(),
        ready_count: rows.len().saturating_sub(flagged.len()),
        warning_count: warn_n,
        blocker_count: block_n,
        passed: block_n == 0,
        issues,
    })
}

#[allow(dead_code)]
pub(crate) async fn run_shazam_identify(
    conn: &mut SqliteConnection,
    track_id: &str,
    path_opt: Option<&str>,
) -> Result<Option<ShazamIdentifyResult>, String> {
    let miss = |src: &str| {
        Some(ShazamIdentifyResult {
            matched: false,
            track_id: track_id.into(),
            title: None,
            artist: None,
            confidence: 0.0,
            source: src.into(),
        })
    };
    let fp: Option<String> = if let Some(p) = path_opt.filter(|s| !s.is_empty()) {
        Some(p.into())
    } else {
        sqlx::query_scalar("SELECT path FROM tracks WHERE id = ?1")
            .bind(track_id)
            .fetch_optional(&mut *conn)
            .await
            .unwrap_or(None)
    };
    let Some(file_path) = fp.filter(|p| Path::new(p).is_file()) else {
        return Ok(None);
    };
    let q_hashes = tokio::task::spawn_blocking(move || {
        decode_audio_file_partial(Path::new(&file_path), 30.0)
            .ok()
            .map(|d| extract_constellation_landmarks(&d.mono, d.sample_rate))
    })
    .await
    .map_err(|e| e.to_string())?
    .unwrap_or_default();
    if q_hashes.is_empty() {
        return Ok(miss("no_landmarks"));
    }
    let mut qb: QueryBuilder<'_, Sqlite> = QueryBuilder::new(
        "SELECT hash_u64, track_id, time_offset_ms FROM acoustic_landmarks WHERE track_id != ",
    );
    qb.push_bind(track_id).push(" AND hash_u64 IN (");
    let mut sep = qb.separated(",");
    for h in q_hashes.iter().take(400) {
        sep.push_bind(h.hash_u64 as i64);
    }
    qb.push(") LIMIT 2000");
    let hits: Vec<(u64, String, i64)> = qb
        .build()
        .fetch_all(&mut *conn)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            (
                r.try_get::<i64, _>("hash_u64").unwrap_or(0) as u64,
                r.try_get("track_id").unwrap_or_default(),
                r.try_get("time_offset_ms").unwrap_or(0),
            )
        })
        .collect();
    if let Some(m) = match_landmarks_against_index(&q_hashes, &hits) {
        let meta: Option<(String, String)> =
            sqlx::query_as("SELECT title, album_artist FROM tracks WHERE id = ?1")
                .bind(&m.track_id)
                .fetch_optional(&mut *conn)
                .await
                .unwrap_or(None);
        return Ok(Some(ShazamIdentifyResult {
            matched: true,
            track_id: m.track_id,
            title: meta.as_ref().map(|x| x.0.clone()),
            artist: meta.map(|x| x.1),
            confidence: m.confidence,
            source: "local_constellation".into(),
        }));
    }
    Ok(miss("unmatched"))
}

pub(crate) struct CuratedPlaylistSelection {
    pub tracks: Vec<Track>,
    pub routed_to_mixes: Vec<String>,
    pub isolated_out_of_genre: Vec<String>,
    pub excluded_tracks: Vec<String>,
}

pub(crate) async fn curate_tracks_with_dj_rules(
    conn: &mut SqliteConnection,
    brief: &str,
    tracks: Vec<Track>,
    allow_crossover: bool,
    target_genre: Option<&str>,
    target_era: Option<u16>,
    max_tracks: Option<usize>,
) -> CuratedPlaylistSelection {
    let gp_rows: Vec<(String, String, Option<f64>, Option<String>)> = sqlx::query_as(
        "SELECT from_track_id, to_track_id, custom_bonus, note FROM dj_golden_pairs",
    )
    .fetch_all(&mut *conn)
    .await
    .unwrap_or_default();
    let golden_pairs = gp_rows
        .into_iter()
        .map(
            |(from_track_id, to_track_id, bonus, note)| GoldenTransitionPair {
                from_track_id,
                to_track_id,
                custom_bonus: bonus.map(|b| b as f32),
                note,
            },
        )
        .collect();
    let ex_rows: Vec<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT track_id, title_pattern FROM dj_excluded_tracks")
            .fetch_all(&mut *conn)
            .await
            .unwrap_or_default();
    let (mut ex_ids, mut ex_pats) = (Vec::new(), Vec::new());
    for (tid, pat) in ex_rows {
        if let Some(id) = tid.filter(|s| !s.is_empty()) {
            ex_ids.push(id);
        }
        if let Some(p) = pat.filter(|s| !s.is_empty()) {
            ex_pats.push(p);
        }
    }
    let mut hints = HashMap::new();
    let mut by_id = HashMap::new();
    let mut cands = Vec::with_capacity(tracks.len());
    for t in tracks {
        let artist = if t.album_artist.is_empty() {
            t.artists.first().cloned().unwrap_or_default()
        } else {
            t.album_artist.clone()
        };
        let era_hint = t.inferred_era_year.map(|y| y as u16).or(t.year);
        let prof =
            classify_track_genre_and_era(&artist, &t.title, &t.genres, &t.path, era_hint, None);
        let fam_code = t
            .inferred_genre_family
            .as_deref()
            .and_then(|n| (0..=14_u8).find(|&c| BoothGenreFamily::from_code(c).as_str() == n))
            .unwrap_or(prof.family_code);
        hints.insert(
            t.id.clone(),
            TrackCurationHint {
                entry_bpm: t.entry_bpm.map(|b| b as f32),
                exit_bpm: t.exit_bpm.map(|b| b as f32),
                is_transition_candidate: t.is_transition_candidate.unwrap_or(false),
                genre_family_code: Some(fam_code),
                inferred_era_year: Some(era_hint.unwrap_or(prof.inferred_era_year)),
                genre_confidence: Some(prof.confidence),
            },
        );
        cands.push(CrateTrackCandidate {
            track_id: t.id.clone(),
            title: t.title.clone(),
            artist,
            file_path: t.path.clone(),
            duration_seconds: t.duration as f32,
            bpm: t.bpm.unwrap_or(124.0) as f32,
            camelot_key: t.camelot.clone().unwrap_or_else(|| "8A".into()),
            energy_score: t.energy.unwrap_or(0.75) as f32,
            punch_index: 0.65,
            groove_type: "FourOnFloor".into(),
            top_subgenre: prof.family_name,
            has_intro_vocals: t.has_intro_vocals.unwrap_or(false),
            has_outro_vocals: t.has_outro_vocals.unwrap_or(false),
            embedding_64d: Vec::new(),
            hot_cues: Vec::new(),
        });
        by_id.insert(t.id.clone(), t);
    }
    let rules = DjCurationRuleSet {
        excluded_track_ids: ex_ids,
        excluded_title_patterns: ex_pats,
        golden_pairs,
        crossover_mode: allow_crossover,
        target_genre_family: target_genre.map(String::from),
        target_era_year: target_era,
        track_hints: hints,
        ..Default::default()
    };
    let res = optimize_crate_with_dj_rules(
        brief,
        "wave",
        cands,
        max_tracks,
        &SetSpineConfig::default(),
        &rules,
    );
    let ordered = res
        .base_result
        .sequence
        .into_iter()
        .filter_map(|c| by_id.remove(&c.track_id))
        .collect();
    CuratedPlaylistSelection {
        tracks: ordered,
        routed_to_mixes: res.routed_to_mixes_over_10m,
        isolated_out_of_genre: res.isolated_out_of_genre_tracks,
        excluded_tracks: res.excluded_tracks,
    }
}

pub(crate) struct CreatedCuratedPlaylist {
    pub playlist: Playlist,
    pub first_track_id: Option<String>,
    pub key_flow: String,
    pub routed_to_mixes: Vec<String>,
    pub isolated_out_of_genre: Vec<String>,
    pub excluded_tracks: Vec<String>,
}

pub(crate) async fn build_curated_playlist_in_db(
    db: &mut DB,
    name: String,
    explicit_ids: Vec<String>,
    sort_harmonic: bool,
    args: &serde_json::Value,
) -> Result<CreatedCuratedPlaylist, String> {
    let allow_crossover = args
        .get("allow_genre_crossover")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let genre_fam = args
        .get("genre_family")
        .and_then(|v| v.as_str())
        .map(String::from);
    let era_min = args.get("era_min").and_then(|v| v.as_i64());
    let era_max = args.get("era_max").and_then(|v| v.as_i64());
    let bpm_min = args.get("bpm_min").and_then(|v| v.as_f64());
    let bpm_max = args.get("bpm_max").and_then(|v| v.as_f64());
    let max_n = args
        .get("target_track_count")
        .and_then(|v| v.as_u64())
        .map(|n| n.clamp(1, 200) as usize);
    let raw_tracks = if explicit_ids.is_empty() {
        let filter = AgentTrackFilter {
            bpm_min,
            bpm_max,
            genre_family: genre_fam.clone(),
            era_min,
            era_max,
            limit: max_n.unwrap_or(120),
            ..Default::default()
        };
        query_agent_tracks_filtered(&mut db.connection, &filter)
            .await
            .map_err(|e| e.to_string())?
    } else {
        db.get_tracks(&explicit_ids)
            .await
            .map_err(|e| e.to_string())?
    };
    let (tracks, routed_to_mixes, isolated_out_of_genre, excluded_tracks) = if sort_harmonic {
        let sel = curate_tracks_with_dj_rules(
            &mut db.connection,
            &name,
            raw_tracks,
            allow_crossover,
            genre_fam.as_deref(),
            era_max.or(era_min).map(|y| y as u16),
            max_n,
        )
        .await;
        (
            sel.tracks,
            sel.routed_to_mixes,
            sel.isolated_out_of_genre,
            sel.excluded_tracks,
        )
    } else {
        (raw_tracks, Vec::new(), Vec::new(), Vec::new())
    };
    let mut keys_dedup: Vec<String> = Vec::new();
    for t in &tracks {
        let k = t.camelot.clone().unwrap_or_else(|| "8A".to_string());
        if keys_dedup.last() != Some(&k) {
            keys_dedup.push(k);
        }
    }
    let key_flow = if keys_dedup.is_empty() {
        "vacía".to_string()
    } else {
        keys_dedup
            .into_iter()
            .take(4)
            .collect::<Vec<_>>()
            .join(" → ")
    };
    let first_track_id = tracks.first().map(|t| t.id.clone());
    let final_ids: Vec<String> = tracks.into_iter().map(|t| t.id).collect();
    let playlist = db
        .create_playlist(name, final_ids, None)
        .await
        .map_err(|e| e.to_string())?;
    Ok(CreatedCuratedPlaylist {
        playlist,
        first_track_id,
        key_flow,
        routed_to_mixes,
        isolated_out_of_genre,
        excluded_tracks,
    })
}

#[allow(clippy::type_complexity)]
pub(crate) fn parse_crate_intent_filters(
    lower: &str,
) -> (
    Option<&'static str>,
    Option<(i64, i64)>,
    Option<(f64, f64)>,
    bool,
) {
    let genre = [
        ("tecnomerengue", "Tecnomerengue"),
        ("merengueton", "MerenguetonModerno"),
        ("merenguetón", "MerenguetonModerno"),
        ("merengue", "MerengueClasico"),
        ("salsa brava", "SalsaBravaChoque"),
        ("salsa", "SalsaBaulRomantica"),
        ("old school", "ReggaetonOldSchool"),
        ("reggaeton", "ReggaetonActualPerreo"),
        ("reggaetón", "ReggaetonActualPerreo"),
        ("perreo", "ReggaetonActualPerreo"),
        ("dembow", "DembowReparto"),
        ("afro", "AfroHouse"),
        ("guaracha", "GuarachaAleteoTech"),
        ("aleteo", "GuarachaAleteoTech"),
        ("bachata", "Bachata"),
        ("cumbia", "CumbiaVallenato"),
        ("vallenato", "CumbiaVallenato"),
        ("hora loca", "HoraLocaGaitas"),
        ("gaita", "HoraLocaGaitas"),
        ("llanera", "LlaneraDespecho"),
        ("despecho", "LlaneraDespecho"),
    ]
    .into_iter()
    .find(|(k, _)| lower.contains(k))
    .map(|(_, v)| v);
    let era = if lower.contains("80s") || lower.contains("ochent") {
        Some((1980, 1989))
    } else if lower.contains("90s") || lower.contains("novent") {
        Some((1990, 1999))
    } else if lower.contains("2000") || lower.contains("clasic") || lower.contains("clásic") {
        Some((2000, 2009))
    } else if lower.contains("2010") {
        Some((2010, 2019))
    } else if lower.contains("2020") || lower.contains("actual") || lower.contains("modern") {
        Some((2020, 2026))
    } else {
        None
    };
    let nums: Vec<f64> = lower
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .filter_map(|s| s.parse::<f64>().ok())
        .filter(|&v| (65.0..=195.0).contains(&v))
        .collect();
    let bpm = if nums.len() >= 2 && nums[0] <= nums[1] {
        Some((nums[0], nums[1]))
    } else if lower.contains("bpm") && !nums.is_empty() {
        Some((nums[0] - 4.0, nums[0] + 4.0))
    } else {
        None
    };
    let allow_cross = ["crossover", "cross over", "variad", "boda", "hora loca"]
        .into_iter()
        .any(|k| lower.contains(k));
    (genre, era, bpm, allow_cross)
}

#[cfg(test)]
mod curation_integration_tests {
    use super::*;
    use sqlx::{Connection, SqliteConnection};

    #[tokio::test]
    async fn test_curate_tracks_isolates_genre_era_and_locks_golden_pair() {
        assert_eq!(
            parse_crate_intent_filters("arma set tecnomerengue 90s 124-130 bpm").0,
            Some("Tecnomerengue")
        );
        let mut c = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/migrations")
            .run_direct(&mut c)
            .await
            .unwrap();
        let make = |id: &str, artist: &str, title: &str, bpm: f64, year: Option<u16>| {
            let path = format!("/{id}.mp3");
            Track {
                id: id.into(),
                path,
                title: title.into(),
                album: "EP".into(),
                album_artist: artist.into(),
                artists: vec![artist.into()],
                genres: Vec::new(),
                year,
                duration: 210,
                track_no: None,
                track_of: None,
                disk_no: None,
                disk_of: None,
                is_compilation: false,
                bpm: Some(bpm),
                camelot: Some("8A".into()),
                energy: Some(0.8),
                hera_seal: None,
                is_video: Some(false),
                scan_level: Some(2),
                is_transition_candidate: Some(false),
                is_fake_320kbps: Some(false),
                waveform_peaks_blob: None,
                is_mix_over_10m: Some(false),
                has_audio_skips: Some(false),
                is_variable_bpm: Some(false),
                entry_bpm: None,
                exit_bpm: None,
                hot_cues: None,
                replay_gain_db: None,
                loudness_lufs: None,
                has_intro_vocals: Some(false),
                has_outro_vocals: Some(false),
                is_own_production: Some(false),
                has_bpm_trust_diff: Some(false),
                waveform_peaks_b64: None,
                embedding_64d_b64: None,
                inferred_genre_family: None,
                inferred_era_year: year.map(i64::from),
                partial_hash_64kb: None,
                file_size_bytes: None,
                legacy_bpm: None,
                bitrate_kbps: None,
                sample_rate_hz: None,
            }
        };
        let list = vec![
            make("s1", "Tony Vega", "Lo Mio Es Amor", 95.0, Some(1992)),
            make("p1", "Feid", "Luna", 96.0, Some(2023)),
        ];
        let res = curate_tracks_with_dj_rules(
            &mut c,
            "Salsa Baul",
            list,
            false,
            Some("SalsaBaulRomantica"),
            Some(1992),
            None,
        )
        .await;
        assert_eq!(
            res.tracks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            vec!["s1"]
        );
        assert_eq!(res.isolated_out_of_genre, vec!["p1"]);
    }
}
