# HERA v0.6.0 — Agent-First Architecture, Unified IPC Contracts & Lock-Free Audio Core (Option A)

**Date:** 2026-11-09  
**Baseline Commit:** `95795ac` (`ADR-023`)  
**Target Version:** `v0.6.0` (`ADR-024`)  
**Repository:** `davidcastilloc/hera`

---

## 1. Intent & Motivation

Between `ADR-016` and `ADR-023` (`95795ac`), `hera` evolved rapidly with a unified Dark Booth Studio UI, minimalist `Transición` (`bridge.tsx`) and `Mi Estilo` (`dna.tsx`) views, single-window `Modo Mini` (`MiniOverlay.tsx`) with live VirtualDJ `.vdjfolder` sync, and an autonomous 44-tool `Agente` workspace (`hera_agent.rs` + `agent.tsx`).

That velocity created five structural bottlenecks for AI agents and maintainers:
1. **Stale Architectural Baseline (`docs/superpowers/DECISIONS.md` Section 1):** Section 1 still documents legacy Python (`uv`, `mypy`, `pytest`, `streamlit`, `librosa`) instead of the `100% Pure Rust + Tauri v2 + Bun 1.4.4 + Vite+ + moon 2.6.5` stack.
2. **Split Cargo Builds & Headless Agent Friction:** `hera-rs` and `hera-desktop/src-tauri` lack a root Cargo workspace, and Desktop IPC DTOs + ACL checks are coupled to `src-tauri` (`webkit2gtk-4.1` / `libsoup-3.0`), preventing sub-5-second `ts-rs` generation and ACL verification on headless Linux agent VMs.
3. **Unchecked Tauri v2 ACL Triad (`ADR-007`, `ADR-022`, `ADR-023`):** `src-tauri` now exposes 8 inlined plugins (including `13` commands in `hera-dj` and `9` commands in `hera-agent`) that silently fail at runtime if `plugins/**/*.rs`, `src/main.rs`, `build.rs`, and `capabilities/main.json` drift out of sync.
4. **Manual TypeScript IPC Duplication:** Both `hera-desktop/src/lib/bridge-hera.ts` (`1,050` LOC) and `hera-desktop/src/lib/bridge-agent.ts` (`231` LOC) duplicate dozens of Rust structs manually alongside large inline offline fixtures.
5. **Post-`95795ac` Monoliths & Browser-Only Audio Transport:** Files such as `hera_agent.rs` (`3,326` LOC), `dj_brain.rs` (`1,911` LOC), `MiniOverlay.tsx` (`1,866` LOC), `bridge.tsx` (`1,796` LOC), `hera_dj.rs` (`1,276` LOC), and `mcp.rs` (`900` LOC) exceed optimal agent context windows, while `hera-rs` lacks a deterministic, lock-free native dual-deck sample transport for sample-accurate 4-Hot-Cue pre-listening without conflicting with VirtualDJ's ASIO/WASAPI drivers.

---

## 2. Goals, Non-Goals & Anti-Goals

### Goals
- Replace Section 1 of `docs/superpowers/DECISIONS.md` with the true `v0.6.0` zero-Python stack baseline and record `ADR-024`.
- Introduce a root `Cargo.toml` workspace (`members = ["hera-rs", "hera-desktop/src-ipc", "hera-desktop/src-tauri"]`, `default-members = ["hera-rs", "hera-desktop/src-ipc"]`) that shares `target/` without breaking `hera-rs` Cloudflare Workers WASM (`axum 0.7` + `worker 0.4`) or `src-tauri` (`axum 0.8.9`).
- Introduce `hera-desktop/src-ipc` (`hera-ipc`), a zero-GTK pure-Rust crate hosting shared Desktop IPC DTOs (`ts-rs` export in `< 5s` on all OSes) and the automated Tauri v2 ACL Triad Guardian (`test_tauri_v2_acl_triad_parity`) covering all 8 inlined plugins (`hera-dj`, `hera-agent`, `database`, `config`, `app-menu`, `cover`, `default-view`, `sleepblocker`).
- Unify TypeScript IPC types in `hera-desktop/src/generated/typings.ts` via `vp run gen:types` (`hera-ipc` + optional `ts` feature on `hera-node-rs`), eliminating manual DTO interfaces in `bridge-hera.ts` and `bridge-agent.ts` and extracting offline fixtures into `hera-desktop/src/lib/hera-offline-fallbacks.ts`.
- Modularize `dj_brain.rs`, `mcp.rs`, `hera_agent.rs`, `hera_dj.rs`, `bridge.tsx`, and `MiniOverlay.tsx` into cohesive submodules (`< 350` LOC for Rust/TS logic modules; `< 500` LOC for StyleX view modules with colocated `stylex.create`), preserving 100% of the `ADR-021..023` minimalist UI and public Rust APIs (`pub use`).
- Implement `hera-rs/src/audio_engine.rs` (`#[cfg(not(target_arch = "wasm32"))]`), a lock-free, zero-`unsafe`, zero-allocation in-memory dual-deck (`DeckA`, `DeckB`) transport core over `symphonia` with sample-accurate 4-Hot-Cue jumping (`1:Intro`, `2:Breakdown`, `3:Drop`, `4:Outro`) and zero OS audio driver contention alongside VirtualDJ.

