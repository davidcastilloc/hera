//! Unified MCP Tool Schemas for HERA Desktop (22 Desktop/OS tools + `hera-rs` DSP/P2P tools).

use hera_node_rs::mcp::list_mcp_tools as list_core_mcp_tools;
use serde_json::{json, Value};

/// Build the 22 HERA Desktop SQLite, Player, VirtualDJ & OS MCP tool schemas.
pub fn desktop_extra_mcp_tools() -> Vec<Value> {
    vec![
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
            "inputSchema": { "type": "object", "properties": {} }
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
                "properties": { "playlist_id": { "type": "string" }, "name": { "type": "string" } },
                "required": ["playlist_id", "name"]
            }
        }),
        json!({
            "name": "delete_playlist",
            "description": "Delete a playlist by ID from the SQLite database without deleting any audio tracks.",
            "inputSchema": {
                "type": "object",
                "properties": { "playlist_id": { "type": "string" } },
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
            "inputSchema": { "type": "object", "properties": {} }
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
                "properties": { "path": { "type": "string" }, "max_bytes": { "type": "integer", "default": 32768 } },
                "required": ["path"]
            }
        }),
        json!({
            "name": "write_text_file",
            "description": "Write UTF-8 text content to a local file, creating parent directories if needed.",
            "inputSchema": {
                "type": "object",
                "properties": { "path": { "type": "string" }, "content": { "type": "string" } },
                "required": ["path", "content"]
            }
        }),
        json!({
            "name": "move_or_rename_path",
            "description": "Move or rename a local file or directory on disk (e.g. organizing audio files into clean folders).",
            "inputSchema": {
                "type": "object",
                "properties": { "from_path": { "type": "string" }, "to_path": { "type": "string" } },
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
        json!({
            "name": "record_golden_pair",
            "description": "Save a proven DJ transition pair into memory with an acoustic synergy bonus (+0.38 by default) for harmonic crate generation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from_track_id": { "type": "string", "description": "ID of the outgoing track" },
                    "to_track_id": { "type": "string", "description": "ID of the incoming track" },
                    "custom_bonus": { "type": "number", "description": "Optional synergy score bonus (default 0.38)", "default": 0.38 },
                    "note": { "type": "string", "description": "Optional note regarding why this transition works" }
                },
                "required": ["from_track_id", "to_track_id"]
            }
        }),
        json!({
            "name": "exclude_track_curation",
            "description": "Permanently exclude a track or title pattern from future automated AI DJ crate suggestions without deleting the file from disk.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "track_id": { "type": "string", "description": "Optional specific SQLite track ID to exclude" },
                    "title_pattern": { "type": "string", "description": "Optional title/artist substring pattern (e.g. 'Clavelito') to exclude" },
                    "reason": { "type": "string", "description": "Reason for exclusion (e.g. 'tired track', 'scratched intro', 'client request')" }
                }
            }
        }),
        json!({
            "name": "record_dj_feedback",
            "description": "Store a natural language DJ preference or curatorial rule into long-term memory for the AI assistant.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "category": { "type": "string", "description": "Category: 'genre', 'pacing', 'energy', 'venue', or 'general'", "default": "general" },
                    "rule_text": { "type": "string", "description": "The exact DJ rule or preference text" }
                },
                "required": ["rule_text"]
            }
        }),
    ]
}

/// Merge the 22 HERA Desktop + OS tools with `hera-rs` DSP/P2P tools into a unified MCP catalog.
pub fn build_unified_mcp_tools_list() -> Vec<Value> {
    let mut tools = desktop_extra_mcp_tools();
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

/// Return true when an OS-mutating tool requires user confirmation under `"confirm"` policy.
pub fn requires_system_confirmation(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "run_powershell_command" | "write_text_file" | "move_or_rename_path" | "open_system_target"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn test_unified_mcp_tools_catalog_and_confirmation_gate() {
        let extra = desktop_extra_mcp_tools();
        assert_eq!(extra.len(), 25);
        let unified = build_unified_mcp_tools_list();
        assert!(unified.len() >= 42, "got {} tools", unified.len());
        let names: BTreeSet<&str> = unified
            .iter()
            .filter_map(|t| t.get("name").and_then(|v| v.as_str()))
            .collect();
        assert_eq!(names.len(), unified.len());
        assert!(requires_system_confirmation("run_powershell_command"));
        assert!(requires_system_confirmation("write_text_file"));
        assert!(!requires_system_confirmation("create_playlist"));
    }
}
