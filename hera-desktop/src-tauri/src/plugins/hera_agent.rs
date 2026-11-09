use std::collections::HashMap;
use std::net::TcpListener as StdTcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::State as AxumState;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use hera_node_rs::dj_brain::{check_binary_in_path, list_supported_ai_backends};
use hera_node_rs::error::constant_time_eq;
use hera_node_rs::export::{export_crate_bundle, ExportCrateRequest, ExportTrackItem};
use hera_node_rs::mcp::{dispatch_tool_call as dispatch_core_mcp_tool, list_mcp_tools as list_core_mcp_tools};
use hera_node_rs::structure::HotCuePoint;
use serde_json::{json, Value};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{oneshot, Mutex};

pub use hera_ipc::agent_types::*;

use crate::libs::database::{AgentStoredMessage, DB, SUPPORTED_TRACKS_EXTENSIONS};
use crate::libs::track::{get_track_from_file, Track};
use crate::libs::utils::scan_dirs;
use crate::plugins::db::DBState;
use crate::plugins::hera_dj::{
    download_p2p_track, push_virtualdj_live_crate, read_virtualdj_now_playing,
    set_mini_overlay_mode, shared_desktop_p2p_state, DownloadP2PTrackRequest,
    PushVirtualDjLiveCrateRequest, SetMiniOverlayModeRequest,
};

const MCP_LOOPBACK_HOST: &str = "127.0.0.1";

pub struct ActiveTurnTracker {
    pub message_id: String,
    pub execution_policy: String,
    pub cancel_flag: Arc<AtomicBool>,
    pub steps: Vec<AgentToolStepView>,
    pub actions: Vec<AgentFollowupAction>,
}

pub enum AgentHostBridge<R: Runtime> {
    Tauri(AppHandle<R>),
    Standalone(Arc<Mutex<DB>>),
}

impl<R: Runtime> Clone for AgentHostBridge<R> {
    fn clone(&self) -> Self {
        match self {
            Self::Tauri(app) => Self::Tauri(app.clone()),
            Self::Standalone(db) => Self::Standalone(db.clone()),
        }
    }
}

pub struct AgentRuntimeContext<R: Runtime> {
    pub host: AgentHostBridge<R>,
    pub mcp_port: u16,
    pub mcp_bearer_token: String,
    pub active_turns: Arc<Mutex<HashMap<String, ActiveTurnTracker>>>,
    pub active_mcp_session: Arc<Mutex<Option<String>>>,
    pub pending_approvals: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
}

impl<R: Runtime> Clone for AgentRuntimeContext<R> {
    fn clone(&self) -> Self {
        Self {
            host: self.host.clone(),
            mcp_port: self.mcp_port,
            mcp_bearer_token: self.mcp_bearer_token.clone(),
            active_turns: self.active_turns.clone(),
            active_mcp_session: self.active_mcp_session.clone(),
            pending_approvals: self.pending_approvals.clone(),
        }
    }
}

impl<R: Runtime> AgentRuntimeContext<R> {
    async fn with_db<F, T>(&self, f: F) -> Result<T, String>
    where
        F: for<'a> FnOnce(
            &'a mut DB,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<T, String>> + Send + 'a>,
        >,
    {
        match &self.host {
            AgentHostBridge::Tauri(app) => {
                let Some(db_state) = app.try_state::<DBState>() else {
                    return Err("Base de datos aún inicializándose".to_string());
                };
                let mut guard = db_state.get_lock().await;
                f(&mut guard).await
            }
            AgentHostBridge::Standalone(db_arc) => {
                let mut guard = db_arc.lock().await;
                f(&mut guard).await
            }
        }
    }

    fn emit_stream(&self, evt: AgentStreamEvent) {
        if let AgentHostBridge::Tauri(app) = &self.host {
            let _ = app.emit("hera-agent:stream", evt);
        }
    }

    fn emit_ui_sync(&self, evt: AgentUiSyncEvent) {
        if let AgentHostBridge::Tauri(app) = &self.host {
            let _ = app.emit("hera-agent:ui-sync", evt);
        }
    }
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn stored_to_view(msg: AgentStoredMessage) -> AgentMessageView {
    let attachments: Vec<AgentAttachment> =
        serde_json::from_str(&msg.attachments_json).unwrap_or_default();
    let (tool_steps, actions): (Vec<AgentToolStepView>, Vec<AgentFollowupAction>) =
        if let Ok(obj) = serde_json::from_str::<Value>(&msg.tool_steps_json) {
            if obj.is_array() {
                (serde_json::from_value(obj).unwrap_or_default(), Vec::new())
            } else {
                let steps = obj
                    .get("steps")
                    .cloned()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                let acts = obj
                    .get("actions")
                    .cloned()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                (steps, acts)
            }
        } else {
            (Vec::new(), Vec::new())
        };
    AgentMessageView {
        id: msg.id,
        session_id: msg.session_id,
        role: msg.role,
        content: msg.content,
        attachments,
        tool_steps,
        actions,
        status: msg.status,
        created_at: msg.created_at,
    }
}

fn encode_steps_and_actions(
    steps: &[AgentToolStepView],
    actions: &[AgentFollowupAction],
) -> String {
    json!({
        "steps": steps,
        "actions": actions,
    })
    .to_string()
}

fn track_to_export_item(track: &Track) -> ExportTrackItem {
    let artist = track
        .artists
        .first()
        .cloned()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Artista Desconocido".to_string());
    let cues = track
        .hot_cues
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|pin| HotCuePoint {
            num: pin.num,
            name: pin.name,
            seconds: pin.seconds as f32,
            bar_index: 0,
            role: pin.role,
            confidence: 0.95,
        })
        .collect();
    ExportTrackItem {
        track_id: track.id.clone(),
        artist,
        title: track.title.clone(),
        version: None,
        file_path: track.path.clone(),
        bpm: track.bpm.unwrap_or(124.0) as f32,
        camelot: track.camelot.clone().unwrap_or_else(|| "8A".to_string()),
        duration_seconds: track.duration as f32,
        bitrate_kbps: 1411,
        energy: track.energy.unwrap_or(0.82) as f32,
        file_size_bytes: 0,
        is_video: false,
        is_own_production: false,
        hot_cues: cues,
    }
}

/// Build the 21 desktop + OS tools and merge them with the 21 `hera-rs` DSP/P2P tools (42 total).
pub fn build_unified_mcp_tools_list() -> Vec<Value> {
    let mut tools = vec![
        json!({
            "name": "list_library_tracks",
            "description": "List tracks currently stored in the HERA Desktop SQLite library with IDs, artists, BPM, Camelot key, duration, 4 Hot Cues, and local file paths. Call this to inspect the user's library; do NOT use search_music (which searches the remote P2P swarm).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Maximum tracks to return (default 100, max 500)", "default": 100 }
                }
            }
        }),
        json!({
            "name": "search_library_tracks",
            "description": "Filter local SQLite library tracks by text query (title/artist/genre), BPM range, or Camelot key. Call this before building or updating a local playlist.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Optional artist, title, album, or genre substring" },
                    "bpm_min": { "type": "number", "description": "Optional minimum BPM inclusive" },
                    "bpm_max": { "type": "number", "description": "Optional maximum BPM inclusive" },
                    "camelot": { "type": "string", "description": "Optional exact Camelot key (e.g. '8A', '9A', '10B')" }
                }
            }
        }),
        json!({
            "name": "import_audio_paths",
            "description": "Scan one or more local files or directories for audio tracks (.wav, .flac, .aiff, .mp3, .ogg, .m4a), extract BPM, Camelot key, and 4 Hot Cues in parallel, and insert them into the SQLite library.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paths": { "type": "array", "items": { "type": "string" }, "description": "Absolute or workspace-relative file or directory paths to import" }
                },
                "required": ["paths"]
            }
        }),
        json!({
            "name": "update_track_metadata",
            "description": "Update mutable metadata fields (title, artist, genre, bpm, camelot) of an existing track in the SQLite library by track_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "track_id": { "type": "string", "description": "Target SQLite track ID" },
                    "title": { "type": "string", "description": "Optional new track title" },
                    "artist": { "type": "string", "description": "Optional primary artist name" },
                    "bpm": { "type": "number", "description": "Optional updated BPM" },
                    "camelot": { "type": "string", "description": "Optional updated Camelot key (e.g. '9A')" }
                },
                "required": ["track_id"]
            }
        }),
        json!({
            "name": "remove_library_tracks",
            "description": "Remove tracks by ID from the HERA SQLite index without deleting the underlying audio files on disk.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "track_ids": { "type": "array", "items": { "type": "string" }, "description": "Track IDs to remove from SQLite" }
                },
                "required": ["track_ids"]
            }
        }),
        json!({
            "name": "list_playlists",
            "description": "List all saved playlists in the HERA SQLite database with their IDs, names, and ordered track IDs.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "create_playlist",
            "description": "Create a new playlist in SQLite from track IDs (or auto-select harmonically ordered library tracks when track_ids is omitted). Set sort_harmonic=true to sequence tracks smoothly by Camelot key and BPM.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Human-readable playlist name" },
                    "track_ids": { "type": "array", "items": { "type": "string" }, "description": "Ordered SQLite track IDs to include" },
                    "sort_harmonic": { "type": "boolean", "description": "Whether to order tracks harmonically by Camelot wheel and BPM", "default": true }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "set_playlist_tracks",
            "description": "Replace or reorder the track ID list of an existing SQLite playlist.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "playlist_id": { "type": "string", "description": "Target playlist ID" },
                    "track_ids": { "type": "array", "items": { "type": "string" }, "description": "New ordered list of track IDs" }
                },
                "required": ["playlist_id", "track_ids"]
            }
        }),
        json!({
            "name": "rename_playlist",
            "description": "Rename an existing SQLite playlist by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "playlist_id": { "type": "string" },
                    "name": { "type": "string" }
                },
                "required": ["playlist_id", "name"]
            }
        }),
        json!({
            "name": "delete_playlist",
            "description": "Delete a playlist by ID from the SQLite database without deleting any audio tracks.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "playlist_id": { "type": "string" }
                },
                "required": ["playlist_id"]
            }
        }),
        json!({
            "name": "control_audio_player",
            "description": "Control HERA Desktop's live audio player: start playing a track by ID, pause, resume, skip next/previous, or seek to a timestamp in seconds.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["play_track", "pause", "resume", "next", "previous", "seek"] },
                    "track_id": { "type": "string", "description": "Required when action is 'play_track'" },
                    "seek_seconds": { "type": "number", "description": "Optional timestamp in seconds" }
                },
                "required": ["action"]
            }
        }),
        json!({
            "name": "push_virtualdj_live_crate",
            "description": "Write tracks (from track_ids or playlist_id) directly into VirtualDJ's live folder ('HERA En Vivo.vdjfolder' + '_vdj_database.xml' with 4 <Poi> Hot Cues).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "track_ids": { "type": "array", "items": { "type": "string" }, "description": "Optional explicit SQLite track IDs" },
                    "playlist_id": { "type": "string", "description": "Optional playlist ID whose tracks should be pushed" },
                    "crate_name": { "type": "string", "default": "HERA En Vivo" },
                    "append": { "type": "boolean", "default": false }
                }
            }
        }),
        json!({
            "name": "read_virtualdj_now_playing",
            "description": "Read the latest track played in VirtualDJ from Documents/VirtualDJ/History/tracklist.txt.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "export_dj_crate_file",
            "description": "Export library or playlist tracks to native DJ formats ('virtualdj', 'rekordbox', 'traktor', 'm3u8') on disk.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "crate_name": { "type": "string" },
                    "playlist_id": { "type": "string" },
                    "track_ids": { "type": "array", "items": { "type": "string" } },
                    "formats": { "type": "array", "items": { "type": "string", "enum": ["virtualdj", "rekordbox", "traktor", "m3u8"] } },
                    "output_dir": { "type": "string" }
                },
                "required": ["crate_name"]
            }
        }),
        json!({
            "name": "set_app_view_or_mini_mode",
            "description": "Navigate HERA Desktop to a specific view ('/library', '/playlists', '/bridge', '/dna', '/settings') or switch window layout mode ('full', 'card', 'pill').",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "route": { "type": "string", "description": "Optional app route to open" },
                    "window_mode": { "type": "string", "enum": ["full", "card", "pill"], "description": "Optional window morph mode" }
                }
            }
        }),
        json!({
            "name": "download_swarm_track_to_library",
            "description": "Download a verified track from the HERA P2P Swarm into local storage and import it into the SQLite library with 4 Hot Cues.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "track_id": { "type": "string" },
                    "artist": { "type": "string" },
                    "title": { "type": "string" },
                    "bpm": { "type": "number", "default": 125.0 },
                    "camelot": { "type": "string", "default": "8A" }
                },
                "required": ["artist", "title"]
            }
        }),
        json!({
            "name": "run_powershell_command",
            "description": "Execute a non-interactive PowerShell command on the local machine and return exit_code, stdout, and stderr. Use for system diagnostics, disk space checks, batch audio/file automation, or tooling inspection.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "PowerShell command string to execute" },
                    "cwd": { "type": "string", "description": "Optional working directory" },
                    "timeout_sec": { "type": "integer", "description": "Execution timeout in seconds (1..120)", "default": 30 }
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "list_directory_entries",
            "description": "List files and subdirectories inside a local filesystem directory with sizes and audio-format indicators.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path to inspect" },
                    "max_entries": { "type": "integer", "default": 200 }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "read_text_file",
            "description": "Read UTF-8 text content from a local file (such as playlists, XML/NML configs, logs, or scripts) with optional line bounds.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "max_bytes": { "type": "integer", "default": 32768 }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "write_text_file",
            "description": "Write UTF-8 text content to a local file, creating parent directories if needed.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"]
            }
        }),
        json!({
            "name": "move_or_rename_path",
            "description": "Move or rename a local file or directory on disk (e.g. organizing audio files into clean folders).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from_path": { "type": "string" },
                    "to_path": { "type": "string" }
                },
                "required": ["from_path", "to_path"]
            }
        }),
        json!({
            "name": "open_system_target",
            "description": "Open a local folder, file, or installed desktop application (such as VirtualDJ or Explorer) using the OS shell.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": { "type": "string", "description": "File path, folder path, or application executable/URI" }
                },
                "required": ["target"]
            }
        }),
    ];

    if let Some(core_arr) = list_core_mcp_tools()
        .get("tools")
        .and_then(|v| v.as_array())
    {
        for item in core_arr {
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if !tools
                .iter()
                .any(|t| t.get("name").and_then(|v| v.as_str()) == Some(name))
            {
                tools.push(item.clone());
            }
        }
    }

    tools
}

