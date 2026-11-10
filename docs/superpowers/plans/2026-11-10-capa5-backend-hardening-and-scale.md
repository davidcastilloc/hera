# HERA Capa 5 — Real-Time Audio, True Scale Wiring & Backend Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate all post-Capa-4 real-time audio races/deallocations, Tauri startup/plugin panics, async Tokio blocking/secret leaks, and *Expand-without-Migrate* gaps (`usearch`, `iroh`, `candle`, MCP) across `hera-rs` and `hera-desktop` with zero regressions on Golden Masters or Architectural Ratchets.

**Architecture:** Execute 7 right-sized (`50–200` LOC) leaf-to-root tasks: harden `audio_engine` + 120 Hz telemetry (`Task 1`), fix `src-tauri` `DBState` startup race + `SqlitePool` + `.unwrap()` panics (`Task 2`), lock down `hera-node` security/clock/`spawn_blocking`/`curl --config -` (`Task 3`), wire cached multi-probe `usearch` HNSW into `optimize_crate_with_spine` (`Task 4`), wire persistent timeout-guarded `IrohQuicAdapter` into P2P downloads (`Task 5`), cache `CandleLocalDjModel` in `OnceLock` with real track features (`Task 6`), and unify MCP JSON-RPC + bounded `u64` byte quotas + zero `clippy`/`rustfmt` debt (`Task 7`).

**Tech Stack:** Rust Workspace (`hera-node-rs` + `hera-ipc` + `hera-desktop` Tauri v2, `axum 0.8.9`, `worker 0.8`, `rtrb 0.4.1`, `realfft 3.5.1`, `fundsp 0.23`, `usearch 2.26.6`, `iroh 1.4.1`, `candle-core 0.11.1`, `rmcp 3.9.0`, `sqlx 0.8.6`, `insta 1.43`).

**Spec:** `docs/superpowers/specs/2026-11-10-capa5-backend-hardening-and-scale-design.md`

## Global Constraints

- Zero OS-dependent user-home or drive-user paths in any code, test, snapshot, or `docs/superpowers/` markdown file (enforced by `ratchet_zero_os_absolute_paths_in_docs_and_plans`).
- Never modify or run `cargo insta accept` on the 4 snapshots in `hera-rs/tests/snapshots/`; `cargo test -p hera-node-rs --test golden_masters` MUST pass with `0` snapshot diffs after every task.
- Every `.rs` file inside `hera-rs/src/{mcp, code_mode, dj_brain, p2p, audio_engine}`, `hera-desktop/src-ipc/src`, and `hera-desktop/src-tauri/src/plugins/{hera_dj, hera_agent}` MUST stay `<= 349` lines (`ratchet_monolith_max_lines_ceiling`). Slice a helper submodule first whenever an edit would push a file above `349` lines.
- Preserve exact Tauri v2 ACL Triad parity (`EXPECTED_TAURI_COMMANDS_COUNT = 58` across `plugins/**/*.rs`, `build.rs`, and `capabilities/main.json`).
- Swarm-First invariant: `beta_mvp`, `free`, and `local_unlimited` profiles MUST always keep `p2p_sharing_enabled = true` and `can_disable_p2p_sharing = false`.

## Review Focus

1. **Multi-producer IPC calls corrupting `CommandQueue`:** Concurrent Tauri async commands calling `CommandQueue::try_push(&self)` without producer serialization overwrite slots; Task 1 pins an 8-thread concurrent stress test asserting zero lost commands while keeping `try_pop(&self)` 100% lock-free.
2. **Real-time `cpal` callback dropping ~138 MiB `PreallocatedDeckBuffer`:** Replacing a deck buffer when `RealtimeRenderWorker` holds the last `Arc` triggers `free()` on the audio thread; Task 1 pins an `Arc::strong_count` test proving `render_stereo_block` retires old buffers to `retired_buf_tx` (`strong_count >= 1` until `drain_retired_buffers()` runs off-thread).
3. **Golden Master preservation when wiring HNSW into `optimize_crate_with_spine`:** `golden_2opt_tsp_and_set_spine_routes` uses `6` tracks and `SpineAnchor` roles (`opener`, `peak`, `closer`); Task 4 gates HNSW on `candidates.len() > HNSW_PREFILTER_THRESHOLD` (`64`) and pins a `200`-track test verifying all `SetSpineConfig.anchors` survive Stage-1 filtering.
4. **Windows cross-drive M3U export panic (`C:/` vs `D:/`):** `pathdiff::diff_paths("D:/Tracks/a.flac", "C:/Playlists")` returns `None`, crashing `export_playlist` at `.unwrap()`; Task 2 pins a unit test verifying fallback to the track's full path string when `diff_paths` returns `None`.
5. **120 Hz spectrum flicker on hardware audio (`cpal`):** Polling `unread_latest.take()` at `8 ms` against `10–21 ms` audio callbacks interleaved fake sine waves; Task 1 pins a unit test verifying ballistic `0.96` hold/decay of the last real FFT frame when `backend_kind != "headless_clock"`.