### Non-Goals
- Opening hardware `cpal` / ASIO / ALSA output streams in `v0.6.0` (deferred to avoid exclusive soundcard contention when running `Modo Mini` on top of VirtualDJ and avoid mandatory `libasound2-dev` C headers on headless nodes).
- Altering any `hera-cloud` private SaaS endpoints or weakening the Swarm-First invariant (`p2p_sharing_enabled = true`, `can_disable_p2p_sharing = false` for `beta_mvp`, `free`, and `local_unlimited`).

### Anti-Goals
- **Never resurrect deleted pre-`ADR-021` UI panels** (`CloudflareEdgeRadarPanel`, 4-filter `SwarmSearchTable`, or multi-column `tool_calls_trace` inside `bridge.tsx`). `bridge.tsx` remains the minimalist `Transición` studio view and `agent.tsx` remains the centered autonomous workspace.
- **Never pollute Cloudflare WASM:** `ts-rs` and `audio_engine` must never compile under `--target wasm32-unknown-unknown --features cloudflare --no-default-features`.
- **Never allocate or lock inside the real-time audio render callback:** `DualDeckEngine::render_interleaved_stereo` is strictly forbidden from calling `Mutex::lock`, `Vec::push`, `Box::new`, `String`, or disk/network I/O.

---

## 3. Approaches Considered

1. **Option A (Selected — Hexagonal `Agent-First` over `95795ac` + In-Memory Lock-Free Audio Core):**
   - Adds root Cargo workspace + pure-Rust `hera-ipc` contract/ACL crate, unifies `ts-rs` across `bridge-hera.ts` and `bridge-agent.ts`, decomposes the real `95795ac` monoliths (`hera_agent.rs`, `dj_brain.rs`, `hera_dj.rs`, `mcp.rs`, `MiniOverlay.tsx`, `bridge.tsx`), and builds `hera-rs/src/audio_engine.rs` as a pure-Rust lock-free transport without `cpal` hardware contention.
   - *Trade-off:* Requires careful feature gating (`ts` feature in `hera-rs`, `default-members` in workspace) so WASM, headless Linux VMs, and full desktop Tauri builds all stay green.
2. **Option B (Refactor Only, Defer Phase 5):**
   - Skips `audio_engine.rs` completely in `v0.6.0`. Rejected because establishing the deterministic sample-accurate 4-Hot-Cue transport contract now gives `v0.7.0` a clean port boundary without hardware risk.
3. **Option C (Immediate `cpal` + `rubato` Hardware Output):**
   - Rejected for `v0.6.0` because exclusive ASIO/WASAPI controllers held by VirtualDJ during `Modo Mini` sessions conflict with secondary native streams, and `cpal` introduces C/ALSA build dependencies on Linux.

---

## 4. Detailed Architecture

### 4.1 Cargo Workspace & Hexagonal `hera-ipc` Crate
- **Root `Cargo.toml`:**
  - `members = ["hera-rs", "hera-desktop/src-ipc", "hera-desktop/src-tauri"]`
  - `default-members = ["hera-rs", "hera-desktop/src-ipc"]`
  - `[workspace.dependencies]` centralizes `serde`, `serde_json`, `thiserror`, `ts-rs = "12.0.1"`. `axum` stays crate-scoped (`0.7` in `hera-rs` for `worker 0.4`; `0.8.9` in `hera-desktop/src-tauri` for loopback MCP).
  - `[profile.release]` lives at workspace root (`lto = true`, `codegen-units = 1`, `opt-level = 3`).