fn requires_system_confirmation(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "run_powershell_command" | "write_text_file" | "move_or_rename_path" | "open_system_target"
    )
}

fn camelot_sort_tuple(camelot: &str) -> (i32, u8) {
    let trimmed = camelot.trim().to_ascii_uppercase();
    let mode = match trimmed.chars().last() {
        Some('A') => 0,
        Some('B') => 1,
        _ => 2,
    };
    let num = trimmed
        .trim_end_matches(['A', 'B'])
        .parse::<i32>()
        .unwrap_or(99);
    (num, mode)
}

fn sort_tracks_harmonically(tracks: &mut [Track]) {
    tracks.sort_by(|a, b| {
        let ca = camelot_sort_tuple(a.camelot.as_deref().unwrap_or("8A"));
        let cb = camelot_sort_tuple(b.camelot.as_deref().unwrap_or("8A"));
        ca.cmp(&cb).then_with(|| {
            let ba = a.bpm.unwrap_or(124.0);
            let bb = b.bpm.unwrap_or(124.0);
            ba.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
        })
    });
}

pub fn human_tool_title(tool_name: &str) -> String {
    match tool_name {
        "list_library_tracks" => "Leer biblioteca".to_string(),
        "search_library_tracks" => "Filtrar biblioteca".to_string(),
        "get_track_details" => "Inspeccionar pista".to_string(),
        "import_audio_paths" => "Importar carpeta de audio".to_string(),
        "update_track_dj_metadata" => "Actualizar metadatos DJ".to_string(),
        "remove_library_tracks" => "Quitar pistas de biblioteca".to_string(),
        "list_playlists" => "Consultar playlists".to_string(),
        "get_playlist_tracks" => "Leer pistas de playlist".to_string(),
        "create_playlist" => "Crear playlist".to_string(),
        "update_playlist_tracks" => "Actualizar orden de playlist".to_string(),
        "rename_playlist" => "Renombrar playlist".to_string(),
        "delete_playlist" => "Eliminar playlist".to_string(),
        "control_audio_player" => "Controlar reproductor".to_string(),
        "set_app_view_or_mini_mode" => "Cambiar vista o Modo Mini".to_string(),
        "push_virtualdj_live_crate" => "Enviar a VirtualDJ".to_string(),
        "get_virtualdj_live_deck" => "Leer deck en VirtualDJ".to_string(),
        "download_p2p_swarm_track" => "Descargar del enjambre P2P".to_string(),
        "list_directory_entries" => "Inspeccionar carpeta".to_string(),
        "read_text_file" => "Leer archivo".to_string(),
        "write_text_file" => "Escribir archivo".to_string(),
        "move_or_rename_path" => "Mover o renombrar archivo".to_string(),
        "run_powershell_command" => "Ejecutar en PowerShell".to_string(),
        "open_system_target" => "Abrir en el sistema".to_string(),
        "audit_repertoire" => "Auditar repertorio".to_string(),
        "curate_dj_crate" => "Ordenar crate armónico".to_string(),
        "export_dj_crate" => "Exportar crate DJ".to_string(),
        "analyze_audio_file" => "Analizar audio".to_string(),
        "detect_track_cues" => "Detectar 4 Hot Cues".to_string(),
        "build_dj_style_dna" => "Calcular perfil musical".to_string(),
        _ => "Ejecutar herramienta".to_string(),
    }
}

pub fn summarize_tool_execution(
    tool_name: &str,
    args: &Value,
    result: Result<&Value, &str>,
) -> (String, String, String) {
    let title = human_tool_title(tool_name);
    match result {
        Err(err_msg) => (
            title,
            format!("Error: {}", truncate_chars(err_msg, 78)),
            format!("argumentos: {}\nerror: {}", args, err_msg),
        ),
        Ok(val) => match tool_name {
            "list_library_tracks" => {
                let count = val.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                (
                    title,
                    format!("{count} pistas en biblioteca local"),
                    format!("total: {count} pistas en hera.db"),
                )
            }
            "search_library_tracks" => {
                let count = val.get("matched").and_then(|v| v.as_u64()).unwrap_or(0);
                let q = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("catálogo completo");
                (
                    title,
                    format!("{count} pistas coincidentes ({q})"),
                    format!("criterio: {}\ncoincidencias: {count}", args),
                )
            }
            "import_audio_paths" => {
                let imported = val.get("imported_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let scanned = val.get("scanned_files").and_then(|v| v.as_u64()).unwrap_or(0);
                (
                    title,
                    format!("{imported} pistas con BPM, tono y 4 Hot Cues"),
                    format!("archivos escaneados: {scanned}\nnuevas pistas en SQLite: {imported}"),
                )
            }
            "create_playlist" => {
                let name = val
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Playlist");
                let count = val.get("track_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let keys = val
                    .get("key_flow")
                    .and_then(|v| v.as_str())
                    .unwrap_or("orden armónico");
                (
                    title,
                    format!("«{name}» ({count} pistas · {keys})"),
                    format!("playlist: {name}\npistas: {count}\nflujo: {keys}"),
                )
            }
            "push_virtualdj_live_crate" => {
                let crate_name = val
                    .get("crate_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("HERA En Vivo");
                let total = val.get("total_tracks").and_then(|v| v.as_u64()).unwrap_or(0);
                let first_file = val
                    .get("written_files")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or("HERA En Vivo.vdjfolder");
                (
                    title,
                    format!("{crate_name}.vdjfolder ({total} pistas)"),
                    format!(
                        "ruta: {first_file}\nestado: {total} pistas · {} Hot Cues",
                        total * 4
                    ),
                )
            }
            "control_audio_player" => {
                let act = args
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("play_track");
                let label = val
                    .get("track_label")
                    .and_then(|v| v.as_str())
                    .unwrap_or(act);
                (
                    title,
                    label.to_string(),
                    format!("acción: {act}\ndetalle: {label}"),
                )
            }
            "list_directory_entries" => {
                let count = val.get("total_entries").and_then(|v| v.as_u64()).unwrap_or(0);
                let audio = val.get("audio_files").and_then(|v| v.as_u64()).unwrap_or(0);
                let p = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                (
                    title,
                    format!("{} ({count} elementos · {audio} audios)", short_path_label(p)),
                    format!("ruta: {p}\nelementos: {count} (archivos de audio: {audio})"),
                )
            }
            "run_powershell_command" => {
                let code = val.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0);
                let cmd_str = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                let out_str = val.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
                (
                    title,
                    format!("{} (código {code})", truncate_chars(cmd_str, 46)),
                    format!(
                        "comando: {cmd_str}\nsalida:\n{}",
                        truncate_chars(if out_str.is_empty() { "(sin salida)" } else { out_str }, 420)
                    ),
                )
            }
            _ => {
                let raw = val.to_string();
                (
                    title,
                    tool_name.replace('_', " "),
                    truncate_chars(&raw, 360),
                )
            }
        },
    }
}

fn short_path_label(p: &str) -> String {
    Path::new(p)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| truncate_chars(p, 32))
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max_chars).collect::<String>())
    }
}

pub async fn dispatch_unified_agent_tool<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session_id: Option<&str>,
    tool_name: &str,
    args: Value,
) -> Result<Value, String> {
    let step_id = format!("step-{}", uuid::Uuid::new_v4());
    let args_preview = truncate_chars(&args.to_string(), 180);

    // Check if an active turn exists to record live step status & check permission policy
    let mut policy = "auto".to_string();
    let mut message_id_opt = None;
    if let Some(sid) = session_id {
        let mut turns = ctx.active_turns.lock().await;
        if let Some(tracker) = turns.get_mut(sid) {
            if tracker.cancel_flag.load(Ordering::SeqCst) {
                return Err("Turno cancelado por el usuario".to_string());
            }
            policy = tracker.execution_policy.clone();
            message_id_opt = Some(tracker.message_id.clone());

            let initial_status = if policy == "confirm" && requires_system_confirmation(tool_name) {
                "awaiting_approval"
            } else {
                "running"
            };
            let step_title = human_tool_title(tool_name);
            let running_step = AgentToolStepView {
                id: step_id.clone(),
                tool_name: tool_name.to_string(),
                title: step_title,
                summary: if initial_status == "awaiting_approval" {
                    format!("Confirmar ejecución · {}", truncate_chars(&args_preview, 54))
                } else {
                    "En progreso…".to_string()
                },
                status: initial_status.to_string(),
                arguments_preview: args_preview.clone(),
                receipt_text: format!("argumentos: {args_preview}"),
            };
            tracker.steps.push(running_step.clone());
            ctx.emit_stream(AgentStreamEvent {
                session_id: sid.to_string(),
                message_id: tracker.message_id.clone(),
                kind: "step_upsert".to_string(),
                text_delta: None,
                full_content: None,
                step: Some(running_step),
                session: None,
                actions: None,
            });
        }
    }

    if policy == "confirm" && requires_system_confirmation(tool_name) {
        let (tx, rx) = oneshot::channel::<bool>();
        {
            let mut approvals = ctx.pending_approvals.lock().await;
            approvals.insert(step_id.clone(), tx);
        }
        let approved = tokio::time::timeout(Duration::from_secs(180), rx)
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or(false);
        if !approved {
            if let (Some(sid), Some(mid)) = (session_id, message_id_opt.as_ref()) {
                finalize_tracked_step(
                    ctx,
                    sid,
                    mid,
                    &step_id,
                    tool_name,
                    &args,
                    Err("Ejecución cancelada por el usuario"),
                    Some("cancelled"),
                )
                .await;
            }
            return Err("Ejecución cancelada por el usuario".to_string());
        }
    }

    let exec_res = execute_tool_inner(ctx, session_id, tool_name, &args).await;

    if let (Some(sid), Some(mid)) = (session_id, message_id_opt.as_ref()) {
        finalize_tracked_step(
            ctx,
            sid,
            mid,
            &step_id,
            tool_name,
            &args,
            exec_res.as_ref().map_err(String::as_str),
            None,
        )
        .await;
    }

    exec_res
}

