use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Connection, SqliteConnection};
use std::path::PathBuf;

use super::error::AnyResult;
use super::playlist::Playlist;
use super::track::{Track, TrackGroup};

// Single source of truth for supported audio formats (extension, MIME type).
// KEEP IN SYNC with Tauri's file associations in tauri.conf.json
pub const SUPPORTED_AUDIO_FORMATS: [(&str, &str); 11] = [
    ("mp3", "audio/mpeg"), /* mp3 / mp4 */
    ("aac", "audio/aac"),
    ("m4a", "audio/mp4"),
    ("3gp", "audio/3gpp"),
    ("wav", "audio/wav"),
    ("aiff", "audio/aiff"),
    ("aif", "audio/aiff"),
    ("ogg", "audio/ogg"), /* Opus */
    ("opus", "audio/opus"),
    ("flac", "audio/flac"), /* Flac */
    ("weba", "audio/webm"), /* Web media */
];

// Derived at compile time from SUPPORTED_AUDIO_FORMATS
pub const SUPPORTED_TRACKS_EXTENSIONS: [&str; 11] = {
    let mut exts = [""; 11];
    let mut i = 0;
    while i < 11 {
        exts[i] = SUPPORTED_AUDIO_FORMATS[i].0;
        i += 1;
    }
    exts
};

pub const SUPPORTED_PLAYLISTS_EXTENSIONS: [&str; 2] = ["m3u", "m3u8"];

/// Get the MIME content type for a supported audio file extension.
pub fn content_type_for_extension(ext: &str) -> Option<&'static str> {
    SUPPORTED_AUDIO_FORMATS
        .iter()
        .find(|(e, _)| e.eq_ignore_ascii_case(ext))
        .map(|(_, ct)| *ct)
}

/** ----------------------------------------------------------------------------
 * Databases
 * exposes databases for tracks and playlists
 * * -------------------------------------------------------------------------- */

pub struct DB {
    pub connection: SqliteConnection,
}

impl DB {
    /**
     * Get only the file paths of all tracks in the database (cheaper than get_all_tracks)
     */
    pub async fn get_all_track_paths(&mut self) -> AnyResult<Vec<String>> {
        super::db_offline_queries::fetch_all_track_paths(&mut self.connection).await
    }

    /**
     * Get all the tracks (and their content) from the database
     */
    pub async fn get_all_tracks(&mut self) -> AnyResult<Vec<Track>> {
        let tracks = sqlx::query_as::<_, Track>("SELECT * FROM tracks")
            .fetch_all(&mut self.connection)
            .await?;
        Ok(tracks)
    }

    /**
     * Get tracks (and their content) given a set of document IDs
     */
    pub async fn get_tracks(&mut self, track_ids: &Vec<String>) -> AnyResult<Vec<Track>> {
        if track_ids.is_empty() {
            return Ok(vec![]);
        }

        // Construct `IN` clause with placeholders
        let in_placeholders = track_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");

        // Construct `ORDER BY CASE` using bindable expressions like: CASE id WHEN ? THEN 0 ...
        let mut order_by_case = String::from("CASE id ");
        for index in 0..track_ids.len() {
            order_by_case.push_str("WHEN ? THEN ");
            order_by_case.push_str(&index.to_string());
            order_by_case.push(' ');
        }
        order_by_case.push_str("END");

        // Improve me once https://github.com/launchbadge/sqlx/issues/875 is fixed
        let sql = format!(
            "SELECT * FROM tracks WHERE id IN ({}) ORDER BY {}",
            in_placeholders, order_by_case
        );

        // Build query and bind all UUIDs twice (once for IN, once for ORDER BY CASE)
        let mut query = sqlx::query_as::<_, Track>(&sql);

        for id in track_ids {
            query = query.bind(id); // for IN (...)
        }

        for id in track_ids {
            query = query.bind(id); // for ORDER BY CASE
        }

        let tracks = query.fetch_all(&mut self.connection).await?;
        Ok(tracks)
    }

    /**
     * Get a single track by ID
     */
    pub async fn get_track(&mut self, track_id: &str) -> AnyResult<Option<Track>> {
        let track = sqlx::query_as::<_, Track>("SELECT * FROM tracks WHERE id = ?")
            .bind(track_id)
            .fetch_optional(&mut self.connection)
            .await?;

        Ok(track)
    }

