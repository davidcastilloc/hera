# HERA v0.6.0 Release Pipeline & Documentation Alignment Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align `.github/workflows/{release,release-binaries}.yml` and top-level documentation (`CHANGELOG.md`, `AGENTS.md`, `CLAUDE.md`, `README.md`, `CONTRIBUTING.md`) with the `v0.6.0` root Cargo workspace and `100%` Pure-Rust + Tauri v2 architecture so the `v0.6.0` release builds cleanly across Linux, macOS, and Windows.

**Architecture:** Guard `.github/workflows/*.yml` against stale crate-local `target/` paths inside `hera-ipc`, update both GitHub Release workflows to consume root workspace `target/` artifacts and root `Cargo.lock` caching, and refresh top-level contributor/agent/release docs to match `ADR-024`.

**Tech Stack:** Rust (`Cargo` workspace, `hera-node-rs` `0.6.0`, `hera-ipc` `0.6.0`), GitHub Actions (`Swatinem/rust-cache@v2`, `tauri-apps/tauri-action@v0`), Markdown.

**Spec:** `docs/superpowers/specs/2026-11-09-v060-release-and-docs-alignment-design.md`

## Global Constraints

- Zero OS-dependent absolute host or drive paths in tracked files (repository-relative paths only).
- Zero OS symlinks in `.agent/skills/` and `.claude/skills/`.
- Zero `hera-cloud` private SaaS route/header leaks in `hera-rs/src/` or `hera-desktop/src/`.
- Commits authored as `David Castillo <vikruzdavid@gmail.com>` with Conventional Commits after green receipts.

## Review Focus

1. **Cross-compiled binary path in `release-binaries.yml`:** With a root Cargo workspace, `cargo build --release --manifest-path hera-rs/Cargo.toml --target <triple>` writes to `target/<triple>/release/<bin_name>`, never `hera-rs/target/...`.
2. **`Swatinem/rust-cache@v2` workspace root:** `Cargo.lock` lives solely at repository root (`. -> target`), so sub-crate `workspaces` entries without lockfiles must be removed.
3. **Historical `CHANGELOG.md` preservation:** Keep `[0.5.0]`..`[0.2.0]` history intact while prepending `[0.6.0] - 2026-11-09`.

---

### Task 1: Root Workspace Release Workflow Fix & Automated Guard in `hera-ipc`

**Files:**
- Create: `hera-desktop/src-ipc/src/workflow_guard.rs` (extracted from `acl_guard.rs` to keep every file `< 350` LOC)
- Modify: `hera-desktop/src-ipc/src/lib.rs`
- Modify: `.github/workflows/release-binaries.yml`
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Produces: `verify_workspace_release_workflows(repo_root: &Path) -> Result<(), String>` tested in `hera-ipc` (`cargo test -p hera-ipc`).

- [x] **Step 1: Add failing test `test_workspace_release_workflows_target_paths` in `hera-desktop/src-ipc/src/workflow_guard.rs`**
  Assert `.github/workflows/release-binaries.yml` copies from `target/${{ matrix.target }}/release/` (never `hera-rs/target/`) and `.github/workflows/release.yml` caches root `. -> target` without stale `hera-rs -> target` / `hera-desktop/src-tauri -> target`.
- [x] **Step 2: Run `cargo test -p hera-ipc` to capture red receipt**
  Run: `cargo test -p hera-ipc`
  Expected: FAIL citing `hera-rs/target/` in `release-binaries.yml` and stale `workspaces:` in `release.yml`.
- [x] **Step 3: Update `.github/workflows/release-binaries.yml` and `.github/workflows/release.yml` to use root workspace `target/`**
  Update artifact staging path in `release-binaries.yml` (`target/${{ matrix.target }}/release/${{ matrix.bin_name }}`) and `Swatinem/rust-cache@v2` `workspaces: . -> target` in `release.yml` (plus `ubuntu-22.04` `libsoup-3.0-dev` / `libdbus-1-dev` parity).
- [x] **Step 4: Run `cargo test -p hera-ipc` and capture green receipt**
  Run: `cargo test -p hera-ipc`
  Expected: PASS (`0` failed).
- [x] **Step 5: Mark `[x]` and commit atomically**
  Commit `fix(ci): align release workflows with v0.6.0 root Cargo workspace target`.

---

### Task 2: `v0.6.0` `CHANGELOG.md` & Pure-Rust Top-Level Documentation Alignment

**Files:**
- Modify: `CHANGELOG.md`
- Modify: `AGENTS.md`
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `CONTRIBUTING.md`

**Interfaces:**
- Produces: Complete `[0.6.0]` release notes and zero stale Python (`uv run`, `pytest`, `ruff`, `mypy`, `src/hera/`, `librosa`) or `v0.2.0` single-file monolith references in `AGENTS.md`, `CLAUDE.md`, `README.md`, and `CONTRIBUTING.md`.

- [ ] **Step 1: Prepend `## [0.6.0] - 2026-11-09` to `CHANGELOG.md`**
  Summarize `ADR-012`..`ADR-024`: 5-stage zero-config P2P + RFC 5766 TURN + streaming Symphonia multi-codec decoder, Dark Booth Studio UI, minimalist `Transición` & `Mi Estilo`, morphing `Modo Mini` + live `HERA En Vivo.vdjfolder`, autonomous 44-tool `/agent` tab + loopback HTTP MCP, root Cargo workspace + `hera-ipc` (`ts-rs` + ACL Triad guardian), `< 350` LOC modularization, and lock-free `DualDeckEngine` (`hera-rs/src/audio_engine/`).
- [ ] **Step 2: Align `AGENTS.md`, `CLAUDE.md`, `README.md`, and `CONTRIBUTING.md` with `v0.6.0`**
  Update crate versions (`v0.6.0`), modular directory layout (`hera-rs/src/{audio_engine,dj_brain,mcp,code_mode,p2p}/`, `hera-desktop/src-ipc/`, `hera-desktop/src-tauri/src/plugins/{hera_agent,hera_dj}/`), and Pure-Rust + Tauri v2 + Bun `1.4.4` + Vite+ (`vp`) + `moon` contributor commands.
- [ ] **Step 3: Run full `hera` verification & ODD hygiene checks**
  Run:
  - `cargo check`
  - `cargo test -p hera-node-rs`
  - `cargo test -p hera-ipc`
  - `rg -n "uv run|pytest|ruff|mypy|src/hera/|librosa" README.md CONTRIBUTING.md AGENTS.md CLAUDE.md` (expect 0 matches)
  Expected: All checks exit `0` with zero stale Python instructions and zero OS absolute paths.
- [ ] **Step 4: Mark `[x]` and commit atomically**
  Commit `docs(release): prepare v0.6.0 changelog and align Pure-Rust workspace docs`.
