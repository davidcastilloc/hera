//! Library Doctor, Producer folders, 2-Stage 64D Sonic Twins, Pre-Gig Audit & Shazam ID.

use std::time::{SystemTime, UNIX_EPOCH};

use hera_ipc::dj_types::{DoctorIssueItem, PreGigAuditReport, ProducerWatchFolder, SonicTwinMatch};
use sqlx::Row;
use tauri::State;
use uuid::Uuid;

use super::scanner_v3::compute_sonic_twins;
use crate::libs::fast_library_scan::{
    ShazamIdentifyResult, compute_pre_gig_audit, run_shazam_identify,
};
use crate::plugins::db::DBState;

#[tauri::command]
pub async fn scanner_get_doctor_issues(
    db_state: State<'_, DBState>,
    include_resolved: Option<bool>,
) -> Result<Vec<DoctorIssueItem>, String> {
    let mut conn = db_state
        .acquire_read_conn()
        .await
        .map_err(|e| e.to_string())?;
    let w = if include_resolved.unwrap_or(false) {
        ""
    } else {
        "WHERE resolved = 0 "
    };
    let sql = format!(
        "SELECT id,track_id,file_path,issue_kind,severity,details,detected_at_unix_ms,resolved FROM library_doctor_issues {w}ORDER BY detected_at_unix_ms DESC LIMIT 500"
    );
    let rows = sqlx::query(&sql)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| e.to_string())?;
    let gs = |r: &sqlx::sqlite::SqliteRow, k: &str| r.try_get::<String, _>(k).unwrap_or_default();
    Ok(rows
        .iter()
        .map(|r| DoctorIssueItem {
            id: r.try_get("id").ok(),
            track_id: gs(r, "track_id"),
            file_path: gs(r, "file_path"),
            issue_kind: gs(r, "issue_kind"),
            severity: gs(r, "severity"),
            details: gs(r, "details"),
            detected_at_unix_ms: r.try_get("detected_at_unix_ms").ok(),
            resolved: r.try_get::<i64, _>("resolved").unwrap_or(0) != 0,
        })
        .collect())
}

#[tauri::command]
pub async fn scanner_configure_producer_folders(
    db_state: State<'_, DBState>,
    folders: Vec<String>,
) -> Result<Vec<ProducerWatchFolder>, String> {
    let mut db = db_state.get_lock().await;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let _ = sqlx::query("UPDATE producer_watch_folders SET enabled = 0")
        .execute(&mut db.connection)
        .await;
    for fp in folders
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        let q = "INSERT INTO producer_watch_folders (id, folder_path, enabled, created_at) VALUES (?1, ?2, 1, ?3) ON CONFLICT(folder_path) DO UPDATE SET enabled = 1";
        sqlx::query(q)
            .bind(format!("pw-{}", Uuid::new_v4()))
            .bind(&fp)
            .bind(now)
            .execute(&mut db.connection)
            .await
            .map_err(|e| e.to_string())?;
    }
    let rows = sqlx::query(
        "SELECT id, folder_path, enabled, created_at FROM producer_watch_folders WHERE enabled = 1 ORDER BY created_at DESC",
    )
    .fetch_all(&mut db.connection)
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|r| ProducerWatchFolder {
            id: r.try_get("id").unwrap_or_default(),
            folder_path: r.try_get("folder_path").unwrap_or_default(),
            enabled: r.try_get::<i64, _>("enabled").unwrap_or(0) != 0,
            created_at: r.try_get("created_at").ok(),
        })
        .collect())
}

#[tauri::command]
pub async fn scanner_find_sonic_twins(
    db_state: State<'_, DBState>,
    track_id: String,
    limit: Option<usize>,
) -> Result<Vec<SonicTwinMatch>, String> {
    let mut conn = db_state
        .acquire_read_conn()
        .await
        .map_err(|e| e.to_string())?;
    compute_sonic_twins(&mut conn, &track_id, limit.unwrap_or(15)).await
}

#[tauri::command]
pub async fn scanner_audit_pre_gig_crate(
    db_state: State<'_, DBState>,
    target_profile: String,
    track_ids: Option<Vec<String>>,
) -> Result<PreGigAuditReport, String> {
    let mut conn = db_state
        .acquire_read_conn()
        .await
        .map_err(|e| e.to_string())?;
    compute_pre_gig_audit(&mut conn, &target_profile, track_ids.as_deref()).await
}

#[tauri::command]
pub async fn identify_track_shazam(
    db_state: State<'_, DBState>,
    track_id: String,
    path: Option<String>,
) -> Result<Option<ShazamIdentifyResult>, String> {
    let mut db = db_state.get_lock().await;
    run_shazam_identify(&mut db.connection, &track_id, path.as_deref()).await
}
