# 🤖 CLAUDE.md — HERA Engineering & Superpowers-ODD Guide

> **Canonical Reference:** This file mirrors and enforces the repository conventions and `superpowers-odd` multi-instance workflow defined in [`AGENTS.md`](AGENTS.md) and [`docs/superpowers/DECISIONS.md`](docs/superpowers/DECISIONS.md).

---

## 1. Quick Start & Verification Commands (Receipt-Driven)

Before claiming any task is complete, execute and attach the real terminal output (`exit code 0`) for:

```bash
# Root Cargo Workspace Verification (hera-node-rs v0.6.0 + hera-ipc v0.6.0, Zero Python)
cargo check
cargo test -p hera-node-rs
cargo test -p hera-ipc

# Desktop Frontend Typegen, Lint/Typecheck, Unit Tests & Bundle (Bun 1.4.4 + Vite+)
cd hera-desktop && bun run gen:types && bun run check && bun run test && bun run build
```

---

## 2. Codebase Conventions & Architecture (Zero-Python `~8 MB` Native Stack)

- **Language & Runtime:** 100% Rust (`Cargo.toml` root workspace: `hera-rs/`, `hera-desktop/src-ipc/`, `hera-desktop/src-tauri/`) + TypeScript / React 19 Compiler / StyleX (`hera-desktop/src/`, Bun `1.4.4` + Vite+ `vp` + `moon 2.6.5`). Zero Python runtime or dependencies.
- **Module Layout (`v0.6.0`, `< 350` LOC Modular Decomposition):**
  - `hera-rs/src/` (`hera-node-rs` `v0.6.0`): Pure-Rust DSP (`forensics.rs`, `structure.rs`, `synergy.rs`), `#![forbid(unsafe_code)]` Lock-Free `DualDeckEngine` (`audio_engine/`), Native DJ Exporters (`export.rs` for VirtualDJ `.vdjfolder`, Rekordbox XML, Traktor NML, M3U8), `DJ Style DNA` & `HeraAgentHarness` (`dj_brain/`), 5-Stage Zero-Config P2P + 5-Layer Federated R2 Swarm (`p2p/`, `p2p.rs`, `p2p_exchange.rs`, `cloudflare_edge.rs`, `error.rs`), and 21-tool MCP Server (`mcp/`, `code_mode/`).
  - `hera-desktop/src-ipc/` (`hera-ipc` `v0.6.0`): Zero-GTK pure-Rust `ts-rs` IPC DTO contract (`typegen.rs` -> `hera-desktop/src/generated/hera-typings.ts`), Tauri v2 ACL Triad guardian (`acl_guard.rs`), and release workflow path guard (`workflow_guard.rs`).
  - `hera-desktop/` (`hera-desktop` `v0.6.0`): Adapted from `martpie/museeks` (`v0.23.4`, MIT) on `Tauri v2` (`src-tauri/src/plugins/{hera_agent,hera_dj}/`, `sqlx`, `lofty`, `rayon`) + `React 19` + `StyleX` (`agent.tsx`, `bridge.tsx`, `dna.tsx`, `MiniOverlay.tsx`, `HeraDjToolbar.tsx`).
- **Coding Standards:**
  - Never commit OS-specific absolute host, user-home, or drive-letter paths (always use repository-relative paths).
  - Conventional Commits / Semantic Release (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`) with `git config user.name "David Castillo"` and `git config user.email "vikruzdavid@gmail.com"` (`davidcastilloc`).

---

## 3. Superpowers-ODD Multi-Instance Protocol (Windows / macOS / Linux / Antigravity)

Skills are physically vendored (zero OS symlinks) inside `.claude/skills/` and `.agent/skills/` from `davidcastilloc/superpowers-odd`:

1. **Always Active Skill — `superpowers:multi-instance-handoff`:**
   - Shared state lives exclusively in Git:
     - `docs/superpowers/specs/` — Design specs
     - `docs/superpowers/plans/` — Step-by-step plans with `- [ ]` / `- [x]` checkboxes
     - `docs/superpowers/DECISIONS.md` — Distilled architectural memory & critical bug log
2. **ODD Ceremony Scaling:**
   - **Changes `< 30` lines:** Skip spec/plan ceremony -> failing test -> minimal code -> terminal verification receipt -> atomic commit.
   - **Changes `>= 30` lines:** `brainstorming` (`docs/superpowers/specs/`) -> `writing-plans` (`docs/superpowers/plans/`) -> `subagent-driven-development` (`50–200` LOC max per ephemeral subagent, zero context rot) -> `verification-before-completion`.
3. **Performance & Anti-Hallucination Gates:**
   - Use surgical `ripgrep` (`rg -n`) before reading files; never bulk-read `target`, `node_modules`, `.wrangler`, `quarantine`, `library`, `exports`, or `logs`.
   - Zero fake tests (mocks testing mocks), zero empty stubs (`TODO` / `unimplemented!()`), zero deleted tests to force green suites, and zero unrequested dependencies.
