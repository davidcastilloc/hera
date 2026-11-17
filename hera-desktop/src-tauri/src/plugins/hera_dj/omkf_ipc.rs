//! OMKF v1 folder crate IPC facade (`OmkfFsCatalog` primary + SQLite fallback when roots empty).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use hera_ipc::dj_types::{
    FolderCrateDetail, FolderCrateSummary, OrganizeFolderRequest, OrganizeFolderResponse,
};
use hera_node_rs::scanner::omkf_capsule::read_omkf_capsule;
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};
use tauri::State;

use crate::libs::fast_library_scan::{
    parse_crate_md_sections, run_omkf_organize_folder, write_crate_md_sections,
};
use crate::libs::omkf_fs_catalog::global_omkf_catalog;
use crate::libs::omkf_fs_organize::{organize_folder_fs, save_folder_rules_fs};
use crate::libs::omkf_sync::compute_folder_crate_id;
use crate::plugins::config::ConfigManager;
use crate::plugins::db::DBState;

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

fn summary_from_row(r: &SqliteRow) -> FolderCrateSummary {
    let rules: String = r.try_get("rules_markdown").unwrap_or_default();
    let ns_ne = |k: &str| {
        r.try_get::<Option<String>, _>(k)
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
    };
    FolderCrateSummary {
        crate_id: r.try_get("crate_id").unwrap_or_default(),
        folder_path: r.try_get("folder_path").unwrap_or_default(),
        display_name: r.try_get("display_name").unwrap_or_default(),
        tree_seal_64: r.try_get("tree_seal_64").unwrap_or_default(),
        track_count: r.try_get::<i64, _>("track_count").unwrap_or(0).max(0) as usize,
        video_count: r.try_get::<i64, _>("video_count").unwrap_or(0).max(0) as usize,
        full_count: r.try_get::<i64, _>("full_count").unwrap_or(0).max(0) as usize,
        bpm_min: r.try_get::<Option<f64>, _>("bpm_min").ok().flatten(),
        bpm_max: r.try_get::<Option<f64>, _>("bpm_max").ok().flatten(),
        dominant_camelot: ns_ne("dominant_camelot"),
        dominant_genre: ns_ne("dominant_genre"),
        era_span: ns_ne("era_span"),
        lufs_avg: r.try_get::<Option<f64>, _>("lufs_avg").ok().flatten(),
        has_custom_rules: !rules.trim().is_empty(),
        updated_at_unix_ms: r
            .try_get::<Option<i64>, _>("updated_at_unix_ms")
            .ok()
            .flatten(),
        parent_crate_id: None,
        rel_path: String::new(),
        depth: 0,
        subfolder_count: 0,
    }
}

async fn backfill_folder_crates_from_tracks(conn: &mut SqliteConnection) -> Result<(), String> {
    let track_rows = sqlx::query(
        "SELECT id, path, bpm, camelot, scan_level, is_video FROM tracks WHERE crate_id IS NULL OR crate_id = ''",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| e.to_string())?;
    if track_rows.is_empty() {
        return Ok(());
    }
    struct FolderAgg {
        cid: String,
        folder: String,
        disp: String,
        track_n: i64,
        video_n: i64,
        full_n: i64,
        bmin: Option<f64>,
        bmax: Option<f64>,
        cam: Option<String>,
    }
    let mut groups: HashMap<String, FolderAgg> = HashMap::new();
    for r in &track_rows {
        let id: String = r.try_get("id").unwrap_or_default();
        let path_str: String = r.try_get("path").unwrap_or_default();
        let Some(parent) = Path::new(&path_str)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        else {
            continue;
        };
        let cid = compute_folder_crate_id(parent);
        let _ = sqlx::query("UPDATE tracks SET crate_id = ?1 WHERE id = ?2")
            .bind(&cid)
            .bind(&id)
            .execute(&mut *conn)
            .await;
        let bpm = r
            .try_get::<Option<f64>, _>("bpm")
            .ok()
            .flatten()
            .filter(|&b| b > 0.0);
        let cam = r
            .try_get::<Option<String>, _>("camelot")
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
        let lvl: i64 = r.try_get("scan_level").unwrap_or(0);
        let is_vid = r.try_get::<i64, _>("is_video").unwrap_or(0) != 0;
        let disp = parent
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Carpeta")
            .to_string();
        let g = groups.entry(cid.clone()).or_insert_with(|| FolderAgg {
            cid,
            folder: parent.to_string_lossy().into_owned(),
            disp,
            track_n: 0,
            video_n: 0,
            full_n: 0,
            bmin: None,
            bmax: None,
            cam: None,
        });
        g.track_n += 1;
        g.video_n += i64::from(is_vid);
        g.full_n += i64::from(lvl >= 2);
        if let Some(b) = bpm {
            g.bmin = Some(g.bmin.map_or(b, |m| m.min(b)));
            g.bmax = Some(g.bmax.map_or(b, |m| m.max(b)));
        }
        if g.cam.is_none() && cam.is_some() {
            g.cam = cam;
        }
    }
    let ts = now_ms();
    for g in groups.into_values() {
        let _ = sqlx::query(
            "INSERT INTO folder_crates (crate_id,folder_path,display_name,tree_seal_64,dir_mtime_ns,track_count,video_count,full_count,bpm_min,bpm_max,dominant_camelot,updated_at_unix_ms)
             VALUES (?1,?2,?3,'0',0,?4,?5,?6,?7,?8,?9,?10)
             ON CONFLICT(crate_id) DO UPDATE SET track_count=MAX(folder_crates.track_count,excluded.track_count), full_count=MAX(folder_crates.full_count,excluded.full_count)",
        )
        .bind(&g.cid)
        .bind(&g.folder)
        .bind(&g.disp)
        .bind(g.track_n)
        .bind(g.video_n)
        .bind(g.full_n)
        .bind(g.bmin)
        .bind(g.bmax)
        .bind(&g.cam)
        .bind(ts)
        .execute(&mut *conn)
        .await;
    }
    Ok(())
}

