use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use lofty::file::AudioFile;
use log::warn;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{Connection, SqliteConnection};
use tauri::{AppHandle, Emitter, Runtime};

use hera_node_rs::bpm_engine::v2_core::audio_ingest::{load_audio_mono_native_sr, ProcessLimiter};
use hera_node_rs::scanner::fast_seek::run_fast_scan_on_file;
use hera_node_rs::scanner::full_pass::run_full_pass_on_file;
use hera_node_rs::scanner::scheduler::{ScanPriority, ScanScheduler};

use crate::libs::db_scanner_queries::{
    apply_scanner_batch_updates, count_scanner_levels, delete_short_sfx_tracks,
    load_pending_full_pass_jobs, load_pending_scanner_jobs, ScannerRowUpdate,
};
use crate::libs::track::HotCuePin;

static DAEMON_STARTED: AtomicBool = AtomicBool::new(false);

struct ProcessedTrackOutcome {
    update: ScannerRowUpdate,
    hot_cues: Option<Vec<HotCuePin>>,
}

fn empty_failure_update(track_id: String, duration_sec: Option<u32>) -> ScannerRowUpdate {
    ScannerRowUpdate {
        track_id,
        bpm: None,
        camelot: None,
        energy: None,
        hot_cues_json: None,
        waveform_peaks_blob: None,
        scan_level: 1,
        is_transition_candidate: false,
        is_fake_320kbps: false,
        duration_sec,
    }
}

