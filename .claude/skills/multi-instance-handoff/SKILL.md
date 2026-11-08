---
name: multi-instance-handoff
description: "Master ODD + RDD + Multi-Instance Git Synchronization skill for Windows, macOS, and Linux/Antigravity. Enforces Zero Context Rot, surgical ripgrep searches, anti-hallucination gates, and atomic checkbox/DECISIONS.md commits."
---

# Multi-Instance Handoff & Organic-Driven Development (ODD + RDD)

## Overview

This master skill governs cross-platform, multi-instance AI engineering across **Windows**, **macOS**, and **Linux / Antigravity**. Every session is stateless; **Git is the single source of truth**.

**Core Principles:**
1. **Zero Context Rot:** Ephemeral subagents per task (`50–200` lines max per iteration). Never drag stale session history across tasks or instances.
2. **100% Git-Backed Multi-Instance Sync:** All specs, plans, and architectural memory live inside the repository at:
   - `docs/superpowers/specs/` — Design specifications (`YYYY-MM-DD-<topic>-design.md`)
   - `docs/superpowers/plans/` — Implementation plans with `- [ ]` / `- [x]` checkboxes (`YYYY-MM-DD-<feature>.md`)
   - `docs/superpowers/DECISIONS.md` — Distilled architectural memory & critical bug log
3. **Zero OS-Specific Absolute Paths & Zero Symlinks:**
   - **FORBIDDEN:** Hardcoding OS-dependent paths (`C:\Users\...`, `/Users/...`, `/home/...`) in tracked documentation, configs, or code. Always use repository-relative paths and `pathlib.Path`.
   - **FORBIDDEN:** OS symlinks for skills (which fail on Windows without Administrator privileges). Skills live as physical files in `.agent/skills/` and `.claude/skills/`.
4. **ODD (Organic-Driven Development) Ceremony Scaling:**
   - **Micro-change / Bugfix (`< 30` lines):** Skip ceremony. Go directly to failing test -> minimal code fix -> terminal receipt verification -> atomic commit.
   - **Feature / Architectural Change (`>= 30` lines):** Follow full flow: `brainstorming` (`docs/superpowers/specs/`) -> `writing-plans` (`docs/superpowers/plans/`) -> `subagent-driven-development` -> `verification-before-completion`.
5. **RDD (Receipt-Driven Development):** Never claim a task is complete without pasting the real terminal output (`exit code 0`) of the project's linters, typecheckers, and test suite.

---

## Mandatory Gates

### 1. Handoff Rule (Atomic Checkpoint Protocol)

At the completion of **every subtask** in `docs/superpowers/plans/<plan>.md`:
1. **Check the box:** Update `- [ ]` to `- [x]` in `docs/superpowers/plans/<plan>.md`.
2. **Distill critical findings:** Update `docs/superpowers/DECISIONS.md` **only** if a critical architectural decision, non-obvious constraint, or root-cause bug discovery occurred. Never dump raw logs into `DECISIONS.md`.
3. **Verify Git Identity & Semantic Release:**
   - Ensure `git config user.name` is `"David Castillo"` and `git config user.email` is `"vikruzdavid@gmail.com"` (GitHub: `davidcastilloc`).
   - Create an atomic Conventional Commit (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`) so that any other instance (Windows, Mac, or Antigravity) can resume immediately with a simple `git pull`.

### 2. Performance Gate (Surgical Discovery)

- **FORBIDDEN:** Bulk-reading or indexing ignored/heavy directories (`node_modules`, `.venv`, `.svelte-kit`, `dist`, `build`, `target`, `.wrangler`, `quarantine`, `library`, `exports`, `logs`).
- **REQUIRED:** Always execute surgical `ripgrep` (`rg -n "pattern" src/`) searches to locate exact files and line numbers **before** reading file ranges.

### 3. Anti-Hallucination & Stack Respect Gate

- **FORBIDDEN:** Writing fake tests (mocks that only assert mock behavior), leaving empty implementations (`TODO`, `pass`, `unimplemented!()`), or deleting/weakening existing tests to make the suite pass.
- **FORBIDDEN:** Forcing unrequested UI libraries (e.g., DaisyUI, Tailwind, Bootstrap if not already in the repo) or inventing external dependencies. Strictly respect `pyproject.toml`, `Cargo.toml`, or `package.json`.

---

## Multi-Instance Resume Checklist (On `git pull`)

When starting or resuming work on any machine (Windows / macOS / Linux / Antigravity):
1. Run `git status -sb` and `git log -n 5 --oneline`.
2. Read `CLAUDE.md` / `AGENTS.md` and `docs/superpowers/DECISIONS.md`.
3. Inspect active plans in `docs/superpowers/plans/` and locate the first unchecked `- [ ]` step.
4. Execute the next subtask (`50–200` LOC max), verify with real terminal receipts, mark `- [x]`, and commit under `David Castillo <vikruzdavid@gmail.com>`.
