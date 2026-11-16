use log::{info, warn};
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolConnection;
use sqlx::sqlite::{
    SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions,
    SqliteSynchronous,
};
use sqlx::{Connection, Sqlite, SqliteConnection};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime, State};
use tauri_plugin_dialog::{DialogExt, FilePath};
use tokio::sync::{Mutex, MutexGuard};
use ts_rs::TS;

use crate::libs::database::{DB, SUPPORTED_PLAYLISTS_EXTENSIONS};
use crate::libs::error::{AnyResult, MuseeksError, handle_fatal_error};
use crate::libs::playlist::Playlist;
use crate::libs::track::{Track, TrackGroup, get_track_id_for_path};

use super::config::get_storage_dir;
use hera_ipc::format_m3u_entry_path;

/// Exclusive SQLite writer serialized by `Mutex<DB>` plus concurrent WAL `SqlitePool` readers.
pub struct DBState {
    writer: Mutex<DB>,
    read_pool: SqlitePool,
}

impl DBState {
    pub async fn get_lock(&self) -> MutexGuard<'_, DB> {
        self.writer.lock().await
    }

    pub async fn acquire_read_conn(&self) -> AnyResult<PoolConnection<Sqlite>> {
        self.read_pool.acquire().await.map_err(Into::into)
    }
}

fn wal_connect_options(database_path: &Path, create_if_missing: bool) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(create_if_missing)
        .auto_vacuum(SqliteAutoVacuum::Incremental)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(std::time::Duration::from_secs(5))
        .pragma("temp_store", "MEMORY")
        .pragma("cache_size", "-64000")
        .pragma("mmap_size", "268435456")
}

/**
 * Database setup
 */
pub(crate) async fn setup_at_path(database_path: &Path) -> AnyResult<DBState> {
    info!("Opening connection to database: {:?}", database_path);

    let writer_options = wal_connect_options(database_path, true).optimize_on_close(true, None);
    let mut connection = SqliteConnection::connect_with(&writer_options).await?;

    // Clear Journal
    sqlx::query("PRAGMA  wal_checkpoint(TRUNCATE)")
        .execute(&mut connection)
        .await?;

    info!("Run possible migrations");

    sqlx::migrate!("src/migrations")
        .run_direct(&mut connection)
        .await?;

    let read_options = wal_connect_options(database_path, false);
    let read_pool = SqlitePoolOptions::new()
        .max_connections(6)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("PRAGMA query_only = ON;").execute(conn).await?;
                Ok(())
            })
        })
        .connect_with(read_options)
        .await?;

    Ok(DBState {
        writer: Mutex::new(DB { connection }),
        read_pool,
    })
}

async fn setup() -> AnyResult<DBState> {
    let database_path = get_storage_dir().join("hera.db");
    setup_at_path(&database_path).await
}