    /**
     * Get tracks (and their content) given a set of document IDs
     */
    pub async fn update_track(&mut self, track: Track) -> AnyResult<Track> {
        sqlx::query(
            r#"
            UPDATE tracks SET
                path = ?,
                title = ?,
                album = ?,
                album_artist = ?,
                artists = ?,
                genres = ?,
                year = ?,
                duration = ?,
                track_no = ?,
                track_of = ?,
                disk_no = ?,
                disk_of = ?,
                is_compilation = ?,
                bpm = ?,
                camelot = ?,
                energy = ?,
                hera_seal = ?,
                hot_cues = ?
            WHERE id = ?
            "#,
        )
        .bind(&track.path)
        .bind(&track.title)
        .bind(&track.album)
        .bind(&track.album_artist)
        .bind(json!(&track.artists))
        .bind(json!(&track.genres))
        .bind(track.year)
        .bind(track.duration)
        .bind(track.track_no)
        .bind(track.track_of)
        .bind(track.disk_no)
        .bind(track.disk_of)
        .bind(track.is_compilation)
        .bind(track.bpm.unwrap_or(124.0))
        .bind(track.camelot.as_deref().unwrap_or("8A"))
        .bind(track.energy.unwrap_or(0.80))
        .bind(&track.hera_seal)
        .bind(json!(track.hot_cues.as_deref().unwrap_or(&[])))
        .bind(&track.id)
        .execute(&mut self.connection)
        .await?;

        Ok(track)
    }

    /** Delete multiple tracks by ID */
    pub async fn remove_tracks(&mut self, track_ids: &Vec<String>) -> AnyResult<()> {
        let placeholders = track_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query = format!("DELETE FROM tracks WHERE id IN ({})", placeholders);

        let mut query_builder = sqlx::query(&query);

        for id in track_ids {
            query_builder = query_builder.bind(id);
        }

        query_builder.execute(&mut self.connection).await?;

        Ok(())
    }

