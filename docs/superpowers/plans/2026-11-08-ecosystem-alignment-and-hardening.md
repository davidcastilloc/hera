# Implementation Plan: Hera Ecosystem Alignment, Swarm MCP Exposure & Static Type Hardening

**Date:** 2026-11-08  
**Spec:** `docs/superpowers/specs/2026-11-08-ecosystem-alignment-and-hardening-design.md`  
**Author:** David Castillo (`@davidcastilloc`, `vikruzdavid@gmail.com`)

---

## Tasks

- [x] **Task 1: Create `hera-rs/src/p2p_exchange.rs`, Fix `README.md` Manifesto Links, and Isolate Test Temp Dir**
  - Create `hera-rs/src/p2p_exchange.rs` and register `pub mod p2p_exchange;` in `hera-rs/src/lib.rs` so all documentation links (`docs/FEDERATED_R2_SWARM.md`, `hera-rs/README.md`, `AGENTS.md`, `CLAUDE.md`) and documented helper symbols (`compute_water_filling_allocations`, `compute_elastic_replication_factor`, `derive_blind_shard_key`, `issue_custody_challenge`) resolve cleanly and are unit-tested.
  - Fix `../../MANIFESTO.md` -> `MANIFESTO.md` in `README.md`.
  - Append `std::process::id()` to `hera_local_p2p_test_dir` in `hera-rs/src/lib.rs` so parallel `cargo test` runs never collide.

- [x] **Task 2: Expose Federated R2 Swarm in MCP (`mcp.rs`), Code Mode (`code_mode.rs`), OpenAPI, and `skills/hera/SKILL.md`**
  - Add `get_swarm_strength`, `swarm_heartbeat`, `swarm_route_ticket`, and `swarm_custody_audit` to `hera-rs/src/mcp.rs` and `hera-rs/src/code_mode.rs` (`hera.getSwarmStrength`, `hera.swarmHeartbeat`, `hera.swarmRouteTicket`, `hera.swarmCustodyAudit`) + OpenAPI 3.1.0 paths.
  - Add unit test assertions in `hera-rs/src/lib.rs` verifying MCP & Code Mode Swarm execution.
  - Update `skills/hera/SKILL.md` with Federated R2 Swarm documentation and sync to `.agent/skills/` and `.claude/skills/` via `scripts/ensure_latest_hera.py`.

- [x] **Task 3: Fix All 42 `mypy` Errors, Fix `agent/tools.py` Signature Bugs, and Harden Headless `desktop/tray.py`**
  - Guard `pystray` import in `src/hera/desktop/tray.py` with `PYSTRAY_BACKEND=dummy` on headless Linux and catch `Exception`.
  - Fix the 3 broken handler invocations in `src/hera/agent/tools.py` (`handle_audit_repertoire`, `handle_build_dj_crate`, `handle_analyze_batch`) and resolve all 42 `uv run mypy src/` errors across `src/hera/`.
  - Add unit test verifying `src/hera/agent/tools.py` tool dispatch and headless `tray.py` import.

- [x] **Task 4: Add `mypy` Gate, Open-Core Leak Guard to `.github/workflows/ci.yml`, and `.github/workflows/release-binaries.yml`**
  - Add `uv run mypy src/` and the Open-Core Security Boundary Check to `.github/workflows/ci.yml`.
  - Create `.github/workflows/release-binaries.yml` to build multi-OS `hera-node` binaries (`Linux`, `macOS`, `Windows`).
  - Run full RDD verification (`ruff`, `mypy`, `pytest`, `cargo check`, `cargo test`) and commit atomically.
