use log::info;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::sync_channel;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Runtime};

use crate::libs::database::{SUPPORTED_PLAYLISTS_EXTENSIONS, SUPPORTED_TRACKS_EXTENSIONS};
use crate::libs::error::AnyResult;
use crate::libs::events::IPCEvent;
use crate::libs::track::{HotCuePin, Track, get_indexed_track_fast};
use crate::plugins::db::{DBState, ScanProgress};
use crate::plugins::hera_dj::workability::{
    ExternalDjSoftwareIndex, is_junk_system_directory, is_scan_cancelled, reset_scan_flag,
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
    let total_discovered = Arc::new(AtomicUsize::new(0));

    let walk_paths = import_paths.clone();
    let walk_existing = Arc::new(existing_paths);
    let walk_discovered = Arc::clone(&total_discovered);

    std::thread::spawn(move || {
        for root in &walk_paths {
            if is_scan_cancelled() {
                break;
            }
            for entry in walkdir::WalkDir::new(root)
                .follow_links(true)
                .into_iter()
                .filter_entry(|e| !is_junk_system_directory(e.path()))
                .filter_map(|e| e.ok())
            {
                if is_scan_cancelled() {
                    break;
                }
                let path = entry.path();
                if entry.file_type().is_file() {
                    if is_supported_playlist(path) {
                        let _ = playlist_tx.send(path.to_path_buf());
                    } else if is_supported_audio(path) && (refresh || !walk_existing.contains(path))
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
        drop(guard);
        let _ = app.emit("hera:scanner-tracks-added", &batch);
    }
    Ok((batch_len, fast_cached, full_cached))
}
