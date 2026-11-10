# HERA v0.6.x (Capa 5) — Real-Time Audio Safety, True Scale Wiring & Backend Hardening Design

**Date:** 2026-11-10  
**Repository:** `davidcastilloc/hera` (Public Engine `hera-rs`, IPC Guardian `hera-ipc`, Desktop App `hera-desktop`)  
**Companion Private Repo:** `davidcastilloc/hera-cloud`  
**Status:** Approved for Surgical Implementation (`superpowers-odd`)

---

## 1. Problem Statement & Empirical Baseline

Commits `8444673`, `fa4c265`, and `d66b5e6` completed Milestones `M1.1`–`M4.4` with `42/42` Rust tests passing (`hera-node-rs` + `architectural_ratchets` + `golden_masters`). However, a post-merge audit against `rust-skills` (265 rules), `rust-patterns`, `rust-architect`, `hexagonal-architecture`, and `workers-best-practices` identified critical *Expand-without-Migrate* gaps and real-time/concurrency hazards:

1. **Real-Time Audio & 120 Hz Telemetry Hazards (`P0`):**
   - `CommandQueue::try_push(&self)` (`hera-rs/src/audio_engine/command_queue.rs:52-61`) implements a non-CAS SPSC ring buffer (`Acquire` load of `write_idx` followed by `Release` store of `w + 1`), yet is exposed via `&self` inside `Arc<SharedAudioAtomics>` (`hera-rs/src/audio_engine/hub.rs:28, 129, 164`) to concurrent async Tauri IPC commands — causing a multi-producer race on `write_idx`.
   - Replacing a track on `DeckA` (`hub.rs:106-109, 240-242`) drops the UI `Arc` first, leaving `RealtimeRenderWorker` on the `cpal` OS audio thread holding the last `Arc<PreallocatedDeckBuffer>` (`~138 MiB`), which triggers `free()` inside the real-time audio callback.
   - `DeckB` playback (`hub.rs:243-263`) bypasses `MasterDspBus` (`master_vol`, `fundsp` 3-band EQ, `Tanh` limiter) and `RealTimeSpectrumAnalyzer` whenever `DeckA` is paused.
   - `subscribe_audio_telemetry` (`hera-desktop/src-tauri/src/plugins/hera_dj/audio_telemetry.rs:106-125`) ticks every `8 ms` (`125 Hz`) calling `unread_latest.take()`; between `10–21 ms` `cpal` callbacks, intermediate ticks see `None` while `is_playing == true` and emit `synthesize_telemetry_frame_from_snapshot` (alternating real `realfft` spectrum bars with synthetic sine waves).
2. **Desktop Startup Race & Legacy `.unwrap()` Panics (`P0`):**
   - `hera-desktop/src-tauri/src/plugins/db.rs:470-486` spawns `setup().await` asynchronously and returns `Ok(())` *before* calling `app_handle.manage(DBState(...))`, risking a runtime panic (`state() called before manage()`) on early frontend/stream-server queries, while serializing all reads behind a single `Mutex<SqliteConnection>` and registering `get_playlist` twice.
   - Legacy `src-tauri` plugins contain crash-prone `.unwrap()` calls (`sleepblocker.rs:23` on headless Linux without D-Bus; `db.rs:410` `diff_paths(...).unwrap()` when exporting M3U across `C:/` and `D:/` drives on Windows; `stream_server.rs` header/socket `.unwrap()`s).
3. **Node Security, Ticket Clocks & Blocking I/O on Async Tokio (`P0`):**
   - `SWARM_TICKET_MASTER_SECRET` (`hera-rs/src/p2p/models_core.rs:78`), hardcoded `issued_at_unix = 1_762_600_000` (`routing_tickets.rs:115`), client-controlled `?now_unix=` (`router.rs:410`), and unchecked `&audio_sha256[..10]` (`routing_tickets.rs:120`) expose ticket forgery/non-expiration and slice panics.
   - `hera-node` binds `0.0.0.0:8080` (`main.rs:91`) exposing unauthenticated `scan-local-dir`, `p2p/download/{track_id}`, and `federate-sync`; `dj_brain/http_client.rs:259` passes `Authorization: Bearer <key>` on `curl` `argv`; `mcp/dispatch_core.rs:23` runs blocking `git pull --rebase`; and `router.rs` runs synchronous Symphonia/FFT/2-Opt DSP directly on Tokio async workers.
4. **Capa 4 *Expand-Without-Migrate* Debt (`P1`):**
   - **`usearch` HNSW (`synergy_hnsw.rs`):** Called only in test-only `optimize_crate_2opt`; 100% of production paths call `optimize_crate_with_spine` (`synergy.rs:362`), bypassing HNSW. Moreover, `query_usearch_hnsw_indices` rebuilds the C++ `usearch::Index` from scratch on every call ($O(N \log N)$) for a single global centroid query that collapses opposite Camelot keys.
   - **`iroh` QUIC (`p2p/quic_iroh.rs`):** Zero production callers; `transfer_verified_shard_loopback` requires the caller to already hold `shard_bytes` in RAM, spawns two ephemeral `iroh::Endpoint`s per call, and lacks `tokio::time::timeout` guards.
   - **`candle` CPU Model (`dj_brain/candle_local.rs`):** Reallocates 4 `Tensor`s with hardcoded `sin`/`cos` formulas on every call (`no OnceLock`), and `turn_eval.rs:193` passes constant dummy arguments `(base_bpm, base_bpm + 1.5, 0.76, 0.84, 0.94, 0.04)`.
