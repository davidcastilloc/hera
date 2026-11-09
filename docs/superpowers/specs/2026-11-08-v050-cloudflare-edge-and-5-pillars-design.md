# 🌐 HERA `v0.5.0` — 5-Pillar Ecosystem Upgrade & `hera-cloud` Cloudflare Edge Optimization Spec

**Date:** 2026-11-08  
**Repositories:** `davidcastilloc/hera` (Public MIT Client/DSP/Desktop) ↔ `davidcastilloc/hera-cloud` (Private Cloudflare Edge Control Plane)  
**Target Release:** `v0.5.0`

---

## 1. Architectural Vision & Open-Core Boundary

1. **Private `hera-cloud` Control Plane (`davidcastilloc/hera-cloud`):**
   - Leverages Cloudflare's full Edge stack at `$0.00` base cost to offload heavy computation and synchronize state across all `HERA` nodes worldwide:
     - **Cloudflare KV Global Forensic Dedup Cache (`POST /api/v1/edge/forensic-cache/lookup`, `POST /api/v1/edge/forensic-cache/store`):** Maps `audio_sha256` / `cas_fingerprint` to verified `ForensicReport` + `TrackStructure` (4 Hot Cues, BPM, Camelot Key, `hera_seal`). When DJ #2 imports a track already scanned by DJ #1 anywhere in the world, `hera-desktop` resolves its 4 Hot Cues and `VERIFIED_STUDIO_MASTER` seal in `< 2 ms` from Cloudflare Edge KV without re-running local FFT.
     - **Cloudflare D1 Zero-Cost `DJ Style DNA` & Set Spine Sync (`POST /api/v1/edge/d1-dna-sync`, `GET /api/v1/edge/d1-dna-sync/:dj_id`):** Synchronizes the compact ~250-token `DJStyleDnaProfile` (`prompt_block`, dominant Camelot keys, BPM zones, Golden Rules, and Turbo Credits) into Cloudflare D1 (`5 GB` free tier = ~14M profiles).
     - **Cloudflare Vectorize + 64-bit `style_simhash_u64` Acoustic Matcher (`POST /api/v1/edge/vectorize-match`):** Ranks global swarm tracks using Hamming POPCNT on `style_simhash_u64` combined with Camelot harmonic compatibility (`±1` step / relative A↔B) and BPM proximity.
     - **Cloudflare Edge Stack Telemetry (`GET /api/v1/edge/cloudflare-capabilities`):** Reports active Cloudflare primitives (`Workers WASM`, `R2 Zero-Egress CAS`, `Workers AI 10k Neurons/day`, `AI Gateway Cache`, `D1 SQLite Sync`, `KV Forensic Dedup Cache`, `Vectorize SimHash`, `Durable Objects MoQ Pub/Sub`).
   - Strictly keeps private SaaS unit economics (`/api/v1/cloud/economics`, `calculate_cloudflare_beta_economics`) and admin tier upgrades (`/api/v1/dj/upgrade-tier`, `X-Hera-Admin-Secret`) inside `hera-cloud` only.

2. **Public `hera` Client & `HERA Desktop` (`davidcastilloc/hera`):**
   - **Pillar 1 — Visual `Set Spine Builder` (`2-Opt TSP` + Pinned Anchors):**
     - Exposes `build_dj_set_spine` in Tauri (`hera-desktop/src-tauri/src/plugins/hera_dj.rs`, `main.rs`, `build.rs`, `capabilities/main.json`).
     - Adds **`🧭 Ordenar Set (2-Opt TSP)`** in `HeraDjToolbar.tsx` and anchor pin indicators (`📌 Opening`, `🔥 Peak`, `🌙 Closing`) in `TrackRow.tsx` so DJs can sort their active crate in `< 5 ms` using the 4-layer Holistic Synergy + `2-Opt TSP` engine.
   - **Pillar 2 — Real Parallel Spectral Forensics + 4 Hot Cues on Folder Import (`src-tauri/src/libs/track.rs` & `db.rs`):**
     - Extends `Track` in Rust (`track.rs`, SQLite `tracks` table in `db.rs`) and TypeScript (`src/types/museeks.ts`) with `bpm`, `camelot_key`, `energy`, `cue_intro`, `cue_breakdown`, `cue_drop`, `cue_outro`, and `hera_seal`.
     - Wires `track.rs` during `rayon` parallel folder scanning to invoke `hera_node_rs::forensics` + `hera_node_rs::structure` streaming the first 45s with Symphonia across all formats (`.flac`, `.mp3`, `.m4a`, `.aac`, `.alac`, `.ogg`, `.wav`).
   - **Pillar 3 — Automated Multi-OS Native Installer Pipeline (`.github/workflows/release.yml`):**
     - Builds and publishes `~8 MB` native installers (`.exe`/`.msi`, `.dmg`, `.deb`/`.AppImage`) on `v*` tags via `tauri-apps/tauri-action` + `Bun 1.4.4`.
   - **Pillar 4 — Live P2P Swarm & Cloudflare Edge Radar (`bridge.tsx` & `HeraDjToolbar.tsx`):**
     - Displays live `MoQ` Harmonic Pub/Sub matches (`hera://swarm/8A/122-128`), `1:1.5` Turbo Credits, and Cloudflare Edge KV/D1/Vectorize optimization metrics.
   - **Pillar 5 — `Rolldown` Code-Splitting (`hera-desktop/vite.config.ts`):**
     - Splits `vendor-react`, `vendor-tanstack`, and app code via `build.rolldownOptions.output.advancedChunks` so every production JS chunk is `< 250 kB` (zero `> 500 kB` warnings).
