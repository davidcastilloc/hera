//! Live VirtualDJ/History bridge (2.5s + auto golden pairs) & Ableton watcher (1.5s + Club Mono).

use super::{scanner_v3::global_scheduler, virtualdj_live::parse_virtualdj_history_content};
use crate::libs::{
    database::{DB, SUPPORTED_TRACKS_EXTENSIONS},
    db_scanner_queries::record_doctor_issue,
    track::get_indexed_track_fast,
    tray::{clear_tray_now_playing, update_tray_now_playing},
    utils::is_file_valid,
};
use crate::plugins::db::DBState;
use hera_ipc::dj_types::VirtualDjNowPlaying;
use hera_node_rs::{
    forensics::decode_audio_file_partial,
    scanner::{audio_metrics::check_stereo_sub_mono_phase, scheduler::ScanPriority},
    synergy::compute_camelot_synergy,
};
use sqlx::SqliteConnection;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, Runtime};

static WATCHERS_STARTED: AtomicBool = AtomicBool::new(false);

pub fn should_record_live_golden_pair(dwell_secs: u64) -> bool {
    (45..=420).contains(&dwell_secs)
}

fn read_latest_vdj_now_playing() -> Option<VirtualDjNowPlaying> {
    let envs = [
        ("HERA_VDJ_HOME", "History"),
        ("USERPROFILE", "Documents/VirtualDJ/History"),
        ("LOCALAPPDATA", "VirtualDJ/History"),
        ("HOME", "Documents/VirtualDJ/History"),
    ];
    let best = envs
        .into_iter()
        .filter_map(|(k, sub)| std::env::var(k).ok().map(|v| PathBuf::from(v).join(sub)))
        .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
        .filter_map(|e| {
            let nm = e.file_name().to_string_lossy().to_ascii_lowercase();
            let ok = nm == "tracklist.txt" || nm.ends_with(".m3u") || nm.ends_with(".m3u8");
            let mt = ok.then(|| e.metadata().and_then(|m| m.modified()).ok());
            mt.flatten().map(|t| (t, e.path()))
        })
        .max_by_key(|(t, _)| *t)?;
    let (artist, title) = parse_virtualdj_history_content(&std::fs::read_to_string(&best.1).ok()?)?;
    let source_path = best.1.to_string_lossy().into();
    Some(VirtualDjNowPlaying {
        artist,
        title,
        source_path,
    })
}

