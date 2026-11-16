# OMKF Cloud Edge Deployment, Milestone C4 Live Sync & HERA v0.7.0 Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deploy `hera-cloud` (`OMKF Cloud Sync v1.0`) and D1 migration `0004` to Cloudflare Edge, wire non-blocking background `Milestone C4` cloud synchronization (`omkf_cloud_client.rs` ↔ `omkf_sync.rs` / `scanner_worker.rs`), and ship the signed `v0.7.0` multi-OS release of `hera`.

**Architecture:**
1. **`hera-cloud` Edge Control Plane (`../hera-cloud`):** Apply additive D1 migration `migrations/0004_omkf_crates_and_swarm_kv_index.sql` to remote `HERA_DB` (`hera-p2p-catalog`), build the release Cloudflare Worker WASM binary (`worker-build --release -- --features cloudflare --no-default-features`), deploy via Wrangler (`HERA_KV_CACHE`, `DJ_ACCOUNT_VAULT_DO`, `HERA_DB`), and verify the 5 live `/api/v1/edge/omkf-swarm/*` and `/api/v1/omkf/crates/*` endpoints.
2. **Non-Blocking `Milestone C4` Booth Sync (`hera-desktop/src-tauri`):** Connect `omkf_cloud_client.rs` to `omkf_sync.rs` (`flush_dirty_omkf_capsules` and Zero-Import / fast-pass lookup) using fire-and-forget background tasks (`tokio::spawn`, `4 s` curl timeout, `CREATE_NO_WINDOW`) gated by `HERA_DISABLE_CLOUD_SYNC` so offline DJ booths and unit tests never block or fail when disconnected.
3. **Unified `v0.7.0` Multi-OS Release (`hera`):** Advance workspace version strings from `0.6.1` to `0.7.0` across `Cargo.toml`, `hera-desktop/package.json`, `hera-desktop/src-tauri/tauri.conf.json`, `hera-desktop/src-ipc/src/lib.rs`, and `hera-rs/package.json`, run the full polyglot verification suite, and push signed commit + signed tag `v0.7.0` (`git tag -s v0.7.0`) to trigger `.github/workflows/release.yml`.

**Tech Stack:** Rust (`edition = "2024"` in `hera-desktop`, `edition = "2021"` in `hera-cloud`), `workers-rs 0.8`, `axum 0.8.9`, Cloudflare Workers (`D1`, `KV`, `Durable Objects SQLite`), `Tauri v2` (`2.12.x`), `sqlx 0.8`, `Bun 1.4.x`, `Vite+`.

**Spec:**
- `docs/superpowers/specs/library_scanner_v3.md` (`OMKF v1` `64 B` header + `928 B` slot)
- `../hera-cloud/docs/superpowers/specs/2026-11-16-omkf-sync-and-swarm-kv-design.md` (`Milestones C1–C4`)

## Global Constraints

- **Zero OS-Specific Absolute Paths:** Never commit host user-home or drive-letter paths in code, tests, plans, or `DECISIONS.md` (`ratchet_zero_os_absolute_paths_in_docs_and_plans`).
- **Leaf-First Line Ceilings:** Keep `hera-desktop/src-tauri/src/libs/omkf_sync.rs <= 349` LOC, `hera-desktop/src-tauri/src/libs/omkf_cloud_client.rs <= 260` LOC, and `hera-desktop/src-tauri/src/plugins/hera_dj/scanner_worker.rs <= 349` LOC post-`cargo fmt`.
- **Immutable Migrations & Zero Audio Egress:** Never edit existing migrations (`01..09` in `hera`, `0001..0004` in `hera-cloud`); OMKF cloud sync transmits `0` bytes of raw audio/video to R2 (`HERA_LIBRARY_BUCKET` untouched).
- **Swarm-First & Anti-Poisoning Invariant:** Never publish tracks with `scan_level != 2`, `bpm_x100 < 4500 || bpm_x100 > 22000`, `OMKF_FLAG_IS_FAKE_320KBPS`, `OMKF_FLAG_HAS_AUDIO_SKIPS`, or `OMKF_FLAG_IS_OWN_PRODUCTION` (`bit 5` / `bit 7`), and always scrub `mtime_ns`, `name_offset`, and `name_len` to `0` before leaving the laptop.
- **Signed Git Commits & Tags:** Every commit (`git commit -S`) and release tag (`git tag -s`) must verify with `Good "git" signature` for `vikruzdavid@gmail.com`.

## Review Focus