async fn finalize_tracked_step<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session_id: &str,
    message_id: &str,
    step_id: &str,
    tool_name: &str,
    args: &Value,
    res: Result<&Value, &str>,
    status_override: Option<&str>,
) {
    let (title, summary, receipt_text) = summarize_tool_execution(tool_name, args, res);
    let status = status_override
        .unwrap_or(if res.is_ok() { "done" } else { "error" })
        .to_string();

    let mut updated_step = None;
    {
        let mut turns = ctx.active_turns.lock().await;
        if let Some(tracker) = turns.get_mut(session_id) {
            if let Some(existing) = tracker.steps.iter_mut().find(|s| s.id == step_id) {
                existing.title = title;
                existing.summary = summary;
                existing.status = status;
                existing.receipt_text = receipt_text;
                updated_step = Some(existing.clone());
            }
        }
    }

    if let Some(step) = updated_step {
        ctx.emit_stream(AgentStreamEvent {
            session_id: session_id.to_string(),
            message_id: message_id.to_string(),
            kind: "step_upsert".to_string(),
            text_delta: None,
            full_content: None,
            step: Some(step),
            session: None,
            actions: None,
        });
    }
}

async fn record_followup_action<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session_id: Option<&str>,
    action: AgentFollowupAction,
) {
    let Some(sid) = session_id else { return };
    let mut turns = ctx.active_turns.lock().await;
    if let Some(tracker) = turns.get_mut(sid) {
        if !tracker
            .actions
            .iter()
            .any(|a| a.kind == action.kind && a.target_id == action.target_id)
        {
            tracker.actions.push(action);
        }
    }
}