---

### Task 1: Real-Time Audio Safety — Producer-Serialized `CommandQueue`, Off-Thread Buffer GC, `DeckB` Bus & Ballistic 120 Hz Telemetry (`P0`)

**Files:**
- Modify: `hera-rs/src/audio_engine/command_queue.rs:25-65`
- Modify: `hera-rs/src/audio_engine/mod.rs:86-93`
- Create: `hera-rs/src/audio_engine/buffer_gc.rs` (keeps `hub.rs <= 349` LOC!)
- Modify: `hera-rs/src/audio_engine/hub.rs:15-274`
- Modify: `hera-rs/src/audio_engine/cpal_backend.rs:138-150`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/audio_telemetry.rs:95-134`
- Test: `hera-rs/src/audio_engine/tests.rs` & `hera-desktop/src-tauri/src/plugins/hera_dj/audio_telemetry.rs`

**Interfaces:**
- `CommandQueue` (`#![forbid(unsafe_code)]` preserved):
  - `pub const fn new() -> Self` (adds `push_lock: std::sync::Mutex::new(())`; `try_push(&self, cmd: TransportCommand) -> bool` locks `push_lock`; `try_pop(&self) -> Option<TransportCommand>` NEVER locks `push_lock`).
- `DualDeckEngine`:
  - `pub fn replace_deck(&mut self, deck: DeckId, buffer: Arc<PreallocatedDeckBuffer>) -> Option<Arc<PreallocatedDeckBuffer>>` (`load_deck` calls `let _ = self.replace_deck(deck, buffer);`).
- `buffer_gc.rs`:
  - `pub(crate) fn create_retired_buffer_channel() -> (Producer<Arc<PreallocatedDeckBuffer>>, Mutex<Consumer<Arc<PreallocatedDeckBuffer>>>)` (`capacity = 16`)
  - `pub(crate) fn drain_retired_buffers(rx_mutex: &Mutex<Consumer<Arc<PreallocatedDeckBuffer>>>) -> usize`

- [x] **Step 1: Write failing concurrency, RT zero-drop, `DeckB` bus, and telemetry hold tests**

Add to `hera-rs/src/audio_engine/tests.rs`:
```rust
#[test]
fn command_queue_concurrent_mpsc_pushes_lose_zero_commands() {
    let q = std::sync::Arc::new(CommandQueue::new());
    std::thread::scope(|s| {
        for t in 0..8u64 {
            let q_ref = std::sync::Arc::clone(&q);
            s.spawn(move || {
                for i in 0..8u64 {
                    while !q_ref.try_push(TransportCommand::SeekToSample(DeckId::DeckA, t * 100 + i)) {
                        std::thread::yield_now();
                    }
                }
            });
        }
    });
    let mut popped = Vec::new();
    while let Some(cmd) = q.try_pop() {
        popped.push(cmd);
    }
    assert_eq!(popped.len(), 64);
}

#[test]
fn realtime_worker_retires_replaced_deck_buffer_without_dropping_on_audio_thread() {
    // Load buf1 on DeckA, render 1 block, load buf2 on DeckA, keep Weak::upgrade(&weak1)
    // Assert strong_count(&buf1) >= 1 immediately after worker.render_stereo_block(&mut out)
    // Assert strong_count(&buf1) == 0 only after hub.drain_retired_buffers()
}
```
And add a unit test in `audio_telemetry.rs` asserting `next_interpolated_or_real_frame` decays `rms_left` by `0.96` instead of calling `synthesize_telemetry_frame_from_snapshot` when `backend_kind == "cpal_shared"` and `last_real_frame.is_some()`.

- [x] **Step 2: Run tests to confirm failure**

Run: `cargo test -p hera-node-rs realtime_worker_retires`
Expected: FAIL (`replace_deck` / `drain_retired_buffers` missing; `buf1` dropped inside `render_stereo_block`).

- [x] **Step 3: Implement producer-locked `CommandQueue`, `buffer_gc.rs`, `DeckB` `MasterDspBus` routing, and ballistic telemetry hold**