/**
 * Scan progress
 */
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/generated/typings.ts")]
pub struct ScanProgress {
    pub current: usize,
    pub total: usize,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/generated/typings.ts")]
pub struct ScanResult {
    pub track_count: usize,
    pub track_failures: usize,
    pub playlist_count: usize,
    pub playlist_failures: usize,
}

/* -----------------------------------------------------------------------------
 * Commands
 * -------------------------------------------------------------------------- */

/**
 * Scan the selected folders, extract all ID3 tags from it, and update the DB
 * accordingly.
 * TODO: move that to libs/database
 */
#[tauri::command]
async fn scan_library<R: Runtime>(
    window: tauri::Window<R>,
    db_state: State<'_, DBState>,
    import_paths: Vec<PathBuf>,
    refresh: bool,
) -> AnyResult<ScanResult> {
    let app_handle = window.app_handle().clone();

    info!("Importing paths to library:");
    for path in &import_paths {
        info!("  - {:?}", path)
    }

    let existing_paths = if !refresh {
        let mut conn = db_state.acquire_read_conn().await?;
        DB::read_all_track_paths(&mut conn)
            .await?
            .into_iter()
            .map(PathBuf::from)
            .collect::<HashSet<_>>()
    } else {
        HashSet::new()
    };

    let scan_res = crate::libs::fast_library_scan::execute_streaming_library_scan(
        &app_handle,
        &db_state,
        import_paths,
        existing_paths,
        refresh,
    )
    .await?;

    let mut scan_result = ScanResult {
        track_count: scan_res.scanned_tracks_count,
        track_failures: scan_res.track_failures,
        playlist_count: 0,
        playlist_failures: 0,
    };

    let mut db = db_state.get_lock().await;
    let mut playlist_paths = scan_res.found_playlists;
    let existing_playlists_paths = db
        .get_all_playlists()
        .await?
        .iter()
        .filter_map(move |playlist| playlist.import_path.to_owned())
        .map(PathBuf::from)
        .collect::<HashSet<_>>();

    playlist_paths.retain(|path| !existing_playlists_paths.contains(path));

    info!("Found {} playlist(s) to import", playlist_paths.len());

    for playlist_path in playlist_paths {
        let res = {
            let mut reader = m3u::Reader::open(&playlist_path)
                .map_err(|e| MuseeksError::Unknown(anyhow::anyhow!("Invalid M3U file: {e}")))?;
            let playlist_dir_path = playlist_path
                .parent()
                .ok_or_else(|| MuseeksError::Unknown(anyhow::anyhow!("Missing playlist parent")))?;

            let track_paths: Vec<PathBuf> = reader
                .entries()
                .filter_map(|entry| {
                    let Ok(entry) = entry else {
                        return None;
                    };

                    match entry {
                        m3u::Entry::Path(path) => Some(playlist_dir_path.join(path)),
                        _ => None,
                    }
                })
                .collect();

            let track_ids = track_paths
                .iter()
                .flat_map(get_track_id_for_path)
                .collect::<Vec<String>>();

            let playlist_name = playlist_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Playlist importada")
                .to_owned();

            let tracks = db.get_tracks(&track_ids).await?;

            if tracks.len() != track_ids.len() {
                warn!(
                    "Playlist track mismatch ({} from playlist, {} from library)",
                    track_paths.len(),
                    tracks.len()
                );
            }

            info!(
                r#"Creating playlist "{}" ({} tracks)"#,
                playlist_name,
                track_ids.len()
            );

            db.create_playlist(playlist_name, track_ids, Some(playlist_path))
                .await?;
            Ok::<(), MuseeksError>(())
        };

        match res {
            Ok(_) => {
                scan_result.playlist_count += 1;
            }
            Err(err) => {
                warn!("Failed to import playlist: {}", err);
                scan_result.playlist_failures += 1;
            }
        }
    }

    Ok(scan_result)
}

#[tauri::command]
async fn get_all_tracks(db_state: State<'_, DBState>) -> AnyResult<Vec<Track>> {
    let mut conn = db_state.acquire_read_conn().await?;
    DB::read_all_tracks(&mut conn).await
}

#[tauri::command]
async fn get_tracks(db_state: State<'_, DBState>, ids: Vec<String>) -> AnyResult<Vec<Track>> {
    let mut conn = db_state.acquire_read_conn().await?;
    DB::read_tracks(&mut conn, &ids).await
}

#[tauri::command]
async fn get_track(db_state: State<'_, DBState>, id: String) -> AnyResult<Option<Track>> {
    let mut conn = db_state.acquire_read_conn().await?;
    DB::read_track(&mut conn, &id).await
}

#[tauri::command]
async fn update_track(db_state: State<'_, DBState>, track: Track) -> AnyResult<Track> {
    db_state.get_lock().await.update_track(track).await
}

#[tauri::command]
async fn remove_tracks(db_state: State<'_, DBState>, ids: Vec<String>) -> AnyResult<()> {
    db_state.get_lock().await.remove_tracks(&ids).await
}

#[tauri::command]
async fn get_artists(db_state: State<'_, DBState>) -> AnyResult<Vec<String>> {
    let mut conn = db_state.acquire_read_conn().await?;
    DB::read_artists(&mut conn).await
}

#[tauri::command]
async fn get_artist_tracks(
    db_state: State<'_, DBState>,
    artist: String,
) -> AnyResult<Vec<TrackGroup>> {
    let mut conn = db_state.acquire_read_conn().await?;
    DB::read_artist_tracks(&mut conn, artist).await
}

#[tauri::command]
async fn get_compilation_albums(db_state: State<'_, DBState>) -> AnyResult<Vec<TrackGroup>> {
    let mut conn = db_state.acquire_read_conn().await?;
    DB::read_compilation_albums(&mut conn).await
}

#[tauri::command]
async fn has_compilations(db_state: State<'_, DBState>) -> AnyResult<bool> {
    let mut conn = db_state.acquire_read_conn().await?;
    DB::read_has_compilations(&mut conn).await
}

#[tauri::command]
async fn get_all_playlists(db_state: State<'_, DBState>) -> AnyResult<Vec<Playlist>> {
    let mut conn = db_state.acquire_read_conn().await?;
    DB::read_all_playlists(&mut conn).await
}

#[tauri::command]
async fn get_playlist(db_state: State<'_, DBState>, id: String) -> AnyResult<Playlist> {
    let mut conn = db_state.acquire_read_conn().await?;
    match DB::read_playlist(&mut conn, &id).await {
        Ok(Some(playlist)) => Ok(playlist),
        Ok(None) => Err(MuseeksError::PlaylistNotFound),
        Err(err) => Err(err),
    }
}

#[tauri::command]
async fn create_playlist(
    db_state: State<'_, DBState>,
    name: String,
    ids: Vec<String>,
    import_path: Option<PathBuf>,
) -> AnyResult<Playlist> {
    db_state
        .get_lock()
        .await
        .create_playlist(name, ids, import_path)
        .await
}

#[tauri::command]
async fn rename_playlist(
    db_state: State<'_, DBState>,
    id: String,
    name: String,
) -> AnyResult<Playlist> {
    db_state.get_lock().await.rename_playlist(&id, name).await
}

#[tauri::command]
async fn set_playlist_tracks(
    db_state: State<'_, DBState>,
    id: String,
    tracks: Vec<String>,
) -> AnyResult<Playlist> {
    db_state
        .get_lock()
        .await
        .set_playlist_tracks(&id, tracks)
        .await
}

#[tauri::command]
async fn export_playlist<R: Runtime>(
    window: tauri::Window<R>,
    db_state: State<'_, DBState>,
    id: String,
) -> AnyResult<()> {
    let tracks = {
        let mut conn = db_state.acquire_read_conn().await?;
        let Some(playlist) = DB::read_playlist(&mut conn, &id).await? else {
            return Ok(());
        };
        DB::read_tracks(&mut conn, &playlist.tracks).await?
    };

    window
        .dialog()
        .file()
        .add_filter("playlist", &SUPPORTED_PLAYLISTS_EXTENSIONS)
        .save_file(move |maybe_playlist_path| {
            let playlist_path = match maybe_playlist_path {
                // We don't support FilePath::Url
                Some(FilePath::Path(path)) => path,
                _ => return,
            };

            let playlist_dir_path = match playlist_path.parent() {
                Some(parent) => parent,
                None => return,
            };

            let playlist = tracks
                .iter()
                .map(|track| {
                    let entry_path =
                        format_m3u_entry_path(std::path::Path::new(&track.path), playlist_dir_path);
                    m3u::path_entry(entry_path)
                })
                .collect::<Vec<m3u::Entry>>();

            let mut file = match std::fs::File::create(&playlist_path) {
                Ok(f) => f,
                Err(err) => {
                    warn!("Failed to create playlist file {playlist_path:?}: {err}");
                    return;
                }
            };
            let mut writer = m3u::Writer::new(&mut file);
            for entry in &playlist {
                if let Err(err) = writer.write_entry(entry) {
                    warn!("Failed to write M3U entry: {err}");
                }
            }
        });

    Ok(())
}

#[tauri::command]
async fn delete_playlist(db_state: State<'_, DBState>, id: String) -> AnyResult<()> {
    db_state.get_lock().await.delete_playlist(&id).await
}

#[tauri::command]
async fn reset(db_state: State<'_, DBState>) -> AnyResult<()> {
    info!("Resetting DB...");

    let mut db = db_state.get_lock().await;
    crate::libs::db_offline_queries::reset_tracks_and_playlists(&mut db.connection).await
}

/**
 * Database plugin, exposing commands and state
 */
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("database")
        .invoke_handler(tauri::generate_handler![
            get_all_tracks,
            get_tracks,
            get_track,
            remove_tracks,
            update_track,
            get_artists,
            get_artist_tracks,
            get_compilation_albums,
            has_compilations,
            get_all_playlists,
            get_playlist,
            create_playlist,
            rename_playlist,
            set_playlist_tracks,
            export_playlist,
            delete_playlist,
            reset,
            scan_library,
        ])
        .setup(|app_handle, _api| {
            let app_handle = app_handle.clone();

            match tauri::async_runtime::block_on(setup()) {
                Ok(db_state) => {
                    app_handle.manage(db_state);
                    Ok(())
                }
                Err(err) => {
                    handle_fatal_error(&app_handle, err);
                    Ok(())
                }
            }
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn test_track(id: &str, path: &str, title: &str, artist: &str) -> Track {
        Track {
            id: id.to_string(),
            path: path.to_string(),
            title: title.to_string(),
            album: "Club Session".to_string(),
            album_artist: artist.to_string(),
            artists: vec![artist.to_string()],
            genres: vec!["House".to_string()],
            year: Some(2026),
            duration: 240,
            track_no: Some(1),
            track_of: None,
            disk_no: None,
            disk_of: None,
            is_compilation: false,
            bpm: Some(124.0),
            camelot: Some("8A".to_string()),
            energy: Some(0.85),
            hera_seal: None,
            is_video: Some(false),
            scan_level: Some(2),
            is_transition_candidate: Some(true),
            is_fake_320kbps: Some(false),
            waveform_peaks_blob: None,
            is_mix_over_10m: Some(false),
            has_audio_skips: Some(false),
            is_variable_bpm: Some(false),
            entry_bpm: Some(124.0),
            exit_bpm: Some(124.0),
            hot_cues: Some(vec![]),
            replay_gain_db: None,
            loudness_lufs: None,
            has_intro_vocals: Some(false),
            has_outro_vocals: Some(false),
            is_own_production: Some(false),
            has_bpm_trust_diff: Some(false),
            waveform_peaks_b64: None,
            embedding_64d_b64: None,
            inferred_genre_family: None,
            inferred_era_year: None,
            partial_hash_64kb: None,
            file_size_bytes: None,
            legacy_bpm: None,
            bitrate_kbps: None,
            sample_rate_hz: None,
        }
    }

    #[tokio::test]
    async fn test_wal_read_pool_serves_queries_concurrently_while_writer_transaction_is_open() {
        let dir = std::env::temp_dir().join(format!(
            "hera_wal_pool_{}_{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("hera.db");

        let state = setup_at_path(&db_path).await.expect("setup_at_path");
        {
            let mut writer = state.get_lock().await;
            writer
                .insert_tracks(vec![test_track(
                    "t-1",
                    "/music/warm-up.flac",
                    "Warm Up",
                    "DaviDJ",
                )])
                .await
                .unwrap();
            writer
                .create_playlist("Main Stage".to_string(), vec!["t-1".to_string()], None)
                .await
                .unwrap();
        }

        // Hold both `Mutex<DB>` AND an active uncommitted SQLite WAL write transaction.
        let mut writer = state.get_lock().await;
        let mut tx = writer.connection.begin().await.unwrap();
        sqlx::query(
            "INSERT INTO tracks (id, path, title, album, album_artist, artists, genres, duration, is_compilation) VALUES ('t-2', '/music/peak.flac', 'Peak', 'Club Session', 'DaviDJ', '[\"DaviDJ\"]', '[\"House\"]', 210, 0)",
        )
        .execute(&mut *tx)
        .await
        .unwrap();

        let started = Instant::now();
        let mut read_conn = state.acquire_read_conn().await.expect("acquire read conn");
        let tracks = DB::read_all_tracks(&mut read_conn).await.unwrap();
        let artist_groups = DB::read_artist_tracks(&mut read_conn, "DaviDJ".to_string())
            .await
            .unwrap();
        let playlists = DB::read_all_playlists(&mut read_conn).await.unwrap();
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_millis(500),
            "WAL reader pool should not block on writer lock ({elapsed:?})"
        );
        assert_eq!(tracks.len(), 1);
        assert_eq!(artist_groups.len(), 1);
        assert_eq!(playlists.len(), 1);

        // Read pool enforces `PRAGMA query_only = ON`, `cache_size = -64000`, and `mmap_size`.
        let (cache_size,): (i64,) = sqlx::query_as("PRAGMA cache_size;")
            .fetch_one(&mut *read_conn)
            .await
            .unwrap();
        assert_eq!(cache_size, -64_000);
        let (mmap_size,): (i64,) = sqlx::query_as("PRAGMA mmap_size;")
            .fetch_one(&mut *read_conn)
            .await
            .unwrap();
        assert_eq!(mmap_size, 268_435_456);
        let (crate_tbl_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM folder_crates")
            .fetch_one(&mut *read_conn)
            .await
            .unwrap();
        assert_eq!(crate_tbl_count, 0);
        assert!(
            sqlx::query("DELETE FROM playlists")
                .execute(&mut *read_conn)
                .await
                .is_err()
        );
        drop(read_conn);

        tx.commit().await.unwrap();
        drop(writer);

        let mut read_after = state.acquire_read_conn().await.unwrap();
        assert_eq!(DB::read_all_tracks(&mut read_after).await.unwrap().len(), 2);
        drop(read_after);

        state.read_pool.close().await;
        let _ = std::fs::remove_dir_all(&dir);
    }
}
