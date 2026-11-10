# AGENTS.md — Operational Guide for HERA Desktop (`hera-desktop v0.6.0`)

## Overview

**HERA Desktop (`v0.6.0`)** is the native **Dark Booth Studio** desktop application for the HERA ecosystem, adapted from `martpie/museeks` (`v0.23.4`, MIT) on **Tauri v2 + 100% Pure Rust (`hera-node-rs` + `hera-ipc`) + React 19 + StyleX + Bun 1.4.4 + Vite+**. It delivers a `~8 MB` native installer with zero Python runtime dependencies.

## Stack

- **Frontend (`hera-desktop/src/`)**
  - Types: TypeScript (strict) + headless `ts-rs` bindings generated from `hera-ipc` (`src/generated/hera-typings.ts`)
  - UI Library: React 19 (with React Compiler)
  - Routing: TanStack Router (`src/routes/`: `library.tsx`, `agent.tsx`, `bridge.tsx`, `dna.tsx`, `settings.*`)
  - Virtualization & Drag-and-Drop: `@tanstack/react-virtual` + `@dnd-kit`
  - Global State Management: Zustand (`src/stores/`)
  - Styling: [StyleX](https://stylexjs.com/docs/learn) — **Dark Booth Studio** palette (`#0c0e12` obsidian canvas + `#14b8a6` acoustic teal)
  - Localization/i18n: [Lingui.js](https://lingui.dev/introduction) (Spanish default)
  - Dev & Build Infra: Bun `1.4.4` + [Vite Plus (`vp`)](https://viteplus.dev/guide/) (Oxlint, Oxfmt, Vitest)

- **Backend (`hera-desktop/src-ipc/` & `hera-desktop/src-tauri/`)**
  - `src-ipc/` (`hera-ipc v0.6.0`): Zero-GTK pure-Rust Hexagonal IPC contract crate, `ts-rs` DTO generator (`typegen.rs`), 44-tool MCP catalog (`agent_catalog.rs`), Tauri v2 ACL Triad verifier (`acl_guard.rs`), and CI workflow/documentation guardian (`workflow_guard.rs`).
  - `src-tauri/` (`hera-desktop v0.6.0`): Tauri v2 desktop shell integrating `hera-node-rs` DSP, SQLite (`sqlx`), and 8 inlined plugins (`config`, `cover`, `db`, `default`, `hera_agent`, `hera_dj`, `shell_extension`, `stream_server`).

## Canonical Commands

> **Important Rule**: Never start or leave running local dev servers (`bun run dev`, `vp dev`, `vp run tauri dev`) in automated agent sessions unless explicitly requested. Verify changes deterministically via the commands below:

```bash
# 1. Headless Rust -> TypeScript DTO Generation (< 5s via hera-ipc)
bun run gen:types

# 2. Format, Lint & Typecheck (Vite+ / Oxfmt / Oxlint)
bun run check

# 3. Frontend Unit Test Suite (Vitest)
bun run test

# 4. Production Frontend Bundle Build
bun run build

# 5. Hexagonal IPC Contracts, Tauri v2 ACL Triad & Documentation Guardian (from repo root)
cargo test -p hera-ipc
```

## Project Structure

```
src-ipc/                 # Zero-GTK pure-Rust Hexagonal IPC & CI Guard crate (hera-ipc)
  src/
    acl_guard.rs         # Enforces Tauri v2 ACL Triad parity across all 8 inlined plugins
    workflow_guard.rs    # Enforces workspace target/ paths, version parity & v0.6.0 doc assets
    agent_catalog*.rs    # 44-tool Desktop + Core MCP schema & human summary catalog
    dj_types.rs          # Shared DJ DSP, Hot Cue, 2-Opt TSP & P2P Swarm DTOs
    agent_types.rs       # Shared Autonomous Agent & MCP tool execution DTOs
    typegen.rs           # Exports src/generated/hera-typings.ts

src/                     # React 19 / TypeScript frontend
  components/            # Feature components:
    Header.tsx           # 4-tab studio navigation (Biblioteca, Agente, Transición, Mi Estilo) + Modo Mini
    TrackProgress.tsx    # Waveform scrubber with 4 clickable Hot Cue pins (Intro, Breakdown, Drop, Outro)
    HeraDjToolbar.tsx    # Symphonia+rustfft audit, 2-Opt TSP curation, VirtualDJ/Rekordbox/Traktor export
    MiniOverlay.tsx      # Morphing VirtualDJ booth card (360x408) & HUD pill (560x52) (Ctrl+Shift+M)
    mini-overlay/        # Modularized MiniOverlay subcomponents & StyleX styles
    transition/          # Modularized 3-band EQ & crossfader harmonic bridge deck cards
  routes/                # File-based routes (TanStack Router):
    library.tsx          # Virtualized DJ track table (BPM, Camelot Key, Puntos, Hot Cues)
    agent.tsx            # Autonomous /agent workspace (Ctrl+J) with @ Contexto & confirmation gates
    bridge.tsx           # Dual-deck Transition Studio + 5-Stage P2P Swarm badge
    dna.tsx              # Mi Estilo (DJ Style DNA Auto-Profiler & SimHash u64 affinity)
  lib/                   # IPC bridges (bridge-hera-dj.ts, bridge-hera-agent.ts, bridge-database.ts)
  generated/             # Auto-generated ts-rs types — regenerate via `bun run gen:types`

src-tauri/               # Tauri v2 desktop backend
  src/
    plugins/             # 8 inlined plugins (db, config, cover, default, hera_agent, hera_dj, shell_extension, stream_server)
    migrations/          # SQLite migrations — frozen, never edit existing .sql files
  capabilities/
    main.json            # Tauri v2 ACL permissions for all 8 plugins
  build.rs               # Registers all inlined plugin commands
```

## Agent Operations Rules

- **Tauri v2 ACL Triad Invariant:** Every `#[tauri::command]` in `src-tauri/src/plugins/<plugin>/` MUST be registered in (1) the plugin's `init()`, (2) `src-tauri/build.rs` `COMMANDS`, and (3) `src-tauri/capabilities/main.json`. Always run `cargo test -p hera-ipc` to verify.
- **IPC Bridge Rule:** Never call `invoke` directly from React components — always go through `src/lib/bridge-*.ts`.
- **DTO Generation:** After modifying any Rust struct in `src-ipc/src/`, run `bun run gen:types` and commit `src/generated/hera-typings.ts`.
- **Verification Gate:** Always run `cargo test -p hera-ipc` and `cd hera-desktop && bun run check && bun run test && bun run build` before completing any task.
