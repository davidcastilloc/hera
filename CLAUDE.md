# 🤖 CLAUDE.md — HERA Engineering & Superpowers-ODD Guide

> **Canonical Reference:** This file mirrors and enforces the repository conventions and `superpowers-odd` multi-instance workflow defined in [`AGENTS.md`](AGENTS.md) and [`docs/superpowers/DECISIONS.md`](docs/superpowers/DECISIONS.md).

---

## 1. Quick Start & Verification Commands (Receipt-Driven)

Before claiming any task is complete, execute and attach the real terminal output (`exit code 0`) for:

```bash
# Python Linter & Formatter (Ruff — py311, line-length 120)
uv run ruff check .
uv run ruff format --check .

# Python Typecheck (Mypy)
uv run mypy src/

# Python Test Suite (Pytest — asyncio_mode = "auto", testpaths = ["tests"])
uv run pytest

# Rust Engine Verification (hera-rs — Rust 2021, hera-node-rs v0.2.0)
cargo check --manifest-path hera-rs/Cargo.toml
cargo test --manifest-path hera-rs/Cargo.toml
```

---

## 2. Codebase Conventions & Architecture

- **Language & Runtime:** Python `>=3.11` (`uv`) + Rust 2021 (`hera-rs/`).
- **Module Layout:**
  - `src/hera/contracts/`: Pydantic v2 schemas (strict validation for all external/config inputs).
  - `src/hera/domain/`: Business logic, `Database` (`aiosqlite`), `HeraConfig`, `CrateExporter`.
  - `src/hera/adapters/`: External tool bridges (`rclone`, `slskd`).
  - `src/hera/mcp/`: MCP server (`stdio` + HTTP + Code Mode).
  - `src/hera/cli.py`: Click CLI commands (`setup`, `status`, `doctor`, `search`, `audit`, `set`, `serve`, `sync`, `ui`).
  - `analyzers/`: Acoustic DSP extractors (`librosa`, `ffmpeg`, `transients.py`, `style_embeddings.py`, `synergy_engine.py`).
  - `hera-rs/src/`: Pure-Rust DSP (`forensics.rs`, `structure.rs`, `synergy.rs`), P2P broker (`p2p.rs`), and MCP (`mcp.rs`, `code_mode.rs`).
- **Coding Standards:**
  - Explicit type annotations (`str`, `Path`, `list[Track]`, `dict[str, Any]`) on all functions/methods.
  - Cross-platform paths via `pathlib.Path` — **never** hardcode OS-specific absolute paths (`C:\`, `/Users/`, `/home/`).
  - Async I/O (`asyncio`, `httpx`, `aiosqlite`) for network, DB, and subprocess tasks.
  - Conventional Commits / Semantic Release (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`) with `git config user.name "David Castillo"` and `git config user.email "vikruzdavid@gmail.com"` (`davidcastilloc`).

---

## 3. Superpowers-ODD Multi-Instance Protocol (Windows / macOS / Linux / Antigravity)

Skills are physically vendored (zero OS symlinks) inside `.claude/skills/` and `.agent/skills/` from `davidcastilloc/superpowers-odd`:

1. **Always Active Skill — `superpowers:multi-instance-handoff`:**
   - Shared state lives exclusively in Git:
     - `docs/superpowers/specs/` — Design specs
     - `docs/superpowers/plans/` — Step-by-step plans with `- [ ]` / `- [x]` checkboxes
     - `docs/superpowers/DECISIONS.md` — Distilled architectural memory & critical bug log
   - **Atomic Handoff & Community PR Governance:**
     - **Core Maintainer (`@davidcastilloc`):** Commits under `David Castillo <vikruzdavid@gmail.com>` (`git config user.name "David Castillo"`, `git config user.email "vikruzdavid@gmail.com"`).
     - **Community Contributors:** Preserve your own `user.name` / `user.email`, work on a feature branch (`feat/*`, `fix/*`), and open a Pull Request via `.github/PULL_REQUEST_TEMPLATE.md` (reviewed via `.github/CODEOWNERS` by `@davidcastilloc`).
     - At the end of every subtask in `docs/superpowers/plans/`, mark `[x]`, update `docs/superpowers/DECISIONS.md` only if a critical technical finding occurred, and create an atomic Conventional Commit.
2. **ODD Ceremony Scaling:**
   - **Changes `< 30` lines:** Skip spec/plan ceremony -> failing test -> minimal code -> terminal verification receipt -> atomic commit.
   - **Changes `>= 30` lines:** `brainstorming` (`docs/superpowers/specs/`) -> `writing-plans` (`docs/superpowers/plans/`) -> `subagent-driven-development` (`50–200` LOC max per ephemeral subagent, zero context rot) -> `verification-before-completion`.
3. **Performance & Anti-Hallucination Gates:**
   - Use surgical `ripgrep` (`rg -n`) before reading files; never bulk-read `.venv`, `target`, `node_modules`, `.wrangler`, `quarantine`, `library`, `exports`, or `logs`.
   - Zero fake tests (mocks testing mocks), zero empty stubs (`TODO` / `pass` / `unimplemented!()`), zero deleted tests to force green suites, and zero unrequested dependencies.
