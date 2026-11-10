use sqlx::SqliteConnection;

use super::error::AnyResult;

/// Fetch all track paths using a compile-time checked `sqlx::query_scalar!` query.
pub async fn fetch_all_track_paths(conn: &mut SqliteConnection) -> AnyResult<Vec<String>> {
    let paths = sqlx::query_scalar!("SELECT path FROM tracks")
        .fetch_all(conn)
        .await?;
    Ok(paths)
}

/// Check whether at least one track is flagged as a compilation (`is_compilation = 1`).
pub async fn has_compilation_tracks(conn: &mut SqliteConnection) -> AnyResult<bool> {
    let exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM tracks WHERE is_compilation = 1) AS "exists!: bool""#
    )
    .fetch_one(conn)
    .await?;
    Ok(exists)
}

/// Clear all tracks and playlists and reclaim SQLite pages (`VACUUM`).
pub async fn reset_tracks_and_playlists(conn: &mut SqliteConnection) -> AnyResult<()> {
    sqlx::query!("DELETE FROM tracks")
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM playlists")
        .execute(&mut *conn)
        .await?;
    sqlx::query!("VACUUM").execute(&mut *conn).await?;
    Ok(())
}

/// Update serialized JSON track IDs for a playlist by ID.
pub async fn update_playlist_tracks_raw(
    conn: &mut SqliteConnection,
    id: &str,
    tracks_json: &str,
) -> AnyResult<()> {
    sqlx::query!(
        "UPDATE playlists SET tracks = ? WHERE id = ?",
        tracks_json,
        id
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Rename a playlist by ID.
pub async fn rename_playlist_raw(
    conn: &mut SqliteConnection,
    id: &str,
    name: &str,
) -> AnyResult<()> {
    sqlx::query!("UPDATE playlists SET name = ? WHERE id = ?", name, id)
        .execute(conn)
        .await?;
    Ok(())
}

/// Delete a playlist by ID.
pub async fn delete_playlist_by_id(conn: &mut SqliteConnection, id: &str) -> AnyResult<()> {
    sqlx::query!("DELETE FROM playlists WHERE id = ?", id)
        .execute(conn)
        .await?;
    Ok(())
}

/// Delete an agent chat session and all its associated messages.
pub async fn delete_agent_session_and_messages(
    conn: &mut SqliteConnection,
    id: &str,
) -> AnyResult<()> {
    sqlx::query!("DELETE FROM agent_messages WHERE session_id = ?", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM agent_sessions WHERE id = ?", id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Monotonically bump `agent_sessions.updated_at` when messages are inserted or updated.
pub async fn touch_agent_session_updated_at(
    conn: &mut SqliteConnection,
    id: &str,
    now_ms: i64,
) -> AnyResult<()> {
    sqlx::query!(
        "UPDATE agent_sessions SET updated_at = MAX(updated_at + 1, ?) WHERE id = ?",
        now_ms,
        id
    )
    .execute(conn)
    .await?;
    Ok(())
}