1. **Offline / Captive-Portal DJ Booth Resilience:** When a DJ is spinning offline or behind a club captive portal (or during deterministic unit tests), background cloud sync in `flush_dirty_omkf_capsules` must never block SQLite writes, delay `.hera/tracks.omkf` local disk flushes, or log unhandled panics; setting `HERA_DISABLE_CLOUD_SYNC=1` (or `cfg(test)`) skips network spawn unless explicitly testing the loopback server.
2. **Private Producer Bounces Never Leak on Flush:** When `flush_dirty_omkf_capsules` serializes a folder containing both verified studio tracks and an Ableton producer bounce (`is_own_production = 1` / `OMKF_FLAG_IS_OWN_PRODUCTION`), `prepare_cloud_sync_payload_from_capsule` must strip the producer bounce from the `CloudSwarmPublishRequest` while still syncing the folder's private `CloudCrateDeltaItem` to the DJ's own `DjAccountVaultDO`.
3. **Append-Only Ledger Preservation During Cloud Delta Push:** When `.hera/crate.md` contains `# Ledger Reversible` lines (`- tx:org_01 | ...`), `extract_crate_delta_from_disk` must parse and forward those `CloudLedgerEntryDto` items so `DjAccountVaultDO` preserves rollback history across machines.
4. **Tauri / IPC / Cargo Semver Triad Parity on `0.7.0` Bump:** Updating `Cargo.toml` to `0.7.0` without updating `hera-desktop/src-ipc/src/lib.rs:31-32` (`assert_eq!(HERA_IPC_VERSION, "0.7.0")`), `hera-desktop/package.json`, and `hera-desktop/src-tauri/tauri.conf.json` breaks `cargo test -p hera-ipc` and `tauri-action` release builds.
5. **Cloudflare D1 Remote Migration Idempotency (`0004`):** Applying `0004_omkf_crates_and_swarm_kv_index.sql` against `hera-p2p-catalog` must use `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS` and pass live HTTP roundtrip smoke verification (`/api/v1/edge/omkf-swarm/publish`, `/api/v1/edge/omkf-swarm/lookup`, `/api/v1/omkf/crates/push-delta`, `/api/v1/omkf/crates/tree-diff`, `/api/v1/omkf/crates/pull/{crate_id}`).

---

### Task 1: Deploy `hera-cloud` (`OMKF Cloud Sync v1.0`) & D1 Migration `0004` to Cloudflare Edge

**Files:**
- Verify (read-only): `../hera-cloud/wrangler.toml:1-53`
- Verify (read-only): `../hera-cloud/migrations/0004_omkf_crates_and_swarm_kv_index.sql:1-35`
- Modify: `../hera-cloud/docs/superpowers/DECISIONS.md` (append live edge deployment receipt to `ADR-020`)

**Interfaces:**
- Consumes: `../hera-cloud/src/router.rs` routes (`POST /api/v1/edge/omkf-swarm/lookup`, `POST /api/v1/edge/omkf-swarm/publish`, `POST /api/v1/omkf/crates/tree-diff`, `POST /api/v1/omkf/crates/push-delta`, `GET /api/v1/omkf/crates/pull/{crate_id}`).
- Produces: Live Cloudflare Worker deployment at `https://hera-node.pc911itservices.workers.dev` backed by remote D1 tables `omkf_swarm_fingerprints` and `omkf_shared_crate_blueprints`.

- [ ] **Step 1: Verify `hera-cloud` WASM release build and test suite before remote changes**

Run in `../hera-cloud`:
```bash
cargo test
cargo check --target wasm32-unknown-unknown --features cloudflare --no-default-features
worker-build --release -- --features cloudflare --no-default-features
```
Expected: `37 passed; 0 failed` lib + `3 passed; 0 failed` `architectural_ratchets`, and `build/worker/shim.mjs` generated with exit code `0`.

- [ ] **Step 2: Apply D1 migration `0004_omkf_crates_and_swarm_kv_index.sql` to remote `HERA_DB` (`hera-p2p-catalog`)**

Run in `../hera-cloud`:
```bash
npx wrangler d1 execute hera-p2p-catalog --remote --file=migrations/0004_omkf_crates_and_swarm_kv_index.sql
npx wrangler d1 execute hera-p2p-catalog --remote --command="SELECT name FROM sqlite_master WHERE type='table' AND name IN ('omkf_swarm_fingerprints', 'omkf_shared_crate_blueprints') ORDER BY name;"
```
Expected: Both `omkf_shared_crate_blueprints` and `omkf_swarm_fingerprints` returned with exit code `0`.

