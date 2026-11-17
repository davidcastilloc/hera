use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{Connection, SqliteConnection};
use tauri::{AppHandle, Emitter, Runtime};

use hera_node_rs::bpm_engine::v2_core::audio_ingest::ProcessLimiter;
use hera_node_rs::scanner::scheduler::{ScanPriority, ScanScheduler};

pub use super::scanner_governor_os::enqueue_fs_discovered_tracks;
use super::scanner_governor_os::{
    ProcessedTrackOutcome, detect_on_battery_power, drain_fs_discovered_tracks,
    execute_isolated_worker_job, lower_current_worker_thread_priority, make_scanner_row_patch_json,
};
use crate::libs::db_scanner_queries::{
    apply_scanner_batch_updates, count_scanner_levels, delete_short_sfx_tracks,
    fetch_track_path_and_level, load_pending_full_pass_jobs, load_pending_scanner_jobs,
    record_doctor_issue,
};
use crate::libs::omkf_fs_catalog::global_omkf_catalog;
use crate::libs::omkf_fs_organize::upsert_scanner_outcomes_to_omkf;

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

        let q_seed = "SELECT id, path, scan_level, duration FROM tracks LIMIT 50000";
        if let Ok(rows) = sqlx::query_as::<_, (String, String, i64, i64)>(q_seed)
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
        let _ = crate::libs::omkf_sync::boot_verify_folder_crates(&mut conn).await;

        for idx in 0..num_workers {
            let (sw, cw, lw) = (Arc::clone(&sched), Arc::clone(&cache), Arc::clone(&limiter));
            let (tw, mw) = (tx.clone(), missing_tx.clone());
            std::thread::Builder::new()
                .name(format!("hera-dsp-worker-{idx}"))
                .spawn(move || {
                    lower_current_worker_thread_priority();
                    loop {
                        if idx >= sw.governor.effective_workers() {
                            std::thread::sleep(Duration::from_millis(250));
                            continue;
                        }
                        let Some((tid, prio)) = sw.wait_next(Duration::from_millis(250)) else {
                            continue;
                        };
                        let _in_flight = InFlightGuard(&sw, tid.clone());
                        if super::workability::is_scan_cancelled()
                            && prio >= ScanPriority::P2FastBackground
                        {
                            continue;
                        }
                        let Some((path, lvl, dur)) = cw.lock().unwrap().get(&tid).cloned() else {
                            let _ = mw.send((tid, prio));
                            continue;
                        };
                        if let Some(out) =
                            execute_isolated_worker_job(tid.clone(), path, lvl, dur, prio, &lw)
                        {
                            sw.mark_completed(&tid, out.update.scan_level.clamp(-1, 2) as i8);
                            let _ = tw.send(out);
                        }
                    }
                })
                .expect("Failed to spawn scanner worker");
        }

        let mut interval = tokio::time::interval(Duration::from_millis(200));
        let (mut last_calc_time, mut last_omkf_emit) =
            (Instant::now(), Instant::now() - Duration::from_secs(2));
        let (mut last_completed_count, mut ema_tps, mut was_idle, mut tick_count) =
            (0_usize, 0.0_f64, false, 0_u64);
        let mut pending_omkf_crates: HashSet<String> = HashSet::new();

        loop {
            interval.tick().await;
            let deck_playing = sched.governor.is_deck_playing();
            if tick_count.is_multiple_of(12) {
                sched
                    .governor
                    .set_on_battery_power(detect_on_battery_power());
            }
            if tick_count.is_multiple_of(5) && !deck_playing {
                let _ = crate::libs::omkf_sync::flush_dirty_omkf_capsules(&mut conn).await;
            }
            tick_count = tick_count.wrapping_add(1);
            if super::workability::is_scan_cancelled() && sched.pending_count() > 0 {
                let _ = sched.cancel_and_drain_background();
            }

            if !deck_playing {
                let fs_items = drain_fs_discovered_tracks();
                if !fs_items.is_empty() {
                    let mut ids = Vec::with_capacity(fs_items.len());
                    let mut g = cache.lock().unwrap();
                    for (id, p) in fs_items {
                        let lvl = g.get(&id).map_or(0, |e| e.1);
                        if lvl == 0 {
                            g.entry(id.clone()).or_insert((p, 0, 0));
                            ids.push(id);
                        }
                    }
                    drop(g);
                    if !ids.is_empty() {
                        sched.enqueue_batch(ids, ScanPriority::P2FastBackground);
                    }
                }
            }

            let mut batch = Vec::new();
            while batch.len() < 128
                && let Ok(outcome) = rx.try_recv()
            {
                batch.push(outcome);
            }

            let (mut updates_to_apply, mut short_sfx_ids, mut patches_to_emit) =
                (Vec::new(), Vec::new(), Vec::new());
            let (mut fs_capsule_batch, mut promote_full_ids) = (Vec::new(), Vec::new());

            for outcome in batch {
                let u = outcome.update;
                if let Some((bp, d)) = outcome.corrupt_issue {
                    let _ = record_doctor_issue(
                        &mut conn,
                        &u.track_id,
                        &bp,
                        "CORRUPT_FILE",
                        "error",
                        &d,
                    )
                    .await;
                }
                if u.duration_sec.is_some_and(|d| d > 0 && d < 25) {
                    short_sfx_ids.push(u.track_id);
                } else {
                    if let Some(p) = cache.lock().unwrap().get(&u.track_id).map(|e| e.0.clone()) {
                        fs_capsule_batch.push((p, u.clone(), outcome.hot_cues.clone()));
                    }
                    if u.scan_level == 1 && !sched.governor.is_throttled_booth_mode() {
                        promote_full_ids.push(u.track_id.clone());
                    }
                    patches_to_emit.push(make_scanner_row_patch_json(&u, &outcome.hot_cues));
                    updates_to_apply.push(u);
                }
            }

            if !short_sfx_ids.is_empty() {
                let _ = delete_short_sfx_tracks(&mut conn, &short_sfx_ids).await;
                cache
                    .lock()
                    .unwrap()
                    .retain(|k, _| !short_sfx_ids.contains(k));
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
                        crate::libs::omkf_sync::mark_crate_dirty_for_track_path(&entry.0);
                    }
                }
                drop(g);
                if !fs_capsule_batch.is_empty() && !deck_playing {
                    pending_omkf_crates.extend(upsert_scanner_outcomes_to_omkf(
                        global_omkf_catalog(),
                        &fs_capsule_batch,
                    ));
                }
                if !promote_full_ids.is_empty() && !deck_playing {
                    sched.enqueue_batch(promote_full_ids, ScanPriority::P3FullElastic);
                }
                let _ = app_handle.emit("hera:scanner-rows-updated", &patches_to_emit);
            }

            if !pending_omkf_crates.is_empty()
                && !deck_playing
                && last_omkf_emit.elapsed() >= Duration::from_millis(2000)
            {
                let crate_ids: Vec<String> = pending_omkf_crates.drain().collect();
                let _ = app_handle.emit(
                    "hera:omkf-folder-updated",
                    serde_json::json!({ "crate_ids": crate_ids }),
                );
                last_omkf_emit = Instant::now();
            }

            while !deck_playing && let Ok((miss_id, miss_prio)) = missing_rx.try_recv() {
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

            if !deck_playing
                && sched.pending_count() < 64
                && !super::workability::is_scan_cancelled()
                && let Ok(pending_fast) = load_pending_scanner_jobs(&mut conn, 256).await
            {
                if !pending_fast.is_empty() {
                    let mut ids = Vec::with_capacity(pending_fast.len());
                    let mut g = cache.lock().unwrap();
                    for (id, p, l) in pending_fast {
                        g.entry(id.clone())
                            .and_modify(|e| (e.0, e.1) = (p.clone(), l))
                            .or_insert((p, l, 0));
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
                    for (id, p) in pending_full {
                        g.entry(id.clone())
                            .and_modify(|e| (e.0, e.1) = (p.clone(), e.1.max(1)))
                            .or_insert((p, 1, 0));
                        ids.push(id);
                    }
                    drop(g);
                    sched.enqueue_batch(ids, ScanPriority::P3FullElastic);
                }
            }

            if !deck_playing
                && let Ok((total, fast_done, full_done)) = count_scanner_levels(&mut conn).await
            {
                let is_busy = (fast_done < total || full_done < total || sched.pending_count() > 0)
                    && !super::workability::is_scan_cancelled();
                let updates_flushed = !updates_to_apply.is_empty() || !short_sfx_ids.is_empty();
                if is_busy || updates_flushed || !was_idle {
                    let elapsed_sec = last_calc_time.elapsed().as_secs_f64();
                    let completed_now = fast_done + full_done;
                    if elapsed_sec >= 1.0 {
                        let inst = (completed_now.saturating_sub(last_completed_count) as f64)
                            / elapsed_sec;
                        ema_tps = if ema_tps == 0.0 {
                            inst
                        } else {
                            0.7 * ema_tps + 0.3 * inst
                        };
                        last_completed_count = completed_now;
                        last_calc_time = Instant::now();
                    }
                    let rem = total.saturating_sub(fast_done) + total.saturating_sub(full_done);
                    let eta = if ema_tps > 0.1 {
                        (rem as f64 / ema_tps).round() as u64
                    } else {
                        0
                    };
                    let ev = serde_json::json!({
                        "phase": if is_busy { "analyzing" } else { "idle" },
                        "fastDone": fast_done, "fullDone": full_done, "total": total,
                        "tracksPerSec": (ema_tps * 10.0).round() / 10.0, "etaSec": eta,
                        "governorActive": sched.governor.is_throttled_booth_mode(),
                    });
                    let _ = app_handle.emit("hera:scanner-telemetry", ev);
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
