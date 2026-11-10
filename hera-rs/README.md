# 🦀 Hera Universal Rust Node (`hera-node-rs v0.6.0`)

**Pure-Rust Audio DSP, `#![forbid(unsafe_code)]` Dual-Deck Audio Engine, 4-Layer 2-Opt TSP Crate Optimizer, 21-Tool MCP Server & Federated R2 Edge Swarm Node**  
*One single `Axum` codebase that compiles to **HERA Desktop (`Tauri v2`)**, **Cloudflare Workers (`wasm32-unknown-unknown`)**, and **Standalone Micro-VPS (`~10 MB RAM`)**.*  
**Swarm Motto:** *«Mientras más nodos de Hera, más fuerte es el sistema de enjambres»*

---

## ⚡ Why 100% Pure Rust for Hera?

| Metric | Legacy v0.1 (`Librosa` + `FFmpeg`) | TypeScript (`Bun` + `Hono`) | 🦀 **Rust v0.6.0 (`Axum` + `Symphonia` + `rustfft`)** |
| :--- | :---: | :---: | :---: |
| **Idle Memory (RAM)** | ~120 MB | ~35 MB | **~10 MB** |
| **Audio Analysis Speed (1 Track)** | ~1.80s | Calls external binary | **~0.08s (100% Pure Rust)** |
| **Supported Streaming Codecs** | Requires external `ffmpeg` | None native | **8 Codecs (`WAV`, `FLAC`, `MP3`, `M4A`, `AAC`, `ALAC`, `OGG`, `AIFF`)** |
| **Runtime Dependencies** | Python + `.venv` + `ffmpeg` | Bun runtime | **Zero (1 static binary / WASM)** |
| **Cloudflare Workers Native?** | ❌ No | ✅ API only (No DSP) | ✅ **Full API + DSP + 2-Opt TSP + R2 Swarm in WASM** |

---

## 🧩 Core Modules ([`src/`](src))

1. **[`src/forensics.rs`](src/forensics.rs) — Zero-Trust Spectral & Club Phase Lab**:
   - Pure-Rust `Symphonia` streaming partial decoder (`decode_audio_file_partial` supporting `WAV`, `FLAC`, `MP3`, `M4A`, `AAC`, `ALAC`, `OGG`, `AIFF`) capped to lightweight windows (`< 25 MB RAM`) so it fits comfortably inside Cloudflare Workers' 128 MB limit.
   - `rustfft` Welch Averaged Power Spectrum (`2048`-sample Hann window) detecting **Fake Lossless** (`128 kbps` MP3 brickwall cutoffs around `15.5 kHz` upscaled to `.flac`/`.wav`).
   - **Synthetic Sine/Test Tone Detector** (`Peak-to-Mean Spectral Ratio > 0.35` & `Wiener Spectral Flatness < 1e-4`).
   - **Sub-Bass (`< 120 Hz`) Stereo Phase Correlation Audit** ensuring mono club subwoofer compatibility (`r >= 0.25`).

2. **[`src/structure.rs`](src/structure.rs) — Acoustic 4-Hot-Cue & Vocal Activity Analyzer**:
   - Real first downbeat kick lock (`Cue 1: Intro`), melodic/vocal breakdown (`Cue 2: Breakdown`), post-breakdown sub-bass surge (`Cue 3: Drop`), and late-track vocal clear-out (`Cue 4: Outro`).
   - Krumhansl-Schmuckler 12-pitch chroma key correlation mapped to the **24-key Camelot Wheel** (`1A`..`12B`) + **64D L2-normalized Style Embedding** & **`u64` Style SimHash (`POPCNT`)**.

3. **[`src/synergy.rs`](src/synergy.rs) — 4-Layer Synergy & Global 2-Opt TSP Crate Optimizer**:
   - Optimizes the entire DJ set sequence globally using **2-Opt Traveling Salesperson (TSP)** across Camelot harmony, BPM curve (`3:4` and `4:3` polyrhythmic bridges), transient punch & vocal clash prevention, and `u64` SimHash style embeddings.

4. **[`src/audio_engine/`](src/audio_engine) — `#![forbid(unsafe_code)]` Lock-Free Dual-Deck Engine**:
   - In-memory `DualDeckEngine` driven by a lock-free `[AtomicU64; 64]` SPSC `CommandQueue`, 3-band Linkwitz-Riley EQ crossover (`Low` / `Mid` / `High`), and equal-power crossfader for deterministic transition previews.