1. Add `push_lock: std::sync::Mutex<()>` to `CommandQueue` in `command_queue.rs`; acquire it only inside `try_push(&self)`.
2. Add `replace_deck` in `audio_engine/mod.rs`, create `audio_engine/buffer_gc.rs`, and update `hub.rs` (`<= 349` LOC) so `RealtimeRenderWorker::render_stereo_block` pushes `old_buf` into `self.retired_buf_tx` and checks `DeckId::DeckA` OR `DeckId::DeckB` before running `self.master_bus.process_interleaved_in_place` + `self.analyzer.analyze_block`. Drain retired buffers inside `install_buffer`, `snapshot`, and `poll_latest_telemetry`.
3. Log stream errors in `cpal_backend.rs:144` via `eprintln!("[hera-audio] cpal stream error: {err}")` instead of `|_err| {}`.
4. Extract pure helper `resolve_tick_telemetry_frame(...)` in `audio_telemetry.rs` holding and decaying `last_real_frame` (`* 0.96`) on hardware backends when `poll_latest_telemetry()` is `None`.

- [x] **Step 4: Run verification & ratchet suite**

Run: `wc -l hera-rs/src/audio_engine/*.rs hera-desktop/src-tauri/src/plugins/hera_dj/*.rs && cargo test -p hera-node-rs && cargo test -p hera-ipc`
Expected: Every file `<= 349` LOC; all `hera-node-rs`, `golden_masters`, `architectural_ratchets`, and `hera-ipc` tests pass (`exit code 0`).

- [x] **Step 5: Mark `[x]` and commit**

```bash
git add hera-rs/src/audio_engine/ hera-desktop/src-tauri/src/plugins/hera_dj/audio_telemetry.rs docs/superpowers/plans/
git commit -m "fix(audio): serialize CommandQueue producers, retire deck buffers off-RT thread, route DeckB via MasterDspBus, and stop 120Hz sine flicker"
```

---

### Task 2: Synchronous Tauri `DBState` (`SqlitePool`) Startup & Crash-Free Desktop Plugins (`P0`)

**Files:**
- Modify: `hera-desktop/src-tauri/src/libs/database.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/db.rs:30-55, 400-488`
- Modify: `hera-desktop/src-tauri/src/plugins/sleepblocker.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/stream_server.rs`
- Modify: `hera-desktop/src-tauri/src/libs/init.rs:42-52`

**Interfaces:**
- `DB` in `libs/database.rs`: replaces `pub connection: SqliteConnection` with `pub pool: sqlx::SqlitePool` (`SqlitePoolOptions::new().max_connections(8)` with WAL + `busy_timeout(5s)`). `DBState(pub DB)` (`Clone` pool, no outer `tokio::sync::Mutex` needed for concurrent reads!).
- Pure helper `pub fn format_m3u_entry_path(track_path: &Path, playlist_parent: &Path) -> String` in `plugins/db.rs`: uses `pathdiff::diff_paths(track_folder, playlist_parent)` when `Some`, otherwise falls back to `track_path.to_string_lossy().into_owned()` without `.unwrap()`.
- Do NOT add or remove any `#[tauri::command]` function (only remove the duplicate `get_playlist` line inside `tauri::generate_handler![...]` at `db.rs:461` so `EXPECTED_TAURI_COMMANDS_COUNT = 58` stays exact).

- [x] **Step 1: Write failing unit tests for cross-drive M3U path formatting in `plugins/db.rs` and headless `sleepblocker` graceful fallback**

```rust
#[test]
fn format_m3u_entry_falls_back_cleanly_when_diff_paths_is_none() {
    let p = format_m3u_entry_path(
        std::path::Path::new("D:/ClubSets/afro.flac"),
        std::path::Path::new("/var/playlists"),
    );
    assert!(p.contains("afro.flac"));
}
```

- [x] **Step 2: Run check/test to confirm failure**

Run: `cargo check --manifest-path hera-desktop/src-tauri/Cargo.toml` (or `cargo test -p hera-ipc` if GTK dev headers are absent on headless CI; verify `libs/database.rs` & `plugins/db.rs` unit tests if `libwebkit2gtk` is installed, otherwise verify via `cargo check` / `acl_guard`).

- [x] **Step 3: Implement synchronous `block_on(setup())` before `.setup()` returns, `SqlitePool`, cross-drive M3U fallback, and `.unwrap()` removal**

1. Upgrade `DB` (`libs/database.rs`) to `SqlitePool` (`&self.pool` in queries instead of `&mut self.connection`), remove duplicate `get_playlist` at `plugins/db.rs:461`, and replace `tauri::async_runtime::spawn(async move { ... app_handle.manage(...) })` (`db.rs:473-483`) with synchronous `tauri::async_runtime::block_on(setup())` inside `.setup()` so `app_handle.manage(DBState(db))` completes BEFORE `.setup()` returns `Ok(())`.
2. Replace `.unwrap()` calls in `db.rs:export_playlist` (`L410-424`) with `Result<_, MuseeksError>` `?` propagation + `format_m3u_entry_path`.
3. In `sleepblocker.rs`, store `Mutex<Option<NoSleep>>`; log a warning if `NoSleep::new()` fails on headless Linux instead of panicking.
4. Replace `.unwrap()` on HTTP response building / header parsing in `stream_server.rs` with graceful `400`/`500` responses, and add a `// SAFETY: Called at process start in main before spawning worker threads.` comment above `unsafe { env::set_var(...) }` in `libs/init.rs:46`.

