# HERA v0.6.0 Agent-First Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade HERA (`v0.5.0` → `v0.6.0`) to a zero-context-rot Agent-First architecture with a root Cargo workspace, pure-Rust `hera-ipc` DTO + Tauri v2 ACL Triad guardian crate, unified `ts-rs` IPC types across `bridge-hera.ts` and `bridge-agent.ts`, cohesive `< 350` LOC modularization of post-`95795ac` monoliths, and a lock-free in-memory Dual-Deck audio transport core (`hera-rs/src/audio_engine.rs`).

**Architecture:** Introduce a root Cargo workspace (`hera-rs`, `hera-desktop/src-ipc`, `hera-desktop/src-tauri`) where `hera-ipc` decouples Desktop IPC contracts and Tauri v2 ACL Triad validation from `webkit2gtk-4.1` so type generation (`vp run gen:types`) and ACL parity tests run in `< 5s` on all platforms while `hera-rs` preserves Cloudflare Workers WASM (`axum 0.7`) compatibility. Split `dj_brain.rs`, `mcp.rs`, `hera_agent.rs`, `hera_dj.rs`, `bridge.tsx` (`Transición`), and `MiniOverlay.tsx` (`Modo Mini`) into cohesive submodules preserving 100% of `ADR-021..023` UX and APIs, and add a zero-`unsafe`, lock-free `AtomicU64` Dual-Deck transport engine over Symphonia PCM frames.

**Tech Stack:** Rust (`edition = 2021` / `2024`, `symphonia 0.5`, `rustfft 6.2`, `axum 0.7` & `0.8`, `ts-rs 12.0.1`), Tauri v2 (`2.12.2`), Bun `1.4.4`, Vite+ (`vp`, `Rolldown`, `Oxlint`, `Oxfmt`), React 19 + StyleX (`@stylexjs/stylex`), `moon 2.6.5`.

**Spec:** `docs/superpowers/specs/2026-11-09-agent-first-architecture-design.md`

## Global Constraints