pub async fn record_live_golden_pair_if_eligible(
    conn: &mut SqliteConnection,
    from: (&str, &str, &str),
    to: (&str, &str, &str),
    dwell_secs: u64,
) -> bool {
    if from.0 == to.0 || !should_record_live_golden_pair(dwell_secs) {
        return false;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let note = format!("Auto-aprendida desde VirtualDJ/History ({dwell_secs}s)");
    let gid = format!("gp-live-{}", uuid::Uuid::new_v4());
    let sql = "INSERT INTO dj_golden_pairs (id, from_track_id, to_track_id, from_artist, from_title, to_artist, to_title, custom_bonus, note, created_at)
               SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, 0.38, ?8, ?9
               WHERE NOT EXISTS (SELECT 1 FROM dj_golden_pairs WHERE from_track_id = ?2 AND to_track_id = ?3)";
    let mut q = sqlx::query(sql);
    for s in [&*gid, from.0, to.0, from.1, from.2, to.1, to.2, &note] {
        q = q.bind(s);
    }
    q.bind(now)
        .execute(conn)
        .await
        .is_ok_and(|r| r.rows_affected() > 0)
}

pub fn collect_stabilized_bounces(
    folders: &[PathBuf],
    seen: &mut HashMap<PathBuf, (u64, Instant, bool)>,
    now: Instant,
    win: Duration,
) -> Vec<PathBuf> {
    let mut ready = Vec::new();
    for e in folders
        .iter()
        .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
    {
        let (p, sz) = (e.path(), e.metadata().map_or(0, |m| m.len()));
        if !p.is_file() || sz == 0 || !is_file_valid(&p, &SUPPORTED_TRACKS_EXTENSIONS) {
            continue;
        }
        match seen.get_mut(&p) {
            Some((psz, ts, idx)) if *psz != sz => (*psz, *ts, *idx) = (sz, now, false),
            Some((_, ts, idx)) if !*idx && now.duration_since(*ts) >= win => {
                *idx = true;
                ready.push(p);
            }
            Some(_) => {}
            None => {
                seen.insert(p, (sz, now, false));
            }
        }
    }
    ready
}

pub async fn process_producer_bounce(db: &mut DB, path: &Path) -> Option<(String, f32, bool)> {
    let mut track = get_indexed_track_fast(&path.to_path_buf()).ok()?;
    track.is_own_production = Some(true);
    let (tid, p_str, p_clone) = (track.id.clone(), track.path.clone(), path.to_path_buf());
    db.upsert_scanned_tracks_batch(&[track], false).await.ok()?;
    let phase = tokio::task::spawn_blocking(move || {
        let dec = decode_audio_file_partial(&p_clone, 60.0).ok()?;
        let r = dec.right.as_deref().unwrap_or(&dec.left);
        let lr: Vec<f32> = dec.left.iter().zip(r).flat_map(|(a, b)| [*a, *b]).collect();
        Some(check_stereo_sub_mono_phase(&lr, dec.sample_rate))
    })
    .await
    .ok()
    .flatten()
    .unwrap_or(1.0);
    let _ = sqlx::query("UPDATE tracks SET is_own_production = 1, sub_phase_corr = ? WHERE id = ?")
        .bind(f64::from(phase))
        .bind(&tid)
        .execute(&mut db.connection)
        .await;
    let risk = phase < 0.25;
    if risk {
        let msg = format!("Sub-bass <120Hz phase correlation {phase:.2} (< 0.25 Club Mono risk)");
        let c = &mut db.connection;
        let _ = record_doctor_issue(c, &tid, &p_str, "CLUB_MONO_PHASE_RISK", "warning", &msg).await;
    }
    Some((tid, phase, risk))
}

async fn run_vdj_tick<R: Runtime>(
    app: &AppHandle<R>,
    last_key: &mut String,
    last_live: &mut Option<(String, String, String, Instant)>,
    booth_exp: &mut Option<Instant>,
) -> Option<()> {
    let sched = global_scheduler();
    if booth_exp.is_some_and(|exp| Instant::now() >= exp) {
        sched.governor.set_external_booth_mode(false);
        *booth_exp = None;
        clear_tray_now_playing(app);
    }
    let np = read_latest_vdj_now_playing()?;
    let key = format!("{}::{}", np.artist.to_lowercase(), np.title.to_lowercase());
    if key == *last_key {
        return None;
    }
    *last_key = key;
    sched.governor.set_external_booth_mode(true);
    *booth_exp = Some(Instant::now() + Duration::from_secs(360));
    let _ = app.emit("hera:vdj-now-playing", &np);
    let db_state = app.try_state::<DBState>()?;
    let mut conn = db_state.acquire_read_conn().await.ok()?;
    let q = "SELECT id, bpm, camelot FROM tracks WHERE scan_level >= 0 AND LOWER(title) LIKE ? AND LOWER(album_artist) LIKE ? LIMIT 1";
    let lo = |s: &str| format!("%{}%", s.to_lowercase());
    let found: Option<(String, Option<f64>, Option<String>)> = sqlx::query_as(q)
        .bind(lo(&np.title))
        .bind(lo(&np.artist))
        .fetch_optional(&mut *conn)
        .await
        .ok()
        .flatten();
    if let Some((ref tid, bpm_opt, ref cam_opt)) = found {
        sched.enqueue_or_promote(tid.clone(), ScanPriority::P0Immediate);
        let (bpm, cam) = (bpm_opt.unwrap_or(124.0), cam_opt.as_deref().unwrap_or(""));
        let nq = "SELECT id, camelot FROM tracks WHERE scan_level >= 0 AND id != ?1 AND bpm BETWEEN ?2 AND ?3 LIMIT 60";
        let cands: Vec<(String, Option<String>)> = sqlx::query_as(nq)
            .bind(tid)
            .bind(bpm * 0.92)
            .bind(bpm * 1.08)
            .fetch_all(&mut *conn)
            .await
            .unwrap_or_default();
        let nbrs: Vec<String> = cands
            .into_iter()
            .filter(|(_, c)| {
                cam.is_empty()
                    || c.as_deref()
                        .is_some_and(|k| compute_camelot_synergy(cam, k).0 >= 0.85)
            })
            .map(|(id, _)| id)
            .take(15)
            .collect();
        sched.enqueue_batch(nbrs, ScanPriority::P1Viewport);
    }
    drop(conn);
    let (bpm, cam) = found
        .as_ref()
        .map_or((None, None), |(_, b, c)| (*b, c.as_deref()));
    update_tray_now_playing(app, &np.artist, &np.title, bpm, cam);
    if let Some((tid, _, _)) = found {
        let now = Instant::now();
        if let Some((pid, pa, pt, started)) = last_live.take() {
            let mut db = db_state.get_lock().await;
            let (f, t) = ((&*pid, &*pa, &*pt), (&*tid, &*np.artist, &*np.title));
            let _ = record_live_golden_pair_if_eligible(
                &mut db.connection,
                f,
                t,
                now.duration_since(started).as_secs(),
            )
            .await;
        }
        *last_live = Some((tid, np.artist, np.title, now));
    }
    Some(())
}

async fn run_producer_tick<R: Runtime>(
    app: &AppHandle<R>,
    seen: &mut HashMap<PathBuf, (u64, Instant, bool)>,
) {
    let Some(db_state) = app.try_state::<DBState>() else {
        return;
    };
    let Ok(mut conn) = db_state.acquire_read_conn().await else {
        return;
    };
    let raw: Vec<String> =
        sqlx::query_scalar("SELECT folder_path FROM producer_watch_folders WHERE enabled = 1")
            .fetch_all(&mut *conn)
            .await
            .unwrap_or_default();
    drop(conn);
    let dirs: Vec<PathBuf> = raw
        .into_iter()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .collect();
    let mut indexed = HashSet::new();
    for b in collect_stabilized_bounces(&dirs, seen, Instant::now(), Duration::from_millis(1500)) {
        let mut db = db_state.get_lock().await;
        if let Some((tid, phase, risk)) = process_producer_bounce(&mut db, &b).await
            && indexed.insert(tid.clone())
        {
            drop(db);
            global_scheduler().enqueue_or_promote(tid.clone(), ScanPriority::P0Immediate);
            let ev = serde_json::json!({ "track_id": tid, "sub_phase_corr": phase, "club_mono_risk": risk });
            let _ = app.emit("hera:producer-bounce-indexed", ev);
        }
    }
}

pub fn spawn_folder_watchers<R: Runtime>(app: AppHandle<R>) {
    if WATCHERS_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    let app_vdj = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut iv = tokio::time::interval(Duration::from_millis(2500));
        let (mut lk, mut ll, mut bx) = (String::new(), None, None);
        loop {
            iv.tick().await;
            let _ = run_vdj_tick(&app_vdj, &mut lk, &mut ll, &mut bx).await;
        }
    });
    tauri::async_runtime::spawn(async move {
        let mut iv = tokio::time::interval(Duration::from_millis(500));
        let mut seen = HashMap::new();
        loop {
            iv.tick().await;
            run_producer_tick(&app, &mut seen).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{Connection, SqliteConnection};

    #[tokio::test]
    async fn test_live_golden_pair_and_bounce_stabilizer() {
        assert!(!should_record_live_golden_pair(30) && should_record_live_golden_pair(120));
        let mut c = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/migrations")
            .run_direct(&mut c)
            .await
            .unwrap();
        let (f, t) = (("a", "A", "1"), ("b", "B", "2"));
        assert!(!record_live_golden_pair_if_eligible(&mut c, f, t, 20).await);
        assert!(record_live_golden_pair_if_eligible(&mut c, f, t, 180).await);
        assert!(!record_live_golden_pair_if_eligible(&mut c, f, t, 180).await);
        let dir = std::env::temp_dir().join(format!("hera_fw_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav = dir.join("b.wav");
        std::fs::write(&wav, [0_u8; 64]).unwrap();
        let (mut seen, t0, w) = (HashMap::new(), Instant::now(), Duration::from_millis(1500));
        let (sl, t1) = (std::slice::from_ref(&dir), t0 + Duration::from_millis(1600));
        assert!(collect_stabilized_bounces(sl, &mut seen, t0, w).is_empty());
        assert_eq!(collect_stabilized_bounces(sl, &mut seen, t1, w), vec![wav]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