async fn execute_tool_inner<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session_id: Option<&str>,
    tool_name: &str,
    args: &Value,
) -> Result<Value, String> {
    match tool_name {
        "list_library_tracks" => {
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(100)
                .clamp(1, 500) as usize;
            let all = ctx
                .with_db(|db| Box::pin(async move { db.get_all_tracks().await.map_err(|e| e.to_string()) }))
                .await?;
            let total = all.len();
            let items: Vec<Value> = all
                .into_iter()
                .take(limit)
                .map(|t| {
                    json!({
                        "id": t.id,
                        "title": t.title,
                        "artists": t.artists,
                        "bpm": t.bpm.unwrap_or(124.0),
                        "camelot": t.camelot.unwrap_or_else(|| "8A".to_string()),
                        "duration": t.duration,
                        "path": t.path,
                    })
                })
                .collect();
            Ok(json!({ "total": total, "tracks": items }))
        }
        "search_library_tracks" => {
            let q = args
                .get("query")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_lowercase())
                .unwrap_or_default();
            let bpm_min = args.get("bpm_min").and_then(|v| v.as_f64());
            let bpm_max = args.get("bpm_max").and_then(|v| v.as_f64());
            let camelot = args
                .get("camelot")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_ascii_uppercase())
                .filter(|s| !s.is_empty());

            let all = ctx
                .with_db(|db| Box::pin(async move { db.get_all_tracks().await.map_err(|e| e.to_string()) }))
                .await?;
            let mut matched: Vec<Track> = all
                .into_iter()
                .filter(|t| {
                    if !q.is_empty() {
                        let hay = format!(
                            "{} {} {} {}",
                            t.title.to_lowercase(),
                            t.artists.join(" ").to_lowercase(),
                            t.album.to_lowercase(),
                            t.genres.join(" ").to_lowercase()
                        );
                        if !hay.contains(&q) {
                            return false;
                        }
                    }
                    let bpm = t.bpm.unwrap_or(124.0);
                    if let Some(min_b) = bpm_min
                        && bpm < min_b
                    {
                        return false;
                    }
                    if let Some(max_b) = bpm_max
                        && bpm > max_b
                    {
                        return false;
                    }
                    if let Some(ref cam) = camelot {
                        let tc = t.camelot.as_deref().unwrap_or("8A").to_ascii_uppercase();
                        if &tc != cam {
                            return false;
                        }
                    }
                    true
                })
                .collect();
            sort_tracks_harmonically(&mut matched);

            let items: Vec<Value> = matched
                .iter()
                .map(|t| {
                    json!({
                        "id": t.id,
                        "title": t.title,
                        "artists": t.artists,
                        "bpm": t.bpm.unwrap_or(124.0),
                        "camelot": t.camelot.clone().unwrap_or_else(|| "8A".to_string()),
                        "path": t.path,
                    })
                })
                .collect();
            Ok(json!({ "matched": items.len(), "tracks": items }))
        }
        "import_audio_paths" => {
            let raw_paths: Vec<PathBuf> = args
                .get("paths")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(PathBuf::from))
                        .collect()
                })
                .unwrap_or_default();
            if raw_paths.is_empty() {
                return Err("Se requiere al menos una ruta en 'paths'".to_string());
            }

            let existing = ctx
                .with_db(|db| Box::pin(async move { db.get_all_track_paths().await.map_err(|e| e.to_string()) }))
                .await?;
            let existing_set: std::collections::HashSet<PathBuf> =
                existing.into_iter().map(PathBuf::from).collect();

            let (scanned_files, parsed_tracks) = tauri::async_runtime::spawn_blocking(move || {
                let mut files = Vec::new();
                for p in &raw_paths {
                    if p.is_file() {
                        files.push(p.clone());
                    } else if p.is_dir() {
                        files.extend(scan_dirs(std::slice::from_ref(p), &SUPPORTED_TRACKS_EXTENSIONS));
                    }
                }
                let total_found = files.len();
                files.retain(|f| !existing_set.contains(f));
                let tracks: Vec<Track> = files
                    .iter()
                    .filter_map(|f| get_track_from_file(f).ok())
                    .collect();
                (total_found, tracks)
            })
            .await
            .map_err(|e| format!("Fallo escaneando audio: {e}"))?;

            let imported_count = parsed_tracks.len();
            let imported_ids: Vec<String> = parsed_tracks.iter().map(|t| t.id.clone()).collect();
            if !parsed_tracks.is_empty() {
                ctx.with_db(|db| {
                    Box::pin(async move {
                        db.insert_tracks(parsed_tracks)
                            .await
                            .map_err(|e| e.to_string())
                    })
                })
                .await?;
                ctx.emit_ui_sync(AgentUiSyncEvent {
                    kind: "invalidate_library".to_string(),
                    action: None,
                    track_id: None,
                    seek_seconds: None,
                    route: None,
                });
            }

            Ok(json!({
                "scanned_files": scanned_files,
                "imported_count": imported_count,
                "track_ids": imported_ids,
            }))
        }
        "update_track_metadata" => {
            let track_id = args
                .get("track_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Falta 'track_id'".to_string())?
                .to_string();
            let title_opt = args.get("title").and_then(|v| v.as_str()).map(String::from);
            let artist_opt = args.get("artist").and_then(|v| v.as_str()).map(String::from);
            let bpm_opt = args.get("bpm").and_then(|v| v.as_f64());
            let cam_opt = args.get("camelot").and_then(|v| v.as_str()).map(String::from);

            let updated = ctx
                .with_db(|db| {
                    Box::pin(async move {
                        let Some(mut track) =
                            db.get_track(&track_id).await.map_err(|e| e.to_string())?
                        else {
                            return Err(format!("Pista '{track_id}' no encontrada"));
                        };
                        if let Some(t) = title_opt {
                            track.title = t;
                        }
                        if let Some(a) = artist_opt {
                            track.artists = vec![a.clone()];
                            track.album_artist = a;
                        }
                        if let Some(b) = bpm_opt {
                            track.bpm = Some(b);
                        }
                        if let Some(c) = cam_opt {
                            track.camelot = Some(c);
                        }
                        db.update_track(track).await.map_err(|e| e.to_string())
                    })
                })
                .await?;
            ctx.emit_ui_sync(AgentUiSyncEvent {
                kind: "invalidate_library".to_string(),
                action: None,
                track_id: None,
                seek_seconds: None,
                route: None,
            });
            Ok(json!({ "updated": true, "track_id": updated.id, "title": updated.title }))
        }
        "remove_library_tracks" => {
            let ids: Vec<String> = args
                .get("track_ids")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let removed = ids.len();
            ctx.with_db(|db| {
                Box::pin(async move { db.remove_tracks(&ids).await.map_err(|e| e.to_string()) })
            })
            .await?;
            ctx.emit_ui_sync(AgentUiSyncEvent {
                kind: "invalidate_library".to_string(),
                action: None,
                track_id: None,
                seek_seconds: None,
                route: None,
            });
            Ok(json!({ "removed_count": removed }))
        }
        "list_playlists" => {
            let playlists = ctx
                .with_db(|db| Box::pin(async move { db.get_all_playlists().await.map_err(|e| e.to_string()) }))
                .await?;
            let out: Vec<Value> = playlists
                .iter()
                .map(|p| json!({ "id": p.id, "name": p.name, "track_count": p.tracks.len(), "track_ids": p.tracks }))
                .collect();
            Ok(json!({ "total": out.len(), "playlists": out }))
        }
        "create_playlist" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("Set HERA")
                .to_string();
            let sort_harmonic = args
                .get("sort_harmonic")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let explicit_ids: Vec<String> = args
                .get("track_ids")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let (playlist, first_track_id, key_flow) = ctx
                .with_db(|db| {
                    Box::pin(async move {
                        let mut tracks = if explicit_ids.is_empty() {
                            db.get_all_tracks().await.map_err(|e| e.to_string())?
                        } else {
                            db.get_tracks(&explicit_ids).await.map_err(|e| e.to_string())?
                        };
                        if sort_harmonic {
                            sort_tracks_harmonically(&mut tracks);
                        }
                        let mut keys_dedup: Vec<String> = Vec::new();
                        for t in &tracks {
                            let k = t.camelot.clone().unwrap_or_else(|| "8A".to_string());
                            if keys_dedup.last() != Some(&k) {
                                keys_dedup.push(k);
                            }
                        }
                        let flow = if keys_dedup.is_empty() {
                            "vacía".to_string()
                        } else {
                            keys_dedup.into_iter().take(4).collect::<Vec<_>>().join(" → ")
                        };
                        let first_id = tracks.first().map(|t| t.id.clone());
                        let final_ids: Vec<String> = tracks.into_iter().map(|t| t.id).collect();
                        let created = db
                            .create_playlist(name, final_ids, None)
                            .await
                            .map_err(|e| e.to_string())?;
                        Ok((created, first_id, flow))
                    })
                })
                .await?;

            ctx.emit_ui_sync(AgentUiSyncEvent {
                kind: "invalidate_library".to_string(),
                action: None,
                track_id: None,
                seek_seconds: None,
                route: None,
            });

            if let Some(fid) = first_track_id {
                record_followup_action(
                    ctx,
                    session_id,
                    AgentFollowupAction {
                        kind: "play_track".to_string(),
                        label: "Escuchar primera pista".to_string(),
                        target_id: fid,
                    },
                )
                .await;
            }
            record_followup_action(
                ctx,
                session_id,
                AgentFollowupAction {
                    kind: "open_playlist".to_string(),
                    label: "Abrir playlist".to_string(),
                    target_id: playlist.id.clone(),
                },
            )
            .await;

            Ok(json!({
                "playlist_id": playlist.id,
                "name": playlist.name,
                "track_count": playlist.tracks.len(),
                "track_ids": playlist.tracks,
                "key_flow": key_flow,
            }))
        }
        "set_playlist_tracks" => {
            let pid = args
                .get("playlist_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Falta 'playlist_id'".to_string())?
                .to_string();
            let ids: Vec<String> = args
                .get("track_ids")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let updated = ctx
                .with_db(|db| {
                    Box::pin(async move {
                        db.set_playlist_tracks(&pid, ids)
                            .await
                            .map_err(|e| e.to_string())
                    })
                })
                .await?;
            ctx.emit_ui_sync(AgentUiSyncEvent {
                kind: "invalidate_library".to_string(),
                action: None,
                track_id: None,
                seek_seconds: None,
                route: None,
            });
            Ok(json!({ "playlist_id": updated.id, "track_count": updated.tracks.len() }))
        }
        "rename_playlist" => {
            let pid = args
                .get("playlist_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Falta 'playlist_id'".to_string())?
                .to_string();
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Falta 'name'".to_string())?
                .to_string();
            let updated = ctx
                .with_db(|db| {
                    Box::pin(async move {
                        db.rename_playlist(&pid, name)
                            .await
                            .map_err(|e| e.to_string())
                    })
                })
                .await?;
            ctx.emit_ui_sync(AgentUiSyncEvent {
                kind: "invalidate_library".to_string(),
                action: None,
                track_id: None,
                seek_seconds: None,
                route: None,
            });
            Ok(json!({ "playlist_id": updated.id, "name": updated.name }))
        }
        "delete_playlist" => {
            let pid = args
                .get("playlist_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Falta 'playlist_id'".to_string())?
                .to_string();
            ctx.with_db(|db| {
                Box::pin(async move { db.delete_playlist(&pid).await.map_err(|e| e.to_string()) })
            })
            .await?;
            ctx.emit_ui_sync(AgentUiSyncEvent {
                kind: "invalidate_library".to_string(),
                action: None,
                track_id: None,
                seek_seconds: None,
                route: None,
            });
            Ok(json!({ "deleted": true }))
        }
        "control_audio_player" => {
            let action = args
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("play_track")
                .to_string();
            let track_id = args.get("track_id").and_then(|v| v.as_str()).map(String::from);
            let seek_seconds = args.get("seek_seconds").and_then(|v| v.as_f64());

            let track_label = if let Some(ref tid) = track_id {
                let tid_cl = tid.clone();
                ctx.with_db(|db| {
                    Box::pin(async move {
                        Ok(db
                            .get_track(&tid_cl)
                            .await
                            .ok()
                            .flatten()
                            .map(|t| format!("{} — {}", t.artists.first().cloned().unwrap_or_default(), t.title))
                            .unwrap_or(tid_cl))
                    })
                })
                .await
                .unwrap_or_else(|_| action.clone())
            } else {
                action.clone()
            };

            ctx.emit_ui_sync(AgentUiSyncEvent {
                kind: "player_control".to_string(),
                action: Some(action.clone()),
                track_id,
                seek_seconds,
                route: None,
            });
            Ok(json!({ "status": "dispatched", "action": action, "track_label": track_label }))
        }
        "push_virtualdj_live_crate" => {
            let crate_name = args
                .get("crate_name")
                .and_then(|v| v.as_str())
                .unwrap_or("HERA En Vivo")
                .to_string();
            let append = args.get("append").and_then(|v| v.as_bool()).unwrap_or(false);
            let playlist_id = args.get("playlist_id").and_then(|v| v.as_str()).map(String::from);
            let explicit_ids: Vec<String> = args
                .get("track_ids")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let tracks = ctx
                .with_db(|db| {
                    Box::pin(async move {
                        if let Some(pid) = playlist_id {
                            if let Some(pl) = db.get_playlist(&pid).await.map_err(|e| e.to_string())? {
                                return db.get_tracks(&pl.tracks).await.map_err(|e| e.to_string());
                            }
                        }
                        if !explicit_ids.is_empty() {
                            return db.get_tracks(&explicit_ids).await.map_err(|e| e.to_string());
                        }
                        let mut all = db.get_all_tracks().await.map_err(|e| e.to_string())?;
                        sort_tracks_harmonically(&mut all);
                        Ok(all)
                    })
                })
                .await?;

            let export_items: Vec<ExportTrackItem> = tracks.iter().map(track_to_export_item).collect();

            match &ctx.host {
                AgentHostBridge::Tauri(app) => {
                    let res = push_virtualdj_live_crate(
                        app.clone(),
                        PushVirtualDjLiveCrateRequest {
                            tracks: export_items,
                            append,
                            crate_name: Some(crate_name),
                        },
                    )
                    .await?;
                    serde_json::to_value(res).map_err(|e| e.to_string())
                }
                AgentHostBridge::Standalone(_) => {
                    let mut target_dirs = Vec::new();
                    if let Ok(vdj_home) = std::env::var("HERA_VDJ_HOME")
                        && !vdj_home.trim().is_empty()
                    {
                        let folders = PathBuf::from(vdj_home).join("Folders");
                        if std::fs::create_dir_all(&folders).is_ok() {
                            target_dirs.push(folders);
                        }
                    }
                    if target_dirs.is_empty() {
                        let tmp = std::env::temp_dir().join("hera_agent_vdj_exports");
                        let _ = std::fs::create_dir_all(&tmp);
                        target_dirs.push(tmp);
                    }
                    let written = crate::plugins::hera_dj::write_virtualdj_live_crate_files(
                        &crate_name,
                        &export_items,
                        &target_dirs,
                    );
                    Ok(json!({
                        "crate_name": crate_name,
                        "total_tracks": export_items.len(),
                        "written_files": written,
                        "synced_to_virtualdj_home": true,
                    }))
                }
            }
        }
        "read_virtualdj_now_playing" => {
            let np = read_virtualdj_now_playing().await?;
            Ok(json!({ "now_playing": np }))
        }
        "export_dj_crate_file" => {
            let crate_name = args
                .get("crate_name")
                .and_then(|v| v.as_str())
                .unwrap_or("HERA Export")
                .to_string();
            let playlist_id = args.get("playlist_id").and_then(|v| v.as_str()).map(String::from);
            let explicit_ids: Vec<String> = args
                .get("track_ids")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let formats: Vec<String> = args
                .get("formats")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .filter(|v: &Vec<String>| !v.is_empty())
                .unwrap_or_else(|| vec!["virtualdj".to_string(), "m3u8".to_string()]);
            let output_dir = args.get("output_dir").and_then(|v| v.as_str()).map(String::from);

            let tracks = ctx
                .with_db(|db| {
                    Box::pin(async move {
                        if let Some(pid) = playlist_id
                            && let Some(pl) = db.get_playlist(&pid).await.map_err(|e| e.to_string())?
                        {
                            return db.get_tracks(&pl.tracks).await.map_err(|e| e.to_string());
                        }
                        if !explicit_ids.is_empty() {
                            return db.get_tracks(&explicit_ids).await.map_err(|e| e.to_string());
                        }
                        db.get_all_tracks().await.map_err(|e| e.to_string())
                    })
                })
                .await?;
            let items: Vec<ExportTrackItem> = tracks.iter().map(track_to_export_item).collect();
            let res = tauri::async_runtime::spawn_blocking(move || {
                export_crate_bundle(&ExportCrateRequest {
                    crate_name,
                    brief: "Exportado por Agente HERA".to_string(),
                    tracks: items,
                    formats,
                    output_dir,
                })
            })
            .await
            .map_err(|e| format!("Fallo exportando crate: {e}"))?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "set_app_view_or_mini_mode" => {
            if let Some(wm) = args.get("window_mode").and_then(|v| v.as_str())
                && let AgentHostBridge::Tauri(app) = &ctx.host
            {
                let _ = set_mini_overlay_mode(
                    app.clone(),
                    SetMiniOverlayModeRequest {
                        mode: wm.to_string(),
                        pinned: wm != "full",
                    },
                )
                .await;
            }
            if let Some(route) = args.get("route").and_then(|v| v.as_str()) {
                ctx.emit_ui_sync(AgentUiSyncEvent {
                    kind: "navigate".to_string(),
                    action: None,
                    track_id: None,
                    seek_seconds: None,
                    route: Some(route.to_string()),
                });
            }
            Ok(json!({ "ok": true }))
        }
        "download_swarm_track_to_library" => {
            let artist = args
                .get("artist")
                .and_then(|v| v.as_str())
                .unwrap_or("DaviDJ")
                .to_string();
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Swarm Master")
                .to_string();
            let bpm = args.get("bpm").and_then(|v| v.as_f64()).unwrap_or(125.0) as f32;
            let camelot = args
                .get("camelot")
                .and_then(|v| v.as_str())
                .unwrap_or("8A")
                .to_string();
            let track_id = args
                .get("track_id")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let AgentHostBridge::Tauri(app) = &ctx.host else {
                return Err("Descarga P2P requiere contexto de aplicación activa".to_string());
            };
            let dl = download_p2p_track(
                app.clone(),
                DownloadP2PTrackRequest {
                    track_id,
                    artist,
                    title,
                    bpm,
                    camelot,
                    hera_seal: "VERIFIED_STUDIO".to_string(),
                    peer_endpoint: None,
                    turn_relay_endpoint: None,
                    chunk_count: None,
                },
            )
            .await?;

            let saved_path = PathBuf::from(&dl.saved_path);
            if let Ok(parsed) = get_track_from_file(&saved_path) {
                let _ = ctx
                    .with_db(|db| {
                        Box::pin(async move {
                            db.insert_tracks(vec![parsed])
                                .await
                                .map_err(|e| e.to_string())
                        })
                    })
                    .await;
                ctx.emit_ui_sync(AgentUiSyncEvent {
                    kind: "invalidate_library".to_string(),
                    action: None,
                    track_id: None,
                    seek_seconds: None,
                    route: None,
                });
            }
            serde_json::to_value(dl).map_err(|e| e.to_string())
        }
        "run_powershell_command" => {
            let command_str = args
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Falta 'command'".to_string())?
                .to_string();
            let cwd_opt = args.get("cwd").and_then(|v| v.as_str()).map(PathBuf::from);
            let timeout_sec = args
                .get("timeout_sec")
                .and_then(|v| v.as_u64())
                .unwrap_or(30)
                .clamp(1, 120);

            let mut cmd = if cfg!(windows) {
                let mut c = tokio::process::Command::new("powershell");
                c.args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!(
                        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; {}",
                        command_str
                    ),
                ]);
                #[cfg(windows)]
                {
                    c.creation_flags(0x08000000); // CREATE_NO_WINDOW
                }
                c
            } else {
                let mut c = tokio::process::Command::new("sh");
                c.args(["-c", &command_str]);
                c
            };

            if let Some(cwd) = cwd_opt
                && cwd.exists()
            {
                cmd.current_dir(cwd);
            }
            cmd.stdin(std::process::Stdio::null());

            let output = tokio::time::timeout(Duration::from_secs(timeout_sec), cmd.output())
                .await
                .map_err(|_| format!("Tiempo de espera agotado ({timeout_sec}s)"))?
                .map_err(|e| format!("Fallo lanzando shell: {e}"))?;

            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let exit_code = output.status.code().unwrap_or(-1);

            Ok(json!({
                "exit_code": exit_code,
                "stdout": truncate_chars(&stdout, 12000),
                "stderr": truncate_chars(&stderr, 4000),
            }))
        }
        "list_directory_entries" => {
            let path_str = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Falta 'path'".to_string())?
                .to_string();
            let max_entries = args
                .get("max_entries")
                .and_then(|v| v.as_u64())
                .unwrap_or(200)
                .clamp(1, 1000) as usize;

            tauri::async_runtime::spawn_blocking(move || {
                let dir = PathBuf::from(&path_str);
                if !dir.exists() {
                    return Err(format!("La ruta '{}' no existe", path_str));
                }
                let read_dir = std::fs::read_dir(&dir)
                    .map_err(|e| format!("No se pudo leer '{}': {e}", path_str))?;
                let mut entries = Vec::new();
                let mut total_entries = 0usize;
                let mut audio_files = 0usize;

                for entry_res in read_dir.flatten() {
                    total_entries += 1;
                    let p = entry_res.path();
                    let is_dir = p.is_dir();
                    let ext = p
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    let is_audio = !is_dir && SUPPORTED_TRACKS_EXTENSIONS.contains(&ext.as_str());
                    if is_audio {
                        audio_files += 1;
                    }
                    if entries.len() < max_entries {
                        let size = entry_res.metadata().map(|m| m.len()).unwrap_or(0);
                        entries.push(json!({
                            "name": entry_res.file_name().to_string_lossy(),
                            "path": p.to_string_lossy(),
                            "is_dir": is_dir,
                            "is_audio": is_audio,
                            "size_bytes": size,
                        }));
                    }
                }
                Ok(json!({
                    "path": path_str,
                    "total_entries": total_entries,
                    "audio_files": audio_files,
                    "entries": entries,
                }))
            })
            .await
            .map_err(|e| format!("Fallo listando directorio: {e}"))?
        }
        "read_text_file" => {
            let path_str = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Falta 'path'".to_string())?
                .to_string();
            let max_bytes = args
                .get("max_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(32768)
                .clamp(256, 262144) as usize;
            tauri::async_runtime::spawn_blocking(move || {
                let bytes = std::fs::read(&path_str)
                    .map_err(|e| format!("No se pudo leer '{}': {e}", path_str))?;
                let truncated = bytes.len() > max_bytes;
                let slice = if truncated { &bytes[..max_bytes] } else { &bytes[..] };
                Ok(json!({
                    "path": path_str,
                    "truncated": truncated,
                    "content": String::from_utf8_lossy(slice),
                }))
            })
            .await
            .map_err(|e| format!("Fallo leyendo archivo: {e}"))?
        }
        "write_text_file" => {
            let path_str = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Falta 'path'".to_string())?
                .to_string();
            let content = args
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Falta 'content'".to_string())?
                .to_string();
            tauri::async_runtime::spawn_blocking(move || {
                let target = PathBuf::from(&path_str);
                if let Some(parent) = target.parent()
                    && !parent.as_os_str().is_empty()
                {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("No se pudo crear carpeta padre: {e}"))?;
                }
                let bytes_len = content.len();
                std::fs::write(&target, content)
                    .map_err(|e| format!("No se pudo escribir '{}': {e}", path_str))?;
                Ok(json!({ "path": path_str, "bytes_written": bytes_len }))
            })
            .await
            .map_err(|e| format!("Fallo escribiendo archivo: {e}"))?
        }
        "move_or_rename_path" => {
            let from_path = args
                .get("from_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Falta 'from_path'".to_string())?
                .to_string();
            let to_path = args
                .get("to_path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Falta 'to_path'".to_string())?
                .to_string();
            tauri::async_runtime::spawn_blocking(move || {
                let dst = PathBuf::from(&to_path);
                if let Some(parent) = dst.parent()
                    && !parent.as_os_str().is_empty()
                {
                    let _ = std::fs::create_dir_all(parent);
                }
                std::fs::rename(&from_path, &dst)
                    .map_err(|e| format!("No se pudo mover '{from_path}' a '{to_path}': {e}"))?;
                Ok(json!({ "from_path": from_path, "to_path": to_path, "moved": true }))
            })
            .await
            .map_err(|e| format!("Fallo moviendo ruta: {e}"))?
        }
        "open_system_target" => {
            let target = args
                .get("target")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Falta 'target'".to_string())?
                .to_string();
            tauri::async_runtime::spawn_blocking(move || {
                let mut cmd = if cfg!(windows) {
                    let mut c = std::process::Command::new("explorer");
                    c.arg(&target);
                    c
                } else if cfg!(target_os = "macos") {
                    let mut c = std::process::Command::new("open");
                    c.arg(&target);
                    c
                } else {
                    let mut c = std::process::Command::new("xdg-open");
                    c.arg(&target);
                    c
                };
                cmd.spawn()
                    .map_err(|e| format!("No se pudo abrir '{target}': {e}"))?;
                Ok(json!({ "opened": target }))
            })
            .await
            .map_err(|e| format!("Fallo abriendo destino: {e}"))?
        }
        _ => {
            let args_cl = args.clone();
            let name_cl = tool_name.to_string();
            tauri::async_runtime::spawn_blocking(move || {
                dispatch_core_mcp_tool(&name_cl, args_cl, shared_desktop_p2p_state())
            })
            .await
            .map_err(|e| format!("Fallo en tool DSP/P2P: {e}"))?
        }
    }
}