- **`hera-desktop/src-ipc` (`hera-ipc`):**
  - Depends on `serde`, `serde_json`, `ts-rs`, `strum`, and `hera-node-rs = { path = "../../hera-rs", default-features = false, features = ["ts"] }`.
  - Exports all desktop IPC DTOs (`Config`, `Track`, `Playlist`, `HotCuePin`, `HarmonicBridgeStep`, `SwarmCatalogItem`, `ResolveHarmonicBridgeRequest/Response`, `DownloadP2PTrackRequest/Response`, `SetMiniOverlayModeRequest/Response`, `PushVirtualDjLiveCrateRequest/Response`, `VirtualDjNowPlaying`, `ProbeAiBackendResponse`, `AgentAttachment`, `AgentFollowupAction`, `AgentToolStepView`, `AgentMessageView`, `AgentSessionSummary`, `AgentBackendDescriptor`, `AgentRuntimeStatus`, `AgentStreamEvent`, `AgentUiSyncEvent`, `CreateAgentSessionRequest`, `UpdateAgentSessionRequest`, `StartAgentTurnRequest`, `StartAgentTurnResponse`) into `hera-desktop/src/generated/typings.ts`.
  - `hera-desktop/scripts/gen.sh` invokes `cargo test -p hera-ipc` so `vp run gen:types` succeeds deterministically on Windows, macOS, and headless Linux without GTK/WebKit2GTK headers.
  - `.github/workflows/ci.yml` verifies `hera-rs` (native + `wasm32-unknown-unknown`), `hera-ipc` (including `test_tauri_v2_acl_triad_parity` and `typings.ts` freshness), `hera-desktop` (`bun run check`, `bun run test`, `bun run build`), and `hera-desktop/src-tauri` (`cargo check` + `cargo test` on Ubuntu with `libwebkit2gtk-4.1-dev`).

### 4.2 Tauri v2 ACL Triad Guardian (`hera-desktop/src-ipc/src/acl_guard.rs`)
- Recursively scans `hera-desktop/src-tauri/src/plugins/**/*.rs` to extract every `#[tauri::command]` function and map its plugin directory/file stem (`app_menu` → `app-menu`, `hera_dj` → `hera-dj`, `hera_agent` → `hera-agent`, `db` → `database`, `default_view` → `default-view`, `config`, `cover`, `sleepblocker`).
- Parses `hera-desktop/src-tauri/src/main.rs`, `hera-desktop/src-tauri/build.rs`, and `hera-desktop/src-tauri/capabilities/main.json` and verifies:
  1. Every plugin with commands is initialized in `src/main.rs`.
  2. Every `#[tauri::command]` fn appears verbatim in `.plugin("<plugin>", tauri_build::InlinedPlugin::new().commands(&[...]))` in `build.rs` (and vice-versa: no ghost commands in `build.rs`).
  3. Every command has `"<plugin>:allow-<command-with-hyphens>"` in `capabilities/main.json`.
- Tested with both live `src-tauri` verification and synthetic drift fixtures asserting descriptive failure messages.

### 4.3 Cohesive Module Decomposition
- **`hera-rs/src/dj_brain/` (`< 350` LOC per file):**
  - `dna.rs`: `AnchorArtistStat`, `MediaMixProfile`, `DjStyleDnaRequest`, `DjStyleDnaProfile`, `compute_dj_style_dna`.
  - `backends.rs`: 12-backend registry, `scan_system_cli_agents`, `scan_local_ai_servers`, `scan_ai_connectivity_onboarding`, `auto_detect_local_backend`.
  - `http_client.rs`: Zero-dependency `TcpStream` HTTP/1.1 client, `curl` HTTPS fallback, `probe_backend_models`.
  - `cli_runner.rs`: Bounded hidden subprocess runner (`CREATE_NO_WINDOW`), `build_cli_agent_args`, `execute_cli_agent_turn`.
  - `prompts.rs`: `build_openai_tools_catalog`, `summarize_swarm_downloadable_catalog`, `build_agent_harness_system_prompt`.
  - `harness.rs`: Multi-turn ReAct loop (`evaluate_agent_harness_turn`, `parse_assistant_tool_calls`, `evaluate_cloudflare_free_ai_turn`).
  - `mod.rs` & `tests.rs`: Clean `pub use` re-exports and unit tests.
