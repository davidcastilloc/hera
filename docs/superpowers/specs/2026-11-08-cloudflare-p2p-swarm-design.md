# 🌐 HERA Cloudflare P2P Swarm Architecture ("Mientras más nodos de Hera, más fuerte es el sistema de enjambres")

**Date:** 2026-11-08  
**Repositories:** `hera` (`hera-rs/`) & `hera-cloud`  
**Status:** Implemented & Verified

---

## 1. Architectural Invariant

Every new community node (`hera-node`) joining the network must superlinearly multiply the storage capacity, daily request ceiling, fault tolerance, and edge delivery throughput of the entire P2P swarm without ever exposing collaborators to unexpected Cloudflare billing or allowing unauthorized scraping of the global catalog.

---

## 2. Core Design Decisions (`/grill-me` Confirmed)

### Decision 1 — Autonomous Constrained Water-Filling Allocation
- Each community Cloudflare Worker contributes a free tier of `10 GB` (`10,000 MB`) R2 storage and `100,000 req/day`.
- Hard safety ceilings are enforced at `CF_WORKER_SAFE_R2_CAP_MB = 9,000.0 MB` (`9.0 GB`) and `CF_WORKER_SAFE_REQ_CAP = 85,000 req/day` (`85%` of free tier) so collaborators never incur charges.
- `compute_swarm_strength()` executes a constrained Water-Filling pass sorted by ascending effective R2 headroom (`min(safe_r2_cap_mb, free_r2_cap_mb) - r2_used_mb`, zeroed if `daily_worker_requests_used >= safe_req_cap`) so constrained nodes are filled to their exact safety ceiling first and all remaining demand spills over cleanly to unconstrained healthy nodes.

### Decision 2 — Meritocratic Autopilot (`Edge Guardian`) + `1 : 1.5` Turbo Credits + 64-Byte `Proof-of-Custody`
- While a DJ's `hera-node` is `ONLINE_HEALTHY` and `edge_guardian_active == true`, the DJ enjoys unlimited R2 Fast Lane access (`EDGE_GUARDIAN_UNLIMITED_FAST_LANE`).
- Every `1 MB` of aligned shard data served by a community node earns `1.5 MB` of Turbo Credit (`SWARM_TURBO_CREDIT_MULTIPLIER = 1.5`), minted via single-use `ProofOfDeliveryReceipt` headers (`X-Hera-Proof-of-Delivery-Receipt`).
- Random 64-byte range audits (`Range: bytes=offset..offset+63`, `/api/v1/swarm/custody-challenge` -> `/api/v1/swarm/custody-probe/:blob_id` -> `/api/v1/swarm/custody-audit`) verify physical shard possession. Any `404` or SHA-256 slice mismatch immediately revokes `Edge Guardian` (`edge_guardian_active = false`), removes the node from `active_healthy_nodes` and Water-Filling allocations, and falls back to redeeming the DJ's accumulated `1 : 1.5` Turbo Credits (`TURBO_CREDIT_REDEMPTION_FAST_LANE`) until exhausted (`COMMUNITY_STANDARD_LANE`).

### Decision 3 — Global Catalog Blinding & 60-Second HMAC-SHA256 Redirect Tickets
- Shards stored in community R2 buckets use opaqueHMAC-SHA256 keys (`shard-blobs/{hmac}.hbin`) derived from the master signing secret so no external party can enumerate artist/title metadata from R2 object listings.
- Community Workers serve `/api/v1/swarm/shard/:blob_id` only when presented with a valid, non-expired (`60s` TTL), single-use `X-Hera-Swarm-Ticket` (`SwarmRedirectTicket`) signed by the control plane. Missing, tampered, expired, or replayed tickets are rejected with `403 Forbidden`.

### Decision 4 — Total Swarm Synergy
- **Elastic Thermal Replication ($K$):** Base replication scales with healthy node count $N$ ($K=1$ for $N=1$, $K=2$ for $N \in [2,5]$, $K=3$ for $N \ge 6$) and boosts to $K \in [3, 5]$ for hot tracks (`heat_score` driven by `p2p_Verification_count` and `active_seeders`), capped by $N$.
- **Power-of-Two-Choices (`P2C`) Load Balancing:** `select_node_p2c()` deterministically samples two healthy candidate nodes below `CF_WORKER_SAFE_REQ_CAP` and routes to the node with the lower `daily_worker_requests_used`.
- **`4 MB` (`4,194,304` Bytes) Immutable Edge Block Alignment:** Every shard response sets `Cache-Control: public, max-age=31536000, immutable`, yielding an `8.4x` throughput multiplier over Cloudflare's Edge Cache (`~88,473` full `38 MB` tracks/day per free Worker).
- **Double Cryptographic Seal & Anti-Bait-and-Switch Quarantine:** `federate_sync` verifies both `hera_seal` and `audio_sha256` against the canonical CAS fingerprint; mismatches immediately reject the payload and quarantine the offending node in `quarantined_nodes` (`GET /api/v1/swarm/quarantine`).
- **Google Drive `>25 MB` Interstitial Bypass & CGNAT Outbound Pull Tickets:** `build_gdrive_large_file_bypass_url()` appends `&confirm=t`, `build_broker_swarm_index()` ranks hot R2 CAS shards at `priority_rank = 1`, and `/api/v1/swarm/heartbeat` issues `outbound_pull_tickets` for CGNAT nodes.