/// Build the Axum router for the Bearer-protected loopback MCP server (`POST /mcp`).
pub fn build_local_mcp_router<R: Runtime>(ctx: AgentRuntimeContext<R>) -> Router {
    Router::new()
        .route("/mcp", post(mcp_http_handler::<R>))
        .with_state(ctx)
}

async fn mcp_http_handler<R: Runtime>(
    AxumState(ctx): AxumState<AgentRuntimeContext<R>>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> Response {
    let expected = format!("Bearer {}", ctx.mcp_bearer_token);
    let provided = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Unauthorized MCP client" })),
        )
            .into_response();
    }

    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));

    match method {
        "notifications/initialized" | "notifications/cancelled" => StatusCode::ACCEPTED.into_response(),
        "initialize" => (
            StatusCode::OK,
            Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "hera-desktop-mcp", "version": "0.5.0" }
                }
            })),
        )
            .into_response(),
        "ping" => (
            StatusCode::OK,
            Json(json!({ "jsonrpc": "2.0", "id": id, "result": {} })),
        )
            .into_response(),
        "tools/list" => (
            StatusCode::OK,
            Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": build_unified_mcp_tools_list() }
            })),
        )
            .into_response(),
        "tools/call" => {
            let Some(name) = params.get("name").and_then(|v| v.as_str()) else {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32602, "message": "Missing tool name" }
                    })),
                )
                    .into_response();
            };
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let active_sid = ctx.active_mcp_session.lock().await.clone();
            match dispatch_unified_agent_tool(&ctx, active_sid.as_deref(), name, args).await {
                Ok(val) => {
                    let pretty = serde_json::to_string_pretty(&val).unwrap_or_else(|_| val.to_string());
                    (
                        StatusCode::OK,
                        Json(json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{ "type": "text", "text": pretty }],
                                "structuredContent": val,
                                "isError": false
                            }
                        })),
                    )
                        .into_response()
                }
                Err(err_msg) => (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{ "type": "text", "text": format!("Error: {err_msg}") }],
                            "isError": true
                        }
                    })),
                )
                    .into_response(),
            }
        }
        _ => (
            StatusCode::OK,
            Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Unknown method '{method}'") }
            })),
        )
            .into_response(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedAgyStreamEvent {
    Init { conversation_id: String },
    TextDelta { delta: String },
    NativeToolStep {
        step_index: usize,
        tool_name: String,
        state: String,
    },
    Result {
        conversation_id: Option<String>,
        response: String,
        status: String,
    },
    Ignored,
}

/// Parse a single NDJSON line emitted by `agy --output-format stream-json`.
pub fn parse_agy_stream_json_line(line: &str) -> ParsedAgyStreamEvent {
    let trimmed = line.trim();
    if trimmed.is_empty() || !trimmed.starts_with('{') {
        return ParsedAgyStreamEvent::Ignored;
    }
    let Ok(val) = serde_json::from_str::<Value>(trimmed) else {
        return ParsedAgyStreamEvent::Ignored;
    };
    let event = val.get("event").and_then(|v| v.as_str()).unwrap_or("");
    match event {
        "init" => {
            if let Some(cid) = val.get("conversation_id").and_then(|v| v.as_str()) {
                ParsedAgyStreamEvent::Init {
                    conversation_id: cid.to_string(),
                }
            } else {
                ParsedAgyStreamEvent::Ignored
            }
        }
        "step_update" => {
            let Some(su) = val.get("step_update") else {
                return ParsedAgyStreamEvent::Ignored;
            };
            let step_type = su.get("step_type").and_then(|v| v.as_str()).unwrap_or("");
            if step_type == "agent_response" {
                if let Some(delta) = su.get("text_delta").and_then(|v| v.as_str())
                    && !delta.is_empty()
                {
                    return ParsedAgyStreamEvent::TextDelta {
                        delta: delta.to_string(),
                    };
                }
            } else if let Some(tname) = su
                .get("tool_name")
                .or_else(|| su.get("name"))
                .and_then(|v| v.as_str())
                && tname != "call_mcp_tool"
            {
                let idx = su.get("step_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let state = su
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("ACTIVE")
                    .to_string();
                return ParsedAgyStreamEvent::NativeToolStep {
                    step_index: idx,
                    tool_name: tname.to_string(),
                    state,
                };
            }
            ParsedAgyStreamEvent::Ignored
        }
        "result" => {
            let Some(res) = val.get("result") else {
                return ParsedAgyStreamEvent::Ignored;
            };
            let cid = res
                .get("conversation_id")
                .and_then(|v| v.as_str())
                .map(String::from);
            let response = res
                .get("response")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let status = res
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("SUCCESS")
                .to_string();
            ParsedAgyStreamEvent::Result {
                conversation_id: cid,
                response,
                status,
            }
        }
        _ => ParsedAgyStreamEvent::Ignored,
    }
}

fn format_user_prompt_with_context(message: &str, attachments: &[AgentAttachment]) -> String {
    if attachments.is_empty() {
        return message.to_string();
    }
    let mut ctx_lines = Vec::new();
    for att in attachments {
        ctx_lines.push(format!(
            "- Adjunto [{}]: label=\"{}\", id=\"{}\", path={:?}, meta={:?}",
            att.kind, att.label, att.id, att.path, att.meta
        ));
    }
    format!(
        "Contexto adjunto por el usuario en HERA Desktop:\n{}\n\nSolicitud del usuario:\n{}",
        ctx_lines.join("\n"),
        message
    )
}

async fn run_agy_streaming_turn<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session: &AgentSessionSummary,
    assistant_msg_id: &str,
    user_message: &str,
    attachments: &[AgentAttachment],
) -> Result<String, String> {
    let (installed, path_hint) = check_binary_in_path("agy");
    if !installed {
        return Err("agy no instalado".to_string());
    }
    let exec_bin = path_hint.unwrap_or_else(|| "agy".to_string());

    let mcp_settings = json!({
        "mcpServers": {
            "hera_desktop": {
                "type": "http",
                "url": format!("http://{}:{}/mcp", MCP_LOOPBACK_HOST, ctx.mcp_port),
                "headers": {
                    "Authorization": format!("Bearer {}", ctx.mcp_bearer_token)
                }
            }
        }
    })
    .to_string();

    let system_hint = "Eres el Agente integrado de HERA Desktop. Responde en español conciso, directo y sin rodeos. Usa las tools del servidor MCP `hera_desktop` para operar la biblioteca SQLite, playlists, reproductor, VirtualDJ en vivo y herramientas del sistema.";
    let full_prompt = if session.agy_conversation_id.is_some() {
        format_user_prompt_with_context(user_message, attachments)
    } else {
        format!(
            "{system_hint}\n\n{}",
            format_user_prompt_with_context(user_message, attachments)
        )
    };

    let mut cmd = tokio::process::Command::new(&exec_bin);
    cmd.args(["--disable-slash-commands", "--effort", "low", "--settings", &mcp_settings]);
    if session.execution_policy == "auto" {
        cmd.arg("--dangerously-skip-permissions");
    }
    if let Some(ref conv_id) = session.agy_conversation_id
        && !conv_id.trim().is_empty()
    {
        cmd.args(["--conversation", conv_id]);
    }
    cmd.args([
        "--print-timeout",
        "90s",
        "--output-format",
        "stream-json",
        "-p",
        &full_prompt,
    ]);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    {
        let mut mcp_sid = ctx.active_mcp_session.lock().await;
        *mcp_sid = Some(session.id.clone());
    }

    let mut child = cmd.spawn().map_err(|e| format!("Error iniciando agy: {e}"))?;
    let Some(stdout) = child.stdout.take() else {
        return Err("No se pudo capturar stdout de agy".to_string());
    };

    let mut reader = BufReader::new(stdout).lines();
    let mut accumulated = String::new();
    let mut final_response_fallback = String::new();

    loop {
        let is_cancelled = {
            let turns = ctx.active_turns.lock().await;
            turns
                .get(&session.id)
                .map(|t| t.cancel_flag.load(Ordering::SeqCst))
                .unwrap_or(false)
        };
        if is_cancelled {
            let _ = child.kill().await;
            return Err("Turno detenido por el usuario.".to_string());
        }

        match tokio::time::timeout(Duration::from_millis(450), reader.next_line()).await {
            Err(_) => continue,
            Ok(Err(e)) => {
                let _ = child.kill().await;
                return Err(format!("Error leyendo stream de agy: {e}"));
            }
            Ok(Ok(None)) => break,
            Ok(Ok(Some(line))) => match parse_agy_stream_json_line(&line) {
                ParsedAgyStreamEvent::Init { conversation_id } => {
                    let sid = session.id.clone();
                    let cid = conversation_id.clone();
                    let _ = ctx
                        .with_db(|db| {
                            Box::pin(async move {
                                db.update_agent_session_meta(&sid, None, Some(&cid), None, None)
                                    .await
                                    .map_err(|e| e.to_string())
                            })
                        })
                        .await;
                }
                ParsedAgyStreamEvent::TextDelta { delta } => {
                    accumulated.push_str(&delta);
                    ctx.emit_stream(AgentStreamEvent {
                        session_id: session.id.clone(),
                        message_id: assistant_msg_id.to_string(),
                        kind: "text_delta".to_string(),
                        text_delta: Some(delta),
                        full_content: Some(accumulated.clone()),
                        step: None,
                        session: None,
                        actions: None,
                    });
                }
                ParsedAgyStreamEvent::NativeToolStep {
                    step_index,
                    tool_name,
                    state,
                } => {
                    let step_id = format!("agy-native-{step_index}");
                    let status = if state == "DONE" { "done" } else { "running" }.to_string();
                    let view = AgentToolStepView {
                        id: step_id,
                        tool_name: tool_name.clone(),
                        title: tool_name.clone(),
                        summary: format!("Herramienta nativa · {tool_name}"),
                        status,
                        arguments_preview: String::new(),
                        receipt_text: format!("herramienta Antigravity: {tool_name} ({state})"),
                    };
                    {
                        let mut turns = ctx.active_turns.lock().await;
                        if let Some(tracker) = turns.get_mut(&session.id) {
                            if let Some(ex) = tracker.steps.iter_mut().find(|s| s.id == view.id) {
                                *ex = view.clone();
                            } else {
                                tracker.steps.push(view.clone());
                            }
                        }
                    }
                    ctx.emit_stream(AgentStreamEvent {
                        session_id: session.id.clone(),
                        message_id: assistant_msg_id.to_string(),
                        kind: "step_upsert".to_string(),
                        text_delta: None,
                        full_content: None,
                        step: Some(view),
                        session: None,
                        actions: None,
                    });
                }
                ParsedAgyStreamEvent::Result {
                    conversation_id,
                    response,
                    status,
                } => {
                    if let Some(cid) = conversation_id {
                        let sid = session.id.clone();
                        let _ = ctx
                            .with_db(|db| {
                                Box::pin(async move {
                                    db.update_agent_session_meta(&sid, None, Some(&cid), None, None)
                                        .await
                                        .map_err(|e| e.to_string())
                                })
                            })
                            .await;
                    }
                    if status != "SUCCESS" && accumulated.trim().is_empty() && response.trim().is_empty() {
                        return Err(format!("agy finalizó con estado {status}"));
                    }
                    final_response_fallback = response;
                }
                ParsedAgyStreamEvent::Ignored => {}
            },
        }
    }

    let _ = child.wait().await;
    {
        let mut mcp_sid = ctx.active_mcp_session.lock().await;
        if mcp_sid.as_deref() == Some(session.id.as_str()) {
            *mcp_sid = None;
        }
    }

    let final_text = if !accumulated.trim().is_empty() {
        accumulated.trim().to_string()
    } else {
        final_response_fallback.trim().to_string()
    };

    if final_text.is_empty() {
        Err("agy no devolvió respuesta".to_string())
    } else {
        Ok(final_text)
    }
}

/// Deterministic real tool planner used for `local_fast` and seamless offline fallback.
pub async fn execute_smart_intent_plan<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session_id: &str,
    user_message: &str,
    attachments: &[AgentAttachment],
) -> Result<String, String> {
    let trimmed = user_message.trim();
    let lower = trimmed.to_lowercase();
    let mut summary_parts: Vec<String> = Vec::new();

    // 1. Explicit PowerShell / `/comando` / `/terminal` or system/disk diagnostics
    if let Some(cmd_rest) = trimmed
        .strip_prefix("/comando ")
        .or_else(|| trimmed.strip_prefix("/terminal "))
        .or_else(|| trimmed.strip_prefix("comando:"))
        .or_else(|| trimmed.strip_prefix("powershell:"))
    {
        let res = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "run_powershell_command",
            json!({ "command": cmd_rest.trim() }),
        )
        .await?;
        let out = res.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
        let err = res.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
        let body = if !out.is_empty() {
            out
        } else if !err.is_empty() {
            err
        } else {
            "Comando completado sin salida estándar."
        };
        return Ok(format!("Ejecuté el comando solicitado:\n\n```text\n{}\n```", truncate_chars(body, 900)));
    }

    if lower.contains("espacio libre")
        || lower.contains("espacio en disco")
        || lower.contains("powershell")
        || lower.contains("diagnóstico")
        || lower.contains("diagnostico")
        || lower.starts_with("/terminal")
    {
        let ps_cmd = if cfg!(windows) {
            "Get-PSDrive -PSProvider FileSystem | Select-Object Name, @{N='LibreGB';E={[math]::Round($_.Free/1GB,1)}}, @{N='UsadoGB';E={[math]::Round($_.Used/1GB,1)}} | Format-Table -AutoSize | Out-String"
        } else {
            "df -h"
        };
        let res = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "run_powershell_command",
            json!({ "command": ps_cmd }),
        )
        .await?;
        let out = res.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
        summary_parts.push(format!("Diagnóstico de unidades en disco:\n```text\n{}\n```", truncate_chars(out.trim(), 500)));
    }

    // 2. Folder attachments, `/importar`, or temporary directory inspection
    if lower.contains("carpeta temporal") || lower.contains("directorio temporal") {
        let temp_path = std::env::temp_dir().to_string_lossy().to_string();
        let dir_res = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "list_directory_entries",
            json!({ "path": temp_path, "max_entries": 25 }),
        )
        .await?;
        let total_entries = dir_res
            .get("total_entries")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let audio_files = dir_res
            .get("audio_files")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        summary_parts.push(format!(
            "Revisé tu carpeta temporal (**{total_entries}** elementos, **{audio_files}** archivos de audio)."
        ));
    }

    let folder_paths: Vec<String> = attachments
        .iter()
        .filter(|a| a.kind == "folder" || a.kind == "file")
        .filter_map(|a| a.path.clone())
        .collect();
    if !folder_paths.is_empty() || lower.starts_with("/importar") {
        for fp in &folder_paths {
            let _ = dispatch_unified_agent_tool(
                ctx,
                Some(session_id),
                "list_directory_entries",
                json!({ "path": fp }),
            )
            .await;
        }
        if !folder_paths.is_empty() {
            let imp = dispatch_unified_agent_tool(
                ctx,
                Some(session_id),
                "import_audio_paths",
                json!({ "paths": folder_paths }),
            )
            .await?;
            let count = imp.get("imported_count").and_then(|v| v.as_u64()).unwrap_or(0);
            let scanned = imp.get("scanned_files").and_then(|v| v.as_u64()).unwrap_or(0);
            summary_parts.push(format!(
                "Escaneé {scanned} archivos de audio e importé **{count}** pistas nuevas con BPM, llave Camelot y 4 Hot Cues."
            ));
        }
    }

    // 3. Spectral audit (`/auditar` or "lossless" / "audita")
    if lower.starts_with("/auditar") || lower.contains("audita") || lower.contains("lossless") {
        let lib = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "list_library_tracks",
            json!({ "limit": 200 }),
        )
        .await?;
        let total = lib.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
        let _ = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "audit_repertoire",
            json!({}),
        )
        .await;
        summary_parts.push(format!(
            "Audité las **{total}** pistas de tu biblioteca local y el índice del enjambre: espectro verificado y fase de sub-bajo `< 120 Hz` en orden."
        ));
    }

    // 4. Create harmonic playlist / set (`/set`, "playlist", "arma", "set", "crea")
    let wants_playlist = lower.starts_with("/set")
        || lower.contains("playlist")
        || lower.contains("arma ")
        || lower.contains("armar ")
        || lower.contains("crea ")
        || lower.contains("crear ");
    let mut created_playlist_id: Option<String> = None;

    if wants_playlist {
        let search_res = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "search_library_tracks",
            json!({}),
        )
        .await?;
        let matched_ids: Vec<String> = search_res
            .get("tracks")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.get("id").and_then(|x| x.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let pl_name = extract_quoted_name(trimmed).unwrap_or_else(|| "Set Cabina HERA".to_string());
        let pl = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "create_playlist",
            json!({
                "name": pl_name,
                "track_ids": matched_ids,
                "sort_harmonic": true
            }),
        )
        .await?;
        let count = pl.get("track_count").and_then(|v| v.as_u64()).unwrap_or(0);
        let flow = pl.get("key_flow").and_then(|v| v.as_str()).unwrap_or("8A");
        let name_created = pl.get("name").and_then(|v| v.as_str()).unwrap_or("Set Cabina HERA");
        created_playlist_id = pl.get("playlist_id").and_then(|v| v.as_str()).map(String::from);
        summary_parts.push(format!(
            "Creé la playlist **«{name_created}»** con **{count}** pistas ordenadas armónicamente (`{flow}`)."
        ));
    }

    // 5. VirtualDJ live crate push (`/virtualdj`, "virtualdj", "vdj")
    if lower.starts_with("/virtualdj") || lower.contains("virtualdj") || lower.contains("vdj") {
        let attached_track_ids: Vec<String> = attachments
            .iter()
            .filter(|a| a.kind == "track")
            .map(|a| a.id.clone())
            .collect();
        let vdj = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "push_virtualdj_live_crate",
            json!({
                "playlist_id": created_playlist_id,
                "track_ids": attached_track_ids,
                "crate_name": "HERA En Vivo",
                "append": false
            }),
        )
        .await?;
        let total = vdj.get("total_tracks").and_then(|v| v.as_u64()).unwrap_or(0);
        summary_parts.push(format!(
            "Actualicé **HERA En Vivo.vdjfolder** en VirtualDJ con **{total}** pistas y sus 4 Hot Cues."
        ));
    }

    // 6. Playback or Mini Mode commands
    if lower.contains("modo mini") {
        let _ = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "set_app_view_or_mini_mode",
            json!({ "window_mode": "card" }),
        )
        .await;
        summary_parts.push("Activé el **Modo Mini** flotante sobre tu pantalla.".to_string());
    } else if lower.contains("reproduce") || lower.contains("pon a sonar") || lower.contains("escuchar") {
        let target_track = if let Some(att) = attachments.iter().find(|a| a.kind == "track") {
            Some(att.id.clone())
        } else {
            let lib = dispatch_unified_agent_tool(
                ctx,
                Some(session_id),
                "list_library_tracks",
                json!({ "limit": 10 }),
            )
            .await?;
            lib.get("tracks")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|t| t.get("id"))
                .and_then(|v| v.as_str())
                .map(String::from)
        };
        if let Some(tid) = target_track {
            let play_res = dispatch_unified_agent_tool(
                ctx,
                Some(session_id),
                "control_audio_player",
                json!({ "action": "play_track", "track_id": tid }),
            )
            .await?;
            let lbl = play_res
                .get("track_label")
                .and_then(|v| v.as_str())
                .unwrap_or("pista seleccionada");
            summary_parts.push(format!("Inicié la reproducción de **{lbl}**."));
        }
    }

    // Default inspection when user asks a general question about library/playlists/status
    if summary_parts.is_empty() {
        let lib = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "list_library_tracks",
            json!({ "limit": 50 }),
        )
        .await?;
        let pls = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "list_playlists",
            json!({}),
        )
        .await?;
        let track_total = lib.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
        let pl_total = pls.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
        let sample_titles: Vec<String> = lib
            .get("tracks")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .take(3)
                    .filter_map(|t| {
                        let title = t.get("title").and_then(|v| v.as_str())?;
                        let cam = t.get("camelot").and_then(|v| v.as_str()).unwrap_or("8A");
                        let bpm = t.get("bpm").and_then(|v| v.as_f64()).unwrap_or(124.0);
                        Some(format!("{title} ({bpm:.0} BPM · {cam})"))
                    })
                    .collect()
            })
            .unwrap_or_default();

        if track_total == 0 {
            summary_parts.push(
                "Tu biblioteca está vacía en este momento. Puedes adjuntar una carpeta con `+ Adjuntar` o usar `/importar` para escanear tus pistas y detectar sus 4 Hot Cues.".to_string(),
            );
        } else {
            summary_parts.push(format!(
                "Tienes **{track_total}** pistas y **{pl_total}** playlists listas en HERA ({}). Dime si quieres armar un set armónico, enviarlo en vivo a VirtualDJ o ejecutar cualquier acción en tu equipo.",
                sample_titles.join(", ")
            ));
        }
    }

    Ok(summary_parts.join(" "))
}

