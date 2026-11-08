# 🦀 Hera Universal Rust Node (`hera-node-rs`)

**Pure-Rust Audio DSP, 4-Layer 2-Opt TSP Crate Optimizer & Federated R2 Edge Swarm Node**  
*One single `Axum` codebase that compiles to both **Cloudflare Workers (WASM)** and **Standalone Micro-VPS (`~10 MB RAM`)**.*  
**Swarm Motto:** *«Mientras más nodos de Hera, más fuerte es el sistema de enjambres»*

---

## ⚡ Why Rust for Hera?

| Metric | Python (`Librosa` + `FFmpeg`) | TypeScript (`Bun` + `Hono`) | 🦀 **Rust (`Axum` + `Symphonia` + `rustfft`)** |
| :--- | :---: | :---: | :---: |
| **Idle Memory (RAM)** | ~120 MB | ~35 MB | **~10 MB** |
| **Audio Analysis Speed (1 Track)** | ~1.80s | Calls external binary | **~0.08s (100% Pure Rust)** |
| **External Dependencies** | Python + `.venv` + `ffmpeg` | Bun runtime | **Zero (1 static binary / WASM)** |
| **Cloudflare Workers Native?** | ❌ No | ✅ API only (No DSP) | ✅ **Full API + DSP + 2-Opt TSP + R2 Swarm in WASM** |

---

## 🧩 Core Modules ([`src/`](src))

1. **[`src/forensics.rs`](src/forensics.rs) — Zero-Trust Spectral & Club Phase Lab**:
   - Pure-Rust `Symphonia` multi-format decoder (`FLAC`, `WAV`, `MP3`, `OGG`) capped to lightweight streaming windows (`< 25 MB RAM`) so it fits inside Cloudflare Workers' 128 MB limit.
   - `rustfft` Welch Averaged Power Spectrum (`2048`-sample Hann window) detecting **Fake Lossless** (`128 kbps` MP3 brickwall cutoffs around `15.5 kHz` upscaled to `.flac`/`.wav`).
   - **Synthetic Sine/Test Tone Detector** (`Peak-to-Mean Spectral Ratio > 0.35` & `Wiener Spectral Flatness < 1e-4`).
   - **Sub-Bass (`< 120 Hz`) Stereo Phase Correlation Audit** ensuring mono club subwoofer compatibility (`r >= 0.25`).

2. **[`src/structure.rs`](src/structure.rs) — Acoustic 4-Hot-Cue & Vocal Activity Analyzer**:
   - Real first downbeat kick lock (`Cue 1: 1.1.1 Downbeat`), melodic/vocal entry (`Cue 2: Phrase In`), post-breakdown sub-bass surge (`Cue 3: Main Drop`), and late-track vocal clear-out (`Cue 4: Mix Out`).
   - Krumhansl-Schmuckler 12-pitch chroma key correlation mapped to the **24-key Camelot Wheel** (`1A`..`12B`) + **64D L2-normalized Style Embedding**.

3. **[`src/synergy.rs`](src/synergy.rs) — 4-Layer Synergy & Global 2-Opt TSP Crate Optimizer**:
   - Optimizes the entire DJ set sequence globally using **2-Opt Traveling Salesperson (TSP)** across Camelot harmony, BPM curve (`3:4` and `4:3` polyrhythmic bridges), transient punch & vocal clash prevention, and 64D style embeddings.
   - Exports ready-to-play `M3U8`, `Rekordbox XML` (with embedded Hot Cues), and Markdown Set Guides.

4. **[`src/p2p_exchange.rs`](src/p2p_exchange.rs) — Federated R2 Edge Swarm, Constrained Water-Filling & `Edge Guardian` Protocol**:
   - **Zero-Cost Billing Safety Caps (`Constrained Water-Filling`):** Dynamically allocates hot shards across community Cloudflare Workers while enforcing `CF_WORKER_SAFE_R2_CAP_MB = 9,000.0 MB` (`9.0 GB` out of `10 GB` free R2) and `CF_WORKER_SAFE_REQ_CAP = 85,000 req/day` (out of `100,000 req/day` free Workers) so collaborators never pay a cent.
   - **Elastic Thermal Replication ($K$) & `Power-of-Two-Choices (P2C)`:** Scales shard replication factor $K \in [1, 5]$ automatically as more community nodes join ($N$) and routes downloads to the healthy node with the lowest `daily_worker_requests_used`.
   - **Meritocratic `Edge Guardian` Autopilot (`1 : 1.5` Turbo Credits):** Active nodes passing `64-byte Proof-of-Custody` range challenges receive unlimited R2 Fast Lane access (`EDGE_GUARDIAN_UNLIMITED_FAST_LANE`) + `1.5 MB` of Turbo Credit per `1.0 MB` served (`SWARM_TURBO_CREDIT_MULTIPLIER = 1.5`) via single-use `ProofOfDeliveryReceipt`.
   - **Blind Catalog Sharding & `4 MB` Immutable Blocks (`8.4x` Multiplier):** Stores opaque `shard-blobs/{hmac}.hbin` chunks (`4,194,304` bytes) with `Cache-Control: public, max-age=31536000, immutable`, gated by `60s` HMAC-signed `X-Hera-Swarm-Ticket` headers and protected by double cryptographic seals (`hera_seal` + `audio_sha256`).

