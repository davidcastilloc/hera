# HERA v0.6.0 — Release Pipeline Workspace Alignment & Pure-Rust Documentation Hygiene

**Date:** 2026-11-09  
**Baseline Commit:** `8c8977b` (`ADR-024`)  
**Target Version:** `v0.6.0`  
**Repository:** `davidcastilloc/hera`

---

## 1. Intent & Motivation

All 7 implementation tasks of `2026-11-09-agent-first-architecture.md` (`ADR-024`) landed on `main` (`8c8977b`) with CI green (`42734036253`), establishing the root Cargo workspace (`Cargo.toml` + `Cargo.lock`), the hexagonal `hera-ipc` (`ts-rs` + Tauri v2 ACL Triad guardian) crate, `< 350` LOC modularization across `hera-rs` and `hera-desktop`, and the lock-free `DualDeckEngine` in `hera-rs/src/audio_engine/`.

Before cutting the `v0.6.0` Git tag, an ecosystem audit uncovered two concrete blockers and documentation drift:
1. **Broken Tag-Triggered Release Workflows After Root Workspace Migration:**
   - `.github/workflows/release-binaries.yml` compiles `hera-node` inside the root workspace (`target/<triple>/release/<bin>`), yet still runs `cp "hera-rs/target/${{ matrix.target }}/release/${{ matrix.bin_name }}"`, which fails on Linux, macOS, and Windows as soon as a `v*` tag is pushed.
   - `.github/workflows/release.yml` configures `Swatinem/rust-cache@v2` with `hera-rs -> target` and `hera-desktop/src-tauri -> target` even though per-crate `Cargo.lock` files were consolidated into the root `Cargo.lock` in `8dd2b4a`.
2. **Missing `[0.6.0]` Changelog & Stale Legacy Python / Monolith References in Top-Level Docs:**
   - `CHANGELOG.md` stops at `[0.5.0]` (`ADR-011`) and omits `ADR-012` through `ADR-024`.
   - `README.md` and `CONTRIBUTING.md` still contain legacy `src/hera/`, `uv run`, `pytest`, `ruff`, `mypy`, and `librosa` sections from pre-`v0.3.0`.
   - `AGENTS.md` and `CLAUDE.md` still list `hera-node-rs v0.2.0` and pre-split `dj_brain.rs`, `mcp.rs`, and `code_mode.rs` single-file paths instead of the `v0.6.0` modular workspace and `hera-ipc` crate.

---

## 2. Goals, Non-Goals & Anti-Goals

### Goals
- Fix `.github/workflows/release-binaries.yml` and `.github/workflows/release.yml` to use root Cargo workspace `target/` paths and add an automated regression test in `hera-desktop/src-ipc` so release workflow paths can never silently regress.
- Document `[0.6.0] - 2026-11-09` in `CHANGELOG.md` covering `ADR-012`..`ADR-024`.
- Align `AGENTS.md`, `CLAUDE.md`, `README.md`, and `CONTRIBUTING.md` with the `v0.6.0` 100% Pure-Rust + Tauri v2 + `hera-ipc` + Bun `1.4.4` + Vite+ (`vp`) + `moon 2.6.5` architecture.
- Cut and push the `v0.6.0` tag after fresh green verification receipts.

### Non-Goals
- Modifying UI components, DSP algorithms, or IPC runtime behavior already verified in `ADR-024`.

### Anti-Goals
- Never introduce OS-specific absolute paths in tracked documentation, workflows, specs, plans, or tests.
- Never leak `hera-cloud` private SaaS routes or admin headers into `hera`.

---

## 3. Approaches Considered

1. **Option A (Selected — Release Workflow Guard Test + Top-Level Docs Alignment + `v0.6.0` Tag):**
   - Adds a static workflow hygiene test in `hera-ipc` verifying `.github/workflows/*.yml` never reference obsolete crate-local `hera-rs/target/` or deleted sub-crate lockfiles, fixes both release workflows, updates `CHANGELOG.md`, `AGENTS.md`, `CLAUDE.md`, `README.md`, and `CONTRIBUTING.md`, verifies all Cargo/ODD checks, and pushes `v0.6.0`.
   - *Trade-off:* Adds ~40 LOC of static verification in `hera-ipc`, guaranteeing tag releases cannot fail from workspace path drift.
2. **Option B (Manual YAML Edit Without Regression Test):**
   - Rejected under ODD/TDD because workflow target-path regressions only surface when tags are pushed unless guarded in `cargo test -p hera-ipc`.