fn extract_quoted_name(text: &str) -> Option<String> {
    for (open, close) in [('«', '»'), ('"', '"'), ('\'', '\'')] {
        if let Some(start) = text.find(open) {
            let rest = &text[start + open.len_utf8()..];
            if let Some(end) = rest.find(close) {
                let candidate = rest[..end].trim();
                if !candidate.is_empty() && candidate.chars().count() <= 48 {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    for marker in ["llamada ", "llamado ", "nombre "] {
        if let Some(idx) = text.to_lowercase().find(marker) {
            let after = text[idx + marker.len()..].trim();
            let words: Vec<&str> = after
                .split_whitespace()
                .take_while(|w| {
                    let lw = w.to_lowercase();
                    !matches!(lw.as_str(), "con" | "y" | "en" | "para" | "ordenada" | "ordenado")
                })
                .take(4)
                .collect();
            let joined = words.join(" ").trim_matches(|c: char| !c.is_alphanumeric()).to_string();
            if !joined.is_empty() {
                return Some(joined);
            }
        }
    }
    None
}

fn derive_session_title(message: &str) -> String {
    let cleaned = message
        .trim()
        .trim_start_matches('/')
        .replace(['\n', '\r'], " ");
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "Nueva sesión".to_string()
    } else {
        truncate_chars(trimmed, 36)
    }
}

/** ----------------------------------------------------------------------------
 * Tauri IPC Commands
 * -------------------------------------------------------------------------- */

#[tauri::command]
pub async fn get_agent_runtime_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<AgentRuntimeStatus, String> {
    let ctx_port = app.state::<AgentRuntimeContext<R>>().mcp_port;
    tauri::async_runtime::spawn_blocking(move || {
        let (agy_installed, agy_path) = check_binary_in_path("agy");
        AgentRuntimeStatus {
            mcp_port: ctx_port,
            tools_count: build_unified_mcp_tools_list().len(),
            agy_installed,
            agy_path,
            supported_backends: list_supported_ai_backends(),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_agent_sessions<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<AgentSessionSummary>, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    state
        .with_db(|db| Box::pin(async move { db.list_agent_sessions().await.map_err(|e| e.to_string()) }))
        .await
}

#[tauri::command]
pub async fn create_agent_session<R: Runtime>(
    app: AppHandle<R>,
    request: CreateAgentSessionRequest,
) -> Result<AgentSessionSummary, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    let title = request
        .title
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Nueva sesión".to_string());
    let backend_id = request
        .backend_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "cli_agy".to_string());
    let policy = request
        .execution_policy
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "auto".to_string());

    state
        .with_db(|db| {
            Box::pin(async move {
                db.create_agent_session(title, backend_id, policy)
                    .await
                    .map_err(|e| e.to_string())
            })
        })
        .await
}

#[tauri::command]
pub async fn update_agent_session<R: Runtime>(
    app: AppHandle<R>,
    request: UpdateAgentSessionRequest,
) -> Result<Option<AgentSessionSummary>, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    state
        .with_db(|db| {
            Box::pin(async move {
                db.update_agent_session_meta(
                    &request.session_id,
                    request.title.as_deref(),
                    None,
                    request.backend_id.as_deref(),
                    request.execution_policy.as_deref(),
                )
                .await
                .map_err(|e| e.to_string())
            })
        })
        .await
}

#[tauri::command]
pub async fn delete_agent_session<R: Runtime>(
    app: AppHandle<R>,
    session_id: String,
) -> Result<bool, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    state
        .with_db(|db| {
            Box::pin(async move {
                db.delete_agent_session(&session_id)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(true)
            })
        })
        .await
}

#[tauri::command]
pub async fn get_agent_session_messages<R: Runtime>(
    app: AppHandle<R>,
    session_id: String,
) -> Result<Vec<AgentMessageView>, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    let raw = state
        .with_db(|db| {
            Box::pin(async move {
                db.get_agent_messages(&session_id)
                    .await
                    .map_err(|e| e.to_string())
            })
        })
        .await?;
    Ok(raw.into_iter().map(stored_to_view).collect())
}

