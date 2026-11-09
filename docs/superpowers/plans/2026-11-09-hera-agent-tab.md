# Minimalist Autonomous Agent Tab (`/agent`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a minimalist `/agent` tab below `Biblioteca` with real-time `agy` `stream-json` + loopback HTTP MCP (`127.0.0.1:0`) + Rust ReAct multi-turn execution across 40+ unified HERA Desktop, DSP/P2P, VirtualDJ, and OS tools (`auto` execution policy by default).

**Architecture:** Persist sessions and messages in SQLite (`05_create_agent_tables.sql`), expose a Bearer-authenticated loopback `axum` MCP endpoint + unified async tool dispatcher in `hera_agent.rs`, stream real-time step/text/UI-sync events (`hera-agent:stream`, `hera-agent:ui-sync`), and render a single-column `720 px` StyleX workspace (`agent.tsx`, `AgentToolStepRow.tsx`, `AgentComposer.tsx`) backed by a global background Zustand store (`agent-store.ts`).

**Tech Stack:** Rust (`tauri` v2, `axum` 0.8, `sqlx` SQLite, `tokio`, `hera-node-rs`), React 19, TypeScript, StyleX, Zustand, TanStack Router, Vitest (`bun run check`, `bun run test`, `bun run build`).

**Spec:** `docs/superpowers/specs/2026-11-09-hera-agent-tab-design.md`

## Global Constraints

- Strictly preserve the dark studio palette (`#0c0e12`, `#10141b`, `#141922`, `#171c24`, `1px` `#232a36`/`#26303d` borders, `#14b8a6` accent); zero light surfaces, zero purple/violet hues, zero kickers/eyebrows, zero nested cards, zero emoji icon stand-ins, zero routine floating toasts.
- Place `-p` immediately before the prompt argument whenever invoking `agy`.
- Bind the embedded HTTP MCP server strictly to `127.0.0.1:0` and require `Authorization: Bearer <ephemeral_secret>`.
- Run all blocking filesystem, DSP, and shell operations off the main Win32 UI thread (`tokio::process::Command` with `CREATE_NO_WINDOW` or `tauri::async_runtime::spawn_blocking`).
- Use repository-relative paths only in specs, plans, tests, and commits.

## Review Focus

- `agy` absent or offline environment: when `agy` is not installed or times out, `start_agent_turn` must seamlessly execute via the selected or fallback Rust ReAct / deterministic tool planner so library, playlist, VirtualDJ, and OS tools still execute end-to-end.
- Switching tabs while a turn is streaming: global `initAgentGlobalBridge()` must keep updating `useAgentStore` in the background, show the live emerald indicator on `Agente` in `Navigation.tsx`, and trigger `invalidate()` / player actions via `hera-agent:ui-sync`.
- `confirm` permission mode on OS tools: when `execution_policy == "confirm"`, `run_powershell_command`, `write_text_file`, and `move_or_rename_path` must pause on a `oneshot` approval channel until `resolve_agent_approval` approves or cancels, and `cancel_agent_turn` must abort active child processes cleanly.
- SQLite migration safety: `05_create_agent_tables.sql` must apply cleanly on both fresh `hera.db` instances and upgraded `04_add_hera_dj_columns.sql` databases.
- Viewport overflow at `1280×800` and `960×580`: long PowerShell outputs or JSON tool payloads inside expanded `<AgentToolStepRow />` blocks must scroll/wrap cleanly with `minWidth: 0` so `scrollWidth <= clientWidth`.

---

### Task 1: Fix `agy -p` Ordering in `hera-rs` & Add SQLite Agent Persistence (`05_create_agent_tables.sql` + `database.rs`)

**Files:**
- Modify: `hera-rs/src/dj_brain.rs`
- Create: `hera-desktop/src-tauri/src/migrations/05_create_agent_tables.sql`
- Modify: `hera-desktop/src-tauri/src/libs/database.rs`
- Modify: `hera-desktop/src-tauri/src/libs/database_tests.rs`

