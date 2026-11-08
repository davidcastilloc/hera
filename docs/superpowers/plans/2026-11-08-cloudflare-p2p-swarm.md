# 📋 Implementation Plan — Cloudflare P2P Swarm (`hera` & `hera-cloud`)

**Spec:** `docs/superpowers/specs/2026-11-08-cloudflare-p2p-swarm-design.md`  
**Date:** 2026-11-08

---

## Tasks

- [x] **Task 1: Cloudflare D1 Schema Migration (`hera-cloud`)**
  - Create `migrations/0002_hera_swarm_guardian_and_shards.sql` defining `swarm_node_allocations` (`free_r2_cap_mb DEFAULT 10000.0`, `safe_r2_cap_mb DEFAULT 9000.0`, `safe_req_cap DEFAULT 85000`), `swarm_opaque_shards`, `swarm_quarantined_nodes`, and `swarm_delivery_receipts`.
  - Export all four tables in `P2PMeshState::export_d1_sql_snapshot()`.

- [x] **Task 2: Constrained Water-Filling Optimizer, Elastic Thermal Replication $K$, and P2C Load Balancer**
  - Implement `compute_swarm_strength()`, `compute_elastic_replication_k()`, and `select_node_p2c()` in `hera-rs/src/p2p.rs` and `hera-cloud/src/p2p.rs`.
  - Ensure only nodes with `edge_guardian_active && custody_status == "ONLINE_HEALTHY"` are counted as active healthy nodes and receive Water-Filling allocations.

- [x] **Task 3: Global Catalog Blinding (`shard-blobs/{hmac}.hbin`), 60s Signed Tickets, and Replay Protection**
  - Implement `compute_opaque_shard_blob_key()`, `issue_route_ticket()`, `verify_swarm_redirect_ticket()`, and `redeem_swarm_shard_delivery()`.
  - Enforce 60-second TTL, HMAC-SHA256 signature verification, unique `nonce` in `ticket_id`, single-use replay rejection against `delivery_receipts`, and `Cache-Control: public, max-age=31536000, immutable` on `4 MB` (`4,194,304` bytes) aligned blocks.

- [x] **Task 4: Meritocratic Autopilot (`Edge Guardian`), `1 : 1.5` Turbo Credits, and 64-Byte `Proof-of-Custody`**
  - Implement `generate_custody_challenge()`, `probe_custody_64_bytes()`, `verify_custody_audit()`, and `process_swarm_heartbeat()`.
  - Revoke `Edge Guardian` immediately on HTTP `404` or slice SHA-256 mismatch, and fall back to `TURBO_CREDIT_REDEMPTION_FAST_LANE` -> `COMMUNITY_STANDARD_LANE`.

- [x] **Task 5: Double Cryptographic Seal (`hera_seal` + `audio_sha256`), Quarantine Blacklist, & Native Binary Compatibility**
  - Enforce `Bait-and-Switch` detection in `federate_sync()`, expose `/api/v1/swarm/*` routes in `router.rs`, and preserve `#[cfg(not(target_arch = "wasm32"))] pub fn push_federation_over_http` for the `hera-node` CLI binary.

- [x] **Task 6: End-to-End Verification & Cloudflare Worker Deployment**
  - Verify `cargo check` and `cargo test` in both `hera-rs` and `hera-cloud`, plus `uv run ruff check .` and `uv run pytest` in `hera`.
  - Deploy updated Workers to `hera-cloud` and `hera-node`.
