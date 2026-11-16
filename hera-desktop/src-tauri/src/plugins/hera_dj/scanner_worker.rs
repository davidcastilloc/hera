use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{Connection, SqliteConnection};
use tauri::{AppHandle, Emitter, Runtime};

use hera_node_rs::bpm_engine::v2_core::audio_ingest::ProcessLimiter;
use hera_node_rs::scanner::scheduler::{ScanPriority, ScanScheduler};

use super::scanner_governor_os::{
    ProcessedTrackOutcome, execute_isolated_worker_job, lower_current_worker_thread_priority,
};
use crate::libs::db_scanner_queries::{
    apply_scanner_batch_updates, count_scanner_levels, delete_short_sfx_tracks,
    fetch_track_path_and_level, load_pending_full_pass_jobs, load_pending_scanner_jobs,
    record_doctor_issue,
};

static DAEMON_STARTED: AtomicBool = AtomicBool::new(false);

struct InFlightGuard<'a>(&'a ScanScheduler, String);
impl Drop for InFlightGuard<'_> {
    fn drop(&mut self) {
        self.0.complete(&self.1);
    }
}

pub fn spawn_scanner_daemon<R: Runtime>(app_handle: AppHandle<R>) {
    if DAEMON_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
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
        let (missing_tx, missing_rx) = mpsc::channel::<(String, ScanPriority)>();
        let cache = Arc::new(Mutex::new(HashMap::<String, (String, i64, u32)>::new()));
        let limiter = Arc::new(ProcessLimiter::new(2));
        let sched = super::scanner_v3::global_scheduler().clone();

        if let Ok(rows) = sqlx::query_as::<_, (String, String, i64, i64)>(
            "SELECT id, path, scan_level, duration FROM tracks LIMIT 50000",
        )
        .fetch_all(&mut conn)
        .await
        {
            let mut g = cache.lock().unwrap();
            let mut seeded = Vec::new();
            for (id, path, lvl, dur) in rows {
                if lvl != 0 {
                    seeded.push((id.clone(), lvl.clamp(-1, 2) as i8));
                }
                if (0..2).contains(&lvl) {
                    g.insert(id, (path, lvl, dur.max(0) as u32));
                }
            }
            sched.seed_completed_levels(seeded);
        }

        for idx in 0..num_workers {
            let (sched_w, cache_w, limiter_w, tx_w, missing_tx_w) = (
                Arc::clone(&sched),
                Arc::clone(&cache),
                Arc::clone(&limiter),
                tx.clone(),
                missing_tx.clone(),
            );
            std::thread::Builder::new()
                .name(format!("hera-dsp-worker-{idx}"))
                .spawn(move || {
                    lower_current_worker_thread_priority();
                    loop {
                        if idx >= sched_w.governor.effective_workers() {
                            std::thread::sleep(Duration::from_millis(250));
                            continue;
                        }
                        let Some((track_id, prio)) = sched_w.wait_next(Duration::from_millis(250))
                        else {
                            continue;
                        };
                        let _in_flight_guard = InFlightGuard(&sched_w, track_id.clone());
                        if super::workability::is_scan_cancelled()
                            && prio >= ScanPriority::P2FastBackground
                        {
                            continue;
                        }
                        let cached = { cache_w.lock().unwrap().get(&track_id).cloned() };
                        let Some((path, current_level, cached_dur)) = cached else {
                            let _ = missing_tx_w.send((track_id, prio));
                            continue;
                        };
                        if let Some(outcome) = execute_isolated_worker_job(
                            track_id.clone(),
                            path,
                            current_level,
                            cached_dur,
                            prio,
                            &limiter_w,
                        ) {
                            sched_w.mark_completed(
                                &track_id,
                                outcome.update.scan_level.clamp(-1, 2) as i8,
                            );
                            let _ = tx_w.send(outcome);
                        }
                    }
                })
                .expect("Failed to spawn scanner worker");
        }

        let mut interval = tokio::time::interval(Duration::from_millis(200));
        let mut last_calc_time = Instant::now();
        let mut last_completed_count = 0_usize;
        let mut ema_tracks_per_sec = 0.0_f64;
        let mut was_idle = false;

        loop {
            interval.tick().await;
            if super::workability::is_scan_cancelled() && sched.pending_count() > 0 {
                let _ = sched.cancel_and_drain_background();
            }

            let mut batch = Vec::new();
            while batch.len() < 128
                && let Ok(outcome) = rx.try_recv()
            {
                batch.push(outcome);
            }

            let mut updates_to_apply = Vec::new();
            let mut short_sfx_ids = Vec::new();
            let mut patches_to_emit = Vec::new();

            for outcome in batch {
                if let Some((bad_path, details)) = outcome.corrupt_issue {
                    let _ = record_doctor_issue(
                        &mut conn,
                        &outcome.update.track_id,
                        &bad_path,
                        "CORRUPT_FILE",
                        "error",
                        &details,
                    )
                    .await;
                }
                if outcome.update.duration_sec.is_some_and(|d| d > 0 && d < 25) {
                    short_sfx_ids.push(outcome.update.track_id);
                } else {
                    patches_to_emit.push(serde_json::json!({
                        "track_id": outcome.update.track_id, "bpm": outcome.update.bpm,
                        "camelot": outcome.update.camelot, "energy": outcome.update.energy,
                        "duration": outcome.update.duration_sec, "hot_cues": outcome.hot_cues,
                        "scan_level": outcome.update.scan_level,
                        "is_transition_candidate": outcome.update.is_transition_candidate,
                        "is_fake_320kbps": outcome.update.is_fake_320kbps,
                        "replay_gain_db": outcome.update.replay_gain_db,
                        "loudness_lufs": outcome.update.loudness_lufs,
                        "waveform_peaks_b64": outcome.update.waveform_peaks_b64,
                        "has_intro_vocals": outcome.update.has_intro_vocals,
                        "has_outro_vocals": outcome.update.has_outro_vocals,
                        "has_audio_skips": outcome.update.has_audio_skips,
                        "entry_bpm": outcome.update.entry_bpm,
                        "exit_bpm": outcome.update.exit_bpm,
                    }));
                    updates_to_apply.push(outcome.update);
                }
            }

            if !short_sfx_ids.is_empty() {
                let _ = delete_short_sfx_tracks(&mut conn, &short_sfx_ids).await;
                let mut g = cache.lock().unwrap();
                for id in &short_sfx_ids {
                    g.remove(id);
                }
                drop(g);
                let _ = app_handle.emit("hera:scanner-tracks-removed", &short_sfx_ids);
            }

            if !updates_to_apply.is_empty() {
                let _ = apply_scanner_batch_updates(&mut conn, &updates_to_apply).await;
                let mut g = cache.lock().unwrap();
                for u in &updates_to_apply {
                    if u.scan_level < 0 {
                        g.remove(&u.track_id);
                    } else if let Some(entry) = g.get_mut(&u.track_id) {
                        entry.1 = entry.1.max(u.scan_level);
                        if let Some(d) = u.duration_sec {
                            entry.2 = d;
                        }
                    }
                }
                drop(g);
                let _ = app_handle.emit("hera:scanner-rows-updated", &patches_to_emit);
            }

            while let Ok((miss_id, miss_prio)) = missing_rx.try_recv() {
                if let Ok(Some((path, lvl))) = fetch_track_path_and_level(&mut conn, &miss_id).await
                {
                    if lvl < 0 {
                        sched.mark_completed(&miss_id, -1);
                        continue;
                    }
                    cache
                        .lock()
                        .unwrap()
                        .entry(miss_id.clone())
                        .or_insert((path, lvl, 0));
                    if lvl < i64::from(miss_prio.target_scan_level()) {
                        sched.enqueue_or_promote(miss_id, miss_prio);
                    }
                }
            }

            if sched.pending_count() < 64
                && !super::workability::is_scan_cancelled()
                && let Ok(pending_fast) = load_pending_scanner_jobs(&mut conn, 256).await
            {
                if !pending_fast.is_empty() {
                    let mut ids = Vec::with_capacity(pending_fast.len());
                    let mut g = cache.lock().unwrap();
                    for (id, path, lvl) in pending_fast {
                        g.entry(id.clone())
                            .and_modify(|e| {
                                e.0 = path.clone();
                                e.1 = lvl;
                            })
                            .or_insert((path, lvl, 0));
                        ids.push(id);
                    }
                    drop(g);
                    sched.enqueue_batch(ids, ScanPriority::P2FastBackground);
                } else if !sched.governor.is_throttled_booth_mode()
                    && let Ok(pending_full) = load_pending_full_pass_jobs(&mut conn, 64).await
                    && !pending_full.is_empty()
                {
                    let mut ids = Vec::with_capacity(pending_full.len());
                    let mut g = cache.lock().unwrap();
                    for (id, path) in pending_full {
                        g.entry(id.clone())
                            .and_modify(|e| {
                                e.0 = path.clone();
                                e.1 = e.1.max(1);
                            })
                            .or_insert((path, 1, 0));
                        ids.push(id);
                    }
                    drop(g);
                    sched.enqueue_batch(ids, ScanPriority::P3FullElastic);
                }
            }

            if let Ok((total, fast_done, full_done)) = count_scanner_levels(&mut conn).await {
                let is_busy = (fast_done < total || full_done < total)
                    && !super::workability::is_scan_cancelled();
                let updates_flushed = !updates_to_apply.is_empty() || !short_sfx_ids.is_empty();
                if is_busy || updates_flushed || !was_idle {
                    let now = Instant::now();
                    let elapsed_sec = now.duration_since(last_calc_time).as_secs_f64();
                    let completed_now = fast_done + full_done;
                    if elapsed_sec >= 1.0 {
                        let inst_tps = (completed_now.saturating_sub(last_completed_count) as f64)
                            / elapsed_sec;
                        ema_tracks_per_sec = if ema_tracks_per_sec == 0.0 {
                            inst_tps
                        } else {
                            0.7 * ema_tracks_per_sec + 0.3 * inst_tps
                        };
                        last_completed_count = completed_now;
                        last_calc_time = now;
                    }
                    let pending_tasks =
                        total.saturating_sub(fast_done) + total.saturating_sub(full_done);
                    let eta_sec = if ema_tracks_per_sec > 0.1 {
                        (pending_tasks as f64 / ema_tracks_per_sec).round() as u64
                    } else {
                        0
                    };
                    let _ = app_handle.emit(
                        "hera:scanner-telemetry",
                        serde_json::json!({
                            "phase": if is_busy { "analyzing" } else { "idle" },
                            "fastDone": fast_done, "fullDone": full_done, "total": total,
                            "tracksPerSec": (ema_tracks_per_sec * 10.0).round() / 10.0,
                            "etaSec": eta_sec,
                            "governorActive": sched.governor.is_throttled_booth_mode(),
                        }),
                    );
                    crate::libs::tray::update_tray_scanner_status(
                        &app_handle,
                        fast_done,
                        full_done,
                        total,
                        is_busy,
                    );
                    was_idle = !is_busy && !updates_flushed;
                }
            }
        }
    });
}