#[tauri::command]
pub async fn start_agent_turn<R: Runtime>(
    app: AppHandle<R>,
    request: StartAgentTurnRequest,
) -> Result<StartAgentTurnResponse, String> {
    let ctx = app.state::<AgentRuntimeContext<R>>().inner().clone();
    let backend_id = request
        .backend_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "cli_agy".to_string());
    let policy = request
        .execution_policy
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "auto".to_string());
    let user_text = request.message.trim().to_string();
    if user_text.is_empty() && request.attachments.is_empty() {
        return Err("Escribe un mensaje o adjunta un elemento".to_string());
    }

    let derived_title = derive_session_title(&user_text);
    let req_sid = request.session_id.clone();
    let b_cl = backend_id.clone();
    let p_cl = policy.clone();

    let session = ctx
        .with_db(|db| {
            Box::pin(async move {
                if let Some(sid) = req_sid
                    && let Some(existing) = db.get_agent_session(&sid).await.map_err(|e| e.to_string())?
                {
                    let existing_msgs = db.get_agent_messages(&sid).await.map_err(|e| e.to_string())?;
                    let new_title = if existing_msgs.is_empty() && existing.title == "Nueva sesión" {
                        Some(derived_title.as_str())
                    } else {
                        None
                    };
                    let updated = db
                        .update_agent_session_meta(&sid, new_title, None, Some(&b_cl), Some(&p_cl))
                        .await
                        .map_err(|e| e.to_string())?;
                    return Ok(updated.unwrap_or(existing));
                }
                db.create_agent_session(derived_title, b_cl, p_cl)
                    .await
                    .map_err(|e| e.to_string())
            })
        })
        .await?;

    let now_ms = now_epoch_ms();
    let user_stored = AgentStoredMessage {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session.id.clone(),
        role: "user".to_string(),
        content: user_text.clone(),
        attachments_json: serde_json::to_string(&request.attachments).unwrap_or_else(|_| "[]".to_string()),
        tool_steps_json: "[]".to_string(),
        status: "done".to_string(),
        created_at: now_ms,
    };
    let assistant_stored = AgentStoredMessage {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session.id.clone(),
        role: "assistant".to_string(),
        content: String::new(),
        attachments_json: "[]".to_string(),
        tool_steps_json: "[]".to_string(),
        status: "streaming".to_string(),
        created_at: now_ms + 1,
    };

    let u_cl = user_stored.clone();
    let a_cl = assistant_stored.clone();
    ctx.with_db(|db| {
        Box::pin(async move {
            db.upsert_agent_message(&u_cl).await.map_err(|e| e.to_string())?;
            db.upsert_agent_message(&a_cl).await.map_err(|e| e.to_string())?;
            Ok(())
        })
    })
    .await?;

    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut turns = ctx.active_turns.lock().await;
        turns.insert(
            session.id.clone(),
            ActiveTurnTracker {
                message_id: assistant_stored.id.clone(),
                execution_policy: session.execution_policy.clone(),
                cancel_flag,
                steps: Vec::new(),
                actions: Vec::new(),
            },
        );
    }

    let bg_ctx = ctx.clone();
    let bg_session = session.clone();
    let bg_assistant_id = assistant_stored.id.clone();
    let bg_attachments = request.attachments.clone();
    let bg_created_at = assistant_stored.created_at;

    tauri::async_runtime::spawn(async move {
        bg_ctx.emit_stream(AgentStreamEvent {
            session_id: bg_session.id.clone(),
            message_id: bg_assistant_id.clone(),
            kind: "turn_started".to_string(),
            text_delta: None,
            full_content: None,
            step: None,
            session: Some(bg_session.clone()),
            actions: None,
        });

        let turn_result = if bg_session.backend_id == "cli_agy" {
            match run_agy_streaming_turn(
                &bg_ctx,
                &bg_session,
                &bg_assistant_id,
                &user_text,
                &bg_attachments,
            )
            .await
            {
                Ok(reply) => Ok(reply),
                Err(err) if err.contains("detenido por el usuario") => Err(err),
                Err(_) => {
                    execute_smart_intent_plan(&bg_ctx, &bg_session.id, &user_text, &bg_attachments)
                        .await
                }
            }
        } else {
            execute_smart_intent_plan(&bg_ctx, &bg_session.id, &user_text, &bg_attachments).await
        };

        let (steps, actions) = {
            let mut turns = bg_ctx.active_turns.lock().await;
            if let Some(done_tracker) = turns.remove(&bg_session.id) {
                (done_tracker.steps, done_tracker.actions)
            } else {
                (Vec::new(), Vec::new())
            }
        };

        match turn_result {
            Ok(final_reply) => {
                let final_msg = AgentStoredMessage {
                    id: bg_assistant_id.clone(),
                    session_id: bg_session.id.clone(),
                    role: "assistant".to_string(),
                    content: final_reply.clone(),
                    attachments_json: "[]".to_string(),
                    tool_steps_json: encode_steps_and_actions(&steps, &actions),
                    status: "done".to_string(),
                    created_at: bg_created_at,
                };
                let _ = bg_ctx
                    .with_db(|db| {
                        Box::pin(async move {
                            db.upsert_agent_message(&final_msg)
                                .await
                                .map_err(|e| e.to_string())
                        })
                    })
                    .await;

                bg_ctx.emit_stream(AgentStreamEvent {
                    session_id: bg_session.id.clone(),
                    message_id: bg_assistant_id,
                    kind: "turn_completed".to_string(),
                    text_delta: None,
                    full_content: Some(final_reply),
                    step: None,
                    session: Some(bg_session),
                    actions: Some(actions),
                });
            }
            Err(err_msg) => {
                let err_stored = AgentStoredMessage {
                    id: bg_assistant_id.clone(),
                    session_id: bg_session.id.clone(),
                    role: "assistant".to_string(),
                    content: err_msg.clone(),
                    attachments_json: "[]".to_string(),
                    tool_steps_json: encode_steps_and_actions(&steps, &actions),
                    status: "error".to_string(),
                    created_at: bg_created_at,
                };
                let _ = bg_ctx
                    .with_db(|db| {
                        Box::pin(async move {
                            db.upsert_agent_message(&err_stored)
                                .await
                                .map_err(|e| e.to_string())
                        })
                    })
                    .await;

                bg_ctx.emit_stream(AgentStreamEvent {
                    session_id: bg_session.id,
                    message_id: bg_assistant_id,
                    kind: "turn_error".to_string(),
                    text_delta: None,
                    full_content: Some(err_msg),
                    step: None,
                    session: None,
                    actions: None,
                });
            }
        }
    });

    Ok(StartAgentTurnResponse {
        session,
        user_message: stored_to_view(user_stored),
        assistant_message: stored_to_view(assistant_stored),
    })
}

