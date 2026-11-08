# Spec: Hera Ecosystem Alignment, Swarm MCP Exposure & Static Type Hardening

**Date:** 2026-11-08  
**Author:** David Castillo (`@davidcastilloc`, `vikruzdavid@gmail.com`)  
**Repositories:** `davidcastilloc/hera` (Public Community) & `davidcastilloc/hera-cloud` (Private SaaS)  
**Status:** Approved

---

## 1. Problem Statement & Audit Findings

Following the merge of the Federated R2 Edge Swarm documentation (`e7f38de` in `hera` and `26f5983` in `hera-cloud`), a cross-repository audit uncovered five concrete gaps:

1. **Module & Link Drift (`src/p2p_exchange.rs` & `MANIFESTO.md`):**
   - `docs/FEDERATED_R2_SWARM.md`, `hera-rs/README.md`, `hera-cloud/README.md`, `AGENTS.md`, and `CLAUDE.md` link to `src/p2p_exchange.rs` and reference helper symbols (`compute_water_filling_allocations`, `compute_elastic_replication_factor`, `derive_blind_shard_key`, `issue_custody_challenge`), whereas all Swarm code currently resides in `src/p2p.rs`.
   - `README.md` in `hera` links to `../../MANIFESTO.md` instead of `MANIFESTO.md`.
2. **D1 Schema Naming Drift (`hera-cloud/README.md`):**
   - `hera-cloud/README.md` lists `swarm_edge_nodes`, `swarm_blind_shards`, and `swarm_custody_challenges`, whereas `migrations/0002_hera_swarm_guardian_and_shards.sql` and `export_d1_sql_snapshot()` define `swarm_node_allocations`, `swarm_opaque_shards`, `swarm_quarantined_nodes`, and `swarm_delivery_receipts`.
3. **MCP, Code Mode & Agent Skill Gap (`/api/v1/swarm/*`):**
   - The HTTP Swarm endpoints (`/api/v1/swarm/*`) are active in `router.rs`, but `src/mcp.rs`, `src/code_mode.rs`, and `skills/hera/SKILL.md` do not yet expose Swarm strength inspection, `Edge Guardian` heartbeats, 60s P2C routing tickets, or 64-byte `Proof-of-Custody` audits to MCP clients.
4. **Python Static Typing (`mypy`), Agent Tool Signatures & Headless `pystray` Stability (`hera`):**
   - `uv run mypy src/` reports 42 errors across 16 files, including 3 real argument-signature bugs in `src/hera/agent/tools.py` (`handle_audit_repertoire`, `handle_build_dj_crate`, `handle_analyze_batch`).
   - `src/hera/desktop/tray.py` crashes on import on headless Linux servers when `pystray` is installed without `$DISPLAY`.
   - `hera_local_p2p_test_dir` in `lib.rs` lacks a PID suffix, causing race conditions if `cargo test` runs concurrently across `hera` and `hera-cloud`.
5. **Automated Open-Core Boundary Guard & Multi-OS Binary Releases (`hera`):**
   - `.github/workflows/ci.yml` needs an explicit Open-Core Leak Guard ensuring private SaaS routes (`/api/v1/cloud/economics`, `/api/v1/dj/upgrade-tier`, `X-Hera-Admin-Secret`) never enter the public repo, plus `uv run mypy src/` enforcement and `.github/workflows/release-binaries.yml` for multi-OS `hera-node` binaries.

---

## 2. Architectural Design

1. **Dedicated `src/p2p_exchange.rs` Module (`hera-rs` & `hera-cloud`):**
   - Create `src/p2p_exchange.rs` in both repositories exposing the Federated R2 Swarm API (`WaterFillingNodeAllocation`, `SwarmStrengthReport`, `SwarmRedirectTicket`, `SwarmRouteRequest`, `SwarmRouteResponse`, `ProofOfDeliveryReceipt`, `CustodyChallengeRequest`, `CustodyChallenge`, `CustodyAuditSubmission`, `CustodyAuditResult`, `OutboundPullTicket`, `SwarmHeartbeatRequest`, `SwarmHeartbeatResponse`, `QuarantinedNodeRecord`) and canonical functions (`compute_water_filling_allocations`, `compute_elastic_replication_factor`, `derive_blind_shard_key`, `issue_custody_challenge`), backed by `crate::p2p`.
2. **Swarm MCP Tools & Code Mode SDK (`src/mcp.rs`, `src/code_mode.rs`, `skills/hera/SKILL.md`):**
   - Expose `get_swarm_strength`, `swarm_heartbeat`, `swarm_route_ticket`, and `swarm_custody_audit` in MCP (`tools/list` & `tools/call`) and in the `hera.*` TypeScript Code Mode SDK (`hera.getSwarmStrength`, `hera.swarmHeartbeat`, `hera.swarmRouteTicket`, `hera.swarmCustodyAudit`) + OpenAPI 3.1.0 spec (`GET /openapi.json`).
   - Document the Federated R2 Edge Swarm in `skills/hera/SKILL.md` and sync to `.agent/skills/hera/SKILL.md` and `.claude/skills/hera/SKILL.md`.
3. **Zero-Error `mypy` & Headless Tray Guard (`hera`):**
   - Fix `src/hera/desktop/tray.py` to default `PYSTRAY_BACKEND=dummy` when headless on Linux and catch `Exception` on import.
   - Fix all 42 `mypy` errors across `src/hera/`, including the 3 broken tool handler invocations in `src/hera/agent/tools.py`.
4. **CI Open-Core Boundary Guard & Release Workflow (`hera`):**
   - Enforce `uv run mypy src/` and an Open-Core Leak Guard in `.github/workflows/ci.yml`, and add `.github/workflows/release-binaries.yml`.