#[tauri::command]
pub async fn omkf_list_folder_crates(
    config: State<'_, ConfigManager>,
    db_state: State<'_, DBState>,
) -> Result<Vec<FolderCrateSummary>, String> {
    let roots = config.get().map(|c| c.library_folders).unwrap_or_default();
    if !roots.is_empty() {
        return Ok(global_omkf_catalog().list_folders(&roots));
    }
    {
        let mut conn = db_state
            .acquire_read_conn()
            .await
            .map_err(|e| e.to_string())?;
        let rows = sqlx::query("SELECT * FROM folder_crates ORDER BY display_name ASC")
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| e.to_string())?;
        if !rows.is_empty() {
            return Ok(rows.iter().map(summary_from_row).collect());
        }
    }
    let mut db = db_state.get_lock().await;
    let _ = backfill_folder_crates_from_tracks(&mut db.connection).await;
    let rows = sqlx::query("SELECT * FROM folder_crates ORDER BY display_name ASC")
        .fetch_all(&mut db.connection)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.iter().map(summary_from_row).collect())
}

#[tauri::command]
pub async fn omkf_read_folder_crate(
    config: State<'_, ConfigManager>,
    db_state: State<'_, DBState>,
    crate_id: String,
) -> Result<Option<FolderCrateDetail>, String> {
    let roots = config.get().map(|c| c.library_folders).unwrap_or_default();
    if !roots.is_empty() {
        let (detail, unanalyzed) = global_omkf_catalog().read_folder(&crate_id, &roots);
        if !unanalyzed.is_empty()
            && !super::scanner_v3::global_scheduler()
                .governor
                .is_deck_playing()
        {
            super::scanner_worker::enqueue_fs_discovered_tracks(unanalyzed);
        }
        return Ok(detail);
    }
    let mut conn = db_state
        .acquire_read_conn()
        .await
        .map_err(|e| e.to_string())?;
    let Some(row) = sqlx::query("SELECT * FROM folder_crates WHERE crate_id = ? LIMIT 1")
        .bind(&crate_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    let summary = summary_from_row(&row);
    let db_rules: String = row.try_get("rules_markdown").unwrap_or_default();
    let track_ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM tracks WHERE crate_id = ? ORDER BY album_artist, title")
            .bind(&crate_id)
            .fetch_all(&mut *conn)
            .await
            .unwrap_or_default();
    let dir = Path::new(&summary.folder_path);
    let disk_online = dir.is_dir();
    let hydrated_from_omkf =
        disk_online && read_omkf_capsule(&dir.join(".hera/tracks.omkf")).is_ok();
    let md_raw = disk_online
        .then(|| std::fs::read_to_string(dir.join(".hera/crate.md")).ok())
        .flatten()
        .unwrap_or_default();
    let (_, md_rules, ledger_entries) = parse_crate_md_sections(&md_raw);
    let rules_markdown = if md_rules.is_empty() {
        db_rules
    } else {
        md_rules
    };
    Ok(Some(FolderCrateDetail {
        summary,
        rules_markdown,
        ledger_entries,
        track_ids,
        hydrated_from_omkf,
        disk_online,
        subfolders: Vec::new(),
        tracks: Vec::new(),
    }))
}

#[tauri::command]
pub async fn omkf_save_crate_rules(
    config: State<'_, ConfigManager>,
    db_state: State<'_, DBState>,
    crate_id: String,
    rules_markdown: String,
) -> Result<bool, String> {
    let roots = config.get().map(|c| c.library_folders).unwrap_or_default();
    if !roots.is_empty() {
        return save_folder_rules_fs(global_omkf_catalog(), &roots, &crate_id, &rules_markdown);
    }
    let mut db = db_state.get_lock().await;
    let res = sqlx::query(
        "UPDATE folder_crates SET rules_markdown = ?1, updated_at_unix_ms = ?2 WHERE crate_id = ?3",
    )
    .bind(&rules_markdown)
    .bind(now_ms())
    .bind(&crate_id)
    .execute(&mut db.connection)
    .await
    .map_err(|e| e.to_string())?;
    let fp: Option<String> =
        sqlx::query_scalar("SELECT folder_path FROM folder_crates WHERE crate_id = ?1")
            .bind(&crate_id)
            .fetch_optional(&mut db.connection)
            .await
            .unwrap_or(None);
    if let Some(dir) = fp.map(PathBuf::from).filter(|p| p.is_dir()) {
        let raw = std::fs::read_to_string(dir.join(".hera/crate.md")).unwrap_or_default();
        let (_, _, ledger) = parse_crate_md_sections(&raw);
        write_crate_md_sections(&dir, &rules_markdown, &ledger)?;
    }
    Ok(res.rows_affected() > 0)
}

#[tauri::command]
pub async fn omkf_preview_or_apply_organize(
    config: State<'_, ConfigManager>,
    db_state: State<'_, DBState>,
    request: OrganizeFolderRequest,
) -> Result<OrganizeFolderResponse, String> {
    let roots = config.get().map(|c| c.library_folders).unwrap_or_default();
    if !roots.is_empty() {
        return organize_folder_fs(global_omkf_catalog(), &roots, request);
    }
    let mut db = db_state.get_lock().await;
    run_omkf_organize_folder(&mut db.connection, request).await
}