- [ ] **Step 3: Deploy `hera-cloud` Worker to Cloudflare and run live 5-route HTTP smoke test**

Run in `../hera-cloud`:
```bash
npx wrangler deploy
```
Then execute a live smoke check against the deployed URL (`https://hera-node.pc911itservices.workers.dev`) verifying:
1. `POST /api/v1/edge/omkf-swarm/publish` accepts a valid `928 B` `scan_level=2` slot (`accepted >= 1` or `skipped_rate_guard >= 1`) and rejects a poisoned `FAKE_320` slot (`rejected_poisoned == 1`).
2. `POST /api/v1/edge/omkf-swarm/lookup` returns `200 OK` with `content-type: application/octet-stream` and `x-hera-omkf-hits: 1` (`932 B` frame).
3. `POST /api/v1/omkf/crates/push-delta` -> `POST /api/v1/omkf/crates/tree-diff` -> `GET /api/v1/omkf/crates/pull/smoke-crate-01?dj_handle=DaviDJ` returns `200 OK` with `"in_sync": ["smoke-crate-01"]` and preserved `ledger_entries`.

- [ ] **Step 4: Record deployment verification in `../hera-cloud/docs/superpowers/DECISIONS.md` and commit signed**

```bash
git add docs/superpowers/DECISIONS.md
git commit -S -m "docs(decisions): record live Cloudflare D1 0004 and Worker deployment receipt for ADR-020"
git log -1 --show-signature
git push origin main
```

---

### Task 2: Wire Live Non-Blocking `Milestone C4` Cloud Sync (`omkf_cloud_client.rs` ↔ `omkf_sync.rs`)

**Files:**
- Modify: `hera-desktop/src-tauri/src/libs/omkf_cloud_client.rs:1-250`
- Modify: `hera-desktop/src-tauri/src/libs/omkf_sync.rs:205-245`
- Modify: `hera-rs/tests/architectural_ratchets.rs:55` (keep `omkf_cloud_client.rs` ceiling `<= 260` LOC and `omkf_sync.rs <= 349` LOC)

**Interfaces:**
- Consumes: `OmkfCapsule`, `decode_omkf_capsule`, `OmkfTrackSlot` from `hera_node_rs::scanner::omkf_capsule`; `parse_crate_intent_and_rules` from `hera_node_rs::curation_rules`.
- Produces:
  - `pub fn build_crate_delta_and_swarm_batch_from_capsule(dj_handle: &str, crate_id: &str, folder_rel_path: &str, capsule_bytes: &[u8], crate_md: &str, now_ms: i64) -> Result<(CloudPushDeltaRequest, CloudSwarmPublishRequest), String>` in `hera-desktop/src-tauri/src/libs/omkf_cloud_client.rs`
  - `pub fn spawn_background_omkf_folder_cloud_sync(dj_handle: String, crate_id: String, folder_path: std::path::PathBuf)` in `hera-desktop/src-tauri/src/libs/omkf_cloud_client.rs`
  - Automatic non-blocking trigger in `hera-desktop/src-tauri/src/libs/omkf_sync.rs::flush_dirty_omkf_capsules`

- [ ] **Step 1: Write the failing unit tests in `hera-desktop/src-tauri/src/libs/omkf_cloud_client.rs`**