5. **MCP Triplication, Unbounded RAM State & Lint/Format Drift (`P2`):**
   - Three separate JSON-RPC dispatchers (`rmcp_server.rs`, `mcp/mod.rs`, and `hera_agent/mcp_http.rs` advertising `"0.5.0"` and mishandling generic notifications); unbounded `delivery_receipts` linear scan; `f32` quota drift; `27` `clippy` warnings and `cargo fmt --check` diffs.

---

## 2. Target Architecture & Design Decisions (`ADR-030`)

### 2.1 Lock-Free RT Consumer + Producer-Serialized `CommandQueue` & Off-Thread Buffer GC
- **Producer-Only Serialization:** `CommandQueue` adds `push_lock: std::sync::Mutex<()>` (initialized in `pub const fn new()`). `try_push(&self)` acquires `push_lock` on non-RT threads so `write_idx` updates are strictly serialized without `unsafe`, while `try_pop(&self)` on the `cpal` real-time callback **never touches `push_lock`** — remaining 100% wait-free and allocation-free.
- **Zero-`free()` Real-Time Deck Swap:** `DualDeckEngine::replace_deck(&mut self, deck: DeckId, buffer: Arc<PreallocatedDeckBuffer>) -> Option<Arc<PreallocatedDeckBuffer>>` returns the displaced buffer (`load_deck` delegates to `replace_deck`). `RealtimeRenderWorker` pushes displaced `Arc`s into a 16-slot `rtrb::Producer<Arc<PreallocatedDeckBuffer>>` (`retired_buf_tx`), drained and dropped on the non-RT controller thread inside `AudioControllerHub::drain_retired_buffers()`.
- **Dual-Deck Bus & Ballistic 120 Hz Telemetry:** `RealtimeRenderWorker` runs `MasterDspBus` and `RealTimeSpectrumAnalyzer` whenever *either* `DeckA` or `DeckB` is active. `subscribe_audio_telemetry` retains `last_real_frame` and applies smooth `0.96` ballistic decay on intermediate `8 ms` ticks between hardware callbacks, reserving synthetic frames strictly for `"headless_clock"`.

### 2.2 Synchronous `SqlitePool` Registration & Crash-Free Desktop Plugins
- `plugins/db.rs` runs `tauri::async_runtime::block_on(setup())` inside `.setup()` *before* returning `Ok(())`, upgrades `DB` to `sqlx::SqlitePool` (`max_connections(8)` with WAL), deduplicates `get_playlist` in `generate_handler!`, falls back to absolute track paths when `pathdiff::diff_paths` returns `None` across Windows drive roots, and eliminates production `.unwrap()`s in `sleepblocker.rs` and `stream_server.rs`.

### 2.3 Loopback-Default Node Security, `curl --config -`, & `spawn_blocking` Offload
- `hera-node` defaults to `127.0.0.1:8080`; binding non-loopback addresses requires `HERA_NODE_API_TOKEN` for local FS/federation mutations.
- `P2PMeshState` uses runtime secrets + `current_unix_secs()` (with test-only `set_clock_override_unix`), UTF-8-safe `.chars().take(10)` prefixing, `2 MiB` JSON `DefaultBodyLimit`, stdin `curl --config -`, read-only `ensure_hera_updated`, and `tokio::task::spawn_blocking` around CPU-bound DSP/2-Opt handlers.

### 2.4 True Capa 4 Production Integration (*Migrate* Phase)
- **Cached Multi-Probe `usearch` HNSW in `optimize_crate_with_spine`:**
  - Triggered only when `candidates.len() > HNSW_PREFILTER_THRESHOLD` (`64`), guaranteeing `0` diffs on the 6-track `golden_masters.rs` snapshot!
  - Caches the built `usearch::Index` in `CachedHnswCatalog` keyed by a 64-bit FNV-1a digest of candidate `(track_id, bpm_bits, camelot_key, embedding_len)` so repeated queries on a `10,000`-track library execute in `< 1 ms` without rebuilding the C++ graph.
  - Queries 3 diverse probes (low-BPM opener quartile, median harmonic bridge, high-BPM peak quartile) AND unconditionally retains all `SetSpineConfig.anchors` `track_id`s.
- **Persistent `IrohQuicAdapter` With Timeouts & Honest P2P Fallbacks:**
  - Holds a reusable `Arc<iroh::Endpoint>`, enforces `5s` `tokio::time::timeout` on `accept`/`connect`/`read_to_end` (removing unbounded `conn.closed().await` hangs), exposes `serve_shard_once` + `fetch_remote_shard`, and wires into `p2p_download.rs` while marking offline preview WAVs with `quarantine_verified: false` unless explicitly requested in demo mode.
- **`OnceLock` `CandleLocalDjModel` With Real Track Telemetry:**
  - Caches `CandleLocalDjModel` in `OnceLock` (`0` per-call tensor allocations) and feeds real candidate transition metrics from `turn_eval.rs`.

---

## 3. Invariants & Guardrails

- **Immutable Golden Masters:** All 4 `.snap` files in `hera-rs/tests/snapshots/` MUST pass `cargo test -p hera-node-rs --test golden_masters` with `0` changes (never run `cargo insta accept`).
- **Architectural Ratchets:** Every `.rs` file in sliced directories MUST stay `<= 349` LOC; `EXPECTED_TAURI_COMMANDS_COUNT` (`58`) and `MAX_MANUAL_EXPORTED_TYPES_IN_BRIDGE_HERA` (`3`) MUST remain satisfied.
- **Swarm-First Invariant:** `beta_mvp`, `free`, and `local_unlimited` nodes always keep `p2p_sharing_enabled = true`.
