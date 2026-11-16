//! OMKF v1 folder crate IPC facade & `.hera/crate.md` custom booth rules.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use hera_ipc::dj_types::{
    FolderCrateDetail, FolderCrateSummary, OrganizeFolderRequest, OrganizeFolderResponse,
};
use hera_node_rs::scanner::omkf_capsule::read_omkf_capsule;
use sqlx::{Row, sqlite::SqliteRow};
use tauri::State;

use crate::libs::fast_library_scan::{
    parse_crate_md_sections, run_omkf_organize_folder, write_crate_md_sections,
};
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
    }
}

#[tauri::command]
pub async fn omkf_list_folder_crates(
    db_state: State<'_, DBState>,
) -> Result<Vec<FolderCrateSummary>, String> {
    let mut conn = db_state
        .acquire_read_conn()
        .await
        .map_err(|e| e.to_string())?;
    let rows = sqlx::query("SELECT * FROM folder_crates ORDER BY display_name ASC")
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.iter().map(summary_from_row).collect())
}

#[tauri::command]
pub async fn omkf_read_folder_crate(
    db_state: State<'_, DBState>,
    crate_id: String,
) -> Result<Option<FolderCrateDetail>, String> {
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
    }))
}

#[tauri::command]
pub async fn omkf_save_crate_rules(
    db_state: State<'_, DBState>,
    crate_id: String,
    rules_markdown: String,
) -> Result<bool, String> {
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
    db_state: State<'_, DBState>,
    request: OrganizeFolderRequest,
) -> Result<OrganizeFolderResponse, String> {
    let mut db = db_state.get_lock().await;
    run_omkf_organize_folder(&mut db.connection, request).await
}