    /**
     * Insert multiple tracks in the DB, will fail in case there is a duplicate
     * unique key (like track.path)
     */
    pub async fn insert_tracks(&mut self, tracks: Vec<Track>) -> AnyResult<()> {
        let mut tx = self.connection.begin().await?;

        for track in tracks {
            sqlx::query(
                r#"
                INSERT INTO tracks (
                    id, path, title, album, album_artist, artists, genres, year,
                    duration, track_no, track_of, disk_no, disk_of, is_compilation,
                    bpm, camelot, energy, hera_seal, hot_cues
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&track.id)
            .bind(&track.path)
            .bind(&track.title)
            .bind(&track.album)
            .bind(&track.album_artist)
            .bind(json!(&track.artists))
            .bind(json!(&track.genres))
            .bind(track.year)
            .bind(track.duration)
            .bind(track.track_no)
            .bind(track.track_of)
            .bind(track.disk_no)
            .bind(track.disk_of)
            .bind(track.is_compilation)
            .bind(track.bpm.unwrap_or(124.0))
            .bind(track.camelot.as_deref().unwrap_or("8A"))
            .bind(track.energy.unwrap_or(0.80))
            .bind(&track.hera_seal)
            .bind(json!(track.hot_cues.as_deref().unwrap_or(&[])))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /**
     * Update a list of tracks in the database by just overriding everything
     */
    pub async fn update_tracks(&mut self, tracks: Vec<Track>) -> AnyResult<()> {
        let mut tx = self.connection.begin().await?;

        for track in tracks {
            sqlx::query(
                r#"
                UPDATE tracks SET
                    path = ?,
                    title = ?,
                    album = ?,
                    album_artist = ?,
                    artists = ?,
                    genres = ?,
                    year = ?,
                    duration = ?,
                    track_no = ?,
                    track_of = ?,
                    disk_no = ?,
                    disk_of = ?,
                    is_compilation = ?,
                    bpm = ?,
                    camelot = ?,
                    energy = ?,
                    hera_seal = ?,
                    hot_cues = ?
                WHERE id = ?
                "#,
            )
            .bind(&track.path)
            .bind(&track.title)
            .bind(&track.album)
            .bind(&track.album_artist)
            .bind(json!(&track.artists))
            .bind(json!(&track.genres))
            .bind(track.year)
            .bind(track.duration)
            .bind(track.track_no)
            .bind(track.track_of)
            .bind(track.disk_no)
            .bind(track.disk_of)
            .bind(track.is_compilation)
            .bind(track.bpm.unwrap_or(124.0))
            .bind(track.camelot.as_deref().unwrap_or("8A"))
            .bind(track.energy.unwrap_or(0.80))
            .bind(&track.hera_seal)
            .bind(json!(track.hot_cues.as_deref().unwrap_or(&[])))
            .bind(&track.id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /**
     * Get the list of artists registered in the database, excluding compilation tracks.
     */
    pub async fn get_artists(&mut self) -> AnyResult<Vec<String>> {
        let result: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT album_artist
             FROM tracks
             WHERE is_compilation = 0
             ORDER BY
               CASE WHEN upper(substr(album_artist, 1, 1)) BETWEEN 'A' AND 'Z' THEN 0 ELSE 1 END,
               album_artist COLLATE NOCASE;",
        )
        .fetch_all(&mut self.connection)
        .await?;

        Ok(result)
    }

    /**
     * Returns true if any tracks are flagged as compilations.
     */
    pub async fn has_compilations(&mut self) -> AnyResult<bool> {
        super::db_offline_queries::has_compilation_tracks(&mut self.connection).await
    }

    /**
     * Get all tracks for a given artist, excluding compilation tracks.
     */
    pub async fn get_artist_tracks(&mut self, artist: String) -> AnyResult<Vec<TrackGroup>> {
        let tracks = sqlx::query_as::<_, Track>(
            "SELECT * FROM tracks WHERE album_artist = ? AND is_compilation = 0 ORDER BY album, disk_no, track_no",
        )
        .bind(&artist)
        .fetch_all(&mut self.connection)
        .await?;

        let mut groups: IndexMap<String, Vec<Track>> = IndexMap::new();

        for item in tracks {
            groups.entry(item.album.clone()).or_default().push(item);
        }

        let mut track_groups = groups
            .into_iter()
            .map(|(album, tracks)| TrackGroup {
                label: album,
                genres: tracks
                    .iter()
                    .flat_map(|s| &s.genres)
                    .cloned()
                    .unique()
                    .collect(),
                duration: tracks.iter().map(|t| t.duration).sum(),
                year: tracks.first().and_then(|t| t.year),
                tracks,
            })
            .collect::<Vec<TrackGroup>>();

        // Sort by group year, we only use the first track's year and different
        // tracks can have different years. If no year is provided, put the group
        // at the end, as it's probably a library management issue, and we want
        // clean lists!
        track_groups.sort_by_key(|group| group.year.unwrap_or(u16::MAX));

        Ok(track_groups)
    }

    /**
     * Get all compilation albums, grouped by album name, sorted by year.
     */
    pub async fn get_compilation_albums(&mut self) -> AnyResult<Vec<TrackGroup>> {
        let tracks = sqlx::query_as::<_, Track>(
            "SELECT * FROM tracks WHERE is_compilation = 1 ORDER BY album, disk_no, track_no",
        )
        .fetch_all(&mut self.connection)
        .await?;

        let mut groups: IndexMap<String, Vec<Track>> = IndexMap::new();

        for item in tracks {
            groups.entry(item.album.clone()).or_default().push(item);
        }

        let mut track_groups = groups
            .into_iter()
            .map(|(album, tracks)| TrackGroup {
                label: album,
                genres: tracks
                    .iter()
                    .flat_map(|s| &s.genres)
                    .cloned()
                    .unique()
                    .collect(),
                duration: tracks.iter().map(|t| t.duration).sum(),
                year: tracks.first().and_then(|t| t.year),
                tracks,
            })
            .collect::<Vec<TrackGroup>>();

        track_groups.sort_by_key(|group| group.year.unwrap_or(u16::MAX));

        Ok(track_groups)
    }

    /** Get all the playlists (and their content) from the database */
    pub async fn get_all_playlists(&mut self) -> AnyResult<Vec<Playlist>> {
        let mut playlists = sqlx::query_as::<_, Playlist>("SELECT * FROM playlists ORDER BY name")
            .fetch_all(&mut self.connection)
            .await?;

        // Ensure the playlists are sorted alphabetically (case-insensitive) for better UX
        playlists.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        Ok(playlists)
    }

    /** Get a single playlist by ID */
    pub async fn get_playlist(&mut self, playlist_id: &String) -> AnyResult<Option<Playlist>> {
        let playlist = sqlx::query_as::<_, Playlist>("SELECT * FROM playlists WHERE id = ?")
            .bind(playlist_id)
            .fetch_optional(&mut self.connection)
            .await?;

        Ok(playlist)
    }

    /** Create a playlist given a name and a set of track IDs */
    pub async fn create_playlist(
        &mut self,
        name: String,
        tracks_ids: Vec<String>,
        import_path: Option<PathBuf>,
    ) -> AnyResult<Playlist> {
        let playlist_path: Option<String> =
            import_path.map(|path| path.to_str().unwrap().to_string());

        let playlist = Playlist {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            tracks: tracks_ids,
            import_path: playlist_path,
        };

        sqlx::query(
            r#"
            INSERT INTO playlists (id, name, tracks, import_path)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&playlist.id)
        .bind(&playlist.name)
        .bind(json!(&playlist.tracks))
        .bind(&playlist.import_path)
        .execute(&mut self.connection)
        .await?;

        Ok(playlist)
    }

    /** Set the tracks of a playlist given its ID and tracks IDs */
    pub async fn set_playlist_tracks(
        &mut self,
        id: &String,
        tracks: Vec<String>,
    ) -> AnyResult<Playlist> {
        let tracks_json = json!(&tracks).to_string();
        super::db_offline_queries::update_playlist_tracks_raw(
            &mut self.connection,
            id,
            &tracks_json,
        )
        .await?;

        let playlist = sqlx::query_as::<_, Playlist>("SELECT * FROM playlists WHERE id = ?")
            .bind(id)
            .fetch_one(&mut self.connection)
            .await?;

        Ok(playlist)
    }

    /** Update a playlist name by ID */
    pub async fn rename_playlist(&mut self, id: &String, name: String) -> AnyResult<Playlist> {
        super::db_offline_queries::rename_playlist_raw(&mut self.connection, id, &name).await?;

        let playlist = sqlx::query_as::<_, Playlist>("SELECT * FROM playlists WHERE id = ?")
            .bind(id)
            .fetch_one(&mut self.connection)
            .await?;

        Ok(playlist)
    }

    /** Delete a playlist by ID */
    pub async fn delete_playlist(&mut self, id: &String) -> AnyResult<()> {
        super::db_offline_queries::delete_playlist_by_id(&mut self.connection, id).await
    }

    /** List all persisted agent chat sessions ordered by most recently updated */
    pub async fn list_agent_sessions(&mut self) -> AnyResult<Vec<AgentSessionSummary>> {
        let rows = sqlx::query_as::<_, AgentSessionRow>(
            r#"
            SELECT
                id,
                title,
                backend_id,
                execution_policy,
                agy_conversation_id,
                created_at,
                updated_at,
                (SELECT COUNT(*) FROM agent_messages m WHERE m.session_id = agent_sessions.id) AS message_count
            FROM agent_sessions
            ORDER BY updated_at DESC, created_at DESC
            "#,
        )
        .fetch_all(&mut self.connection)
        .await?;
        Ok(rows.into_iter().map(map_agent_session_row).collect())
    }

    /** Get a single agent chat session by ID */
    pub async fn get_agent_session(&mut self, id: &str) -> AnyResult<Option<AgentSessionSummary>> {
        let row = sqlx::query_as::<_, AgentSessionRow>(
            r#"
            SELECT
                id,
                title,
                backend_id,
                execution_policy,
                agy_conversation_id,
                created_at,
                updated_at,
                (SELECT COUNT(*) FROM agent_messages m WHERE m.session_id = agent_sessions.id) AS message_count
            FROM agent_sessions
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&mut self.connection)
        .await?;
        Ok(row.map(map_agent_session_row))
    }

    /** Create a new persisted agent chat session */
    pub async fn create_agent_session(
        &mut self,
        title: String,
        backend_id: String,
        execution_policy: String,
    ) -> AnyResult<AgentSessionSummary> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let session = AgentSessionSummary {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            backend_id,
            execution_policy,
            agy_conversation_id: None,
            created_at: now_ms,
            updated_at: now_ms,
            message_count: 0,
        };
        sqlx::query(
            r#"
            INSERT INTO agent_sessions (id, title, backend_id, execution_policy, agy_conversation_id, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session.id)
        .bind(&session.title)
        .bind(&session.backend_id)
        .bind(&session.execution_policy)
        .bind(&session.agy_conversation_id)
        .bind(session.created_at)
        .bind(session.updated_at)
        .execute(&mut self.connection)
        .await?;
        Ok(session)
    }

    /** Update agent session metadata (title, agy_conversation_id, backend, or policy) */
    pub async fn update_agent_session_meta(
        &mut self,
        id: &str,
        title: Option<&str>,
        agy_conversation_id: Option<&str>,
        backend_id: Option<&str>,
        execution_policy: Option<&str>,
    ) -> AnyResult<Option<AgentSessionSummary>> {
        let Some(existing) = self.get_agent_session(id).await? else {
            return Ok(None);
        };
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(existing.updated_at + 1)
            .max(existing.updated_at + 1);
        let next_title = title.unwrap_or(&existing.title);
        let next_agy_id = agy_conversation_id.or(existing.agy_conversation_id.as_deref());
        let next_backend = backend_id.unwrap_or(&existing.backend_id);
        let next_policy = execution_policy.unwrap_or(&existing.execution_policy);

        sqlx::query(
            r#"
            UPDATE agent_sessions
            SET title = ?, agy_conversation_id = ?, backend_id = ?, execution_policy = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(next_title)
        .bind(next_agy_id)
        .bind(next_backend)
        .bind(next_policy)
        .bind(now_ms)
        .bind(id)
        .execute(&mut self.connection)
        .await?;

        self.get_agent_session(id).await
    }

    /** Delete an agent session (cascades to agent_messages when foreign keys are enabled, plus explicit cleanup) */
    pub async fn delete_agent_session(&mut self, id: &str) -> AnyResult<()> {
        super::db_offline_queries::delete_agent_session_and_messages(&mut self.connection, id).await
    }

    /** Fetch all messages for a given agent session in chronological order */
    pub async fn get_agent_messages(
        &mut self,
        session_id: &str,
    ) -> AnyResult<Vec<AgentStoredMessage>> {
        let msgs = sqlx::query_as::<_, AgentStoredMessage>(
            r#"
            SELECT id, session_id, role, content, attachments_json, tool_steps_json, status, created_at
            FROM agent_messages
            WHERE session_id = ?
            ORDER BY created_at ASC, rowid ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&mut self.connection)
        .await?;
        Ok(msgs)
    }

    /** Insert or update an agent message and bump parent session's updated_at */
    pub async fn upsert_agent_message(&mut self, msg: &AgentStoredMessage) -> AnyResult<()> {
        sqlx::query(
            r#"
            INSERT INTO agent_messages (id, session_id, role, content, attachments_json, tool_steps_json, status, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                content = excluded.content,
                attachments_json = excluded.attachments_json,
                tool_steps_json = excluded.tool_steps_json,
                status = excluded.status
            "#,
        )
        .bind(&msg.id)
        .bind(&msg.session_id)
        .bind(&msg.role)
        .bind(&msg.content)
        .bind(&msg.attachments_json)
        .bind(&msg.tool_steps_json)
        .bind(&msg.status)
        .bind(msg.created_at)
        .execute(&mut self.connection)
        .await?;

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(msg.created_at);
        super::db_offline_queries::touch_agent_session_updated_at(
            &mut self.connection,
            &msg.session_id,
            now_ms,
        )
        .await?;

        Ok(())
    }
}

pub use hera_ipc::agent_types::AgentSessionSummary;

type AgentSessionRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    i64,
    i64,
    i64,
);

fn map_agent_session_row(row: AgentSessionRow) -> AgentSessionSummary {
    let (
        id,
        title,
        backend_id,
        execution_policy,
        agy_conversation_id,
        created_at,
        updated_at,
        message_count,
    ) = row;
    AgentSessionSummary {
        id,
        title,
        backend_id,
        execution_policy,
        agy_conversation_id,
        created_at,
        updated_at,
        message_count,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::FromRow)]
pub struct AgentStoredMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub attachments_json: String,
    pub tool_steps_json: String,
    pub status: String,
    pub created_at: i64,
}
