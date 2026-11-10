# 🤖 HERA — Operational Guide for AI Agents & MCP Clients

**Package:** `hera` (`Cargo.toml` workspace: `hera-rs/` + `hera-desktop/src-ipc/` + `hera-desktop/src-tauri/` + `hera-desktop/package.json`, **v0.6.0 — 100% Pure Rust + Tauri v2, Zero Python**)  
**Protocol:** Model Context Protocol (MCP) `stdio` (`hera-node --mcp`) + HTTP (`POST /mcp`) + Code Mode (`POST /mcp/code-mode`) + Desktop Loopback (`127.0.0.1:0` Bearer MCP)  
**Harness:** `superpowers-odd` (`https://github.com/davidcastilloc/superpowers-odd`)  
**Philosophy:** *"Todo es posible si lo imaginas (If you can imagine it, you can build it)"*

---

## 🎯 Package Overview & Architecture (Zero-Python `~8 MB` Native Engine)

This repository contains the canonical **100% Pure-Rust + Tauri v2** implementation of **HERA (`v0.6.0`)** orchestrated via a unified root Cargo workspace (`Cargo.toml` + `Cargo.lock`, `resolver = "2"`) and `moon 2.6.5`:
- `hera-rs/` (`hera-node-rs` `v0.6.0`, `axum`, `symphonia`, `rustfft`, features `native` and `cloudflare`):
  - `hera-rs/src/forensics.rs`: Zero-Trust spectral forensics (`Symphonia` + `rustfft`) and universal streaming partial decoder (`decode_audio_file_partial`).
  - `hera-rs/src/structure.rs`: 4 Acoustic Hot Cues (`1:Intro`, `2:Breakdown`, `3:Drop`, `4:Outro`), BPM, and Camelot key detection.
  - `hera-rs/src/synergy.rs`: 4-Layer Holistic Synergy Engine + Global `2-Opt TSP` + `u64 SimHash POPCNT`.
  - `hera-rs/src/audio_engine/`: `#![forbid(unsafe_code)]` in-memory `DualDeckEngine` driven by a lock-free `[AtomicU64; 64]` SPSC `CommandQueue`.
  - `hera-rs/src/export.rs`: Pure-Rust generators for **VirtualDJ** (`.vdjfolder` + `_vdj_database.xml`), **Rekordbox XML**, **Traktor Pro NML** (`_traktor.nml`), and **M3U8** (`POST /api/v1/crates/export`).
  - `hera-rs/src/dj_brain/`: Pure-Rust `DJ Style DNA` Auto-Profiler (`POST /api/v1/dj/style-dna`), ReAct `HeraAgentHarness`, bounded parallel local AI discovery, and `Cloudflare Workers AI Free` (`10,000 Neurons/day` free at `$0.00`, `POST /api/v1/ai/chat`, `GET /api/v1/ai/backends`).
  - `hera-rs/src/mcp/` & `hera-rs/src/code_mode/`: 21-tool Pure-Rust MCP Server (`stdio` via `hera-node --mcp`, HTTP `POST /mcp`, and TypeScript Code Mode `POST /mcp/code-mode`).
  - `hera-rs/src/p2p/` (`p2p.rs`, `p2p_exchange.rs`, `cloudflare_edge.rs`, `error.rs`): 5-Stage Zero-Config P2P Cascade (`LAN` -> `Multi-STUN` -> `16-port spray` -> `RFC 5766 TURN` -> `R2 Boost`) and 5-Layer Federated R2 Swarm Orchestrator.
- `hera-desktop/src-ipc/` (`hera-ipc` `v0.6.0`, zero-GTK pure-Rust Hexagonal contract & CI guardian):
  - `acl_guard.rs` & `workflow_guard.rs`: Automated Tauri v2 ACL Triad parity verification across all 8 inlined plugins and release workflow `target/` path validation.
  - `typegen.rs`, `dj_types.rs`, `agent_types.rs`: Headless `< 5s` `ts-rs` DTO generator exporting `hera-desktop/src/generated/hera-typings.ts`.
