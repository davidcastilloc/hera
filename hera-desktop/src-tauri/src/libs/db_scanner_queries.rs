use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::{Connection, SqliteConnection};

pub use super::db_scanner_row::ScannerRowUpdate;
use super::db_scanner_row::merge_preserving_manual_hot_cues;
use super::error::AnyResult;

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
            let conflict: Option<(i64,)> =
                sqlx::query_as("SELECT 1 FROM tracks WHERE path = ? LIMIT 1")
                    .bind(new_path)
                    .fetch_optional(&mut *conn)
                    .await?;
            if conflict.is_none() {
                sqlx::query("UPDATE tracks SET path = ? WHERE id = ?")
                    .bind(new_path)
                    .bind(&id)
                    .execute(&mut *conn)
                    .await?;
                return Ok(Some(id));
            }
        }
    }
    Ok(None)
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub async fn record_doctor_issue(
    conn: &mut SqliteConnection,
    track_id: &str,
    file_path: &str,
    issue_kind: &str,
    severity: &str,
    details: &str,
) -> AnyResult<()> {
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM library_doctor_issues WHERE track_id = ? AND issue_kind = ? AND resolved = 0 LIMIT 1",
    )
    .bind(track_id)
    .bind(issue_kind)
    .fetch_optional(&mut *conn)
    .await?;
    if existing.is_some() {
        return Ok(());
    }
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
    .bind(now_unix_ms())
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
        let cur: Option<(String, Option<f64>, Option<String>, i64)> = sqlx::query_as(
            "SELECT path, legacy_bpm, hot_cues, user_locked_cues FROM tracks WHERE id = ?",
        )
        .bind(&u.track_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (file_path, legacy_bpm, existing_cues, locked_cues) =
            cur.unwrap_or_else(|| (String::new(), None, None, 0));

        let merged_cues = if locked_cues != 0 {
            None
        } else {
            merge_preserving_manual_hot_cues(existing_cues.as_deref(), u.hot_cues_json.as_deref())
        };
        let bpm_diff = match (legacy_bpm, u.bpm) {
            (Some(leg), Some(det)) if leg > 0.0 => (leg - det).abs() > 1.8,
            _ => false,
        };
        let dur_i64 = u.duration_sec.map(i64::from);

        sqlx::query(
            "UPDATE tracks SET
                bpm = CASE WHEN user_locked_bpm_key = 0 THEN COALESCE(?, bpm) ELSE bpm END,
                camelot = CASE WHEN user_locked_bpm_key = 0 THEN COALESCE(?, camelot) ELSE camelot END,
                energy = COALESCE(?, energy),
                hot_cues = COALESCE(?, hot_cues),
                waveform_peaks_blob = COALESCE(?, waveform_peaks_blob),
                waveform_peaks_b64 = COALESCE(?, waveform_peaks_b64),
                embedding_64d_b64 = COALESCE(?, embedding_64d_b64),
                loudness_lufs = COALESCE(?, loudness_lufs),
                true_peak_dbfs = COALESCE(?, true_peak_dbfs),
                replay_gain_db = COALESCE(?, replay_gain_db),
                entry_bpm = COALESCE(?, entry_bpm),
                exit_bpm = COALESCE(?, exit_bpm),
                has_intro_vocals = COALESCE(?, has_intro_vocals),
                has_outro_vocals = COALESCE(?, has_outro_vocals),
                has_audio_skips = COALESCE(?, has_audio_skips),
                is_variable_bpm = CASE WHEN ? = 1 THEN 1 ELSE is_variable_bpm END,
                has_bpm_trust_diff = CASE WHEN ? = 1 THEN 1 ELSE has_bpm_trust_diff END,
                scan_level = CASE WHEN ? < 0 THEN -1 ELSE MAX(scan_level, ?) END,
                is_transition_candidate = ?,
                is_fake_320kbps = CASE WHEN ? = 1 THEN 1 ELSE is_fake_320kbps END,
                duration = CASE WHEN ? IS NOT NULL AND ? > 0 THEN ? ELSE duration END,
                is_mix_over_10m = CASE WHEN ? IS NOT NULL AND ? > 600 THEN 1 ELSE is_mix_over_10m END
             WHERE id = ?",
        )
        .bind(u.bpm)
        .bind(&u.camelot)
        .bind(u.energy)
        .bind(&merged_cues)
        .bind(&u.waveform_peaks_blob)
        .bind(&u.waveform_peaks_b64)
        .bind(&u.embedding_64d_b64)
        .bind(u.loudness_lufs)
        .bind(u.true_peak_dbfs)
        .bind(u.replay_gain_db)
        .bind(u.entry_bpm)
        .bind(u.exit_bpm)
        .bind(u.has_intro_vocals)
        .bind(u.has_outro_vocals)
        .bind(u.has_audio_skips)
        .bind(if u.is_transition_candidate { 1_i64 } else { 0_i64 })
        .bind(if bpm_diff { 1_i64 } else { 0_i64 })
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

        if u.has_audio_skips == Some(true) && !file_path.is_empty() {
            let details = u
                .glitch_details
                .as_deref()
                .unwrap_or("Audio skip / dropout detected during FullPass");
            record_doctor_issue(
                &mut tx,
                &u.track_id,
                &file_path,
                "AUDIO_GLITCH_SKIP",
                "warning",
                details,
            )
            .await?;
        }
        if bpm_diff
            && !file_path.is_empty()
            && let (Some(leg), Some(det)) = (legacy_bpm, u.bpm)
        {
            let msg = format!(
                "Legacy BPM {leg:.2} differs from HERA acoustic BPM {det:.2} by {:.2} BPM",
                (det - leg).abs()
            );
            record_doctor_issue(
                &mut tx,
                &u.track_id,
                &file_path,
                "BPM_TRUST_DIFF",
                "warning",
                &msg,
            )
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

pub async fn load_pending_scanner_jobs(
    conn: &mut SqliteConnection,
    limit: i64,
) -> AnyResult<Vec<(String, String, i64)>> {
    Ok(
        sqlx::query_as("SELECT id, path, scan_level FROM tracks WHERE scan_level = 0 LIMIT ?")
            .bind(limit)
            .fetch_all(conn)
            .await?,
    )
}

pub async fn load_pending_full_pass_jobs(
    conn: &mut SqliteConnection,
    limit: i64,
) -> AnyResult<Vec<(String, String)>> {
    Ok(
        sqlx::query_as("SELECT id, path FROM tracks WHERE scan_level = 1 LIMIT ?")
            .bind(limit)
            .fetch_all(conn)
            .await?,
    )
}

pub async fn fetch_track_path_and_level(
    conn: &mut SqliteConnection,
    track_id: &str,
) -> AnyResult<Option<(String, i64)>> {
    Ok(
        sqlx::query_as("SELECT path, scan_level FROM tracks WHERE id = ?")
            .bind(track_id)
            .fetch_optional(conn)
            .await?,
    )
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
