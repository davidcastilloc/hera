# Rust & Cloudflare Workers Modular Architecture & Performance Hardening (`ADR-013`)

**Date:** `2026-11-08`  
**Status:** Implemented & Verified (`v0.5.0+`)  
**Repositories:** `davidcastilloc/hera` (`hera-rs/`) & `davidcastilloc/hera-cloud` (`src/`)

---

## 1. Context & Architectural Diagnosis

Following the `v0.5.0` release (100% Pure-Rust Zero-Python engine, Cloudflare Edge KV/D1/Vectorize primitives in `hera-rs/src/cloudflare_edge.rs`, and the 4-stage Multi-STUN UDP P2P engine), a deep audit using the vendored Rust & Cloudflare Workers skills (`rust-skills`, `rust-patterns`, `rust-architect`, `hexagonal-architecture`, `rust-testing`, `workers-best-practices`, `workers-development`, `workers-profiling`, `durable-objects`, `wrangler`) identified 6 high-leverage improvements:

1. **`5,000x` Disk I/O Amplification in UDP Seeder (`spawn_desktop_udp_p2p_daemon`)**:
   - Reading the entire `~40 MB` audio file into memory on every `8 KiB` UDP packet (`HERA_REQ_CHUNK`).
   - **Resolution**: Added `P2PMeshState::get_local_file_chunk_slice` using `File::open` + `Seek::seek(SeekFrom::Start(start))` + `Read::take(8192)`, reading strictly `8 KiB` per UDP datagram.
2. **Punched Port Capture on Symmetric CGNAT (`request_chunk_from_peer_with_spray`)**:
   - Spraying `HERA_PUNCH_SYN` across 16 candidate ports previously discarded the responding `SocketAddr` from `HERA_PUNCH_ACK`.
   - **Resolution**: Captured `ack_sender` from `sock.recv_from(&mut ack_buf)` so subsequent `HERA_REQ_CHUNK` datagrams target the actual punched port.
3. **Zero-Allocation `2-Opt TSP` Crate Optimizer (`hera-rs/src/synergy.rs`)**:
   - Replaced nested `Vec<Vec<f32>>` with a contiguous L1-cache-friendly 1D matrix (`vec![0.0_f32; n_all * n_all]`) and replaced `order.clone()` inside the 35-pass inner loop with in-place reversals/swaps (`order[i..=k].reverse()`, `order.swap(i, k)`), achieving zero heap allocations during optimization.
4. **Warm Cloudflare Worker Isolate State & Constant-Time Auth (`hera-rs/src/lib.rs`, `hera-rs/src/error.rs`)**:
   - Added `ISOLATE_MESH_STATE: OnceLock<P2PMeshState>` in `hera-rs/src/lib.rs` to preserve in-memory state across warm Worker invocations, injected `HERA_ADMIN_SECRET` from `worker::Env`, and enforced `constant_time_eq` across admin headers, 60s HMAC route tickets (`verify_swarm_redirect_ticket`), and SHA-256 Merkle proofs (`verify_merkle_inclusion_proof`).
5. **Typed Domain & Error Boundary (`hera-rs/src/error.rs`, `hera-rs/src/router.rs`)**:
   - Replaced ad-hoc `(StatusCode, Json<Value>)` error tuples with `Result<T, HeraError>` implementing `axum::response::IntoResponse`, alongside `SubscriptionTier`, `StorageProvider`, and `ForensicVerdict` domain enums.
6. **Modular Decomposition of `hera-rs/src/p2p.rs` (`5,944` LOC -> 6 Cohesive Modules)**:
   - `hera-rs/src/p2p.rs`: Core domain types, `P2PMeshState`, DJ onboarding, local directory scan, and D1 SQL snapshot.
   - `hera-rs/src/p2p/broker.rs`: Internal P2P Data Broker, CAS deduplication (`0.0 MB` R2 copy), Curator Bounty (`+38.0 MB`), and Hot/Cold R2 Eviction.
   - `hera-rs/src/p2p/cloud_links.rs`: Zero-OAuth cloud share normalizers, `compute_cas_fingerprint`, `compute_style_simhash_u64`, `compute_hera_seal`, and SaaS economics.
   - `hera-rs/src/p2p/crypto_merkle.rs`: Pure-Rust FIPS 180-4 SHA-256, RFC 2104 HMAC-SHA256, 16-leaf Merkle Tree, constant-time ticket/Merkle verification, `zk-ctr-sha256-v1` cipher, `Delta-CRDT`, and MoQ topics.
   - `hera-rs/src/p2p/nat_stun.rs`: 4-Stage Multi-STUN classifier (`RFC 5389`), persistent `8 KiB` UDP seeder daemon, Symmetric CGNAT port spray, and live loopback UDP hole-punch verification.
   - `hera-rs/src/p2p/swarm_custody.rs`: Federated R2 Swarm Orchestrator, Constrained Water-Filling, P2C 60s tickets, 64-byte Proof-of-Custody range audits, quarantine, heartbeat, and CRDT/MoQ sync.

---

## 2. Handoff Guide for Next AI Agent

- **All public imports (`crate::p2p::*`) remain 100% backward-compatible** via `pub use` re-exports in `hera-rs/src/p2p.rs`.
- **Where to add new P2P functionality**:
  - **Broker / CAS / R2 Tiering**: Edit `hera-rs/src/p2p/broker.rs`.
  - **Cloud URLs / Fingerprints / Economics**: Edit `hera-rs/src/p2p/cloud_links.rs`.
  - **Crypto / Merkle / CRDT / MoQ Topics**: Edit `hera-rs/src/p2p/crypto_merkle.rs`.
  - **STUN / UDP Hole-Punching / Desktop Seeder**: Edit `hera-rs/src/p2p/nat_stun.rs`.
  - **Federated Swarm / Water-Filling / Proof-of-Custody**: Edit `hera-rs/src/p2p/swarm_custody.rs`.
- **Vendored Skills**: All 10 Rust & Cloudflare skills are physically vendored under `.agent/skills/` and `.claude/skills/`.