- `hera-desktop/` (`hera-desktop` `v0.6.0`, adapted from `martpie/museeks` `v0.23.4` MIT on `Tauri v2` + `Bun 1.4.4` + `Vite+` + `React 19 Compiler` + `StyleX` + `@tanstack/react-virtual` + `@tanstack/react-router` + `Zustand` + `@dnd-kit` + `sqlx` + `lofty` + `ts-rs`, `~8 MB` native installer with zero Python runtime):
  - `hera-desktop/src-tauri/src/plugins/hera_agent/` & `hera-desktop/src/routes/agent.tsx`: Autonomous `/agent` workspace (`Ctrl+J`) backed by an authenticated 44-tool `127.0.0.1:0` Bearer HTTP MCP router, `@ Contexto` attachments, and inline confirmation gates.
  - `hera-desktop/src-tauri/src/plugins/hera_dj/` & `hera-desktop/src/components/MiniOverlay.tsx` (`src/components/mini-overlay/`): Single-window morphing VirtualDJ `Modo Mini` (`360x408` booth card & `560x52` HUD pill, `Ctrl+Shift+M`), live `HERA En Vivo.vdjfolder` push, and P2P UDP seeder daemon.
  - `hera-desktop/src/routes/bridge.tsx` (`src/components/transition/`) & `hera-desktop/src/routes/dna.tsx`: Unified Dark Booth Studio (`#0c0e12` + `#14b8a6`) `Transición` and `Mi Estilo` views.
  - `hera-desktop/src/components/TrackProgress.tsx`, `TrackListHeader.tsx`, `TrackRow.tsx`, & `HeraDjToolbar.tsx`: 4 clickable Hot Cue pins, virtualized library table (`BPM`, `Camelot Key`, `Puntos`), and deterministic crate curation/export.
- `.agent/skills/` & `.claude/skills/`: Multi-OS vendored `superpowers-odd` + `hera` skills (zero OS symlinks).
- `docs/superpowers/`: Multi-instance Git state (`specs/`, `plans/`, and `DECISIONS.md`).

---

## 🛠️ Canonical Build, Typecheck & Test Commands

Always use these exact commands verified against `Cargo.toml`, `.moon/workspace.yml`, and `hera-desktop/package.json`:

```bash
# 1. Polyglot Monorepo Check, Test & Build via moon (Rust + Bun + Vite+ with content-addressed cache)
moon run :check :test hera-desktop:build

# 2. Fast Root Cargo Workspace Check (default headless members: hera-rs + hera-ipc)
cargo check

# 3. Core Rust Engine & DSP Test Suite (DSP, DualDeckEngine, Exporters, DJ Brain, MCP, P2P Swarm)
cargo test -p hera-node-rs

# 4. Hexagonal IPC Contracts, Tauri v2 ACL Triad & Release Workflow Guardian
cargo test -p hera-ipc

# 5. HERA Desktop Typegen, Lint/Typecheck, Unit Tests & Production Build (Bun 1.4.4 + Vite+)
cd hera-desktop && bun run gen:types && bun run check && bun run test && bun run build

# 6. Cloudflare Workers WASM Target Check & Release Build
cargo check --manifest-path hera-rs/Cargo.toml --target wasm32-unknown-unknown --features cloudflare --no-default-features
cd hera-rs && worker-build --release -- --features cloudflare --no-default-features
```

---

## ⚡ Superpowers-ODD + RDD Mandatory Protocol (`superpowers-odd`)

Every AI agent working across **Windows**, **macOS**, or **Linux / Antigravity** MUST obey these rules:

1. **Zero Context Rot:**
   - Dispatch ephemeral subagents per task (`50–200` lines of code max per iteration).
   - Never drag stale session history between tasks or across sessions.