- [x] **Step 1: Fix `-p` flag placement in `hera-rs/src/dj_brain.rs`, add multi-message history support, and verify `cargo test --manifest-path hera-rs/Cargo.toml`**
- [x] **Step 2: Create `05_create_agent_tables.sql` and implement `agent_sessions` / `agent_messages` CRUD methods + unit tests in `database.rs` / `database_tests.rs`**

### Task 2: Implement `hera_agent.rs` Plugin (Loopback HTTP MCP Server, 40 Unified Tools, `agy stream-json` + Rust ReAct Streaming & Permissions)

**Files:**
- Create: `hera-desktop/src-tauri/src/plugins/hera_agent.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/mod.rs`
- Modify: `hera-desktop/src-tauri/src/main.rs`
- Modify: `hera-desktop/src-tauri/build.rs`
- Modify: `hera-desktop/src-tauri/capabilities/main.json`

- [x] **Step 1: Implement unified desktop + OS + DSP/P2P tool catalog, Bearer-protected `127.0.0.1:0` `axum` MCP server, `agy stream-json` parser, Rust ReAct / fast intent planner, and approval/cancel channels in `hera_agent.rs`**
- [x] **Step 2: Register `hera-agent` in `mod.rs`, `main.rs`, `build.rs`, and `capabilities/main.json`, write integration tests for MCP `401`/auth, SQLite playlist tool execution, OS directory/command tools, and `stream-json` parsing, and verify `cargo test --manifest-path hera-desktop/src-tauri/Cargo.toml`**

### Task 3: Build Minimalist `/agent` Frontend (`agent.tsx`, `AgentComposer.tsx`, `AgentToolStepRow.tsx`, `agent-store.ts`, `Navigation.tsx`)

**Files:**
- Create: `hera-desktop/src/lib/bridge-agent.ts`
- Create: `hera-desktop/src/lib/agent-store.ts`
- Create: `hera-desktop/src/components/AgentToolStepRow.tsx`
- Create: `hera-desktop/src/components/AgentComposer.tsx`
- Create: `hera-desktop/src/routes/agent.tsx`
- Modify: `hera-desktop/src/components/Navigation.tsx`
- Modify: `hera-desktop/src/routes/__root.tsx`
- Modify: `hera-desktop/src/routes/bridge.tsx`
- Modify: `hera-desktop/src/components/MiniOverlay.tsx`

- [x] **Step 1: Create `bridge-agent.ts` and `agent-store.ts` with single global Tauri stream + UI-sync listeners (`invalidate`, player control, route navigation, background running indicator)**
- [x] **Step 2: Build `AgentToolStepRow.tsx`, `AgentComposer.tsx` (`/` commands, `@` mentions, `+ Adjuntar`, engine & `Auto`/`Confirmar` selectors), and `/agent` route (`agent.tsx`), wire `Navigation.tsx` (`Agente` below `Biblioteca`), `__root.tsx` (`Ctrl+J`), `bridge.tsx`, and `MiniOverlay.tsx`**
- [x] **Step 3: Run `bun run check`, `bun run test`, `bun run build`, and craft detector (`0` findings)**

### Task 4: End-to-End Live WebView2 Verification via Tauri MCP, Release Build & Atomic Commit

**Files:**
- Modify: `docs/superpowers/plans/2026-11-09-hera-agent-tab.md`
- Modify: `docs/superpowers/DECISIONS.md`

- [x] **Step 1: Execute live WebView2 QA (`run_agent_mcp_qa.mjs`) verifying `Agente` empty state, `/` & `@` menus, real multi-tool execution (SQLite playlist creation + VirtualDJ `.vdjfolder` + OS inspection), collapsible tool row inspection, background indicator, `0` console errors, and `0` overflow**
- [x] **Step 2: Build optimized `release` binary, visually inspect live screenshots, mark `- [x]`, append `ADR-022` in `DECISIONS.md`, and commit atomically**
