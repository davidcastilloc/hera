# Superpowers-ODD Multi-Instance Integration Design (`hera`)

- **Date:** 2026-11-08
- **Author:** `david`
- **Repository:** `davidcastilloc/hera`
- **Fork Source:** `https://github.com/davidcastilloc/superpowers-odd`

## 1. Intent & Why
Enable seamless, zero-friction handoff across **Windows**, **macOS**, and **Linux / Antigravity** instances without context rot, OS symlink breakage, or unverified agent claims.

## 2. Goals & Non-Goals
- **Goals:**
  - Vendor `superpowers-odd` skills directly into `.agent/skills/` and `.claude/skills/` as regular files (zero OS symlinks).
  - Centralize specs, plans, and architectural memory under `docs/superpowers/`.
  - Enforce Organic-Driven Development (ODD: `<30` LOC direct TDD+RDD vs `>=30` LOC full spec->plan->subagent flow) and Receipt-Driven Development (RDD: mandatory terminal output receipts for `ruff`, `mypy`, `pytest`, and `cargo`).
  - Eliminate OS-dependent paths from `AGENTS.md` and `CLAUDE.md`.
- **Non-Goals:**
  - Altering runtime audio DSP algorithms or introducing new third-party dependencies in `pyproject.toml` or `hera-rs/Cargo.toml`.

## 3. Approaches Considered
1. **OS Symlinks to `~/.ai-harnesses/superpowers-odd`:** Rejected because `git pull` on Windows fails or requires elevated Administrator privileges when creating directory/file symlinks.
2. **Direct Physical Skill Injection in `.agent/skills/` and `.claude/skills/` (Selected):** Works instantaneously on Windows, macOS, and Linux upon `git clone` / `git pull` with zero OS-level setup.