5. **[`src/mcp.rs`](src/mcp.rs), [`src/p2p.rs`](src/p2p.rs) & [`src/router.rs`](src/router.rs) — Universal `Axum` API, MCP JSON-RPC 2.0 Server & Quarantine-First P2P Mesh**:
   - **MCP `stdio` Server (`hera-node --mcp`)**: Native JSON-RPC 2.0 over `stdin`/`stdout` for AI coding/curation clients (Antigravity, Claude Desktop, Cursor, Codex) with zero Python dependency.
   - **MCP HTTP/Edge Server (`POST /mcp` & `GET /mcp/tools`)**: Exposes all MCP tools + Code Mode (`POST /mcp/code-mode`) over HTTP and Cloudflare Workers.
   - **Core DSP & P2P Endpoints**:
     - `GET  /health` — Node status & runtime target (`cloudflare-workers-wasm32` or `native-vps-tokio`).
     - `GET  /api/v1/node/manifest` — Capabilities, invariants & Cloud Referral catalog.
     - `POST /api/v1/audio/analyze-file` — Upload raw `.wav`/`.flac`/`.mp3` bytes (up to 64 MB) for instant pure-Rust DSP audit & 4 Hot Cues.
     - `POST /api/v1/audio/analyze-pcm` — Analyze raw float32 PCM channels.
     - `POST /api/v1/crates/build` — Run global **2-Opt TSP** set curation.
     - `GET  /api/v1/p2p/search` — Search verified studio masters on the node.
     - `POST /api/v1/p2p/announce` — Quarantine-First P2P Gatekeeper (rejects fake FLACs, sine tones, and out-of-phase sub-bass).
   - **Federated R2 Swarm Endpoints ([`docs/FEDERATED_R2_SWARM.md`](../docs/FEDERATED_R2_SWARM.md))**:
     - `GET  /api/v1/swarm/strength` — Live swarm metrics (`SwarmStrengthMetrics`), active `Edge Guardians`, elastic $K$, and swarm motto.
     - `POST /api/v1/swarm/heartbeat` — Node telemetry check-in, `Constrained Water-Filling` allocation, `Edge Guardian` privilege verification, and CGNAT `outbound_pull_tickets`.
     - `POST /api/v1/swarm/route-ticket` — `Power-of-Two-Choices (P2C)` node selection + `60s` signed `SwarmRedirectTicket` + `1:1.5` `ProofOfDeliveryReceipt` credit.
     - `GET  /api/v1/swarm/shard/:blob_id` — Serves `4 MB` immutable R2 shard blocks (`HERA_R2_VAULT`) gated by `X-Hera-Swarm-Ticket`.
     - `POST /api/v1/swarm/custody-challenge` — Issues a 64-byte random range `Proof-of-Custody` challenge.
     - `GET  /api/v1/swarm/custody-probe/:blob_id` — Reads the challenged 64-byte slice directly from R2 and returns its cryptographic digest.
     - `POST /api/v1/swarm/custody-audit` — Verifies the 64-byte custody response (maintains or revokes `Edge Guardian` status immediately).
     - `GET  /api/v1/swarm/quarantine` — Returns the `quarantined_nodes` ban list isolated for Bait-and-Switch / poisoned shard attempts.

---

## 🚀 Deployment Options (Free & Referral-Monetized)

### Option A: Cloudflare Workers + R2 `Edge Guardian` (`$0.00 / Forever Free Edge WASM`)
```bash
cd hera-rs
npx wrangler deploy
```

### Option B: 1-Click Micro-VPS & Container Providers (Referral Monetization)
Replace the placeholder codes (`HERA_REFERRAL`, `HERA_VULTR_REF`, `HERA_DO_REF`, `HERA_HETZNER_REF`) in [`src/p2p.rs`](src/p2p.rs) and your GitHub README with your personal referral codes to earn cash/credits on every community node deployment:

- **Railway.app (Up to 25%–50% recurring monthly template kickbacks)**: Uses [`../railway.json`](../railway.json) + [`Dockerfile`](Dockerfile).
- **Vultr (\$100–\$300 Free User Credit / \$35–\$100 USD payout per referral)**
- **DigitalOcean (\$200 Free User Credit / \$25–\$200 USD per referral)**
- **Hetzner Cloud (€20 Free User Credit / €10 per referral)**

One-line installer on any Linux VPS:
```bash
curl -fsSL https://raw.githubusercontent.com/davidcastilloc/hera/main/deploy/install-node.sh | bash
```