5. **[`src/export.rs`](src/export.rs) — Universal DJ Ecosystem Exporters**:
   - Pure-Rust generators for **VirtualDJ** (`.vdjfolder` + `_vdj_database.xml`), **Rekordbox XML** (with embedded 4 Hot Cues), **Traktor Pro NML** (`_traktor.nml`), and **M3U8** (`POST /api/v1/crates/export`).

6. **[`src/dj_brain/`](src/dj_brain) — Autonomous `DJ Style DNA` Profiler & Multi-Backend AI**:
   - Pure-Rust `DJ Style DNA` Auto-Profiler (`POST /api/v1/dj/style-dna`), ReAct `HeraAgentHarness`, bounded parallel local AI discovery (`Ollama`, `LM Studio`, `Llama.cpp`, `Jan`, `vLLM`), and `Cloudflare Workers AI Free` (`10,000 Neurons/day` free at `$0.00`, `POST /api/v1/ai/chat`, `GET /api/v1/ai/backends`).

7. **[`src/mcp/`](src/mcp) & [`src/code_mode/`](src/code_mode) — 21-Tool Pure-Rust MCP Server & Code Mode**:
   - **MCP `stdio` Server (`hera-node --mcp`)**: Native JSON-RPC 2.0 over `stdin`/`stdout` for AI coding/curation clients (Antigravity, Claude Desktop, Cursor, Codex).
   - **MCP HTTP/Edge Server (`POST /mcp` & `GET /mcp/tools`) + Code Mode (`POST /mcp/code-mode`)**: Exposes all 21 core MCP tools and single-turn multi-step pipeline execution over HTTP and Cloudflare Workers.

8. **[`src/p2p/`](src/p2p), [`src/p2p_exchange.rs`](src/p2p_exchange.rs) & [`src/cloudflare_edge.rs`](src/cloudflare_edge.rs) — 5-Stage Zero-Config P2P & Federated R2 Edge Swarm**:
   - **5-Stage Zero-Config P2P Cascade**: `1. LAN mDNS` -> `2. Multi-STUN (RFC 5389)` -> `3. 16-Port Birthday Spray` -> `4. Ephemeral TURN (RFC 5766)` -> `5. R2 Edge Boost`.
   - **Zero-Cost Billing Safety Caps (`Constrained Water-Filling`)**: Dynamically allocates hot shards across community Cloudflare Workers while enforcing `CF_WORKER_SAFE_R2_CAP_MB = 9,000.0 MB` (`9.0 GB` out of `10 GB` free R2) and `CF_WORKER_SAFE_REQ_CAP = 85,000 req/day` (out of `100,000 req/day` free Workers).
   - **Meritocratic `Edge Guardian` Autopilot (`1 : 1.5` Turbo Credits)**: Active nodes passing `64-byte Proof-of-Custody` range challenges receive unlimited R2 Fast Lane access + `1.5 MB` of Turbo Credit per `1.0 MB` served via `ProofOfDeliveryReceipt`.
   - **Blind Catalog Sharding & `4 MB` Immutable Blocks (`8.4x` Multiplier)**: Stores opaque `shard-blobs/{hmac}.hbin` chunks (`4,194,304` bytes) with `16 x 256 KiB` BEP 52 Merkle trees, gated by `60s` HMAC-signed `X-Hera-Swarm-Ticket` headers and double cryptographic seals (`hera_seal` + `audio_sha256`).

---

## 🛠️ Canonical Commands (`v0.6.0`)

```bash
# 1. Fast Typecheck & Native Compilation Check
cargo check -p hera-node-rs

# 2. Full Rust Unit & Integration Test Suite (DSP, DualDeckEngine, Exporters, DJ Brain, MCP, P2P Swarm)
cargo test -p hera-node-rs

# 3. Run the 21-Tool MCP Server over stdio
cargo run --release -p hera-node-rs -- --mcp

# 4. Cloudflare Workers WASM Target Check & Release Build
cargo check --manifest-path Cargo.toml --target wasm32-unknown-unknown --features cloudflare --no-default-features
worker-build --release -- --features cloudflare --no-default-features
```