pub fn spawn_scanner_daemon<R: Runtime>(app_handle: AppHandle<R>) {
    if DAEMON_STARTED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return;
    }

    tauri::async_runtime::spawn(async move {
        let db_path = crate::plugins::config::get_storage_dir().join("hera.db");
        let mut conn = loop {
            let opts = SqliteConnectOptions::new()
                .filename(&db_path)
                .create_if_missing(false)
                .journal_mode(SqliteJournalMode::Wal)
                .busy_timeout(Duration::from_secs(5));
            match SqliteConnection::connect_with(&opts).await {
                Ok(c) => break c,
                Err(_) => tokio::time::sleep(Duration::from_millis(500)).await,
            }
        };

        let num_workers = std::thread::available_parallelism()
            .map(|n| (n.get() / 2).clamp(2, 6))
            .unwrap_or(4);
        let (tx, rx) = mpsc::channel::<ProcessedTrackOutcome>();
        let cache = Arc::new(Mutex::new(HashMap::<String, (String, i64)>::new()));
        let limiter = Arc::new(ProcessLimiter::new(2));
        let sched = super::scanner_v3::global_scheduler().clone();

        // Seed initial cache from SQLite (< 2ms)
        if let Ok(rows) = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT id, path, scan_level FROM tracks WHERE scan_level < 2 LIMIT 512",
        ).fetch_all(&mut conn).await {
            let mut g = cache.lock().unwrap();
            for (id, path, lvl) in rows { g.insert(id, (path, lvl)); }
        }

        for idx in 0..num_workers {
            let (sched_w, cache_w, limiter_w, tx_w) = (Arc::clone(&sched), Arc::clone(&cache), Arc::clone(&limiter), tx.clone());
            std::thread::Builder::new().name(format!("hera-dsp-worker-{idx}")).spawn(move || loop {
                if idx >= sched_w.governor.effective_workers() {
                    std::thread::sleep(Duration::from_millis(250));
                    continue;
                }
                let Some((track_id, prio)) = sched_w.wait_next(Duration::from_millis(250)) else { continue; };
                if super::workability::is_scan_cancelled() && prio >= ScanPriority::P2FastBackground { continue; }
                let cached = { cache_w.lock().unwrap().get(&track_id).cloned() };
                let Some((path, current_level)) = cached else { continue; };

                let path_obj = Path::new(&path);
                let is_level_2 = prio == ScanPriority::P0Immediate || prio == ScanPriority::P3FullElastic;

                let probed_dur = lofty::probe::Probe::open(path_obj).ok()
                    .and_then(|p| p.options(lofty::config::ParseOptions::new().read_cover_art(false)).read().ok())
                    .map(|tf| tf.properties().duration().as_secs() as u32).unwrap_or(0);
                let real_dur = if probed_dur > 0 { probed_dur } else {
                    load_audio_mono_native_sr(path_obj, Some(&limiter_w)).map(|d| d.duration_sec.round() as u32).unwrap_or(0)
                };
                let duration_opt = if real_dur > 0 { Some(real_dur) } else { None };

                if is_level_2 {
                    if current_level >= 2 { continue; }
                    match run_full_pass_on_file(path_obj, None, Some(&limiter_w)) {
                        Ok(res) => {
                            let pins: Vec<HotCuePin> = res.hot_cues.into_iter().take(4).map(|c| {
                                let (color, role) = match c.num {
                                    1 => ("#22c55e", "Intro"),
                                    2 => ("#38bdf8", "Breakdown"),
                                    3 => ("#ef4444", "Drop"),
                                    _ => ("#f59e0b", "Outro"),
                                };
                                HotCuePin { num: c.num, name: c.name, seconds: ((c.seconds * 10.0).round()) / 10.0, color: color.to_string(), role: role.to_string() }
                            }).collect();
                            let hot_cues_json = serde_json::to_string(&pins).ok();
                            let update = ScannerRowUpdate {
                                track_id, bpm: Some(res.bpm), camelot: Some(res.camelot_key), energy: None,
                                hot_cues_json, waveform_peaks_blob: Some(res.waveform_peaks_blob), scan_level: 2,
                                is_transition_candidate: false, is_fake_320kbps: false, duration_sec: duration_opt,
                            };
                            let _ = tx_w.send(ProcessedTrackOutcome { update, hot_cues: Some(pins) });
                        }
                        Err(e) => {
                            warn!("Full pass failed for {path}: {e}");
                            let _ = tx_w.send(ProcessedTrackOutcome { update: empty_failure_update(track_id, duration_opt), hot_cues: None });
                        }
                    }
                } else {
                    if current_level >= 1 { continue; }
                    match run_fast_scan_on_file(path_obj, None, 320, Some(&limiter_w)) {
                        Ok(res) => {
                            let update = ScannerRowUpdate {
                                track_id, bpm: Some(res.bpm), camelot: Some(res.camelot_key), energy: None,
                                hot_cues_json: None, waveform_peaks_blob: None, scan_level: 1,
                                is_transition_candidate: res.is_transition_candidate, is_fake_320kbps: res.is_fake_320kbps,
                                duration_sec: duration_opt,
                            };
                            let _ = tx_w.send(ProcessedTrackOutcome { update, hot_cues: None });
                        }
                        Err(e) => {
                            warn!("Fast scan failed for {path}: {e}");
                            let _ = tx_w.send(ProcessedTrackOutcome { update: empty_failure_update(track_id, duration_opt), hot_cues: None });
                        }
                    }
                }
            }).expect("Failed to spawn scanner worker");
        }

        let mut interval = tokio::time::interval(Duration::from_millis(200));
        let mut last_calc_time = Instant::now();
        let mut last_completed_count = 0_usize;
        let mut ema_tracks_per_sec = 0.0_f64;
        let mut was_idle = false;

        loop {
            interval.tick().await;

            // 1. Drain up to 128 outcomes from workers
            let mut batch = Vec::new();
            while batch.len() < 128 {
                match rx.try_recv() {
                    Ok(outcome) => batch.push(outcome),
                    Err(_) => break,
                }
            }

            let mut updates_to_apply = Vec::new();
            let mut short_sfx_ids = Vec::new();
            let mut patches_to_emit = Vec::new();

            for outcome in batch {
                if outcome.update.duration_sec.is_some_and(|d| d > 0 && d < 25) {
                    short_sfx_ids.push(outcome.update.track_id);
                } else {
                    patches_to_emit.push(serde_json::json!({
                        "track_id": outcome.update.track_id,
                        "bpm": outcome.update.bpm,
                        "camelot": outcome.update.camelot,
                        "energy": outcome.update.energy,
                        "duration": outcome.update.duration_sec,
                        "hot_cues": outcome.hot_cues,
                        "scan_level": outcome.update.scan_level,
                        "is_transition_candidate": outcome.update.is_transition_candidate,
                        "is_fake_320kbps": outcome.update.is_fake_320kbps,
                    }));
                    updates_to_apply.push(outcome.update);
                }
            }

            if !short_sfx_ids.is_empty() {
                let _ = delete_short_sfx_tracks(&mut conn, &short_sfx_ids).await;
                {
                    let mut g = cache.lock().unwrap();
                    for id in &short_sfx_ids { g.remove(id); }
                }
                let _ = app_handle.emit("hera:scanner-tracks-removed", &short_sfx_ids);
            }

            if !updates_to_apply.is_empty() {
                let _ = apply_scanner_batch_updates(&mut conn, &updates_to_apply).await;
                {
                    let mut g = cache.lock().unwrap();
                    for u in &updates_to_apply {
                        if let Some(entry) = g.get_mut(&u.track_id) { entry.1 = entry.1.max(u.scan_level); }
                    }
                }
                let _ = app_handle.emit("hera:scanner-rows-updated", &patches_to_emit);
            }

            // 2. Refill scheduler when low
            if sched.pending_count() < 64 && !super::workability::is_scan_cancelled() {
                if let Ok(pending_fast) = load_pending_scanner_jobs(&mut conn, 256).await {
                    if !pending_fast.is_empty() {
                        let mut ids = Vec::with_capacity(pending_fast.len());
                        let mut g = cache.lock().unwrap();
                        for (id, path, lvl) in pending_fast {
                            g.insert(id.clone(), (path, lvl));
                            ids.push(id);
                        }
                        drop(g);
                        sched.enqueue_batch(ids, ScanPriority::P2FastBackground);
                    } else if !sched.governor.is_deck_playing() {
                        if let Ok(pending_full) = load_pending_full_pass_jobs(&mut conn, 64).await {
                            if !pending_full.is_empty() {
                                let mut ids = Vec::with_capacity(pending_full.len());
                                let mut g = cache.lock().unwrap();
                                for (id, path) in pending_full {
                                    g.insert(id.clone(), (path, 1));
                                    ids.push(id);
                                }
                                drop(g);
                                sched.enqueue_batch(ids, ScanPriority::P3FullElastic);
                            }
                        }
                    }
                }
            }

            // 3. Telemetry calculation & emission
            if let Ok((total, fast_done, full_done)) = count_scanner_levels(&mut conn).await {
                let is_busy = (fast_done < total || full_done < total) && !super::workability::is_scan_cancelled();
                let updates_flushed = !updates_to_apply.is_empty() || !short_sfx_ids.is_empty();

                if is_busy || updates_flushed || !was_idle {
                    let now = Instant::now();
                    let elapsed_sec = now.duration_since(last_calc_time).as_secs_f64();
                    let completed_now = fast_done + full_done;
                    let delta_tracks = completed_now.saturating_sub(last_completed_count);

                    if elapsed_sec >= 1.0 {
                        let inst_tps = (delta_tracks as f64) / elapsed_sec;
                        ema_tracks_per_sec = if ema_tracks_per_sec == 0.0 { inst_tps } else { 0.7 * ema_tracks_per_sec + 0.3 * inst_tps };
                        last_completed_count = completed_now;
                        last_calc_time = now;
                    }

                    let pending_tasks = (total.saturating_sub(fast_done)) + (total.saturating_sub(full_done));
                    let eta_sec = if ema_tracks_per_sec > 0.1 {
                        (pending_tasks as f64 / ema_tracks_per_sec).round() as u64
                    } else {
                        0
                    };

                    let telemetry = serde_json::json!({
                        "phase": if is_busy { "analyzing" } else { "idle" },
                        "fastDone": fast_done,
                        "fullDone": full_done,
                        "total": total,
                        "tracksPerSec": (ema_tracks_per_sec * 10.0).round() / 10.0,
                        "etaSec": eta_sec,
                        "governorActive": sched.governor.is_deck_playing(),
                    });
                    let _ = app_handle.emit("hera:scanner-telemetry", telemetry);
                    was_idle = !is_busy && !updates_flushed;
                }
            }
        }
    });
}
