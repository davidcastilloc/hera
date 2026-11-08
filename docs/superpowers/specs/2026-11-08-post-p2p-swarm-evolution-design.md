# Design Spec: Post-P2P Swarm Evolution (5-Layer Incremental Architecture)

**Date:** 2026-11-08  
**Author:** David Castillo (`@davidcastilloc`, `vikruzdavid@gmail.com`)  
**Repositories:** `davidcastilloc/hera` & `davidcastilloc/hera-cloud`  
**Reference:** `especificacion_arquitecturas_post_p2p_ia.md`

---

## 1. Objective & Zero-Regression Principle

Evolve the Hera Federated R2 Swarm into a modern **5-Layer Post-P2P Architecture** (inspired by Iroh, BitTorrent v2 BEP 52, Automerge CRDTs, and Media over QUIC `MoQ`) **one self-contained iteration at a time**:
- **100% Backwards Compatible:** Every new struct field uses `#[serde(default)]` and every new capability is additive (`p2p_exchange.rs` + `p2p.rs` + `lib.rs` + `mcp.rs` + `code_mode.rs`), ensuring all 56 Python tests and all 7 Rust tests remain green at every step.
- **Zero External Crate Bloat:** Implemented in 100% deterministic pure Rust so both `native` (`tokio`/`axum` binary) and `cloudflare` (`wasm32-unknown-unknown` Worker) targets compile cleanly without adding heavy native-only C/UDP dependencies.
- **On-the-Spot Verification (RDD):** Each of the 5 iterations has its own dedicated unit test and live terminal simulation before moving to the next iteration.

---

## 2. Detailed Breakdown of the 5 Incremental Improvements

### Iteración 1 (Capa 3 — BitTorrent v2 BEP 52 / Iroh): Streaming Merkle Proofs por Sub-Bloque (`16 x 256 KiB`)
- **Punto de mejora:** Actualmente validamos `audio_sha256` del archivo completo o una rebanada fija de 64 bytes (`Proof-of-Custody`), pero al descargar un shard de `4 MB` (`4,194,304` B) el receptor no puede verificar sub-fragmentos al vuelo desde el primer kilobyte.
- **Diseño técnico sin romper nada:**
  - Dividir lógicamente cada shard de `4 MB` en **16 hojas de `256 KiB` (`262,144` bytes)** y construir un **Árbol de Merkle Binario SHA-256** de 4 niveles ($\log_2(16) = 4$ hermanos por prueba).
  - Agregar `#[serde(default)] pub merkle_root_sha256: String` a `SwarmRedirectTicket` y `BrokerIntelRecord`.
  - Exponer `compute_shard_merkle_tree`, `generate_merkle_inclusion_proof(leaf_index)` y `verify_merkle_inclusion_proof` en `src/p2p_exchange.rs`.
  - En `GET /api/v1/swarm/shard/:blob_id` (con `?leaf_index=0..15`), devolver la cabecera `X-Hera-Merkle-Proof` con el JSON de `MerkleInclusionProof` para que el cliente verifique la hoja recibida en el kilobyte cero sin descargar el resto del archivo.
- **Demostración en el acto:** Test y simulación pidiendo la hoja `leaf_index = 3`, verificando `verify_merkle_inclusion_proof == true` en `< 0.1 ms` y comprobando que si un solo bit de la hoja se altera, la prueba de Merkle falla al instante.

### Iteración 2 (Capa 2 — Estilo Iroh): Identidad Criptográfica (`endpoint_pubkey_id`) + Migración de Red (`Wi-Fi -> 5G`) y Stateless Relay CGNAT
- **Punto de mejora:** Actualmente `P2PPeerNode` depende de `node_id` y `endpoint_url`. Si la laptop de un DJ cambia de red (`Wi-Fi del hotel -> Hotspot 5G en el club`), su IP/URL cambia y las rutas quedan desactualizadas; o si ambos extremos están en CGNAT simétrico, necesitan un *Relay* sin estado inmediato.
- **Diseño técnico sin romper nada:**
  - Agregar `#[serde(default)] pub endpoint_pubkey_id: String`, `#[serde(default)] pub network_generation: u64`, `#[serde(default)] pub relay_assisted_url: Option<String>`, y `#[serde(default)] pub ewma_rtt_ms: f32` a `P2PPeerNode` y `SwarmHeartbeatRequest`.
  - Derivar `endpoint_pubkey_id` determinísticamente (`ed25519-hera-{sha256(owner_dj + node_id)[..24]}`) si no se provee.
  - Cuando un nodo existente envía un `SwarmHeartbeatRequest` con el mismo `endpoint_pubkey_id` pero un nuevo `endpoint_url` (cambio de IP/red), incrementar `network_generation`, migrar su ruta física en `< 1 ms` preservando 100% de sus `credits_earned_mb`, `edge_guardian_active` y asignación de *Water-Filling*, y asignarle automáticamente un **Stateless Worker Relay (`relay_assisted_url`)** cuando `behind_cgnat == true`.
