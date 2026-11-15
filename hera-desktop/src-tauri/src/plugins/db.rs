use log::{info, warn};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode};
use sqlx::{Connection, SqliteConnection};
use std::collections::HashSet;
use std::path::PathBuf;
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

// Connection is mutable, so we must wrap the state in a mutex to make sure there
// are no concurrency issues
pub struct DBState(Mutex<DB>);

impl DBState {
    pub async fn get_lock(&self) -> MutexGuard<'_, DB> {
        self.0.lock().await
    }
}

/**
 * Database setup
 */
async fn setup() -> AnyResult<DB> {
    let database_path = get_storage_dir().join("hera.db");

    info!("Opening connection to database: {:?}", database_path);

    let options = SqliteConnectOptions::new()
        .filename(&database_path)
        .create_if_missing(true)
        .optimize_on_close(true, None)
        .auto_vacuum(SqliteAutoVacuum::Incremental)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5));

    let mut connection = SqliteConnection::connect_with(&options).await?;

    // Clear Journal
    sqlx::query("PRAGMA  wal_checkpoint(TRUNCATE)")
        .execute(&mut connection)
        .await?;

    info!("Run possible migrations");

    sqlx::migrate!("src/migrations")
        .run_direct(&mut connection)
        .await?;

    Ok(DB { connection })
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

/** ----------------------------------------------------------------------------
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
        let mut db = db_state.get_lock().await;
        db.get_all_track_paths()
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
                &playlist_name,
                &track_ids.len()
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
    db_state.get_lock().await.get_all_tracks().await
}

#[tauri::command]
async fn get_tracks(db_state: State<'_, DBState>, ids: Vec<String>) -> AnyResult<Vec<Track>> {
    db_state.get_lock().await.get_tracks(&ids).await
}

#[tauri::command]
async fn get_track(db_state: State<'_, DBState>, id: String) -> AnyResult<Option<Track>> {
    db_state.get_lock().await.get_track(&id).await
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
    db_state.get_lock().await.get_artists().await
}

#[tauri::command]
async fn get_artist_tracks(
    db_state: State<'_, DBState>,
    artist: String,
) -> AnyResult<Vec<TrackGroup>> {
    db_state.get_lock().await.get_artist_tracks(artist).await
}

#[tauri::command]
async fn get_compilation_albums(db_state: State<'_, DBState>) -> AnyResult<Vec<TrackGroup>> {
    db_state.get_lock().await.get_compilation_albums().await
}

#[tauri::command]
async fn has_compilations(db_state: State<'_, DBState>) -> AnyResult<bool> {
    db_state.get_lock().await.has_compilations().await
}

#[tauri::command]
async fn get_all_playlists(db_state: State<'_, DBState>) -> AnyResult<Vec<Playlist>> {
    db_state.get_lock().await.get_all_playlists().await
}

#[tauri::command]
async fn get_playlist(db_state: State<'_, DBState>, id: String) -> AnyResult<Playlist> {
    match db_state.get_lock().await.get_playlist(&id).await {
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
    let mut db = db_state.get_lock().await;

    let Some(playlist) = db.get_playlist(&id).await? else {
        return Ok(());
    };

    let tracks = db.get_tracks(&playlist.tracks).await?;

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
                Ok(db) => {
                    app_handle.manage(DBState(Mutex::new(db)));
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
