use std::path::Path;

use sqlx::SqliteConnection;

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
}

/// If a track with the same `(partial_hash_64kb, file_size_bytes)` exists in `tracks`
/// whose old `path` no longer exists on disk, update `tracks.path = new_path`
/// and return the existing `tracks.id` so playlists remain intact.
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

pub async fn apply_scanner_batch_updates(
    conn: &mut SqliteConnection,
    updates: &[ScannerRowUpdate],
) -> AnyResult<()> {
    if updates.is_empty() {
        return Ok(());
    }
    for u in updates {
        sqlx::query(
            "UPDATE tracks SET
                bpm = COALESCE(?, bpm),
                camelot = COALESCE(?, camelot),
                energy = COALESCE(?, energy),
                hot_cues = COALESCE(?, hot_cues),
                waveform_peaks_blob = COALESCE(?, waveform_peaks_blob),
                scan_level = MAX(scan_level, ?),
                is_transition_candidate = ?,
                is_fake_320kbps = ?
             WHERE id = ? AND user_locked_bpm_key = 0",
        )
        .bind(u.bpm)
        .bind(&u.camelot)
        .bind(u.energy)
        .bind(&u.hot_cues_json)
        .bind(&u.waveform_peaks_blob)
        .bind(u.scan_level)
        .bind(if u.is_transition_candidate { 1_i64 } else { 0_i64 })
        .bind(if u.is_fake_320kbps { 1_i64 } else { 0_i64 })
        .bind(&u.track_id)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

pub async fn load_pending_scanner_jobs(
    conn: &mut SqliteConnection,
    limit: i64,
) -> AnyResult<Vec<(String, String, i64)>> {
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT id, path, scan_level FROM tracks WHERE scan_level < 1 LIMIT ?",
    )
    .bind(limit)
    .fetch_all(conn)
    .await?;
    Ok(rows)
}