- [x] **Step 4: Run ACL triad & workspace verification**

Run: `cargo test -p hera-ipc && cargo test -p hera-node-rs --test architectural_ratchets`
Expected: `58` Tauri commands verified 1:1:1 across `plugins`, `build.rs`, and `capabilities/main.json`; `0` errors.

- [x] **Step 5: Mark `[x]` and commit**

```bash
git add hera-desktop/src-tauri/ docs/superpowers/plans/
git commit -m "fix(desktop): register SqlitePool DBState synchronously in setup and eliminate plugin unwrap panics"
```

---

### Task 3: Node Security, Real Ticket Clock, `curl --config -` & `spawn_blocking` Async Hygiene in `hera-rs` (`P0`)

**Files:**
- Modify: `hera-rs/src/p2p/models_core.rs:72-82`
- Modify: `hera-rs/src/p2p/state_init.rs`
- Modify: `hera-rs/src/p2p/crypto_merkle.rs:1-145`
- Modify: `hera-rs/src/p2p/swarm_custody/{routing_tickets.rs, merkle_delivery.rs}`
- Modify: `hera-rs/src/dj_brain/http_client.rs:245-285`
- Modify: `hera-rs/src/cloudflare_edge.rs:695-775`
- Modify: `hera-rs/src/mcp/dispatch_core.rs:20-50`
- Modify: `hera-rs/src/router.rs:116-175, 210-230, 405-435, 630-775`
- Modify: `hera-rs/src/main.rs:80-118`
- Modify: `hera-rs/src/lib.rs:1150-1170`

**Interfaces:**
- `P2PMeshState`:
  - `pub fn set_swarm_secrets(&self, ticket_secret: String, blinding_secret: String)`
  - `pub fn set_node_api_token(&self, token: Option<String>)`
  - `pub fn verify_node_api_token(&self, provided: Option<&str>) -> Result<(), String>`
  - `pub fn set_clock_override_unix(&self, unix_secs: Option<u64>)`
  - `pub fn current_unix_secs(&self) -> u64` (`SystemTime::now()` on `not(target_arch = "wasm32")`, `worker::Date::now().as_millis() / 1000` on `wasm32`)

- [x] **Step 1: Write failing tests for real ticket TTL expiration without `?now_unix=`, short `audio_sha256` slice safety, and `> 2 MiB` JSON rejection**

Add tests in `hera-rs/src/lib.rs` verifying:
1. A ticket issued at `t = 2_000_000` (`state.set_clock_override_unix(Some(2_000_000))`) fails with `403 FORBIDDEN` when `state.set_clock_override_unix(Some(2_000_061))` on `GET /api/v1/swarm/shard/{blob_id}` (with NO `?now_unix=` query parameter).
2. `issue_route_ticket` with `audio_sha256 = "abc"` does not panic.
3. `ensure_hera_updated` MCP tool returns local version status without spawning `git pull --rebase`.

- [x] **Step 2: Run tests to verify failure**

Run: `cargo test -p hera-node-rs ticket_expires_via_server_clock`
Expected: FAIL (`set_clock_override_unix` missing; `?now_unix=` currently required to expire tickets).

- [x] **Step 3: Implement injectable clock, runtime secrets, loopback-default bind, `curl --config -`, read-only `ensure_hera_updated`, and `spawn_blocking` DSP wrappers**

