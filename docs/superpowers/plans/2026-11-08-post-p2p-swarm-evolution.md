# Implementation Plan: Post-P2P Swarm Evolution (Step-by-Step Incremental Verification)

**Date:** 2026-11-08  
**Spec:** `docs/superpowers/specs/2026-11-08-post-p2p-swarm-evolution-design.md`  
**Author:** David Castillo (`@davidcastilloc`, `vikruzdavid@gmail.com`)

---

## Incremental Iterations (Execute & Verify One at a Time)

- [x] **Iteración 1 (Capa 3 — Streaming Merkle Proofs `16 x 256 KiB`):**
  - Implementar árbol de Merkle binario SHA-256 por shard de `4 MB` (`compute_shard_merkle_tree`, `generate_merkle_inclusion_proof`, `verify_merkle_inclusion_proof`) en `src/p2p.rs` y re-exportarlo en `src/p2p_exchange.rs`.
  - Incluir `merkle_root_sha256` en `SwarmRedirectTicket` / `BrokerIntelRecord` y cabecera `X-Hera-Merkle-Proof` en `GET /api/v1/swarm/shard/:blob_id`.
  - Ejecutar `cargo test` y simulación en vivo de verificación en el kilobyte cero antes de pasar a la Iteración 2.

- [x] **Iteración 2 (Capa 2 — Identidad Criptográfica `endpoint_pubkey_id` + Migración `Wi-Fi -> 5G` + CGNAT Relay):**
  - Añadir `endpoint_pubkey_id`, `network_generation`, `relay_assisted_url` y `ewma_rtt_ms` con `#[serde(default)]` en `P2PPeerNode` y `SwarmHeartbeatRequest`.
  - Soportar migración transparente de IP/endpoint en `process_swarm_heartbeat` preservando `Edge Guardian`, créditos y asignación de *Water-Filling*.
  - Ejecutar `cargo test` y simulación en vivo de cambio de red (`Wi-Fi -> 5G`) antes de pasar a la Iteración 3.

- [x] **Iteración 3 (Capa 1 y 2 — `Latency-Aware P2C` + Manifiesto Multi-Nodo `Swarm Striping`):**
  - Ponderar `ewma_rtt_ms` en `select_node_p2c` y crear `POST /api/v1/swarm/parallel-manifest` (`swarm_parallel_manifest` en MCP / `hera.swarmParallelManifest` en Code Mode) para descargar los bloques de `4 MB` de un track desde múltiples Workers en paralelo.
  - Ejecutar `cargo test` y simulación en vivo de descarga distribuida multi-nodo antes de pasar a la Iteración 4.

- [x] **Iteración 4 (Capa 4 — Sincronización Delta-CRDT `GCounter` + `LWW-Element-Set`):**
  - Implementar `SwarmCrdtDelta` y `POST /api/v1/swarm/crdt-sync` (`swarm_crdt_sync` en MCP / `hera.swarmCrdtSync` en Code Mode) con fusión conmutativa, asociativa e idempotente.
  - Ejecutar `cargo test` y simulación en vivo de convergencia *offline-first* (`merge(A, B) == merge(B, A)`) antes de pasar a la Iteración 5.

- [x] **Iteración 5 (Capa 5 — Pub/Sub Semántico Armónico estilo `MoQ`):**
  - Implementar `SwarmTopicSubscription` (`hera://swarm/<camelot>/<bpm_band>`) y `POST /api/v1/swarm/subscribe`, entregando notificaciones proactivas en `SwarmHeartbeatResponse`.
  - Sincronizar con `hera-cloud`, actualizar `DECISIONS.md` y correr la suite RDD completa (`ruff`, `mypy`, `pytest`, `cargo test`).