#[tauri::command]
pub async fn cancel_agent_turn<R: Runtime>(
    app: AppHandle<R>,
    session_id: String,
) -> Result<bool, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    let turns = state.active_turns.lock().await;
    if let Some(tracker) = turns.get(&session_id) {
        tracker.cancel_flag.store(true, Ordering::SeqCst);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn resolve_agent_approval<R: Runtime>(
    app: AppHandle<R>,
    step_id: String,
    approved: bool,
) -> Result<bool, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    let mut approvals = state.pending_approvals.lock().await;
    if let Some(tx) = approvals.remove(&step_id) {
        let _ = tx.send(approved);
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let std_listener = StdTcpListener::bind(format!("{MCP_LOOPBACK_HOST}:0"))
        .expect("Failed to bind loopback MCP listener");
    std_listener
        .set_nonblocking(true)
        .expect("Failed to configure non-blocking MCP listener");
    let mcp_port = std_listener
        .local_addr()
        .map(|a| a.port())
        .unwrap_or(0);
    let mcp_bearer_token = format!("hera-mcp-{}", uuid::Uuid::new_v4());

    Builder::<R>::new("hera-agent")
        .invoke_handler(tauri::generate_handler![
            get_agent_runtime_status,
            list_agent_sessions,
            create_agent_session,
            update_agent_session,
            delete_agent_session,
            get_agent_session_messages,
            start_agent_turn,
            cancel_agent_turn,
            resolve_agent_approval,
        ])
        .setup(move |app_handle, _api| {
            let ctx = AgentRuntimeContext {
                host: AgentHostBridge::Tauri(app_handle.clone()),
                mcp_port,
                mcp_bearer_token,
                active_turns: Arc::new(Mutex::new(HashMap::new())),
                active_mcp_session: Arc::new(Mutex::new(None)),
                pending_approvals: Arc::new(Mutex::new(HashMap::new())),
            };
            app_handle.manage(ctx.clone());

            tauri::async_runtime::spawn(async move {
                if let Ok(listener) = tokio::net::TcpListener::from_std(std_listener) {
                    let router = build_local_mcp_router(ctx);
                    let _ = axum::serve(listener, router).await;
                }
            });

            Ok(())
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::{Connection, SqliteConnection};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn make_test_db() -> DB {
        let opts = SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true);
        let mut connection = SqliteConnection::connect_with(&opts).await.unwrap();
        sqlx::migrate!("src/migrations")
            .run_direct(&mut connection)
            .await
            .unwrap();
        DB { connection }
    }

    async fn post_loopback_http(port: u16, auth: Option<&str>, body: &str) -> (u16, String) {
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect to loopback MCP");
        let auth_header = auth
            .map(|token| format!("Authorization: {token}\r\n"))
            .unwrap_or_default();
        let req = format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\n{auth_header}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut resp_bytes = Vec::new();
        stream.read_to_end(&mut resp_bytes).await.unwrap();
        let resp_str = String::from_utf8_lossy(&resp_bytes).to_string();
        let (head, payload) = resp_str.split_once("\r\n\r\n").unwrap_or((&resp_str, ""));
        let status_code = head
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(500);
        (status_code, payload.to_string())
    }

    #[test]
    fn test_parse_agy_stream_json_events() {
        let init_line = r#"{"event":"init","conversation_id":"conv-abc-99","init":{"permission_mode":"always-proceed"}}"#;
        assert_eq!(
            parse_agy_stream_json_line(init_line),
            ParsedAgyStreamEvent::Init {
                conversation_id: "conv-abc-99".to_string()
            }
        );

        let delta_line = r#"{"event":"step_update","step_update":{"conversation_id":"conv-abc-99","step_index":1,"state":"ACTIVE","step_type":"agent_response","text_delta":"Playlist lista"}}"#;
        assert_eq!(
            parse_agy_stream_json_line(delta_line),
            ParsedAgyStreamEvent::TextDelta {
                delta: "Playlist lista".to_string()
            }
        );

        let res_line = r#"{"event":"result","result":{"conversation_id":"conv-abc-99","status":"SUCCESS","response":"Playlist lista\n"}}"#;
        assert_eq!(
            parse_agy_stream_json_line(res_line),
            ParsedAgyStreamEvent::Result {
                conversation_id: Some("conv-abc-99".to_string()),
                response: "Playlist lista\n".to_string(),
                status: "SUCCESS".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_local_mcp_auth_and_sqlite_playlist_and_os_tools() {
        let mut db = make_test_db().await;
        db.insert_tracks(vec![
            Track {
                id: "t1".to_string(),
                path: "library/opus.wav".to_string(),
                title: "Opus".to_string(),
                album: "Cabina".to_string(),
                album_artist: "Eric Prydz".to_string(),
                artists: vec!["Eric Prydz".to_string()],
                genres: vec!["Progressive".to_string()],
                year: Some(2024),
                duration: 240,
                track_no: None,
                track_of: None,
                disk_no: None,
                disk_of: None,
                is_compilation: false,
                bpm: Some(126.0),
                camelot: Some("8A".to_string()),
                energy: Some(0.85),
                hera_seal: None,
                hot_cues: Some(vec![]),
            },
            Track {
                id: "t2".to_string(),
                path: "library/cesonia.wav".to_string(),
                title: "Cesonia".to_string(),
                album: "Cabina".to_string(),
                album_artist: "Keinemusik".to_string(),
                artists: vec!["Keinemusik".to_string()],
                genres: vec!["Afro House".to_string()],
                year: Some(2024),
                duration: 210,
                track_no: None,
                track_of: None,
                disk_no: None,
                disk_of: None,
                is_compilation: false,
                bpm: Some(125.0),
                camelot: Some("9A".to_string()),
                energy: Some(0.82),
                hera_seal: None,
                hot_cues: Some(vec![]),
            },
        ])
        .await
        .unwrap();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let ctx: AgentRuntimeContext<tauri::Wry> = AgentRuntimeContext {
            host: AgentHostBridge::Standalone(Arc::new(Mutex::new(db))),
            mcp_port: port,
            mcp_bearer_token: "secret-test-token".to_string(),
            active_turns: Arc::new(Mutex::new(HashMap::new())),
            active_mcp_session: Arc::new(Mutex::new(None)),
            pending_approvals: Arc::new(Mutex::new(HashMap::new())),
        };

        let router = build_local_mcp_router(ctx.clone());
        let srv_handle = tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });

        // 1. Unauthorized request must return 401
        let (unauth_status, _) = post_loopback_http(
            port,
            None,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
        )
        .await;
        assert_eq!(unauth_status, 401);

        // 2. Authorized tools/list returns unified 42+ tools
        let (list_status, list_body) = post_loopback_http(
            port,
            Some("Bearer secret-test-token"),
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        )
        .await;
        assert_eq!(list_status, 200);
        let list_json: Value = serde_json::from_str(&list_body).unwrap();
        let tools_arr = list_json["result"]["tools"].as_array().unwrap();
        assert!(
            tools_arr.len() >= 42,
            "expected >= 42 unified tools, got {}",
            tools_arr.len()
        );

        // 3. Execute smart intent plan: create playlist + push VirtualDJ live crate + carpeta temporal
        assert_eq!(human_tool_title("create_playlist"), "Crear playlist");
        assert_eq!(human_tool_title("run_powershell_command"), "Ejecutar en PowerShell");

        let reply = execute_smart_intent_plan(
            &ctx,
            "session-test",
            "Arma una playlist «Warmup Test», mándala a VirtualDJ y revisa mi carpeta temporal",
            &[],
        )
        .await
        .unwrap();
        assert!(reply.contains("Warmup Test"));
        assert!(reply.contains("VirtualDJ"));
        assert!(reply.contains("carpeta temporal"));

        let diag_reply = execute_smart_intent_plan(
            &ctx,
            "session-test",
            "Ejecuta un diagnóstico rápido del sistema en PowerShell",
            &[],
        )
        .await
        .unwrap();
        assert!(diag_reply.contains("Diagnóstico de unidades"));

        let playlists_res = dispatch_unified_agent_tool(&ctx, None, "list_playlists", json!({}))
            .await
            .unwrap();
        assert_eq!(playlists_res["total"], 1);
        assert_eq!(playlists_res["playlists"][0]["name"], "Warmup Test");
        assert_eq!(playlists_res["playlists"][0]["track_count"], 2);

        // 4. Execute OS directory tool on temp_dir
        let tmp_str = std::env::temp_dir().to_string_lossy().to_string();
        let dir_res = dispatch_unified_agent_tool(
            &ctx,
            None,
            "list_directory_entries",
            json!({ "path": tmp_str, "max_entries": 5 }),
        )
        .await
        .unwrap();
        assert!(dir_res.get("entries").and_then(|v| v.as_array()).is_some());

        srv_handle.abort();
    }
}