- Zero OS-dependent absolute paths (`C:\`, `/Users/`, `/home/`) in tracked code, configs, specs, or plans.
- Swarm-First invariant: `beta_mvp`, `free`, and `local_unlimited` nodes always keep `p2p_sharing_enabled = true` and `can_disable_p2p_sharing = false`.
- `hera-rs` must compile cleanly under `--target wasm32-unknown-unknown --features cloudflare --no-default-features` (`axum = "0.7"` retained in `hera-rs`; `ts-rs` strictly optional behind `feature = "ts"`).
- `hera-desktop/src-tauri` retains `axum = "0.8.9"` for its loopback MCP server.
- Never resurrect pre-`ADR-021` UI panels in `hera-desktop/src/routes/bridge.tsx`; keep the minimalist `Transición` studio layout and `Modo Mini` (`360×408` / `560×52`) intact.
- Rust and TypeScript logic modules targeted by this refactor must stay `< 350` LOC; StyleX route/overlay view files with colocated `stylex.create` must stay `< 500` LOC.
- `DualDeckEngine::render_interleaved_stereo` in `hera-rs/src/audio_engine.rs` must use `#![forbid(unsafe_code)]`, zero `Mutex`, zero heap allocations, and zero disk/network I/O.
- Git commits must use `David Castillo <vikruzdavid@gmail.com>` with Conventional Commits after real green terminal receipts.

## Review Focus

1. **Multi-file plugin directories in ACL Triad scan:** When `hera_dj.rs` and `hera_agent.rs` become `hera_dj/*.rs` and `hera_agent/*.rs` in Task 5, `verify_tauri_v2_acl_triad` must discover `#[tauri::command]` across nested submodule files (`plugins/<dir>/*.rs`) and still map them to `"hera-dj"` and `"hera-agent"`.
2. **WASM feature leak from `ts-rs`:** Enabling `#[cfg_attr(feature = "ts", derive(ts_rs::TS))]` on `hera-rs` structs (`u64` / `serde_json::Value` fields) must neither break `--target wasm32-unknown-unknown --features cloudflare --no-default-features` nor generate invalid TypeScript (`bigint` vs `number` mismatch where frontend expects `number`). Use `#[cfg_attr(feature = "ts", ts(type = "number"))]` on `u64`/`usize` fields serialized as JSON numbers!
3. **Existing `Track` & `SortBy` `ts-rs` contract (`ADR-017`):** Regenerating `hera-desktop/src/generated/typings.ts` must preserve `SortBy` (`"Bpm" | "Camelot"`) and `#[ts(optional)]` on `Track.{bpm, camelot, energy, hera_seal, hot_cues}`.
4. **Workspace `target/` output path in `moon`:** Moving from `hera-rs/target` to workspace root `target/` breaks `hera-rs/moon.yml` if `outputs` still points to crate-local `target/release/hera-node`.
5. **Out-of-bounds Hot Cue or EOF seek in `DualDeckEngine`:** Jumping to an unset cue index (`0` or `> 4`) or seeking past `buffer.total_frames()` must clamp safely without panicking or producing NaN/out-of-bounds slice reads.

---

### Task 1: Architectural Memory Hygiene (`DECISIONS.md` + `ADR-024`) & Root Cargo Workspace

**Files:**
- Modify: `docs/superpowers/DECISIONS.md`
- Create: `Cargo.toml`
- Modify: `hera-rs/Cargo.toml`
- Modify: `hera-rs/moon.yml`
- Create: `hera-desktop/src-ipc/Cargo.toml`
- Create: `hera-desktop/src-ipc/src/lib.rs`
- Modify: `hera-desktop/src-tauri/Cargo.toml`
- Modify: `.moon/workspace.yml`

**Interfaces:**
- Produces: Root Cargo workspace (`members = ["hera-rs", "hera-desktop/src-ipc", "hera-desktop/src-tauri"]`, `default-members = ["hera-rs", "hera-desktop/src-ipc"]`) bumped to `version = "0.6.0"`, plus initial `hera-ipc` crate shell.

- [ ] **Step 1: Update Section 1 & append `ADR-024` in `docs/superpowers/DECISIONS.md`**
  Replace the legacy Python (`uv`/`ruff`/`mypy`/`pytest`/`streamlit`/`librosa`) Section 1 with the `v0.6.0` Pure-Rust + Tauri v2 + `hera-ipc` + Bun `1.4.4` + Vite+ (`vp`) + `moon 2.6.5` baseline and canonical verification commands, and record `ADR-024` documenting the `v0.6.0` Agent-First Workspace, `hera-ipc` ACL/TS-RS separation, modularization, and lock-free `audio_engine`.
- [ ] **Step 2: Create root `Cargo.toml` and `hera-desktop/src-ipc` skeleton, and align member `Cargo.toml` & `hera-rs/moon.yml`**
  Define `[workspace]` (`resolver = "2"`, `members`, `default-members`), `[workspace.package]` (`version = "0.6.0"`, `license = "MIT"`), `[workspace.dependencies]` (`serde`, `serde_json`, `thiserror`, `ts-rs = "12.0.1"`), and root `[profile.release]`. Keep `axum = "0.7"` in `hera-rs/Cargo.toml` and `axum = "0.8.9"` in `hera-desktop/src-tauri/Cargo.toml`. Update `hera-rs/moon.yml` `inputs`/`outputs` for workspace-root `target/`.
- [ ] **Step 3: Verify native workspace check, tests, and Cloudflare WASM target**
  Run:
  - `cargo check`
  - `cargo test -p hera-node-rs`
  - `cargo check --manifest-path hera-rs/Cargo.toml --target wasm32-unknown-unknown --features cloudflare --no-default-features`
  Expected: All exit `0` with zero errors.
- [ ] **Step 4: Mark `[x]` and create atomic commit**
  Commit `build(workspace): establish v0.6.0 root Cargo workspace, hera-ipc crate, and ADR-024`.

---

### Task 2: Tauri v2 ACL Triad Guardian (`test_tauri_v2_acl_triad_parity`)

**Files:**
- Create: `hera-desktop/src-ipc/src/acl_guard.rs`
- Modify: `hera-desktop/src-ipc/src/lib.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj.rs`

**Interfaces:**
- Produces: `hera_ipc::acl_guard::{AclParityReport, verify_tauri_v2_acl_triad}` scanning both single-file (`plugins/<name>.rs`) and directory-module (`plugins/<name>/**/*.rs`) Tauri plugins against `src/main.rs`, `build.rs`, and `capabilities/main.json`.

- [ ] **Step 1: Write failing unit tests in `hera-desktop/src-ipc/src/acl_guard.rs`**
  Include:
  1. `test_tauri_v2_acl_triad_parity`: runs `verify_tauri_v2_acl_triad` on `../src-tauri` and asserts all 8 inlined plugins (`app-menu`, `config`, `cover`, `database`, `default-view`, `hera-dj` [13 cmds], `hera-agent` [9 cmds], `sleepblocker`) match across `plugins/**/*.rs`, `main.rs`, `build.rs`, and `capabilities/main.json`.
  2. `test_acl_guard_detects_missing_build_rs_and_capability_and_nested_mod_files`: creates a temp fixture with `plugins/hera_dj/submod.rs` containing an unregistered `#[tauri::command] pub async fn orphan_cmd()` and asserts the error explicitly names `build.rs` and `capabilities/main.json`.
- [ ] **Step 2: Implement `verify_tauri_v2_acl_triad` in `hera-desktop/src-ipc/src/acl_guard.rs` and wire re-test in `hera_dj.rs`**
  Keep `acl_guard.rs` `< 300` LOC with zero external regex dependencies. Also reference `hera_ipc::acl_guard::verify_tauri_v2_acl_triad` inside `hera_dj`'s `#[cfg(test)]` module.
- [ ] **Step 3: Run `cargo test -p hera-ipc` and capture green receipt**
  Run: `cargo test -p hera-ipc`
  Expected: `test_tauri_v2_acl_triad_parity` and negative drift tests pass (`exit code 0`).
- [ ] **Step 4: Mark `[x]` and create atomic commit**
  Commit `test(ipc): add automated Tauri v2 ACL triad parity guardian across all plugins`.

---

### Task 3: Unified IPC Type Contracts (`ts-rs`) Across `hera-rs`, `hera-ipc`, `bridge-hera.ts` & `bridge-agent.ts`

**Files:**
- Modify: `hera-rs/Cargo.toml`
- Modify: `hera-rs/src/dj_brain.rs`
- Modify: `hera-rs/src/export.rs`
- Modify: `hera-rs/src/cloudflare_edge.rs`
- Create: `hera-desktop/src-ipc/src/core_types.rs`
- Create: `hera-desktop/src-ipc/src/dj_types.rs`
- Create: `hera-desktop/src-ipc/src/agent_types.rs`
- Modify: `hera-desktop/src-ipc/src/lib.rs`
- Modify: `hera-desktop/src-tauri/src/libs/track.rs`
- Modify: `hera-desktop/src-tauri/src/libs/playlist.rs`
- Modify: `hera-desktop/src-tauri/src/libs/events.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/config.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/db.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_agent.rs`
- Modify: `hera-desktop/scripts/gen.sh`
- Modify: `hera-desktop/src/generated/typings.ts`
- Create: `hera-desktop/src/lib/hera-offline-fallbacks.ts`
- Modify: `hera-desktop/src/lib/bridge-hera.ts`
- Modify: `hera-desktop/src/lib/bridge-agent.ts`

**Interfaces:**
- Produces: Single-source `hera-desktop/src/generated/typings.ts` generated by `bash scripts/gen.sh` (`cargo test -p hera-ipc`), re-exported cleanly by `bridge-hera.ts` (`< 320` LOC) and `bridge-agent.ts` (`< 160` LOC).

- [ ] **Step 1: Gate `ts-rs` on `hera-rs` (`feature = "ts"`) and create `core_types.rs`, `dj_types.rs`, `agent_types.rs` in `hera-ipc`**
  Annotate exported `hera-rs` structs (`DjStyleDnaProfile`, `ExportCrateResponse`, `CloudflareServicePrimitive`, `SwarmMoqHarmonicEvent`, `CloudflareEdgeCapabilities`, `AgentToolExecutionStep`, `CloudflareFreeAiTurnResponse`, `AiBackendOption`, `DetectedCliAgent`, `DetectedLocalServer`, `McpHostIntegrationGuide`, `SwarmProtectionStatus`, `AiConnectivityOnboardingReport`) with `#[cfg_attr(feature = "ts", derive(ts_rs::TS))]` (using `#[cfg_attr(feature = "ts", ts(type = "number"))]` on `u64`/`usize` fields and `#[cfg_attr(feature = "ts", ts(type = "unknown"))]` on `serde_json::Value`). Move desktop IPC DTOs into `hera-ipc` (`core_types.rs`, `dj_types.rs`, `agent_types.rs`) and `pub use` them in `src-tauri` so Rust and TypeScript share one canonical definition.
- [ ] **Step 2: Update `hera-desktop/scripts/gen.sh` to run `cargo test -p hera-ipc` and regenerate `src/generated/typings.ts`**
  Add an export test in `hera-ipc` verifying `typings.ts` contains `Track`, `SortBy`, `DjStyleDnaProfile`, `ResolveHarmonicBridgeResponse`, `SetMiniOverlayModeResponse`, `AgentSessionSummary`, and `AgentRuntimeStatus`.
- [ ] **Step 3: Extract `hera-offline-fallbacks.ts` and refactor `bridge-hera.ts` & `bridge-agent.ts` to import generated types**
  Remove manual duplicate DTO interfaces from `bridge-hera.ts` and `bridge-agent.ts` (re-exporting type aliases from `../generated/typings` so downstream component imports remain 100% unbroken) and ensure both files are `< 350` LOC.
- [ ] **Step 4: Verify Rust, WASM, Typegen, Vite+ Check, and Unit Tests**
  Run:
  - `cd hera-desktop && bun run gen:types && bun run check && bun run test`
  - `cargo check --manifest-path hera-rs/Cargo.toml --target wasm32-unknown-unknown --features cloudflare --no-default-features`
  Expected: `0` TypeScript/StyleX/Oxlint errors, `15/15` unit tests pass, WASM compiles cleanly.
- [ ] **Step 5: Mark `[x]` and create atomic commit**
  Commit `refactor(ipc): unify Rust-to-TypeScript DTO generation via hera-ipc and ts-rs`.

---

### Task 4: Modular Decomposition of `hera-rs` Monoliths (`dj_brain` & `mcp`, `< 350` LOC/file)

**Files:**
- Delete/Replace: `hera-rs/src/dj_brain.rs` → `hera-rs/src/dj_brain/{mod.rs, dna.rs, backends.rs, http_client.rs, cli_runner.rs, prompts.rs, harness.rs, tests.rs}`
- Delete/Replace: `hera-rs/src/mcp.rs` → `hera-rs/src/mcp/{mod.rs, schemas.rs, dispatch.rs}`

**Interfaces:**
- Preserves 100% of `hera_node_rs::dj_brain::*` and `hera_node_rs::mcp::*` public signatures (`compute_dj_style_dna`, `list_supported_ai_backends`, `scan_ai_connectivity_onboarding`, `probe_backend_models`, `build_cli_agent_args`, `execute_cli_agent_turn`, `evaluate_agent_harness_turn`, `evaluate_cloudflare_free_ai_turn`, `list_mcp_tools`, `handle_mcp_request`, `dispatch_tool_call`, `run_mcp_stdio_loop`).

- [ ] **Step 1: Split `hera-rs/src/mcp.rs` (`900` LOC) into `mcp/{mod.rs, schemas.rs, dispatch.rs}` (`< 350` LOC each)**
  Keep `list_mcp_tools` tool schemas in `schemas.rs`, tool execution in `dispatch.rs` (split cleanly if needed so each file is `< 350` LOC), and JSON-RPC framing + stdio loop + `pub use` in `mod.rs`.
- [ ] **Step 2: Split `hera-rs/src/dj_brain.rs` (`1,911` LOC) into `dj_brain/{mod.rs, dna.rs, backends.rs, http_client.rs, cli_runner.rs, prompts.rs, harness.rs, tests.rs}` (`< 350` LOC each)**
  Preserve every `#[cfg(not(target_arch = "wasm32"))]` gate and `#[cfg_attr(feature = "ts", ...)]` attribute from Task 3.
- [ ] **Step 3: Verify line counts (`wc -l < 350`), `hera-rs` unit tests, `hera-ipc` typegen, and WASM build**
  Run:
  - `wc -l hera-rs/src/dj_brain/*.rs hera-rs/src/mcp/*.rs`
  - `cargo test -p hera-node-rs && cargo test -p hera-ipc`
  - `cargo check --manifest-path hera-rs/Cargo.toml --target wasm32-unknown-unknown --features cloudflare --no-default-features`
  Expected: Every file under `hera-rs/src/dj_brain/` and `hera-rs/src/mcp/` is `< 350` LOC; all `19+` `hera-rs` tests and WASM check exit `0`.
- [ ] **Step 4: Mark `[x]` and create atomic commit**
  Commit `refactor(hera-rs): modularize dj_brain and mcp into <350 LOC cohesive submodules`.

---

### Task 5: Modular Decomposition of `src-tauri` Plugins (`hera_agent` & `hera_dj`) + CI Tauri Gate

**Files:**
- Create: `hera-desktop/src-ipc/src/agy_stream.rs`
- Modify: `hera-desktop/src-ipc/src/lib.rs`
- Delete/Replace: `hera-desktop/src-tauri/src/plugins/hera_agent.rs` → `hera-desktop/src-tauri/src/plugins/hera_agent/{mod.rs, context.rs, tools_schema.rs, tools_library.rs, tools_playlist_vdj.rs, tools_host.rs, mcp_http.rs, turn_agy.rs, smart_intent.rs, commands.rs, tests.rs}`
- Delete/Replace: `hera-desktop/src-tauri/src/plugins/hera_dj.rs` → `hera-desktop/src-tauri/src/plugins/hera_dj/{mod.rs, bridge_p2p.rs, p2p_download.rs, mini_overlay.rs, virtualdj_live.rs, tests.rs}`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Preserves `plugins::hera_dj::init()` (13 commands) and `plugins::hera_agent::init()` (9 commands) and moves pure stream-JSON parsing (`ParsedAgyStreamEvent`, `parse_agy_stream_json_line`) + unified tool metadata helpers into `hera-ipc` so they are unit-tested on every OS even without GTK headers.

- [ ] **Step 1: Extract pure `agy_stream.rs` into `hera-ipc` with unit tests (`test_parse_agy_stream_json_events`)**
  Verify `cargo test -p hera-ipc` runs stream-JSON parsing tests cleanly.
- [ ] **Step 2: Split `plugins/hera_dj.rs` (`1,276` LOC) and `plugins/hera_agent.rs` (`3,326` LOC) into `< 350` LOC submodules**
  Keep exact `#[tauri::command]` signatures and `tauri::plugin::Builder::new("hera-dj")` / `Builder::new("hera-agent")` registrations intact.
- [ ] **Step 3: Upgrade `.github/workflows/ci.yml` to verify WASM, `hera-ipc` ACL/types, and full `src-tauri` build/tests (`libwebkit2gtk-4.1-dev`)**
  Ensure `verify_tauri_v2_acl_triad` passes over the new nested `plugins/hera_dj/*.rs` and `plugins/hera_agent/*.rs` directories (`cargo test -p hera-ipc`) and check `wc -l` across all new plugin files (`< 350` LOC each).
- [ ] **Step 4: Mark `[x]` and create atomic commit**
  Commit `refactor(tauri): decompose hera_agent and hera_dj monoliths and add CI full-stack checks`.

---

### Task 6: Frontend Modularization of `Transición` (`bridge.tsx`) & `Modo Mini` (`MiniOverlay.tsx`)

**Files:**
- Create: `hera-desktop/src/lib/hera-harmonic-utils.ts`
- Create: `hera-desktop/src/lib/__tests__/hera-harmonic-utils.test.ts`
- Create: `hera-desktop/src/components/transition/TransitionDeckCard.tsx`
- Create: `hera-desktop/src/components/transition/TransitionSuggestionsSection.tsx`
- Create: `hera-desktop/src/components/transition/TransitionCommandDock.tsx`
- Modify: `hera-desktop/src/routes/bridge.tsx`
- Create: `hera-desktop/src/components/mini-overlay/MiniIcons.tsx`
- Create: `hera-desktop/src/components/mini-overlay/MiniPillHud.tsx`
- Create: `hera-desktop/src/components/mini-overlay/MiniSuggestionsTab.tsx`
- Create: `hera-desktop/src/components/mini-overlay/MiniBridgeTab.tsx`
- Modify: `hera-desktop/src/components/MiniOverlay.tsx`

**Interfaces:**
- Preserves `/bridge` (`Transición` route) and default export `MiniOverlay` (`React.lazy` in `__root.tsx`) with identical DOM aria-labels, dark booth tokens, and VirtualDJ live crate actions (`ADR-021..023`).

- [ ] **Step 1: Write unit tests for `hera-harmonic-utils.ts` (`parseCamelot`, `camelotDistance`, `nextHarmonicCamelot`, `camelotRootHz`, `buildSyntheticTrackDescriptor`)**
  Run `cd hera-desktop && bun run test` to verify red → green cycle.
- [ ] **Step 2: Extract `components/transition/*` from `routes/bridge.tsx` and `components/mini-overlay/*` from `components/MiniOverlay.tsx`**
  Keep each extracted module focused (`< 350` LOC for TS helpers/subcomponents; `< 500` LOC for StyleX root containers with colocated `stylex.create`). Do not alter any `ADR-021..023` minimalist UI behavior.
- [ ] **Step 3: Verify `wc -l`, `bun run fmt`, `bun run check`, `bun run test`, and `bun run build` in `hera-desktop`**
  Run: `cd hera-desktop && bun run check && bun run test && bun run build`
  Expected: `0` Oxlint/StyleX/TypeScript errors, all unit tests green, lazy chunks built cleanly.
- [ ] **Step 4: Mark `[x]` and create atomic commit**
  Commit `refactor(desktop): modularize Transition route, MiniOverlay, and shared harmonic utilities`.

---

### Task 7: Lock-Free In-Memory Native Dual-Deck Audio Core (`hera-rs/src/audio_engine.rs`)

**Files:**
- Create: `hera-rs/src/audio_engine.rs`
- Modify: `hera-rs/src/lib.rs`

**Interfaces:**
- Produces (`#[cfg(not(target_arch = "wasm32"))] pub mod audio_engine`):
  - `DeckId::{DeckA, DeckB}`
  - `TransportCommand::{Play(DeckId), Pause(DeckId), SeekToSample(DeckId, u64), JumpToHotCue(DeckId, u8), SetTempoRatio(DeckId, f32), SetGain(DeckId, f32)}`
  - `CommandQueue` (`[AtomicU64; 64]` SPSC lock-free ring buffer: `try_push(&self, cmd: TransportCommand) -> bool`, `try_pop(&self) -> Option<TransportCommand>`)
  - `HotCueSampleMarker { pub num: u8, pub label: &'static str, pub frame_index: u64 }`
  - `PreallocatedDeckBuffer` (`from_stereo_frames`, `decode_from_bytes`, `decode_from_file` via Symphonia + automatic 4 Hot Cue frame calculation via `crate::structure::analyze_track_structure`)
  - `DualDeckEngine::render_interleaved_stereo(&mut self, queue: &CommandQueue, out_stereo: &mut [f32]) -> usize` (`#![forbid(unsafe_code)]`, zero `Mutex`, zero heap allocation)

- [ ] **Step 1: Write deterministic TDD unit tests in `hera-rs/src/audio_engine.rs`**
  Test cases:
  1. `test_lock_free_command_queue_pack_unpack_and_ring_wraparound`
  2. `test_sample_accurate_jump_to_four_hot_cues` (synthesizes distinct amplitude markers at `1:Intro`, `2:Breakdown`, `3:Drop`, `4:Outro` frame offsets, pushes `JumpToHotCue(DeckA, 1..=4)`, and asserts the very first rendered sample in `out_stereo` matches each cue's exact marker value)
  3. `test_dual_deck_mix_tempo_ratio_and_eof_clamp` (verifies `SetTempoRatio(DeckA, 1.25)`, simultaneous `DeckA + DeckB` summing, out-of-range cue index `0`/`9` clamping, and seek past EOF clamping without panic or NaN)
  4. `test_decode_wav_bytes_into_preallocated_deck_buffer` (decodes a real RIFF WAV via Symphonia into `PreallocatedDeckBuffer` with 4 Hot Cue sample markers)
- [ ] **Step 2: Implement `hera-rs/src/audio_engine.rs` (`< 350` LOC) and export in `hera-rs/src/lib.rs`**
  Guard with `#[cfg(not(target_arch = "wasm32"))]` in `lib.rs` and `#![forbid(unsafe_code)]` inside `audio_engine.rs`.
- [ ] **Step 3: Verify `cargo test -p hera-node-rs`, WASM build, and full monorepo `moon` suite**
  Run:
  - `cargo check --manifest-path hera-rs/Cargo.toml`
  - `cargo test --manifest-path hera-rs/Cargo.toml`
  - `cargo check --manifest-path hera-rs/Cargo.toml --target wasm32-unknown-unknown --features cloudflare --no-default-features`
  - `cargo test -p hera-ipc`
  - `cd hera-desktop && bun run gen:types && bun run check && bun run test && bun run build`
  - `moon run :check :test hera-desktop:build`
  Expected: All commands exit `0` with green terminal receipts.
- [ ] **Step 4: Mark `[x]` in plan and commit atomically**
  Commit `feat(audio): add lock-free DualDeckEngine transport with sample-accurate 4-Hot-Cue seek`.
