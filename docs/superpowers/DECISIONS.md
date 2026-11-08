# 🧠 HERA — Architectural Decisions & Distilled Memory (`DECISIONS.md`)

> **Multi-Instance Source of Truth (Windows / macOS / Linux / Antigravity)**  
> This file stores distilled architectural decisions, stack invariants, and critical root-cause findings. Update it **only** when a task reveals a critical technical constraint or architectural decision (`superpowers:multi-instance-handoff`). Never dump raw logs or OS-specific absolute paths here.

---

## 1. Detected Stack & Tooling Baseline (`hera`)

- **Python Runtime & Package Manager:** Python `>=3.11` managed deterministically via `uv` (`pyproject.toml` + `uv.lock`, build backend `hatchling`).
- **Core Python Libraries:** `pydantic>=2.0`, `mcp>=1.0.0`, `aiosqlite>=0.20.0`, `click>=8.0.0`, `httpx>=0.27.0`, `structlog>=24.0.0`, `mutagen>=1.47.0`, `streamlit>=1.35.0`, `google-antigravity>=0.1.14`, `google-genai>=2.20.0`.
- **Optional Audio DSP (`[project.optional-dependencies.analysis]`):** `librosa>=0.10.0`, `pyacoustid>=1.3.0`, `numpy>=1.26.0`, `soundfile>=0.12.0`.
- **Rust Edge/Native DSP Node (`hera-rs/Cargo.toml`):** `hera-node-rs` (`v0.2.0`, Rust Edition 2021) using `axum 0.7`, `symphonia 0.5`, `rustfft 6.2`, dual-runtime feature flags `native` (`tokio 1.40`) and `cloudflare` (`worker 0.4`).
- **Quality & Verification Commands:**
  - **Python Lint:** `uv run ruff check .`
  - **Python Format Check:** `uv run ruff format --check .`
  - **Python Typecheck:** `uv run mypy src/`
  - **Python Test Suite:** `uv run pytest`
  - **Rust Check & Tests:** `cargo check --manifest-path hera-rs/Cargo.toml` / `cargo test --manifest-path hera-rs/Cargo.toml`

---

## 2. Core Architectural Invariants

1. **Quarantine First:** All external downloads land in `quarantine/` first; tracks are promoted to `library/` only after passing zero-trust FFT spectral validation (zero synthetic sine waves or fake upscales).
2. **4-Layer Unified Holistic Synergy:** Crate sequencing combines Camelot Wheel harmonic compatibility, gradual BPM tempo curves, transient punch matching (Crest Factor), and 64D style embeddings (`u64` SimHash POPCNT).
3. **Open-Core Repository Boundary (`hera` <-> `hera-cloud`):**
   - `hera` (Public Repo): Local/VPS/Edge client, DSP forensics, MCP server, and P2P community node (`share_on_p2p = true` by default).
   - `hera-cloud` (Private Repo): SaaS control plane, Cloudflare R2 CAS vault, paid tier enforcement (`beta_mvp` vs `pro_monthly`), and private unit economics. Never leak private SaaS pricing/admin routes into `hera`.
4. **Cross-Platform Multi-Instance Portability:** All code, configs, plans, and docs must use repository-relative paths (`pathlib.Path`) and regular files in `.agent/skills/` and `.claude/skills/` (zero OS symlinks, zero hardcoded drive letters or home directories).

---

## 3. Chronological Decision Log

### [2026-11-08] ADR-001: Adoption of `superpowers-odd` Multi-Instance Workflow (ODD + RDD)
- **Context:** Engineering takes place across three distinct environments (Windows, macOS, and Linux/Antigravity). OS symlinks fail on Windows without Administrator permissions, and long sessions suffer from context rot.
- **Decision:**
  1. Forked `obra/superpowers` to `davidcastilloc/superpowers-odd` with custom `multi-instance-handoff` master skill and ODD/RDD gates.
  2. Installed skills as regular tracked files in `.agent/skills/` and `.claude/skills/` (zero OS symlinks).
  3. Established `docs/superpowers/{specs,plans,DECISIONS.md}` as the Git-backed synchronization state across all instances.
  4. Enforced Semantic Release / Conventional Commits under `git config user.name "david"` and removed hardcoded Windows paths from `AGENTS.md`.

### [2026-11-08] ADR-002: Test Suite Environment Requirements (`--all-extras` & Temp Dir Isolation)
- **Context:** During Phase 3 zero-regression verification:
  1. `tests/unit/test_transient_synergy_regression.py` requires `librosa`, which is declared under `[project.optional-dependencies] analysis`.
  2. Both `hera/hera-rs` and `hera-cloud` use `std::env::temp_dir().join("hera_local_p2p_test_dir")` in `test_multi_storage_r2_quota_zero_oauth_gdrive_and_local_p2p_seeder`.
- **Decision:**
  1. Always initialize the Python development environment with `uv sync --all-extras` so `dev` (`pytest`, `ruff`, `mypy`) and `analysis` (`librosa`, `soundfile`) extras are present.
  2. Run `cargo test` in `hera-rs` and `hera-cloud` sequentially when both repositories are mounted on the same host to avoid `/tmp/hera_local_p2p_test_dir` race conditions.