Add `test_build_crate_delta_and_swarm_batch_from_capsule_isolates_private_bounce_and_parses_ledger` to `omkf_cloud_client::tests`:
```rust
#[test]
fn test_build_crate_delta_and_swarm_batch_from_capsule_isolates_private_bounce_and_parses_ledger() {
    use hera_node_rs::scanner::omkf_capsule::{
        OMKF_FLAG_IS_OWN_PRODUCTION, OmkfHeader, OmkfTrackEntry, encode_omkf_capsule,
    };
    let pub_slot = OmkfTrackSlot {
        partial_hash_64kb: [0x11; 16],
        file_size_bytes: 8_000_000,
        scan_level: 2,
        bpm_x100: 12500,
        camelot_code: 8,
        hot_cues_ms: [1000, 32000, 64000, 96000, 0, 0, 0, 0],
        ..Default::default()
    };
    let own_bounce = OmkfTrackSlot {
        partial_hash_64kb: [0x22; 16],
        file_size_bytes: 12_000_000,
        scan_level: 2,
        bpm_x100: 12600,
        flags_bits: OMKF_FLAG_IS_OWN_PRODUCTION,
        ..Default::default()
    };
    let hdr = OmkfHeader { tree_seal_64: 0xCAFE_BABE, track_count: 2, ..Default::default() };
    let entries = vec![
        OmkfTrackEntry { slot: pub_slot, filename: "club.flac".into(), artist: "A".into(), title: "B".into() },
        OmkfTrackEntry { slot: own_bounce, filename: "my_edit.wav".into(), artist: "Me".into(), title: "VIP".into() },
    ];
    let bytes = encode_omkf_capsule(&hdr, &entries);
    let md = "---\nomkf: 1\n---\n# Intención y Reglas de Cabina\nSolo afro house.\n\n# Ledger Reversible\n- tx:org_77 | 2026-11-16T12:00Z | 2 moves\n";
    let (delta_req, swarm_req) = build_crate_delta_and_swarm_batch_from_capsule(
        "DaviDJ", "crate-01", "Afro House", &bytes, md, 1_700_000_000_000,
    ).unwrap();
    assert_eq!(delta_req.crates.len(), 1);
    assert_eq!(delta_req.crates[0].track_refs.len(), 2); // Both tracks stay in DJ's private vault
    assert_eq!(delta_req.crates[0].ledger_entries.len(), 1);
    assert_eq!(delta_req.crates[0].ledger_entries[0].tx_id, "org_77");
    assert_eq!(swarm_req.items.len(), 1); // Own producer bounce is stripped from public Swarm KV
    assert_eq!(swarm_req.items[0].partial_hash_64kb, "11".repeat(16));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:
```bash
cargo test -p hera-desktop --bins omkf_cloud_client::tests::test_build_crate_delta_and_swarm_batch_from_capsule_isolates_private_bounce_and_parses_ledger
```
Expected: FAIL with `cannot find function build_crate_delta_and_swarm_batch_from_capsule`.

- [ ] **Step 3: Implement `build_crate_delta_and_swarm_batch_from_capsule` and `spawn_background_omkf_folder_cloud_sync` in `omkf_cloud_client.rs`, and wire into `omkf_sync.rs::flush_dirty_omkf_capsules`**

1. In `hera-desktop/src-tauri/src/libs/omkf_cloud_client.rs`:
   - Remove `#![allow(dead_code)]` once wired into `omkf_sync.rs` (keeping `#[allow(dead_code)]` only on pull/diff helpers reserved for on-demand multi-machine restore).
   - Implement `build_crate_delta_and_swarm_batch_from_capsule` using `decode_omkf_capsule(capsule_bytes)`:
     - Builds `CloudTrackRefDto` for every track entry (`partial_hash_64kb` as 32-char hex, `rel_filename = entry.filename`, `user_hot_cues_ms_json = serde_json::to_string(&entry.slot.hot_cues_ms)`).
     - Extracts `# Ledger Reversible` lines containing `tx:` (supporting both `tx:id` and `tx:#id`) into `Vec<CloudLedgerEntryDto>`.
     - Computes `rules_sha256` (FNV-1a/hex digest of `crate_md`) and builds `CloudPushDeltaRequest` + `CloudSwarmPublishRequest` (via `build_swarm_publish_batch`).
   - Implement `spawn_background_omkf_folder_cloud_sync(dj_handle: String, crate_id: String, folder_path: std::path::PathBuf)`:
     - Returns immediately (`no-op`) when `cfg!(test)` or `std::env::var("HERA_DISABLE_CLOUD_SYNC").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))`.
     - Otherwise spawns a detached `tokio::spawn` task that reads `.hera/tracks.omkf` and `.hera/crate.md`, calls `build_crate_delta_and_swarm_batch_from_capsule`, and fires `push_omkf_crate_deltas_async` + `publish_swarm_slots_async`.
2. In `hera-desktop/src-tauri/src/libs/omkf_sync.rs::flush_dirty_omkf_capsules`:
   - When `write_folder_omkf_from_db(conn, &d).await` returns `Ok(cid)`, call `crate::libs::omkf_cloud_client::spawn_background_omkf_folder_cloud_sync("DaviDJ".to_string(), cid, d);`.

- [ ] **Step 4: Run tests, clippy, and line-ceiling ratchets to verify they pass**