- **Demostración en el acto:** Simular a `DJLuna` cambiando de `https://wifi-hotel.local` a `https://5g-festival-booth.mobile` con la misma `endpoint_pubkey_id`: verificar que `network_generation` sube a `2`, su ruta migra sin perder ni un megabyte de crédito y su túnel *Relay* queda activo.

### Iteración 3 (Capa 1 y 2 — QUIC / HTTP/3 Striping): `Latency-Aware P2C` + Manifiesto Paralelo Multi-Nodo (`Swarm Striping`)
- **Punto de mejora:** `select_node_p2c` hoy solo compara `daily_worker_requests_used` y `r2_used_mb`, ignorando la latencia (`ewma_rtt_ms`), y emite tickets de un solo bloque a la vez.
- **Diseño técnico sin romper nada:**
  - Actualizar la función de costo de `select_node_p2c` para ponderar:
    $$\text{LoadScore}(n) = 0.60 \cdot \frac{\text{reqs}_n}{85{,}000} + 0.25 \cdot \frac{\text{ewma\_rtt\_ms}_n}{250.0} + 0.15 \cdot \frac{\text{r2\_used\_mb}_n}{9{,}000.0}$$
  - Crear `POST /api/v1/swarm/parallel-manifest` (`issue_parallel_swarm_manifest` en `p2p_exchange.rs` y MCP `swarm_parallel_manifest` / Code Mode `hera.swarmParallelManifest`): dado un `track_id_or_cas` y `file_size_mb` (ej. `38 MB` = 10 bloques de `4 MB`), reparte los 10 bloques en paralelo (*round-robin P2C striping*) entre los $K$ mejores nodos sanos sobre HTTP/3 QUIC, devolviendo los 10 tickets firmados con su `merkle_root_sha256` listos para descarga concurrente sin *Head-of-Line Blocking*.
- **Demostración en el acto:** Pedir el manifiesto paralelo de un Studio Master de `38 MB` (`10 bloques x 4 MB`) en un enjambre de 4 nodos y demostrar cómo los 10 bloques se distribuyen entre distintos Workers con menor RTT y menor uso diario.

### Iteración 4 (Capa 4 — Estilo Automerge / Hypercore): Sincronización Delta-CRDT Libre de Conflictos (`POST /api/v1/swarm/crdt-sync`)
- **Punto de mejora:** `POST /api/v1/p2p/federate-sync` envía catálogos completos. Si dos nodos operan *offline* y actualizan contadores de popularidad (`heat_score`), créditos o pistas simultáneamente, necesitan convergencia matemática conmutativa, asociativa e idempotente ($\text{merge}(A, B) = \text{merge}(B, A)$).
- **Diseño técnico sin romper nada:**
  - Implementar en `p2p_exchange.rs` / `p2p.rs` el motor **Delta-CRDT (`SwarmCrdtDelta`)** compuesto por:
    1. **`GCounter` vectorial por `(cas_fingerprint, node_id)`** para `heat_score` y descargas: la fusión toma `max(local_count, remote_count)` por cada par `(cas, node_id)`. Reenviar el mismo delta 10 veces produce **exactamente el mismo resultado (idempotencia 100%)**.
    2. **`LWW-Element-Set` determinista por `cas_fingerprint`** para pistas y Hot Cues: ordena por `(logical_timestamp, quality_score, hera_seal)` con sello criptográfico verificado.
  - Exponer `POST /api/v1/swarm/crdt-sync` (más MCP `swarm_crdt_sync` y Code Mode `hera.swarmCrdtSync`).
- **Demostración en el acto:** Simular dos nodos que modifican el estado desconectados y aplicar `merge(A, B)` vs `merge(B, A)` + re-aplicación duplicada (`merge(A, A)`), comprobando convergencia exacta e idempotencia.

### Iteración 5 (Capa 5 — Estilo Media over QUIC `MoQ`): Pub/Sub Semántico por Namespace Armónico (`hera://swarm/<camelot>/<bpm_band>`)
- **Punto de mejora:** Actualmente los agentes y nodos descubren nuevos tracks compatibles únicamente haciendo *polling* de búsqueda.
- **Diseño técnico sin romper nada:**
  - Crear suscripciones semánticas ligeras (`SwarmTopicSubscription`: `subscriber_dj`, `camelot_key`, `bpm_min`, `bpm_max`, `max_simhash_hamming`).
  - Exponer `POST /api/v1/swarm/subscribe` y entregar automáticamente los eventos coincidentes (`matched_pubsub_announcements`) dentro de la respuesta de `POST /api/v1/swarm/heartbeat` y `GET /api/v1/swarm/subscriptions/:dj_handle`.
- **Demostración en el acto:** Suscribir a `DJFoo` al *namespace* `hera://swarm/8A/120-125`, anunciar un nuevo Studio Master `8A @ 122 BPM`, y ver cómo el siguiente `heartbeat` de `DJFoo` recibe proactivamente el track + su `opaque_blob_key` y su `merkle_root_sha256` sin hacer *polling*.
