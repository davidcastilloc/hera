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
  4. Enforced Semantic Release / Conventional Commits under `git config user.name "David Castillo"` and `git config user.email "vikruzdavid@gmail.com"` (`davidcastilloc`) and removed hardcoded Windows paths from `AGENTS.md`.

### [2026-11-08] ADR-002: Test Suite Environment Requirements (`--all-extras` & Temp Dir Isolation)
- **Context:** During Phase 3 zero-regression verification:
  1. `tests/unit/test_transient_synergy_regression.py` requires `librosa`, which is declared under `[project.optional-dependencies] analysis`.
  2. Both `hera/hera-rs` and `hera-cloud` use `std::env::temp_dir().join("hera_local_p2p_test_dir")` in `test_multi_storage_r2_quota_zero_oauth_gdrive_and_local_p2p_seeder`.
- **Decision:**
  1. Always initialize the Python development environment with `uv sync --all-extras` so `dev` (`pytest`, `ruff`, `mypy`) and `analysis` (`librosa`, `soundfile`) extras are present.
  2. Run `cargo test` in `hera-rs` and `hera-cloud` sequentially when both repositories are mounted on the same host to avoid `/tmp/hera_local_p2p_test_dir` race conditions.

### [2026-11-08] ADR-003: Cloudflare P2P Swarm Water-Filling, Catalog Blinding & Custody Invariants
- **Context:** Scaling the P2P swarm across community-contributed Cloudflare Workers (`10 GB` free R2, `100,000 req/day`) and `hera-cloud` required strict billing safety caps, anti-scraping catalog protection, and zero-trust custody enforcement while keeping `hera-rs` dual-target compatible (`native` binary + `wasm32` Worker).
- **Decision:**
  1. **Constrained Water-Filling & Active Node Filtering:** Cap community Workers at `9,000 MB` R2 (`9.0 GB`) and `85,000 req/day`. Sort nodes by ascending effective headroom during Water-Filling so constrained nodes saturate first and overflow demand spills over to unconstrained nodes. Filter healthy nodes strictly by `edge_guardian_active && custody_status == "ONLINE_HEALTHY"` so revoked nodes immediately drop out of swarm capacity and P2C routing.
  2. **Single-Use 60s Signed Tickets & 4 MB Immutable Shards:** Blind R2 shard keys as `shard-blobs/{hmac}.hbin`, require HMAC-SHA256 `X-Hera-Swarm-Ticket` headers with a 60s TTL and unique `nonce` in `ticket_id`, reject ticket replays against `delivery_receipts`, and serve `4 MB` (`4,194,304` B) blocks with `Cache-Control: public, max-age=31536000, immutable` (`1 : 1.5` Turbo Credit ratio).
  3. **Dual-Target Parity (`hera-rs` <-> `hera-cloud`):** Keep `#[cfg(not(target_arch = "wasm32"))] pub fn push_federation_over_http` in `P2PMeshState` so the native `hera-node` binary (`src/main.rs`) compiles cleanly alongside the Cloudflare Worker target, while keeping `/api/v1/cloud/economics` exclusively in `hera-cloud`.

### [2026-11-08] ADR-004: `p2p_exchange.rs` Canonical Module, Swarm MCP/Code Mode Parity, Static Type Hardening & CI Guards
- **Context:** Auditing the documentation after landing the Federated R2 Swarm (`e7f38de`) revealed 5 gaps: `hera-rs/src/p2p_exchange.rs` was referenced in docs (`docs/FEDERATED_R2_SWARM.md`, `AGENTS.md`, `CLAUDE.md`) without a physical module file; the Swarm endpoints were exposed over HTTP REST but missing in MCP (`mcp.rs`), Code Mode (`code_mode.rs`), OpenAPI, and `skills/hera/SKILL.md`; `uv run mypy src/` had 42 errors (including 3 real handler signature bugs in `src/hera/agent/tools.py` and an unguarded headless `pystray` import in `src/hera/desktop/tray.py`); and `.github/workflows/ci.yml` lacked a `mypy` gate and an Open-Core leak check.
- **Decision:**
  1. **Physical `hera-rs/src/p2p_exchange.rs` Module & Process-Isolated Test Dir:** Created `hera-rs/src/p2p_exchange.rs` re-exporting all Swarm types and providing `compute_water_filling_allocations`, `compute_elastic_replication_factor`, `derive_blind_shard_key`, and `issue_custody_challenge`. Isolated `hera_local_p2p_test_dir` with `std::process::id()` so parallel `cargo test` runs across `hera` and `hera-cloud` never collide.
  2. **Full MCP, Code Mode & OpenAPI Parity for the Swarm:** Exposed `get_swarm_strength`, `swarm_heartbeat`, `swarm_route_ticket`, and `swarm_custody_audit` (`hera.getSwarmStrength`, `hera.swarmHeartbeat`, `hera.swarmRouteTicket`, `hera.swarmCustodyAudit`) across `mcp.rs`, `code_mode.rs`, `/openapi.json`, and `skills/hera/SKILL.md`.
  3. **Zero `mypy` Errors & Headless Linux Safety:** Fixed all 42 `mypy` errors across 19 files (`0 issues in 61 source files`), wired `Database(cfg.db_path)` into `src/hera/agent/tools.py` (`handle_audit_repertoire`, `handle_build_dj_crate`, `handle_analyze_batch`), and guarded `src/hera/desktop/tray.py` with `PYSTRAY_BACKEND=dummy` on headless Linux.
  4. **CI Gate & Multi-OS Binary Releases:** Added `uv run mypy src/` and an automated Open-Core Security Boundary Check to `.github/workflows/ci.yml`, plus `.github/workflows/release-binaries.yml` for `Linux`, `macOS`, and `Windows` `hera-node` builds.