- **`hera-rs/src/mcp/` (`< 350` LOC per file):**
  - `schemas.rs`: Declarative JSON-RPC schemas for the 21 MCP tools (`list_mcp_tools`).
  - `dispatch.rs`: `dispatch_tool_call` and tool handlers.
  - `mod.rs`: `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `handle_mcp_request`, `run_mcp_stdio_loop`, and `pub use`.
- **`hera-desktop/src-tauri/src/plugins/hera_agent/` (`< 350` LOC per file):**
  - `context.rs`: `ActiveTurnTracker`, `AgentHostBridge`, `AgentRuntimeContext`, approval gates, event emitters.
  - `tools_schema.rs`: Unified 44-tool MCP catalog (`build_unified_mcp_tools_list`, `human_tool_title`, `summarize_tool_execution`).
  - `tools_library.rs`: Desktop SQLite library, playlist, player, and VirtualDJ/Mini-mode tool handlers.
  - `tools_host.rs`: Host OS filesystem, shell/PowerShell, and directory inspection tool handlers.
  - `mcp_http.rs`: Authenticated `127.0.0.1:0` loopback `axum` MCP router (`build_local_mcp_router`).
  - `agy_stream.rs`: Pure `ParsedAgyStreamEvent` & `parse_agy_stream_json_line` + `cli_agy` streaming turn driver.
  - `smart_intent.rs`: Deterministic `local_fast` / `local_ollama` multi-step turn executor (`execute_smart_intent_plan`).
  - `commands.rs` & `mod.rs` & `tests.rs`: The 9 `#[tauri::command]` handlers, `init()`, and unit tests.
- **`hera-desktop/src-tauri/src/plugins/hera_dj/` (`< 350` LOC per file):**
  - `bridge_p2p.rs`: `resolve_harmonic_bridge`, `download_p2p_track`, warm desktop UDP daemon & RIFF WAV synthesis.
  - `mini_overlay.rs`: `NormalWindowSnapshot` and `set_mini_overlay_mode`.
  - `virtualdj_live.rs`: `push_virtualdj_live_crate` and `read_virtualdj_now_playing`.
  - `mod.rs` & `tests.rs`: Core DSP/AI commands (`compute_style_dna`, `export_crate`, `build_dj_set_spine`, `get_cloudflare_edge_radar`, `chat_copilot`, `probe_ai_backend`, `list_ai_backends`, `scan_ai_connectivity`), `init()`, and tests.
- **`hera-desktop/src/` Frontend:**
  - `src/lib/hera-harmonic-utils.ts`: Shared `CAMELOT_OPTIONS`, `CUE_DOT_COLORS`, `parseCamelot`, `camelotDistance`, `nextHarmonicCamelot`, `camelotRootHz`, `buildSyntheticTrackDescriptor`, and WebAudio preview synthesizer.
  - `src/components/transition/`: Extracted subcomponents (`TransitionDeckSelectorCard.tsx`, `TransitionBridgeFlow.tsx`, `TransitionAssistantPopover.tsx`) keeping `src/routes/bridge.tsx` (`Transición`) concise and `< 500` LOC.
  - `src/components/mini-overlay/`: Extracted `MiniIcons.tsx`, `MiniPillHud.tsx`, `MiniSuggestionsTab.tsx`, and `MiniBridgeTab.tsx`, keeping `src/components/MiniOverlay.tsx` concise and `< 500` LOC.

### 4.4 Lock-Free Native Audio Transport Core (`hera-rs/src/audio_engine.rs`)
- Compiled under `#[cfg(not(target_arch = "wasm32"))]` with `#![forbid(unsafe_code)]` inside the module.
- **Data Model:**
  - `DeckId` (`DeckA`, `DeckB`)
  - `TransportCommand` (`Play`, `Pause`, `SeekToSample(u64)`, `JumpToHotCue(u8)`, `SetTempoRatio(f32)`, `SetGain(f32)`) packed losslessly into non-zero `u64` words inside a fixed `[AtomicU64; 64]` SPSC lock-free ring buffer (`CommandQueue`).
  - `PreallocatedDeckBuffer`: Stereo `f32` frames decoded via Symphonia (`load_deck_buffer_from_bytes` / `load_deck_buffer_from_file`) + 4 sample-accurate `HotCueSampleMarker` pins (`1:Intro`, `2:Breakdown`, `3:Drop`, `4:Outro`).
  - `DualDeckEngine`: Owns `DeckState` for `DeckA` and `DeckB` and renders sample-accurate stereo blocks (`render_interleaved_stereo(&mut self, queue: &CommandQueue, out_stereo: &mut [f32]) -> usize`) with fractional linear resampling for `tempo_ratio` (`0.5..=2.0`) and zero heap allocations or mutex locks.