2. **100% Git-Backed Multi-Instance Synchronization (`superpowers:multi-instance-handoff`):**
   - All specifications live in `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`.
   - All implementation plans live in `docs/superpowers/plans/YYYY-MM-DD-<feature>.md` with `- [ ]` / `- [x]` checkboxes.
   - All distilled architectural decisions and root-cause bug findings live in `docs/superpowers/DECISIONS.md`.
   - **Handoff & Git Author Rule (Maintainer vs. Community Contributors):**
     - **Core Maintainer (`@davidcastilloc`):** Uses `git config user.name "David Castillo"` and `git config user.email "vikruzdavid@gmail.com"`.
     - **Community Contributors:** Preserve your own GitHub `user.name` and `user.email`. Work on a feature branch (`feat/*` or `fix/*`) in your fork and submit a Pull Request against `main` using `.github/PULL_REQUEST_TEMPLATE.md`.
     - **Atomic Checkpoints:** Upon finishing each subtask in `docs/superpowers/plans/`, mark `[x]`, update `docs/superpowers/DECISIONS.md` only if there was a critical technical finding, and create an atomic Semantic Release / Conventional Commit (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`).
   - **Zero OS-Specific Absolute Paths & Zero Symlinks:** Never commit OS-specific absolute host, user-home, or drive-letter paths. Always use repository-relative paths. Keep skills as physical files in `.agent/skills/` and `.claude/skills/`.
3. **ODD (Organic-Driven Development) Ceremony Scaling:**
   - **Micro-changes / Bugfixes (`< 30` LOC):** Go straight to failing test + minimal code fix + terminal verification receipt + atomic commit.
   - **Features / Architectural Changes (`>= 30` LOC):** Follow `brainstorming` (`docs/superpowers/specs/`) -> `writing-plans` (`docs/superpowers/plans/`) -> `subagent-driven-development` -> `verification-before-completion`.
4. **RDD (Receipt-Driven Development) & Anti-Hallucination Gate:**
   - Never claim a task is complete without pasting real terminal output (`exit code 0`) from `cargo check`, `cargo test -p hera-node-rs`, and `cargo test -p hera-ipc`.
   - **Forbidden:** Fake tests (mocks testing mocks), empty stubs (`TODO`, `unimplemented!()`), or deleting existing tests to pass the suite.
5. **Performance Gate & Stack Respect:**
   - **Surgical Search First:** Always use `ripgrep` (`rg -n`) before reading full files. Never bulk-read ignored directories (`target`, `node_modules`, `.wrangler`, `quarantine`, `library`, `exports`, `logs`).
   - **Stack Respect:** Zero Python dependencies; strictly respect root `Cargo.toml`, `hera-rs/Cargo.toml`, `hera-desktop/src-ipc/Cargo.toml`, `hera-desktop/src-tauri/Cargo.toml`, and `hera-desktop/package.json`.
6. **Local-First Verification & GitHub Actions Cost Discipline:**
   - Execute all routine verification locally (`cargo check`, `cargo test -p hera-ipc`, `cargo test -p hera-node-rs`, `bun run check`, `bun run test`, `bun run test:e2e`, `bun run build`) before pushing.
   - Reserve GitHub Actions strictly for critical operations: Multi-OS binary release compilation (`release.yml` and `release-binaries.yml` on `v*` tags / `workflow_dispatch`) and external Pull Request gatekeeping (`ci.yml` on `pull_request` / `workflow_dispatch`). Never run Playwright E2E, `libwebkit2gtk-4.1-dev` apt installs, or routine `push: [main]` runs in `ci.yml`.

---

## 🔌 Running the Pure-Rust MCP Server

To expose HERA's 21 native Rust tools over `stdio` to any MCP-compatible agent (Claude, Codex, Antigravity, Cursor):

```bash
cargo run --release -p hera-node-rs -- --mcp
```

---

## 🛡️ Key Invariants for AI Operators

- **Swarm-First Invariant ("Por el bien del enjambre"):** Every product, P2P, and business decision MUST prioritize the strength of the global P2P swarm (*"Mientras más nodos de Hera, más fuerte es el sistema de enjambres"*). 100% of HERA's local DSP engine, 4 Hot Cues, 2-Opt TSP crate curation, DJ exporters, local LLMs, and Cloudflare Workers AI (`10k Neurons/day`) are `$0.00` free; in exchange, all free/local/beta nodes (`beta_mvp`, `free`, `local_unlimited`) ALWAYS keep P2P sharing ON (`p2p_sharing_enabled = true`, `can_disable_p2p_sharing = false`). Only `pro_monthly` / `founder_unlimited` accounts can disable P2P sharing.
- **Quarantine First:** Never write directly to `library/` without Zero-Trust forensic validation.
- **Explain Rankings:** Present score breakdown to users before recommending downloads.
- **Unified Holistic Synergy:** Always use the 4-layer synergy engine (Camelot harmonic flow, gradual BPM tempo curves, transient punch matching, and `u64` style SimHash) when arranging crates.
- **Local Isolation:** Do not send audio files or library contents to external paid APIs.

---

## 🌐 Two-Repository Open-Core Ecosystem (`hera` ↔ `hera-cloud`)

1. **`hera` (Public Repository — `davidcastilloc/hera`, root `.`)**:
   - **Role:** Open-Source Desktop App (`hera-desktop/`), Local/Edge Pure-Rust DSP Engine (`hera-rs/`), MCP Server (`stdio` + HTTP + Code Mode), and Community P2P / `Edge Guardian` Node (`DaviDJ` ↔ `DJFoo`).
2. **`hera-cloud` (Private Repository — `davidcastilloc/hera-cloud`, companion repo `../hera-cloud`)**:
   - **Role:** Private Cloudflare SaaS Control Plane, Paid R2 CAS Vault, Internal P2P Data Broker, Cloudflare Workers AI Free Gateway (`10,000 Neurons/day`), and Subscription Tier Gatekeeper.

See [`MANIFESTO.md`](MANIFESTO.md) and [`docs/superpowers/DECISIONS.md`](docs/superpowers/DECISIONS.md) for full architectural guidelines.
