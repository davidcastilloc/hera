# 🤝 Contributing to Hera

Thank you for your interest in contributing to **Hera**! We welcome contributions from developers, DJs, audio engineers, and music enthusiasts.

Hera is an open-source, local-first super-agent designed to make audio discovery, acoustic DSP analysis, and DJ crate curation seamless, intelligent, and human-friendly.

---

## 📑 Table of Contents
- [Code of Conduct & Core Principles](#-code-of-conduct--core-principles)
- [Development Setup](#-development-setup)
- [Codebase Architecture](#-codebase-architecture)
- [Coding Standards](#-coding-standards)
- [Running Tests & Linting](#-running-tests--linting)
- [Pull Request Workflow](#-pull-request-workflow)

---

## 🎯 Code of Conduct & Core Principles

When contributing to Hera, please adhere to our core project tenets:

1. **100% Real Audio — Zero Placeholders:** Never commit or generate synthetic sine waves or beep placeholders in user-facing crates. Every track promoted to a library must be a genuine, validated audio master.
2. **Local-First & Privacy-Preserving:** Hera runs on the user's local machine, NAS, VPS, or Docker container with SQLite and stdio-based MCP. Avoid introducing mandatory dependencies on proprietary cloud APIs or cloud databases.
3. **Cross-Platform Portability:** All features must operate seamlessly across **Linux**, **macOS**, and **Windows**. Never commit OS-specific absolute host, user-home, or drive-letter paths; always use Rust `std::path::Path` / `PathBuf` and repository-relative paths.
4. **Swarm-First & Human-Centric UX (*"Por el bien del enjambre"*):** Keep free/local P2P sharing enabled by default to strengthen the community swarm, and design booth workflows (`Biblioteca`, `Agente`, `Transición`, `Mi Estilo`, `Modo Mini`) for fast, low-text clarity in dark club environments.

---

## 🚀 Development Setup

HERA (`v0.6.0`) is a **100% Pure-Rust + Tauri v2 + TypeScript** polyglot monorepo with **zero Python dependencies**, orchestrated via a root Cargo workspace (`Cargo.toml`) and `moon 2.6.5`.

### 1. Prerequisites
- **Rust Stable Toolchain** (`rustup`, Cargo Edition 2021 & 2024; optional `rustup target add wasm32-unknown-unknown` for Cloudflare Workers)
- **Bun `1.4.4`** & **Node.js 24 LTS** (used by `hera-desktop` for `Vite+` / `vp`)
- **Git**
- **Optional (Full Native Desktop Window Build Only):**
  - **Linux:** `pkg-config`, `libdbus-1-dev`, `libwebkit2gtk-4.1-dev`, `libsoup-3.0-dev`, `librsvg2-dev`, `libayatana-appindicator3-dev`
  - **macOS:** Xcode Command Line Tools
  - **Windows:** Microsoft Edge WebView2 Runtime & MSVC Build Tools
  - **Polyglot Cache Runner:** [`moon`](https://moonrepo.dev) (`v2.6.5+`)

> **Note:** Headless verification of the core DSP/P2P engine (`hera-node-rs`) and Hexagonal IPC contracts / Tauri v2 ACL Triad guardian (`hera-ipc`) requires **zero** GTK or WebView headers on any OS.

### 2. Clone and Install
```bash
git clone https://github.com/davidcastilloc/hera.git
cd hera

# Verify root Cargo workspace headless members (hera-rs + hera-ipc)
cargo check

# Install HERA Desktop frontend dependencies
cd hera-desktop
bun install
```

---

## 🏗️ Codebase Architecture

The monorepo is organized into three Rust workspace crates and a React 19 + StyleX frontend:

```text
hera/ (v0.6.0)
├── Cargo.toml                        # Unified root Cargo workspace (resolver = "2")
├── .moon/workspace.yml               # Content-addressed polyglot task graph (moon 2.6.5)
├── hera-rs/                          # hera-node-rs v0.6.0 (Pure-Rust DSP, MCP & P2P Node)
│   └── src/
│       ├── audio_engine/             # #![forbid(unsafe_code)] lock-free [AtomicU64; 64] DualDeckEngine
│       ├── forensics.rs              # Symphonia partial streaming decoder + rustfft spectral audit
│       ├── structure.rs              # BPM, Camelot Wheel key & 4 Acoustic Hot Cues
│       ├── synergy.rs                # 4-Layer Holistic Synergy + zero-alloc 1D 2-Opt TSP
│       ├── export.rs                 # VirtualDJ (.vdjfolder), Rekordbox XML, Traktor NML, M3U8
│       ├── dj_brain/                 # DJ Style DNA, 12-backend registry & ReAct HeraAgentHarness
│       ├── mcp/                      # 21-tool Pure-Rust MCP Server (stdio + HTTP)
│       ├── code_mode/                # TypeScript Code Mode dispatcher
│       ├── p2p/                      # 5-Stage Zero-Config P2P (LAN/STUN/Spray/TURN/R2) & Custody
│       ├── cloudflare_edge.rs        # Cloudflare KV Forensic Dedup, D1 Sync & Vectorize SimHash
│       └── error.rs                  # Typed HeraError & subscription tier invariants
├── hera-desktop/
│   ├── src-ipc/                      # hera-ipc v0.6.0 (Zero-GTK ts-rs DTOs & CI Guardians)
│   │   └── src/
│   │       ├── acl_guard.rs          # Tauri v2 ACL Triad parity checker across 8 inlined plugins
│   │       ├── workflow_guard.rs     # Release workflow root target/ & Cargo.lock path guard
│   │       ├── typegen.rs            # Headless ts-rs exporter -> src/generated/hera-typings.ts
│   │       ├── dj_types.rs           # Shared DJ, P2P, Style DNA & Modo Mini DTOs
│   │       └── agent_types.rs        # Autonomous /agent session, stream & tool DTOs
│   ├── src-tauri/                    # hera-desktop v0.6.0 (Tauri v2, sqlx SQLite, lofty, rayon)
│   │   └── src/plugins/
│   │       ├── hera_agent/           # 44-tool 127.0.0.1:0 Bearer HTTP MCP & streaming runner
│   │       └── hera_dj/              # Off-main-thread DSP/P2P IPC, Modo Mini & live VirtualDJ push
│   └── src/                          # React 19 Compiler + StyleX + TanStack Router/Virtual + Zustand
│       ├── routes/                   # library.tsx, agent.tsx, bridge.tsx, dna.tsx, settings*.tsx
│       └── components/               # MiniOverlay.tsx, AgentComposer.tsx, HeraDjToolbar.tsx
└── docs/superpowers/                 # Multi-instance ODD specs/, plans/, and DECISIONS.md
```

### Key Subsystems:
* **Streaming Acoustic DSP (`hera-rs/src/forensics.rs` & `structure.rs`):** Streams audio headers and PCM via `Symphonia` + `rustfft` (`WAV`, `FLAC`, `MP3`, `M4A`, `AAC`, `ALAC`, `OGG`, `AIFF`) to extract BPM, 24-key Camelot notation, 4 Hot Cues, and spectral authenticity seals.
* **Lock-Free Dual-Deck Core (`hera-rs/src/audio_engine/`):** `#![forbid(unsafe_code)]` `[AtomicU64; 64]` SPSC command queue driving sample-accurate cue jumps and linear pitch resampling with zero render-thread allocations.
* **Hexagonal IPC & ACL Guardian (`hera-desktop/src-ipc/`):** Generates TypeScript bindings via `ts-rs` and validates Tauri v2 plugin ACL registrations in `< 5s` without requiring GUI libraries.
* **Native Tagging & Exporters (`lofty` & `hera-rs/src/export.rs`):** Reads/injects ID3v2.4 and Vorbis tags and writes native VirtualDJ `.vdjfolder` (`<Poi>` Hot Cues), Rekordbox XML, Traktor NML, and M3U8 crates.

---

## 📐 Coding Standards

* **Modular File Size (`< 350` LOC Logic / `< 500` LOC StyleX Views):** Split large subsystems into focused submodules with clean `pub use` re-exports rather than growing monolithic files.
* **Typed Errors & Shared `ts-rs` Contracts:** Use `HeraError` (`hera-rs/src/error.rs`) instead of ad-hoc string/status tuples, and define shared Desktop IPC payloads in `hera-desktop/src-ipc/` (`bun run gen:types`) rather than duplicating TypeScript interfaces by hand.
* **Tauri v2 Plugin ACL Triad (`ADR-007`):** Every `#[tauri::command]` added under `hera-desktop/src-tauri/src/plugins/` MUST be wired in `src/main.rs`, `build.rs` (`tauri_build::InlinedPlugin`), and `capabilities/main.json` (`cargo test -p hera-ipc` enforces 100% parity).
* **Non-Blocking Async IPC:** Never block the native window message pump; wrap CPU-bound DSP, disk I/O, or socket calls inside `tauri::async_runtime::spawn_blocking`.

---

## 🧪 Running Tests & Linting

### 1. Rust Workspace Compilation & Unit/Integration Suites
```bash
# Fast workspace check (hera-rs + hera-ipc)
cargo check

# Full DSP, 2-Opt TSP, DualDeckEngine, DJ Brain, MCP & 5-Stage P2P test suite
cargo test -p hera-node-rs

# Hexagonal ts-rs typegen, Tauri v2 ACL Triad & Release Workflow guards
cargo test -p hera-ipc

# Cloudflare Workers WASM target verification
cargo check --manifest-path hera-rs/Cargo.toml --target wasm32-unknown-unknown --features cloudflare --no-default-features
```

### 2. HERA Desktop Formatting, Linting, Typecheck & Unit Tests (`Vite+` / `Bun`)
```bash
cd hera-desktop

# Regenerate TypeScript IPC contracts from Rust
bun run gen:types

# Format, lint (Oxlint), and typecheck (TypeScript + StyleX)
bun run fmt
bun run check

# Run unit tests (Vitest) and production bundle (Rolldown)
bun run test
bun run build
```

### 3. Cached Polyglot Pipeline (`moon`)
```bash
moon run :check :test hera-desktop:build
```

---

## 🤖 Contributing with AI Agents (`superpowers-odd` Protocol)

Hera ships with pre-configured, zero-symlink AI engineering skills in `.agent/skills/` and `.claude/skills/` (compatible out-of-the-box with **Windows**, **macOS**, and **Linux** in Claude Code, Cursor, Antigravity, Codex, and Gemini CLI):

- **Organic-Driven Development (ODD):**
  - **Bugfixes / Micro-changes (`< 30` lines):** Write a failing test -> implement the minimal fix -> attach real terminal verification output.
  - **New Features (`>= 30` lines):** Document the design in `docs/superpowers/specs/`, create a step-by-step plan in `docs/superpowers/plans/`, and record critical architectural findings in `docs/superpowers/DECISIONS.md`.
- **Receipt-Driven Development (RDD):** Every Pull Request must paste real terminal output (`exit code 0`) from `cargo check`, `cargo test -p hera-node-rs`, `cargo test -p hera-ipc`, and (when touching `hera-desktop/`) `bun run check && bun run test && bun run build`.
- **Git Identity Preservation:** Community contributors preserve their personal GitHub `user.name` and `user.email` on feature/bugfix branches (`feat/*` or `fix/*`) in their fork.

---

## 🔄 Pull Request Workflow

1. **Fork the Repository:** Create a personal fork of `davidcastilloc/hera` on GitHub.
2. **Create a Feature Branch:**
   ```bash
   git checkout -b feat/your-feature-name
   # or
   git checkout -b fix/your-bugfix-name
   ```
3. **Commit Your Changes:** Follow Conventional Commits / Semantic Release guidelines:
   - `feat: add automatic DJ cue sheet generator`
   - `fix: handle uppercase AIFF extensions during library import`
   - `docs: align workspace architecture guide with v0.6.0`
   - `refactor: optimize Camelot harmonic bridge scoring`
4. **Push to Your Fork:**
   ```bash
   git push origin feat/your-feature-name
   ```
5. **Open a Pull Request:** Submit a PR against the `main` branch of `davidcastilloc/hera` using `.github/PULL_REQUEST_TEMPLATE.md` (reviewed by `@davidcastilloc`). Include your real terminal test receipts and any relevant issue numbers.

---

## 💬 Questions & Community

Feel free to open an [Issue](https://github.com/davidcastilloc/hera/issues) or start a [Discussion](https://github.com/davidcastilloc/hera/discussions) on GitHub for feature requests, architectural ideas, or questions.

Thank you for helping make Hera the best AI super-agent for music curation! 🎧