Run:
```bash
cargo fmt --all -- --check
cargo test -p hera-desktop --bins omkf_cloud_client
cargo test -p hera-desktop --bins omkf_sync
cargo test -p hera-node-rs --test architectural_ratchets
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: All tests and `clippy` exit with code `0`; `omkf_cloud_client.rs <= 260` LOC and `omkf_sync.rs <= 349` LOC.

- [ ] **Step 5: Commit signed**

```bash
git add hera-desktop/src-tauri/src/libs/omkf_cloud_client.rs hera-desktop/src-tauri/src/libs/omkf_sync.rs
git commit -S -m "feat(omkf,cloud): wire non-blocking Milestone C4 capsule delta and Swarm KV sync on flush"
git log -1 --show-signature
```

---

### Task 3: Bump Workspace Version to `v0.7.0`, Verify Full Polyglot Ratchets & Tag Signed Release

**Files:**
- Modify: `Cargo.toml:14` (`version = "0.7.0"`)
- Modify: `hera-desktop/package.json:3` (`"version": "0.7.0"`)
- Modify: `hera-desktop/src-tauri/tauri.conf.json:5` (`"version": "0.7.0"`)
- Modify: `hera-desktop/src-ipc/src/lib.rs:31-32` (`"0.7.0"`)
- Modify: `hera-rs/package.json:3` & `hera-rs/package-lock.json:3,9` (`"version": "0.7.0"`)
- Modify: `docs/superpowers/DECISIONS.md`

**Interfaces:**
- Consumes: `HERA_IPC_VERSION` in `hera-desktop/src-ipc/src/lib.rs`, `verify_tauri_version_parity` and `verify_workspace_release_workflows` in `hera-desktop/src-ipc/src/workflow_guard.rs`.
- Produces: Synchronized `0.7.0` manifests, `Cargo.lock` updated to `0.7.0`, and signed git tag `v0.7.0` pushed to `origin` to trigger `.github/workflows/release.yml`.

- [ ] **Step 1: Update `hera-desktop/src-ipc/src/lib.rs` test assertion to expect `"0.7.0"` and verify it fails before bumping `Cargo.toml`**

In `hera-desktop/src-ipc/src/lib.rs`:
```rust
assert_eq!(env!("CARGO_PKG_VERSION"), "0.7.0");
assert_eq!(HERA_IPC_VERSION, "0.7.0");
```
Run: `cargo test -p hera-ipc test_workspace_ipc_crate_ready`
Expected: FAIL (`left: "0.6.1", right: "0.7.0"`).

- [ ] **Step 2: Bump version to `0.7.0` across `Cargo.toml`, `hera-desktop/package.json`, `hera-desktop/src-tauri/tauri.conf.json`, `hera-rs/package.json`, and `hera-rs/package-lock.json`**

Update the 5 manifest files to `"0.7.0"` and run `cargo check` so `Cargo.lock` updates workspace member versions to `0.7.0`.

- [ ] **Step 3: Run the complete polyglot verification gate (`Rust` + `WASM` + `TypeScript` + `E2E` + `Vite+ Build`)**

Run:
```bash
cargo fmt --all -- --check
cargo test -p hera-ipc
cargo test -p hera-node-rs --test architectural_ratchets
cargo test -p hera-node-rs --test golden_masters
cargo test -p hera-node-rs -- --test-threads=4
cargo test -p hera-desktop --bins -- --skip export_bindings
cargo clippy --workspace --all-targets -- -D warnings
cargo check --manifest-path hera-rs/Cargo.toml --target wasm32-unknown-unknown --features cloudflare --no-default-features
cd hera-desktop && bun run gen:types && bun run knip && bun run check && bun run test -- --run && bun run build
```
Expected: `exit code 0` across every command, with zero dirty diffs on `hera-desktop/src/generated/` after `gen:types`.

- [ ] **Step 4: Commit signed `v0.7.0` release, create signed tag `v0.7.0`, and push to `origin`**

```bash
git add Cargo.toml Cargo.lock hera-desktop/package.json hera-desktop/src-tauri/tauri.conf.json hera-desktop/src-ipc/src/lib.rs hera-rs/package.json hera-rs/package-lock.json docs/superpowers/DECISIONS.md docs/superpowers/plans/2026-11-16-omkf-cloud-deploy-live-sync-and-v070-release.md
git commit -S -m "chore(release): bump HERA workspace to v0.7.0"
git tag -s v0.7.0 -m "HERA v0.7.0 — Bicameral OMKF v1, 50k Zero-IO Booth, 14-Family Genre x Era & Cloud Sync"
git log -1 --show-signature
git tag -v v0.7.0
git push origin main --follow-tags
```