1. Add `swarm_secrets`, `node_api_token`, and `clock_override_unix` to `P2PMeshState` (`state_init.rs`, keeping `<= 349` LOC); replace `issued_at_unix = 1_762_600_000` (`routing_tickets.rs:115`) and `ticket.issued_at_unix + 5` (`merkle_delivery.rs:45`) with `self.current_unix_secs()`, replace `&audio_sha256[..10]` (`routing_tickets.rs:120`) with `audio_sha256.chars().take(10).collect::<String>()`, remove `now_unix` from `ShardDeliveryQuery`, and update `hera-rs/src/lib.rs:1159` to call `state.set_clock_override_unix(...)`.
2. In `dj_brain/http_client.rs` and `cloudflare_edge.rs`, pass URL and `Authorization: Bearer ...` via `curl --config -` over `Stdio::piped()` stdin (`0` secrets in `argv`), align `sync_d1_dj_dna` (`cloudflare_edge.rs:436`) column name to `updated_at_unix` (matching D1 migration `0003_d1_dj_style_dna.sql`), cache `(P2PMeshState, axum::Router)` inside `OnceLock` in `hera-rs/src/lib.rs:25-49` so warm Worker requests do not rebuild the 45-route router, and add `5s` connect/read timeouts + HTTPS scheme rejection on plaintext `TcpStream` in `p2p/federation_sync.rs`.
3. In `mcp/dispatch_core.rs`, replace `Command::new("git").args(["pull", "--rebase", ...])` inside `ensure_hera_updated` with a non-mutating check (`CARGO_PKG_VERSION` + read-only `git rev-parse --short HEAD` with `2s` timeout) so `golden_mcp_21_tools_catalog` keeps all 22 tool schemas unchanged.
4. In `router.rs`, set `DefaultBodyLimit::max(2 * 1024 * 1024)` globally, scope `64 MiB` (`native`) / `16 MiB` (`wasm32`) strictly to `/api/v1/audio/analyze-file`, gate `scan-local-dir` / `p2p/download` / `federate-sync` with `verify_node_api_token` when configured, and wrap CPU-heavy `analyze_pcm_handler`, `analyze_file_handler`, and `build_crate_handler` in `tokio::task::spawn_blocking` (`#[cfg(feature = "native")]`).
5. In `main.rs`, change default bind from `0.0.0.0:8080` to `127.0.0.1:8080` (overridable via `HERA_BIND_ADDR` / `--bind`) and add `.with_graceful_shutdown(tokio::signal::ctrl_c())`.

- [x] **Step 4: Run native, WASM, Golden Master & Ratchet suites**

Run: `cargo check -p hera-node-rs && cargo check -p hera-node-rs --no-default-features --features cloudflare && cargo test -p hera-node-rs`
Expected: `42+` tests pass (`0` Golden Master diffs, `0` Ratchet violations).

- [x] **Step 5: Mark `[x]` and commit**

```bash
git add hera-rs/ docs/superpowers/plans/
git commit -m "fix(security): enforce server-side ticket clocks, loopback-default bind, curl stdin config, and spawn_blocking DSP"
```

---

### Task 4: Wire Cached Multi-Probe `usearch` HNSW Into Production `optimize_crate_with_spine` (`P1`)

**Files:**
- Create: `hera-rs/src/synergy_hnsw_cache.rs` (keeps both HNSW files `<= 349` LOC!)
- Modify: `hera-rs/src/synergy_hnsw.rs`
- Modify: `hera-rs/src/synergy.rs:335-405`
- Modify: `hera-rs/src/lib.rs` (register `pub mod synergy_hnsw_cache;`)
- Modify: `hera-rs/tests/architectural_ratchets.rs` (add `("hera-rs/src/synergy_hnsw_cache.rs", 349)` to `FILE_LINE_CEILINGS`)

**Interfaces:**
- Produces in `synergy_hnsw.rs`:
  - `pub fn prefilter_candidates_hnsw_for_spine(candidates: &[CrateTrackCandidate], max_candidates: usize, spine: &SetSpineConfig) -> Vec<CrateTrackCandidate>`
  - Enriches existing 64-D embeddings by blending normalized harmonic Camelot `(cos, sin)`, normalized BPM, and energy into dims `0..4` followed by L2 re-normalization.
  - Queries 3 diverse probes (`opener` 15th-percentile BPM, `bridge` 50th-percentile BPM, `peak` 85th-percentile BPM) against `synergy_hnsw_cache::query_or_build_cached_hnsw(...)` (`Mutex<Option<CachedCatalogHnsw>>` keyed by 64-bit FNV-1a library fingerprint) and **unconditionally retains every `SpineAnchor.track_id` in `spine.anchors`**.
- Wired at the top of `optimize_crate_with_spine` (`synergy.rs:362`) **only when `candidates.len() > HNSW_PREFILTER_THRESHOLD` (`64`)** (`optimize_crate_2opt` simply delegates directly to `optimize_crate_with_spine` so HNSW never runs twice).

- [ ] **Step 1: Write failing tests for Spine anchor retention on a 200-track library and HNSW cache hit reuse**

```rust
#[test]
fn optimize_crate_with_spine_uses_cached_hnsw_above_64_tracks_and_retains_all_anchors() {
    // Build 200 synthetic CrateTrackCandidates spanning 118..134 BPM and opposite keys 2A / 8A
    // Pin track_0 (118 BPM, 2A) as opener and track_199 (134 BPM, 8A) as peak in SetSpineConfig
    // Call optimize_crate_with_spine(..., Some(12), &spine) twice
    // Assert both anchors appear in result.ordered_tracks AND cache_hits_count() >= 1
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test -p hera-node-rs optimize_crate_with_spine_uses_cached_hnsw`
Expected: FAIL (`synergy_hnsw_cache` not yet implemented; `optimize_crate_with_spine` currently bypasses HNSW).

