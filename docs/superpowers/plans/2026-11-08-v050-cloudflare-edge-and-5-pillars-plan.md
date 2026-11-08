# 📋 HERA `v0.5.0` — Implementation Plan (5 Pillars + Cloudflare Edge Optimization)

**Spec:** `docs/superpowers/specs/2026-11-08-v050-cloudflare-edge-and-5-pillars-design.md`

---

## Tasks

- [x] **Task 1 (`hera-cloud` + `hera-rs`): Cloudflare Edge Optimization Primitives (KV Forensic Dedup Cache, D1 DNA Sync, Vectorize SimHash Matcher)**
  - Implement `EdgeForensicCacheEntry`, `D1DnaSyncRecord`, `VectorizeMatchRequest`, `VectorizeMatchCandidate`, and `CloudflareEdgeCapabilities` in `hera-cloud/src/cloudflare_edge.rs` & `hera-cloud/src/lib.rs` (and public client types/endpoints in `hera-rs`).
  - Expose `/api/v1/edge/cloudflare-capabilities`, `/api/v1/edge/forensic-cache/lookup`, `/api/v1/edge/forensic-cache/store`, `/api/v1/edge/d1-dna-sync`, and `/api/v1/edge/vectorize-match` in Axum router.
  - Add integration test `test_cloudflare_edge_kv_forensic_cache_d1_dna_sync_and_vectorize_simhash` verifying `< 2 ms` KV forensic cache hit, D1 DNA persistence, and 64-bit `style_simhash_u64` POPCNT ranking.

- [x] **Task 2 (Pillar 2 — `hera-desktop/src-tauri`): Real Spectral Forensics & 4 Hot Cues on Folder Import (`track.rs` + `database.rs`)**
  - Extend `Track` struct in `hera-desktop/src-tauri/src/libs/track.rs` and SQLite `tracks` persistence in `hera-desktop/src-tauri/src/libs/database.rs` with `bpm`, `camelot`, `energy`, `hera_seal`, and `hot_cues`.
  - Wire `get_track_from_file` in `track.rs` to run `hera_node_rs::forensics::decode_audio_bytes` + `audit_pcm_channels` + `hera_node_rs::structure::analyze_structure_and_style` during `rayon` parallel folder import.

- [x] **Task 3 (Pillar 1 & Pillar 4 — `hera-desktop`): Visual `2-Opt TSP Set Spine Builder` + Live `P2P Swarm & Cloudflare Edge Radar`**
  - Register `build_dj_set_spine` and `get_cloudflare_edge_radar` across the Tauri v2 ACL triad (`src-tauri/src/plugins/hera_dj.rs`, `src-tauri/src/main.rs`, `src-tauri/build.rs`, `src-tauri/capabilities/main.json`).
  - Expose `buildSetSpineForLibrary` and `getCloudflareEdgeRadar` in `hera-desktop/src/lib/bridge-hera.ts`.
  - Add **`🧭 Ordenar Set (2-Opt TSP)`** button in `HeraDjToolbar.tsx` that reorders the active library tracks in-place using `buildSetSpineForLibrary` (pinning Opening #1, Peak Midpoint, and Closing tracks) and shows the Spine Synergy Score badge.
  - Add the **☁️ Cloudflare Edge Optimization & Radar P2P (`KV Cache` · `D1 Sync` · `Vectorize` · `MoQ Pub/Sub`)** panel in `hera-desktop/src/routes/bridge.tsx`.

- [x] **Task 4 (Pillar 5 & Pillar 3 — `hera-desktop/vite.config.ts` & `.github/workflows/release.yml`): `Rolldown` Code-Splitting & Multi-OS Release Workflow**
  - Configure `build.rolldownOptions.output.manualChunks` in `hera-desktop/vite.config.ts` so `index-*.js` is split into `< 300 kB` chunks (`vendor-react`, `vendor-tanstack`, `vendor-ui`, `index`), eliminating the `> 500 kB` chunk warning.
  - Create `.github/workflows/release.yml` using `oven-sh/setup-bun@v2` + `tauri-apps/tauri-action@v0` for automated Windows, macOS, and Linux native installer releases on `v*` tags.

- [x] **Task 5 (Verification, Live Playwright QA & Semantic Release `v0.5.0`):**
  - Run `moon run :check :test hera-desktop:build` in `hera` and `cargo check && cargo test` in `hera-cloud`.
  - Capture live Playwright screenshots verifying the new `🧭 Ordenar Set (2-Opt TSP)` sorting in `/library` and the Cloudflare Edge + MoQ Swarm Radar in `/bridge`.
  - Bump versions to `0.5.0`, update `CHANGELOG.md` and `DECISIONS.md`, tag `v0.5.0`, and push both repositories to `origin/main --follow-tags`.
