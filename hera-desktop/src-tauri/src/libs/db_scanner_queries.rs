use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::{Connection, SqliteConnection};

use super::error::AnyResult;

#[derive(Debug, Clone)]
pub struct ScannerRowUpdate {
    pub track_id: String,
    pub bpm: Option<f64>,
    pub camelot: Option<String>,
    pub energy: Option<f64>,
    pub hot_cues_json: Option<String>,
    pub waveform_peaks_blob: Option<Vec<u8>>,
    pub scan_level: i64,
    pub is_transition_candidate: bool,
    pub is_fake_320kbps: bool,
    pub duration_sec: Option<u32>,
}

/// If a track with the same `(partial_hash_64kb, file_size_bytes)` exists in `tracks`
/// whose old `path` no longer exists on disk, update `tracks.path = new_path`
/// and return the existing `tracks.id` so playlists remain intact.
#[allow(dead_code)]
pub async fn try_heal_moved_track_path(
    conn: &mut SqliteConnection,
    partial_hash_64kb: &str,
    file_size_bytes: i64,
    new_path: &str,
) -> AnyResult<Option<String>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, path FROM tracks WHERE partial_hash_64kb = ? AND file_size_bytes = ?",
    )
    .bind(partial_hash_64kb)
    .bind(file_size_bytes)
    .fetch_all(&mut *conn)
    .await?;

    for (id, old_path) in rows {
        if old_path == new_path {
            return Ok(Some(id));
        }
        if !Path::new(&old_path).exists() {
            sqlx::query("UPDATE tracks SET path = ? WHERE id = ?")
                .bind(new_path)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
            return Ok(Some(id));
        }
    }

    Ok(None)
}

pub async fn record_doctor_issue(
    conn: &mut SqliteConnection,
    track_id: &str,
    file_path: &str,
    issue_kind: &str,
    severity: &str,
    details: &str,
) -> AnyResult<()> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    sqlx::query(
        "INSERT INTO library_doctor_issues (
            track_id, file_path, issue_kind, severity, details, resolved, detected_at_unix_ms
         ) VALUES (?, ?, ?, ?, ?, 0, ?)",
    )
    .bind(track_id)
    .bind(file_path)
    .bind(issue_kind)
    .bind(severity)
    .bind(details)
    .bind(now_ms)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn apply_scanner_batch_updates(
    conn: &mut SqliteConnection,
    updates: &[ScannerRowUpdate],
) -> AnyResult<()> {
    if updates.is_empty() {
        return Ok(());
    }
    let mut tx = conn.begin().await?;
    for u in updates {
        let dur_i64 = u.duration_sec.map(|d| d as i64);
        sqlx::query(
            "UPDATE tracks SET
                bpm = CASE WHEN user_locked_bpm_key = 0 THEN COALESCE(?, bpm) ELSE bpm END,
                camelot = CASE WHEN user_locked_bpm_key = 0 THEN COALESCE(?, camelot) ELSE camelot END,
                energy = COALESCE(?, energy),
                hot_cues = COALESCE(?, hot_cues),
                waveform_peaks_blob = COALESCE(?, waveform_peaks_blob),
                scan_level = CASE WHEN ? < 0 THEN -1 ELSE MAX(scan_level, ?) END,
                is_transition_candidate = ?,
                is_fake_320kbps = ?,
                duration = CASE WHEN ? IS NOT NULL AND ? > 0 THEN ? ELSE duration END,
                is_mix_over_10m = CASE WHEN ? IS NOT NULL AND ? > 600 THEN 1 ELSE is_mix_over_10m END
             WHERE id = ?",
        )
        .bind(u.bpm)
        .bind(&u.camelot)
        .bind(u.energy)
        .bind(&u.hot_cues_json)
        .bind(&u.waveform_peaks_blob)
        .bind(u.scan_level)
        .bind(u.scan_level)
        .bind(if u.is_transition_candidate { 1_i64 } else { 0_i64 })
        .bind(if u.is_fake_320kbps { 1_i64 } else { 0_i64 })
        .bind(dur_i64)
        .bind(dur_i64)
        .bind(dur_i64)
        .bind(dur_i64)
        .bind(dur_i64)
        .bind(&u.track_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn load_pending_scanner_jobs(
    conn: &mut SqliteConnection,
    limit: i64,
) -> AnyResult<Vec<(String, String, i64)>> {
    let rows: Vec<(String, String, i64)> =
        sqlx::query_as("SELECT id, path, scan_level FROM tracks WHERE scan_level = 0 LIMIT ?")
            .bind(limit)
            .fetch_all(conn)
            .await?;
    Ok(rows)
}

pub async fn load_pending_full_pass_jobs(
    conn: &mut SqliteConnection,
    limit: i64,
) -> AnyResult<Vec<(String, String)>> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT id, path FROM tracks WHERE scan_level = 1 LIMIT ?")
            .bind(limit)
            .fetch_all(conn)
            .await?;
    Ok(rows)
}

pub async fn fetch_track_path_and_level(
    conn: &mut SqliteConnection,
    track_id: &str,
) -> AnyResult<Option<(String, i64)>> {
    let row: Option<(String, i64)> =
        sqlx::query_as("SELECT path, scan_level FROM tracks WHERE id = ?")
            .bind(track_id)
            .fetch_optional(conn)
            .await?;
    Ok(row)
}

pub async fn count_scanner_levels(conn: &mut SqliteConnection) -> AnyResult<(usize, usize, usize)> {
    let row: (i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(CASE WHEN scan_level >= 1 THEN 1 ELSE 0 END), 0), COALESCE(SUM(CASE WHEN scan_level >= 2 THEN 1 ELSE 0 END), 0) FROM tracks WHERE scan_level >= 0",
    )
    .fetch_one(conn)
    .await?;
    Ok((
        row.0.max(0) as usize,
        row.1.max(0) as usize,
        row.2.max(0) as usize,
    ))
}

pub async fn delete_short_sfx_tracks(conn: &mut SqliteConnection, ids: &[String]) -> AnyResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let mut tx = conn.begin().await?;
    for id in ids {
        sqlx::query("DELETE FROM tracks WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
