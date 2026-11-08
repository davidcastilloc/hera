# Superpowers-ODD Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans` and `superpowers:multi-instance-handoff` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate the customized `superpowers-odd` harness, multi-instance Git state (`docs/superpowers/`), and verified `CLAUDE.md` / `AGENTS.md` conventions into `hera`.

**Architecture:** Physical skill files reside in `.agent/skills/` and `.claude/skills/` (zero OS symlinks) alongside repository-native skills (`skills/hera`, `skills/hera-dj-set-curator`). State and architectural memory are tracked in `docs/superpowers/{specs,plans,DECISIONS.md}`.

**Tech Stack:** Python `>=3.11` (`uv`, `ruff`, `mypy`, `pytest`), Rust 2021 (`hera-rs/Cargo.toml`), Git.

**Spec:** `docs/superpowers/specs/2026-11-08-superpowers-odd-integration-design.md`

## Global Constraints

- Zero OS-dependent absolute paths (`C:\`, `/Users/`, `/home/`) in tracked files.
- `git config user.name` must be `"David Castillo"` and `git config user.email` must be `"vikruzdavid@gmail.com"` (`davidcastilloc`) with Semantic Release / Conventional Commits.
- Every task completion requires real terminal receipts (`ruff`, `mypy`, `pytest`, `cargo check`).

## Review Focus

1. Windows git clone compatibility (zero symlinks in `.agent/skills/` and `.claude/skills/`).
2. Exact match between `CLAUDE.md` / `AGENTS.md` commands and `pyproject.toml` / `hera-rs/Cargo.toml`.
3. Zero regressions across the existing Python and Rust test suites.

---

### Task 1: Vendor `superpowers-odd` Skills & Shared State Structure

**Files:**
- Create: `.agent/skills/*`
- Create: `.claude/skills/*`
- Create: `docs/superpowers/DECISIONS.md`
- Create: `docs/superpowers/specs/2026-11-08-superpowers-odd-integration-design.md`
- Create: `docs/superpowers/plans/2026-11-08-superpowers-odd-integration.md`
- Modify: `scripts/ensure_latest_hera.py`

- [x] **Step 1: Copy customized `superpowers-odd` skills into `.agent/skills/` and `.claude/skills/` without symlinks**
- [x] **Step 2: Initialize `docs/superpowers/DECISIONS.md`, `specs/`, and `plans/`**
- [x] **Step 3: Update `scripts/ensure_latest_hera.py` to keep `.agent/skills/` and `.claude/skills/` synchronized**

### Task 2: Unify `AGENTS.md` and `CLAUDE.md` with Multi-OS & ODD+RDD Rules

**Files:**
- Modify: `AGENTS.md`
- Create: `CLAUDE.md`

- [x] **Step 1: Replace Windows absolute paths in `AGENTS.md` with OS-agnostic relative paths and add `superpowers-odd` protocol**
- [x] **Step 2: Create `CLAUDE.md` aligned 100% with `pyproject.toml`, `hera-rs/Cargo.toml`, and `AGENTS.md`**

### Task 3: Execute Phase 3 Receipt-Driven Verification & ODD Smoke Test

**Files:**
- Verify: `tests/`
- Verify: `docs/superpowers/plans/2026-11-08-superpowers-odd-integration.md`

- [x] **Step 1: Run zero-regression suite (`uv run ruff check .`, `uv run pytest`, `cargo check --manifest-path hera-rs/Cargo.toml`)**
- [x] **Step 2: Audit Git status, `user.name="David Castillo"`, `user.email="vikruzdavid@gmail.com"`, and zero OS-specific paths**
- [x] **Step 3: Run ODD Smoke Test and commit atomic handoff**