- [ ] **Step 3: Implement `synergy_hnsw_cache.rs`, multi-probe + anchor-preserving `prefilter_candidates_hnsw_for_spine`, and wire into `optimize_crate_with_spine`**

1. Create `hera-rs/src/synergy_hnsw_cache.rs` (`<= 349` LOC) storing the reusable `usearch::Index` (`#[cfg(feature = "native")]`) keyed by library FNV-1a digest with atomic `cache_hits` telemetry counter for deterministic testing.
2. Update `hera-rs/src/synergy_hnsw.rs` (`<= 349` LOC) to enrich existing `embedding_64d`, run 3-probe retrieval (opener/bridge/peak), and union `spine.anchors` `track_id`s before truncating to `max_candidates`.
3. Call `prefilter_candidates_hnsw_for_spine` inside `optimize_crate_with_spine` when `candidates.len() > HNSW_PREFILTER_THRESHOLD` (`64`), and simplify `optimize_crate_2opt` to delegate directly to `optimize_crate_with_spine`.

- [ ] **Step 4: Verify Golden Masters (`0` diffs!), Ratchets, WASM & Unit Tests**

Run: `wc -l hera-rs/src/synergy_hnsw*.rs && cargo check -p hera-node-rs --no-default-features --features cloudflare && cargo test -p hera-node-rs`
Expected: `golden_2opt_tsp_and_set_spine_routes` passes with `0` snapshot changes; new 200-track HNSW cache + spine test passes; all line ceilings respected.

- [ ] **Step 5: Mark `[x]` and commit**

```bash
git add hera-rs/ docs/superpowers/plans/
git commit -m "feat(curator): wire cached multi-probe usearch HNSW into optimize_crate_with_spine preserving spine anchors"
```

---

### Task 5: Wire Persistent Timeout-Guarded `IrohQuicAdapter` Into P2P Cascade & Honest Quarantine Verification (`P1`)

