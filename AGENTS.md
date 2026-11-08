# 🤖 HERA — Operational Guide for AI Agents & MCP Clients

**Package:** `hera` (`hera-rs/Cargo.toml` + `hera-desktop/package.json` + `hera-desktop/src-tauri/Cargo.toml`, **100% Pure Rust + Tauri v2, Zero Python**)  
**Protocol:** Model Context Protocol (MCP) `stdio` (`hera-node --mcp`) + HTTP (`POST /mcp`) + Code Mode (`POST /mcp/code-mode`)  
**Harness:** `superpowers-odd` (`https://github.com/davidcastilloc/superpowers-odd`)  
**Philosophy:** *"Todo es posible si lo imaginas (If you can imagine it, you can build it)"*

---

## 🎯 Package Overview & Architecture (Zero-Python `~8 MB` Native Engine)

This repository contains the canonical **100% Pure-Rust + Tauri v2** implementation of **HERA**:
- `hera-rs/`: Pure-Rust Universal Audio DSP, 2-Opt TSP Crate Curator, Native Exporters (`VirtualDJ`, `Rekordbox`, `Traktor`, `M3U8`), `DJ Style DNA` Auto-Profiler, `Cloudflare Workers AI Free` Client, MCP Server (`stdio` + HTTP + Code Mode) & 5-Layer Post-P2P Node (`hera-node-rs` v0.2.0, `axum`, `symphonia`, `rustfft`, features `native` and `cloudflare`):
  - `hera-rs/src/forensics.rs`: Zero-Trust spectral forensics (`Symphonia` + `rustfft`).
  - `hera-rs/src/structure.rs`: 4 Acoustic Hot Cues (`1:Intro`, `2:Breakdown`, `3:Drop`, `4:Outro`), BPM, and Camelot key detection.
  - `hera-rs/src/synergy.rs`: 4-Layer Holistic Synergy Engine + Global `2-Opt TSP` + `u64 SimHash POPCNT`.
  - `hera-rs/src/export.rs`: Pure-Rust generators for **VirtualDJ** (`.vdjfolder` + `_vdj_database.xml`), **Rekordbox XML**, **Traktor Pro NML** (`_traktor.nml`), and **M3U8** (`POST /api/v1/crates/export`).
  - `hera-rs/src/dj_brain.rs`: Pure-Rust `DJ Style DNA` Auto-Profiler (`POST /api/v1/dj/style-dna`), `DJ Brain` prompt block builder, and `Cloudflare Workers AI Free` (`10,000 Neurons/day` free at `$0.00`, `POST /api/v1/ai/chat`, `GET /api/v1/ai/backends`).
  - `hera-rs/src/mcp.rs` & `hera-rs/src/code_mode.rs`: 21-tool Pure-Rust MCP Server (`stdio` via `hera-node --mcp`, HTTP `POST /mcp`, and TypeScript Code Mode `POST /mcp/code-mode`).
  - `hera-rs/src/p2p.rs` & `hera-rs/src/p2p_exchange.rs`: 5-Layer Post-P2P Federated R2 Swarm Orchestrator.
- `hera-desktop/`: Direct clone of `martpie/museeks` (`v0.23.4`, MIT) adapted for HERA (`Tauri v2` + `React 19 Compiler` + `StyleX` + `@tanstack/react-virtual` + `@tanstack/react-router` + `Zustand` + `@dnd-kit` + `sqlx` + `lofty` + `ts-rs`, `~8 MB` native installer with zero Python runtime):
  - `hera-desktop/src/components/TrackProgress.tsx`: Top single-track preview bar with **4 clickable Hot Cue pins** (`🟢 1:Intro`, `🔵 2:Breakdown`, `🔴 3:Drop`, `🟡 4:Outro`).
  - `hera-desktop/src/components/TrackListHeader.tsx` & `TrackRow.tsx`: Virtualized DJ library table with `BPM`, `Camelot Key` badge, and `4 Hot Cues`.
  - `hera-desktop/src/components/HeraDjToolbar.tsx` & `hera-desktop/src/lib/bridge-hera.ts`: 100% deterministic offline toolbar for `🧬 Mi ADN DJ`, `🎛️ Exportar VDJ / Rekordbox / Traktor`, and `🌍 Puente P2P & Copiloto IA`.
- `.agent/skills/` & `.claude/skills/`: Multi-OS vendored `superpowers-odd` + `hera` skills (zero OS symlinks).
- `docs/superpowers/`: Multi-instance Git state (`specs/`, `plans/`, and `DECISIONS.md`).

---

## 🛠️ Canonical Build, Typecheck & Test Commands

Always use these exact commands verified against `.moon/workspace.yml`, `hera-rs/Cargo.toml`, and `hera-desktop/package.json`:

```bash
# 1. Polyglot Monorepo Check, Test & Build via moon (Rust + Bun + Vite+ with content-addressed cache)
moon run :check :test hera-desktop:build

# 2. Fast Typecheck & Native Compilation Check (hera-rs)
cargo check --manifest-path hera-rs/Cargo.toml

# 3. Full Rust Unit & Integration Test Suite (DSP, Exporters, DJ Brain ReAct Harness, MCP, P2P Swarm)
cargo test --manifest-path hera-rs/Cargo.toml

# 4. HERA Desktop Frontend Check, Build & Unit Tests (Bun + Vite+ Rolldown/Oxlint/Oxfmt)
cd hera-desktop && bun run check && bun run test && bun run build

# 5. Cloudflare Workers WASM Build
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
   - **Zero OS-Specific Absolute Paths & Zero Symlinks:** Never commit paths containing `C:\`, `/Users/`, or `/home/`. Always use repository-relative paths. Keep skills as physical files in `.agent/skills/` and `.claude/skills/`.
3. **ODD (Organic-Driven Development) Ceremony Scaling:**
   - **Micro-changes / Bugfixes (`< 30` LOC):** Go straight to failing test + minimal code fix + terminal verification receipt + atomic commit.
   - **Features / Architectural Changes (`>= 30` LOC):** Follow `brainstorming` (`docs/superpowers/specs/`) -> `writing-plans` (`docs/superpowers/plans/`) -> `subagent-driven-development` -> `verification-before-completion`.
4. **RDD (Receipt-Driven Development) & Anti-Hallucination Gate:**
   - Never claim a task is complete without pasting real terminal output (`exit code 0`) from `cargo check` and `cargo test`.
   - **Forbidden:** Fake tests (mocks testing mocks), empty stubs (`TODO`, `unimplemented!()`), or deleting existing tests to pass the suite.
5. **Performance Gate & Stack Respect:**
   - **Surgical Search First:** Always use `ripgrep` (`rg -n`) before reading full files. Never bulk-read ignored directories (`target`, `node_modules`, `.wrangler`, `quarantine`, `library`, `exports`, `logs`).
   - **Stack Respect:** Zero Python dependencies; strictly respect `hera-rs/Cargo.toml` and `hera-desktop/package.json`.

---

## 🔌 Running the Pure-Rust MCP Server

To expose HERA's 21 native Rust tools to any MCP-compatible agent (Claude, Codex, Antigravity, Cursor):

```bash
cargo run --release --manifest-path hera-rs/Cargo.toml -- --mcp
```

---

## 🛡️ Key Invariants for AI Operators

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
