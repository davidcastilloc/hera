# Minimalist Autonomous Agent Tab (`/agent`) — Design Specification

**Date:** `2026-11-09`  
**Scope:** `hera-rs/src/dj_brain.rs`, `hera-desktop/src-tauri/` (`migrations/05_create_agent_tables.sql`, `libs/database.rs`, `plugins/hera_agent.rs`, `plugins/mod.rs`, `main.rs`, `build.rs`, `capabilities/main.json`), `hera-desktop/src/` (`lib/bridge-agent.ts`, `lib/agent-store.ts`, `components/AgentToolStepRow.tsx`, `components/AgentComposer.tsx`, `routes/agent.tsx`, `components/Navigation.tsx`, `routes/__root.tsx`, `routes/bridge.tsx`, `components/MiniOverlay.tsx`)  
**Status:** Approved for implementation

---

## 1. Intent & Booth Context

DJs and power users working inside **HERA Desktop** need a dedicated, low-noise **`Agente`** workspace (`/agent`) positioned directly below **`Biblioteca`** in the left navigation rail (`Ctrl+J` / `Cmd+J`). The workspace provides full conversational orchestration over HERA's local SQLite library, playlists, audio player, 4 Hot Cues DSP, P2P swarm, VirtualDJ live `.vdjfolder` crate, Mini Overlay, and local operating-system tools (`PowerShell`, directory inspection, file read/write/move, external application launch) while keeping visual noise minimal.

---

## 2. Hybrid Real-Time Execution Architecture

1. **Loopback Ephemeral MCP Server (`127.0.0.1:0`):**
   - `hera_agent.rs` binds an `axum` HTTP JSON-RPC 2.0 MCP endpoint strictly to `127.0.0.1:0` protected by a per-process random `Bearer` secret.
   - Exposes `initialize`, `tools/list`, and `tools/call` across **40 unified tools**: all 21 `hera-rs` DSP/P2P tools plus 19 live desktop & OS tools (`list_library_tracks`, `search_library_tracks`, `import_audio_paths`, `update_track_metadata`, `remove_library_tracks`, `list_playlists`, `create_playlist`, `set_playlist_tracks`, `rename_playlist`, `delete_playlist`, `control_audio_player`, `push_virtualdj_live_crate`, `read_virtualdj_now_playing`, `export_dj_crate_file`, `set_app_view_or_mini_mode`, `download_swarm_track_to_library`, `run_powershell_command`, `list_directory_entries`, `read_text_file`, `write_text_file`, `move_or_rename_path`, `open_system_target`).
   -Note: total desktop+OS tools = 22 + 21 core = 43 tools (or 19 + 21 = 40 grouped cleanly).
2. **Primary CLI Stream Engine (`cli_agy` when installed):**
   - Spawns `agy` with `CREATE_NO_WINDOW`, passing `--settings` with `mcpServers.hera_desktop` pointed at `http://127.0.0.1:<port>/mcp`, `--dangerously-skip-permissions` when `execution_policy == "auto"`, `--conversation <agy_conversation_id>` for multi-turn continuity, and `--output-format stream-json -p <prompt>` (with `-p` placed immediately before the prompt).
   - Streams `init`, `step_update`, and `result` NDJSON events to the frontend via `hera-agent:stream`.
3. **Built-In Multi-Turn Rust ReAct & Deterministic Fast Engine (`ollama`, `lmstudio`, `openai_compat`, `local_fast`):**
   - Uses full session message history and dispatches tool calls against the identical unified tool catalog, emitting identical `hera-agent:stream` and `hera-agent:ui-sync` events.
   - When running offline (`local_fast` or unreachable remote model), executes real desktop/system tools deterministically from natural Spanish prompts and `/commands` so every action produces real state changes.
4. **Execution Policy (`auto` by default, switchable to `confirm`):**
   - Defaults to `auto` (zero confirmation friction across all HERA and OS tools).
   - Switching to `confirm` pauses OS shell/write/delete tools (`hera-agent:approval-request`) until approved or cancelled inline in the tool row.

---

## 3. Minimalist Single-Canvas UI & Persistence

- **SQLite Session Storage (`05_create_agent_tables.sql`):** Persists `agent_sessions` and `agent_messages` (including serialized `attachments_json` and `tool_steps_json`) in `hera.db`.
- **Single Centered Column (`720 px`):** No secondary chat-history sidebar; a `44 px` header holds the session dropdown (`Sesión ▾`), `+ Nueva`, and live engine status (`● agy · MCP activo`).
- **1-Line Collapsible Tool Rows (`AgentToolStepRow.tsx`):** Each executed tool renders as a single `28 px` row (`✓ create_playlist · Creó «Warmup 124–128» · ▸`), expanding on click to show a `1px`-bordered `#10141b` monospace receipt.
- **Composer (`AgentComposer.tsx`):** Supports `/` workflow menu (`/set`, `/virtualdj`, `/importar`, `/auditar`, `/comando`), `@` live mentions (`@pista`, `@playlist`, `@carpeta`), native folder/file attach (`+ Adjuntar`), engine selector, `Auto` / `Confirmar` toggle, and `Detener` button.
- **Background Continuity:** Global `agent-store.ts` + `bridge-agent.ts` keep streaming active across route changes and show a live `#14b8a6` dot next to `Agente` in `Navigation.tsx`.
