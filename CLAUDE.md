# 🤖 CLAUDE.md — HERA Engineering & Superpowers-ODD Guide

> **Canonical Reference:** This file mirrors and enforces the repository conventions and `superpowers-odd` multi-instance workflow defined in [`AGENTS.md`](AGENTS.md) and [`docs/superpowers/DECISIONS.md`](docs/superpowers/DECISIONS.md).

---

## 1. Quick Start & Verification Commands (Receipt-Driven)

Before claiming any task is complete, execute and attach the real terminal output (`exit code 0`) for:

```bash
# Rust Engine Verification (hera-rs — Rust 2021, hera-node-rs v0.2.0, Zero Python)
cargo check --manifest-path hera-rs/Cargo.toml
cargo test --manifest-path hera-rs/Cargo.toml
```

---

## 2. Codebase Conventions & Architecture (Zero-Python `~8 MB` Native Stack)

- **Language & Runtime:** 100% Rust 2021 (`hera-rs/` + `hera-desktop/src-tauri/`) + TypeScript / React 19 Compiler / StyleX (`hera-desktop/src/`). Zero Python runtime or dependencies.
- **Module Layout:**
  - `hera-rs/src/`: Pure-Rust DSP (`forensics.rs`, `structure.rs`, `synergy.rs`), Native DJ Exporters (`export.rs` for VirtualDJ `.vdjfolder`, Rekordbox XML, Traktor NML, M3U8), `DJ Style DNA` & `Cloudflare Workers AI Free` (`dj_brain.rs`), 5-Layer Post-P2P Swarm (`p2p.rs`, `p2p_exchange.rs`), and 21-tool MCP Server (`mcp.rs`, `code_mode.rs`).
  - `hera-desktop/`: Direct clone of `martpie/museeks` (`v0.23.4`, MIT) adapted for HERA (`Tauri v2` + `React 19` + `StyleX` + `@tanstack/react-virtual` + `sqlx` + `lofty` + `ts-rs`).
- **Coding Standards:**
  - Never commit OS-specific absolute paths (`C:\`, `/Users/`, `/home/`).
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