**Files:**
- Replace/Slice: `hera-rs/src/p2p/quic_iroh.rs` → `hera-rs/src/p2p/quic_iroh/mod.rs` + `hera-rs/src/p2p/quic_iroh/peer_transfer.rs` (`<= 349` LOC each)
- Modify: `hera-rs/src/p2p/nat_stun.rs:245-275`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/p2p_download.rs:146-225`
- Modify: `hera-rs/tests/architectural_ratchets.rs` (add `("hera-rs/src/p2p/quic_iroh", 349)` to `SLICED_SUBMODULE_DIRS`)

**Interfaces:**
- `IrohQuicAdapter`:
  - Holds reusable `endpoint: tokio::sync::OnceCell<iroh::Endpoint>` (`QUIC_STEP_TIMEOUT = Duration::from_secs(5)` on every `accept`, `connect`, `open_bi`, `read_to_end`, and graceful `finish` without unbounded `conn.closed().await`).
  - `pub async fn spawn_shard_provider(&self, store: Arc<RwLock<HashMap<String, Vec<u8>>>>) -> Result<iroh::EndpointAddr, String>`
  - `pub async fn fetch_verified_shard(&self, remote_addr: iroh::EndpointAddr, shard_id: &str, expected_blake3_or_sha256: Option<&str>) -> Result<QuicShardReceipt, String>`
- Honest P2P fallback in `nat_stun.rs` & `p2p_download.rs`:
  - `nat_stun.rs` returns `"HERA_ERR_NOT_FOUND"` when `get_local_file_chunk_slice` is `None` instead of fabricating pseudo-random bytes with a valid SHA-256.
  - `p2p_download.rs` attempts `IrohQuicAdapter` when a QUIC peer ticket/address is present; if no real audio payload is retrieved, only generates `render_harmonic_preview_wav` when `request.peer_endpoint.is_none()` (offline demo preview) AND explicitly sets `quarantine_verified: false` and `stage_used = "OFFLINE_SYNTHETIC_PREVIEW_UNVERIFIED"`.

- [ ] **Step 1: Write failing tests for remote two-adapter `fetch_verified_shard`, timeout enforcement on stalled peers, and `HERA_ERR_NOT_FOUND` on missing UDP chunks**

```rust
#[tokio::test]
async fn iroh_provider_and_separate_leecher_transfer_shard_with_reused_endpoint() {
    // Provider adapter serves shard map; separate leecher adapter fetches two shards sequentially
    // reusing the same underlying iroh::Endpoint; assert sha256 matches and total elapsed < 5s
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test -p hera-node-rs iroh_provider_and_separate_leecher`
Expected: FAIL (`spawn_shard_provider` / `fetch_verified_shard` not yet defined).

- [ ] **Step 3: Implement reusable `IrohQuicAdapter` with `5s` timeouts, wire into `p2p_download.rs`, and remove silent fake-byte masquerading in `nat_stun.rs`**

1. Split `quic_iroh` into `mod.rs` + `peer_transfer.rs` (`<= 349` LOC each), wrapping every async network step in `tokio::time::timeout(QUIC_STEP_TIMEOUT, ...)` and keeping `transfer_verified_shard_loopback` implemented on top of `spawn_shard_provider` + `fetch_verified_shard`.
2. In `nat_stun.rs:255-264`, reply `b"HERA_ERR_CHUNK_NOT_FOUND"` when `get_local_file_chunk_slice` returns `None` instead of synthesizing fake bytes.
3. In `p2p_download.rs`, invoke QUIC shard transfer first when `peer_endpoint` uses `iroh://` or QUIC node addressing, return `Err` if an explicit `peer_endpoint` fails to deliver valid audio magic, and mark offline local demo previews with `quarantine_verified: false`.

- [ ] **Step 4: Verify line counts, unit tests & ratchets**

Run: `wc -l hera-rs/src/p2p/quic_iroh/*.rs hera-rs/src/p2p/nat_stun.rs hera-desktop/src-tauri/src/plugins/hera_dj/p2p_download.rs && cargo test -p hera-node-rs && cargo test -p hera-ipc`
Expected: All files within ceilings; all tests green (`exit code 0`).

- [ ] **Step 5: Mark `[x]` and commit**

```bash
git add hera-rs/ hera-desktop/src-tauri/src/plugins/hera_dj/p2p_download.rs docs/superpowers/plans/
git commit -m "feat(p2p): add persistent timeout-guarded IrohQuicAdapter peer transfer and remove fake-chunk masquerading"
```

---

### Task 6: Cache `CandleLocalDjModel` in `OnceLock` & Feed Real Candidate Telemetry in `turn_eval.rs` (`P1`)

**Files:**
- Modify: `hera-rs/src/dj_brain/candle_local.rs`
- Modify: `hera-rs/src/dj_brain/turn_eval.rs:175-215`

**Interfaces:**
- `CandleLocalDjModel::global_cpu() -> Result<&'static CandleLocalDjModel, String>` backed by `static GLOBAL_CANDLE_DJ_MODEL: std::sync::OnceLock<Result<CandleLocalDjModel, String>> = std::sync::OnceLock::new();` (`0` per-call tensor allocations after first init; optional `HERA_CANDLE_WEIGHTS_JSON` override for calibrated offline weights).
- `turn_eval.rs` extracts real `deck_a` / top candidate `(bpm_a, bpm_b, energy_a, energy_b, camelot_compatibility, vocal_clash_risk)` from the turn context / catalog instead of `(base_bpm, base_bpm + 1.5, 0.76, 0.84, 0.94, 0.04)`.

- [ ] **Step 1: Write failing pointer-identity caching test & distinct-input sensitivity test**

```rust
#[test]
fn candle_global_cpu_reuses_singleton_tensors_and_reacts_to_real_clash_inputs() {
    let m1 = CandleLocalDjModel::global_cpu().expect("cpu model") as *const _;
    let m2 = CandleLocalDjModel::global_cpu().expect("cpu model") as *const _;
    assert_eq!(m1, m2, "OnceLock must reuse identical model allocation");
    let clean = score_transition_neural(124.0, 124.0, 0.75, 0.78, 1.0, 0.0).unwrap();
    let clash = score_transition_neural(124.0, 136.0, 0.90, 0.40, 0.2, 0.95).unwrap();
    assert!(clean.synergy_probability > clash.synergy_probability + 0.15);
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test -p hera-node-rs candle_global_cpu_reuses_singleton`
Expected: FAIL (`global_cpu` missing).

- [ ] **Step 3: Implement `OnceLock` model caching and wire real track pair metrics in `turn_eval.rs`**

1. Add `GLOBAL_CANDLE_DJ_MODEL: OnceLock<...>` and `CandleLocalDjModel::global_cpu()` in `candle_local.rs` (`<= 349` LOC), updating `score_transition_neural` to borrow `CandleLocalDjModel::global_cpu()?`.
2. In `turn_eval.rs` (`<= 349` LOC), derive `(target_bpm, cand_energy, camelot_compat, vocal_risk)` from actual catalog/context tracks when available instead of hardcoded constants.

- [ ] **Step 4: Run verification suite**

Run: `wc -l hera-rs/src/dj_brain/*.rs && cargo test -p hera-node-rs`
Expected: Every `dj_brain/*.rs <= 349` LOC; all tests + Golden Masters pass (`0` diffs).

- [ ] **Step 5: Mark `[x]` and commit**

```bash
git add hera-rs/src/dj_brain/ docs/superpowers/plans/
git commit -m "perf(dj-brain): cache CandleLocalDjModel in OnceLock and score real candidate track transitions"
```

---

### Task 7: Unify MCP JSON-RPC Envelope, Bound `P2PMeshState` (`u64` Quotas + `HashSet` Replay) & Zero Clippy/Fmt Warnings (`P2`)

**Files:**
- Modify: `Cargo.toml` (`[workspace.lints]`)
- Modify: `hera-rs/Cargo.toml`, `hera-desktop/src-ipc/Cargo.toml` (`[lints] workspace = true`)
- Modify: `hera-rs/src/mcp/mod.rs:80-150`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_agent/mcp_http.rs:10-122`
- Modify: `hera-rs/src/p2p/models_core.rs`
- Modify: `hera-rs/src/p2p/state_init.rs`
- Modify: `hera-rs/src/p2p/swarm_custody/{merkle_delivery.rs, routing_tickets.rs}`
- Modify: `hera-rs/src/{synergy_hnsw.rs, structure.rs, synergy.rs, lib.rs}` (resolve all 27 `clippy` warnings)

**Interfaces:**
- Shared JSON-RPC 2.0 protocol rules across `hera-rs/src/mcp/mod.rs` and `hera_agent/mcp_http.rs`:
  - `serverInfo.version` uses `env!("CARGO_PKG_VERSION")` (`"0.6.0"`, fixing stale `"0.5.0"` in `mcp_http.rs:44`).
  - Any request where `req.id.is_none()` OR `req.method.starts_with("notifications/")` returns `202 ACCEPTED` with an empty body (JSON-RPC 2.0 §4.1 notification compliance).
- `P2PMeshState` memory & precision:
  - `redeemed_ticket_ids: Arc<RwLock<HashSet<String>>>` ($O(1)$ replay lookup) + `delivery_receipts: Arc<RwLock<VecDeque<ProofOfDeliveryReceipt>>>` capped at `10_000`.
  - Internal `u64` byte counters (`BYTES_PER_MIB = 1_048_576`) keeping serialized `f32` `*_mb` fields (`#[cfg_attr(feature = "ts", ts(...))]` untouched so `hera-typings.ts` stays 100% compatible).

- [ ] **Step 1: Write failing tests for JSON-RPC notification `202` compliance, `0.6.0` version parity, and `10_000`-cap `VecDeque` receipt pruning**

```rust
#[test]
fn delivery_receipt_history_stays_bounded_at_max_cap_with_o1_ticket_replay_block() {
    // Redeem > 16 tickets with cap configured; assert redeemed_ticket_ids blocks replay in O(1)
    // and receipt deque length never exceeds configured cap
}
```

- [ ] **Step 2: Implement MCP JSON-RPC parity, `HashSet` + `VecDeque` bounded state, `u64` byte math, `[workspace.lints]`, clippy fixes, and `cargo fmt --all`**

1. Fix `mcp_http.rs:33, 44` (`env!("CARGO_PKG_VERSION")` and `req.id.is_none() || req.method.starts_with("notifications/")` -> `StatusCode::ACCEPTED`).
2. Add `redeemed_ticket_ids: Arc<RwLock<HashSet<String>>>` and `VecDeque` cap (`10_000`) in `state_init.rs` + `merkle_delivery.rs`, and compute quota/credit deltas in `u64` bytes before projecting `*_mb` `f32` fields.
3. Add `[workspace.lints.clippy]` in root `Cargo.toml`, fix all 27 `clippy` warnings across `hera-rs` and `hera-ipc`, and run `cargo fmt --all`.

- [ ] **Step 3: Run complete monorepo verification gate**

Run:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --exclude hera-desktop --all-targets -- -D warnings`
- `cargo check -p hera-node-rs --no-default-features --features cloudflare`
- `cargo test -p hera-ipc`
- `cargo test -p hera-node-rs`
Expected: `0` fmt diffs, `0` clippy warnings, `0` Golden Master diffs, `6/6` architectural ratchets green, `42+` tests passing (`exit code 0`).

- [ ] **Step 4: Mark `[x]`, append `ADR-030` in `docs/superpowers/DECISIONS.md`, and commit**

```bash
git add Cargo.toml hera-rs/ hera-desktop/ docs/superpowers/
git commit -m "chore(capa5): unify MCP JSON-RPC notifications, bound P2P receipts with u64 byte math, and enforce zero clippy/fmt warnings"
```
