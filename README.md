<div align="center">

<img src="assets/hera_banner_y2k.jpg" alt="HERA Y2K Cyber DJ Banner" width="100%" />

# 🎧 HERA
### *The Autonomous, Local-First AI Super-Agent for DJs & Music Curators*

**«Todo es posible si lo imaginas: De la intención musical al booth del DJ en segundos»**

[![Pure Rust](https://img.shields.io/badge/Rust-100%25%20Pure%20Rust%20v0.6.0-dea584.svg?style=flat&logo=rust)](hera-rs/)
[![Tauri v2](https://img.shields.io/badge/Desktop-Tauri%20v2%20%2B%20React%2019-24C8D8.svg?style=flat&logo=tauri)](hera-desktop/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Protocol: MCP](https://img.shields.io/badge/protocol-MCP%20stdio%20%7C%20HTTP-FF6B6B.svg)](https://modelcontextprotocol.io/)
[![Cloud: Cloudflare Edge](https://img.shields.io/badge/Cloud-D1%20%7C%20R2%20%7C%20KV%20%7C%20Vectorize%20%7C%20Workers%20AI-orange.svg)](https://workers.cloudflare.com/)
[![P2P: 5-Stage Zero-Config Cascade](https://img.shields.io/badge/P2P-LAN%20%7C%20STUN%20%7C%20Spray%20%7C%20TURN%20%7C%20R2-cyan.svg)](AGENTS.md)
[![Philosophy: Local--First](https://img.shields.io/badge/philosophy-Local--First-orange.svg)](MANIFESTO.md)

<p align="center">
  <b>Hera (`v0.6.0`)</b> is an intelligent, cross-platform audio curation and asset orchestrator designed for real human DJs.<br/>
  100% Pure-Rust + Tauri v2 native engine (`~8 MB` installer, zero Python). Analyzes audio with Symphonia + rustfft (BPM, Camelot Wheel, 4 Hot Cues, Sub-bass phase safety), arranges crates via 2-Opt TSP, morphs into a live VirtualDJ `Modo Mini` overlay, exports directly to VirtualDJ, Rekordbox and Traktor, and shares Lossless Masters across a 5-stage zero-config P2P swarm.
</p>

[✨ Read Manifesto](MANIFESTO.md) • [🤖 Agent Guide (AGENTS.md)](AGENTS.md) • [🚀 Quickstart](#-quickstart--installation) • [🎛️ CLI Reference](#️-cli-reference-hera-node) • [📋 Roadmap](#-wishlist--roadmap-board-dj--human-centric)

---

</div>

## 📑 Table of Contents
- [🌟 Real-World Use Case: The Symbiotic P2P Super-Node](#-real-world-use-case-the-symbiotic-p2p-super-node)
- [✨ Core Philosophy](#-core-philosophy)
- [⚡ Key Features](#-key-features)
- [🔌 3 Ways to Use Hera](#-3-ways-to-use-hera)
- [🏗️ System Architecture](#️-system-architecture)
- [🚀 Quickstart & Installation](#-quickstart--installation)
- [🎛️ CLI Reference (`hera-node`)](#️-cli-reference-hera-node)
- [🧠 Supported AI Copilot Backends](#-supported-ai-copilot-backends-hera-rssrcdj_brain)
- [☁️ Zero-OAuth Cloud Onboarding](#️-zero-oauth-cloud-onboarding-hera-rssrcp2pcloud_linksrs)
- [📋 Wishlist & Roadmap Board (DJ & Human-Centric)](#-wishlist--roadmap-board-dj--human-centric)
- [🤝 Contributing](#-contributing)
- [📄 License](#-license)

---

## 🌟 Real-World Use Case: The Symbiotic P2P Super-Node

> *"We just acquired and built 8 full 60+ minute DJ sets, and simultaneously we are enabling the entire global community to download our curated selection at gigabit fiber speeds."*

```text
       [ Human / DJ Intention ]
  ("Build 8 Sets: 2000-2005 Millennium Anthems & Deep Digs")
                  │
                  ▼
         [ HERA Pure-Rust Engine ]
  ├── 1. 5-Stage Zero-Config P2P Search (LAN -> Multi-STUN -> Spray -> TURN -> R2)
  ├── 2. Zero-Trust Quarantine & rustfft Spectral Cutoff Audit
  ├── 3. Symphonia Streaming DSP: BPM, Camelot Key (8A, 10B), 4 Hot Cues
  ├── 4. Native ID3v2.4 / Vorbis Tag Injection (lofty) & KV Forensic Dedup
  └── 5. 2-Opt TSP Harmonic Crate Sequencing & VirtualDJ / Rekordbox / Traktor Export
                  │
        ┌─────────┴─────────┐
        ▼                   ▼
[ 8 Curated DJ Sets ]  [ 800 Mbps Super-Seed ]
(8.58h Lossless Mix)   (UDP + Edge Guardian Swarm)
        │                   │
        ▼                   ▼
 [ Zero-OAuth Cloud ]  [ Global P2P Peers ]
 (3.08 GB in Cloud)    (1:1.5 Turbo Credits, No Leech)
```

### 1. Zero-Trust Ingestion & Pure Studio Master Validation
Incoming audio lands in `quarantine/`. Hera automatically executes **Symphonia + `rustfft` Peak Ratio and Spectral Flatness audits** (`hera-rs/src/forensics.rs`) to reject fake upscaled transcodes and synthetic sine wave tones. Only verified studio masters (FLAC / WAV / AIFF / 320k MP3) are promoted to `library/`.

### 2. Autonomous 8-Crate Harmonic Construction (Side A & Side B)
Hera builds physical DJ crates for club and festival performance:
* **Side A (Peak-Time & Primetime Hits)**: French Touch, Electro Club, Eurodance/Trance, Vocal Divas.
* **Side B (Deep Digging & Progressive Gems)**: Deep Disco, Dirty Electro, Progressive Trance, Soulful Underground.
* **Strict 60+ Minute Rule**: Every single set contains 11–15 sequenced tracks spanning **60.2 to 73.3 minutes** with harmonic key progression.

### 3. Good P2P Citizen & 800 Mbps Dedicated Super-Node
Instead of just taking from the community (*leeching*), Hera configures every free/local node (`p2p_sharing_enabled = true` — *"Por el bien del enjambre"*) as a high-speed swarm participant:
* Exposes **143+ mastered audio files (3.13 GB)** across verified CAS fingerprints.
* Traverses NATs automatically via **5-Stage Zero-Config P2P** (`LAN` -> `Multi-STUN` -> `16-port spray` -> `RFC 5766 TURN` -> `R2 Boost`) without router port forwarding.
* Rewards active seeders with **`1 : 1.5` Turbo Credits** and **`+5.0 MB` Pioneer Forensic Bounties**.

### 4. Multi-Cloud Zero-OAuth Redundancy
All curated sets, audio masters, and `.vdjfolder` / XML / NML cue sheets connect seamlessly to **Google Drive, Dropbox, OneDrive, and Cloudflare R2** with zero OAuth console setup, guaranteeing immediate access from any CDJ, VirtualDJ booth laptop, or standalone player.

### 5. 🐝 Federated R2 Edge Swarm & `Edge Guardian` Nodes (*«Mientras más nodos de Hera, más fuerte es el sistema de enjambres»*)
Any community member who deploys [`hera-rs`](hera-rs/README.md) on the **Cloudflare Workers + R2 Free Tier (`$0.00/month`)** automatically strengthens the global P2P swarm ([full guide in `docs/FEDERATED_R2_SWARM.md`](docs/FEDERATED_R2_SWARM.md)):
* **Constrained Water-Filling (`$0.00` Billing Protection)**: The swarm dynamically calculates optimal shard allocations across all community nodes while strictly respecting `9.0 GB` R2 (`CF_WORKER_SAFE_R2_CAP_MB`) and `85,000 req/day` (`CF_WORKER_SAFE_REQ_CAP`) safety ceilings so collaborators never incur Cloudflare charges.
* **Meritocratic `Edge Guardian` Autopilot (`1 : 1.5` Turbo Credits)**: Nodes passing random **64-byte `Proof-of-Custody` range audits** unlock unlimited R2 Fast Lane access (`EDGE_GUARDIAN_UNLIMITED_FAST_LANE`) while online and earn **`1.5 MB` of Turbo Credit per `1.0 MB` served** (`ProofOfDeliveryReceipt`) for offline laptop sets.
* **Blind Catalog Sharding & `4 MB` Immutable Blocks (`8.4x` Multiplier)**: Shards are stored as opaque `shard-blobs/{hmac}.hbin` blocks (`4 MB` aligned with `Cache-Control: public, max-age=31536000, immutable`), gated by single-use `60s` HMAC-signed `X-Hera-Swarm-Ticket` headers and verified by a double cryptographic seal (`hera_seal` + `audio_sha256`).

---

## ✨ Core Philosophy

* **100% Real Audio — Zero Synthetic Placeholders:** No synthetic tone generators or beep placeholders. Hera only promotes verified, full-length lossless FLAC/WAV/AIFF and 320kbps MP3 master files to your library.
* **Human-Centric & DJ-First:** Built for the dark DJ booth (`#0c0e12` slate + `#14b8a6` studio emerald) and single-screen laptops (`Modo Mini` `Ctrl+Shift+M`). Songs are organized into clean folders with rich metadata (BPM, Camelot Key, 4 Hot Cues) ready for live decks.
* **Swarm-First & Local-First (`$0.00` Free Core):** Runs natively on your laptop or VPS (`~8 MB` binary, SQLite + Pure-Rust DSP) with `$0.00` Cloudflare Workers AI (`10,000 Neurons/day`) and local LLMs, while nourishing the global P2P swarm by default.
* **Cross-Platform Portability:** Operates identically on **Linux (`x86_64`, `ARM64`)**, **macOS (`Apple Silicon`, `Intel`)**, and **Windows (`x86_64`)**.

---

## ⚡ Key Features

| Capability | Description |
|---|---|
| **5-Stage Zero-Config P2P** | Cascading `LAN Direct` -> `Multi-STUN` -> `16-Port Prediction Spray` -> `Cloudflare RFC 5766 TURN` (`0` R2 bytes) -> `R2 P2P Boost` + 5-Layer Federated R2 Swarm. |
| **Pure-Rust Streaming DSP** | Streaming partial decoding (`decode_audio_file_partial` via `Symphonia` + `rustfft` across `WAV`, `FLAC`, `MP3`, `M4A`, `AAC`, `ALAC`, `OGG`, `AIFF`) for **BPM**, **Camelot Key**, and **4 Hot Cues**. |
| **Lock-Free Dual-Deck Core** | `#![forbid(unsafe_code)]` `[AtomicU64; 64]` SPSC `DualDeckEngine` (`hera-rs/src/audio_engine/`) for sample-accurate 4-Hot-Cue jumping and linear tempo resampling without ASIO/WASAPI contention. |
| **Morphing `Modo Mini` & Exporters** | Single-window VirtualDJ companion (`360x408` booth card & `560x52` HUD pill, `Ctrl+Shift+M`), live `HERA En Vivo.vdjfolder` push, plus 1-click **VirtualDJ**, **Rekordbox XML**, **Traktor NML**, and **M3U8** export. |
| **Autonomous `/agent` & 44-Tool MCP** | Centered `/agent` workspace (`Ctrl+J`) backed by a per-launch Bearer `127.0.0.1:0` HTTP MCP server (`44` tools), `@ Contexto` attachments, inline `Confirmar` gates, plus `hera-node --mcp` (`stdio` / HTTP / Code Mode). |
| **Hexagonal `hera-ipc` Guardian** | Zero-GTK pure-Rust crate (`hera-desktop/src-ipc/`) generating `ts-rs` TypeScript contracts and verifying Tauri v2 ACL Triad parity in `< 5s` across all platforms. |

---

## 🔌 3 Ways to Use Hera

Hera works seamlessly at every level — from a deterministic offline DJ booth app to a 44-tool autonomous agent. **No LLM is required for Modes 1 and 2.**

> 💡 **Guías en español:** Consulta la [Guía Completa de Modos de Uso](docs/MODOS_DE_USO.md) y la [Guía de HERA en VPS Remoto + Curadora AI](docs/VPS_REMOTE_GUIDE.md).

### Mode 1: Deterministic DJ Studio & `Modo Mini` Overlay (No LLM — For Humans)
Launch **HERA Desktop** alongside VirtualDJ, Rekordbox, or Traktor with 100% offline Pure-Rust curation:
* Import folders (`WAV`, `FLAC`, `MP3`, `M4A`, `ALAC`, `AIFF`, `OGG`) with automatic 4-Hot-Cue (`1:Intro`, `2:Breakdown`, `3:Drop`, `4:Outro`), BPM, and Camelot analysis.
* Sequence crates with `2-Opt TSP`, resolve harmonic bridges in `Transición`, tune your profile in `Mi Estilo`, and toggle `Modo Mini` (`Ctrl+Shift+M`) to push tracks live into `HERA En Vivo.vdjfolder`.

### Mode 2: Pure-Rust MCP Server (No LLM — For Any External AI Agent)
Expose HERA's 21 native Rust DSP, crate-curation, and P2P swarm tools over `stdio`, HTTP (`POST /mcp`), or TypeScript Code Mode (`POST /mcp/code-mode`) to Claude Code, Antigravity, Cursor, or Codex:

```bash
# Start the Pure-Rust MCP server over stdio
cargo run --release -p hera-node-rs -- --mcp
```

Connect from any MCP-compatible client:
```json
{
  "mcpServers": {
    "hera": {
      "command": "cargo",
      "args": ["run", "--release", "-p", "hera-node-rs", "--", "--mcp"]
    }
  }
}
```

**Core MCP Tools (`hera-rs/src/mcp/`):** `analyze_audio_forensics`, `build_dj_crate`, `build_set_spine`, `export_dj_crate`, `compute_dj_style_dna`, `get_swarm_strength`, `swarm_heartbeat`, `swarm_route_ticket`, `swarm_custody_audit`, `swarm_parallel_manifest`, `swarm_crdt_sync`, `swarm_subscribe`, and more (`21` tools in `hera-node-rs`, `44` tools in `HERA Desktop` `/agent`).

### Mode 3: Autonomous `/agent` Workspace & `HeraAgentHarness` (12 AI Backends)
Open the `Agente` tab (`Ctrl+J`) in `HERA Desktop` or call `POST /api/v1/ai/chat`. Attach `@ Contexto` tracks, playlists, or folders and run multi-step workflows across **Cloudflare Workers AI Free (`$0.00`)**, local CLI agents (`agy`, `claude`, `gemini`, `codex`), or offline servers (`Ollama`, `LM Studio`, `llama.cpp`) with `Auto` or inline `Confirmar` approval gates.

---

## 🏗️ System Architecture

```text
 hera/ (v0.6.0 — 100% Pure Rust + Tauri v2 + Bun 1.4.4 + Vite+ + moon 2.6.5)
 ├── Cargo.toml                      # Unified root Cargo workspace (resolver = "2")
 ├── .moon/workspace.yml             # Polyglot content-addressed build graph (moon 2.6.5)
 ├── hera-rs/                        # hera-node-rs v0.6.0 (Native Tokio + Cloudflare WASM)
 │   └── src/
 │       ├── audio_engine/           # #![forbid(unsafe_code)] lock-free [AtomicU64; 64] DualDeckEngine
 │       ├── forensics.rs            # Symphonia partial streaming decoder + rustfft spectral audit
 │       ├── structure.rs            # BPM, 24-key Camelot Wheel & 4 Acoustic Hot Cues
 │       ├── synergy.rs              # 4-Layer Holistic Synergy + zero-alloc 1D 2-Opt TSP
 │       ├── export.rs               # VirtualDJ (.vdjfolder), Rekordbox XML, Traktor NML, M3U8
 │       ├── dj_brain/               # DJ Style DNA, 12-backend registry & ReAct HeraAgentHarness
 │       ├── mcp/                    # 21-tool Pure-Rust MCP Server (stdio + HTTP)
 │       ├── code_mode/              # TypeScript Code Mode sandboxed dispatcher
 │       ├── p2p/                    # 5-Stage Zero-Config P2P (LAN/STUN/Spray/TURN/R2) & Custody
 │       ├── cloudflare_edge.rs      # KV Forensic Dedup, D1 DNA Sync & Vectorize u64 SimHash
 │       └── error.rs                # Typed HeraError & Swarm-First subscription invariants
 ├── hera-desktop/
 │   ├── src-ipc/                    # hera-ipc v0.6.0 (Zero-GTK ts-rs DTOs + ACL & Workflow Guards)
 │   │   └── src/                    # acl_guard.rs, workflow_guard.rs, typegen.rs, dj/agent_types.rs
 │   ├── src-tauri/                  # hera-desktop v0.6.0 (Tauri v2, sqlx SQLite, lofty, rayon)
 │   │   └── src/plugins/
 │   │       ├── hera_agent/         # 44-tool 127.0.0.1:0 Bearer HTTP MCP & streaming runner
 │   │       └── hera_dj/            # Async DSP/P2P IPC, Modo Mini morphing & live VirtualDJ push
 │   └── src/                        # React 19 Compiler + StyleX Dark Booth Studio (#0c0e12)
 │       ├── routes/                 # library.tsx, agent.tsx, bridge.tsx (Transición), dna.tsx
 │       └── components/             # MiniOverlay.tsx, AgentComposer.tsx, HeraDjToolbar.tsx
 └── docs/superpowers/               # Multi-instance ODD specs/, plans/, and DECISIONS.md
```

---

## 🚀 Quickstart & Installation

HERA (`v0.6.0`) is 100% native Rust + TypeScript with **zero Python dependencies**.

### 1. Fast Workspace Check & Tests (`hera-rs` + `hera-ipc`)

```bash
# Fast workspace check + full DSP, P2P, DualDeckEngine, ts-rs & Tauri ACL Triad suites
cargo check
cargo test -p hera-node-rs
cargo test -p hera-ipc
```

### 2. Run the Native Pure-Rust MCP Server

```bash
cargo run --release -p hera-node-rs -- --mcp
```

### 3. Build & Verify HERA Desktop (`Tauri v2` + `Bun 1.4.4` + `Vite+`)

```bash
cd hera-desktop
bun install
bun run gen:types && bun run check && bun run test && bun run build
```

Or run the full cached polyglot pipeline from the repository root with `moon`:

```bash
moon run :check :test hera-desktop:build
```

---

## 🎛️ CLI Reference (`hera-node`)

```bash
# Run local HERA P2P & DSP Node (Native Tokio VPS / Laptop Daemon)
cargo run --release -p hera-node-rs -- --port 8080

# Run in MCP Server Mode (stdio JSON-RPC for Copilots)
cargo run --release -p hera-node-rs -- --mcp

# Build DJ Set Spine via 2-Opt TSP directly via HTTP
curl -X POST http://localhost:8080/api/v1/crates/build \
  -H "Content-Type: application/json" \
  -d '{"brief":"Club Primetime","energy_arc":"steady_climb","candidates":[]}'

# Analyze audio file with Symphonia & rustfft forensics
curl -X POST "http://localhost:8080/api/v1/audio/analyze-file?ext=flac" \
  --data-binary "@my_master.flac"
```

---

## 🧠 Supported AI Copilot Backends (`hera-rs/src/dj_brain/`)

HERA's AI Copilot defaults out-of-the-box to **Cloudflare Workers AI (`10,000 Neurons/day` free)** without requiring an API key, alongside automatic parallel `127.0.0.1` discovery (`< 400 ms`) of local CLI agents (`agy`, `claude`, `gemini`, `codex`), local LLM servers, and BYOK providers:

### Free Cloud & Gateway Defaults

| Backend | Endpoint / ID | Description | Cost |
|---------|---------------|-------------|------|
| **Cloudflare Workers AI Free** | `@cf/meta/llama-3.1-8b-instruct-fp8-fast` | Copilot ReAct nativo por defecto en HERA Desktop | **$0.00** (10k Neurons/día) |
| **Cloudflare AI Gateway** | `/api/v1/ai/chat` | Caché global de consultas repetidas y rate-limiting | **$0.00** |

### Local Offline Engines (100% Free & Private)

| Backend | Default Port | Model Example | Installation |
|---------|-------------|---------------|--------------|
| [LM Studio](https://lmstudio.ai) | `http://127.0.0.1:1234/v1` | `meta-llama-3.1-8b-instruct` | Desktop app |
| [Ollama](https://ollama.com) | `http://127.0.0.1:11434/v1` | `llama3.2:3b` | `curl -fsSL https://ollama.com/install.sh \| sh` |
| [llama.cpp](https://github.com/ggerganov/llama.cpp) | `http://127.0.0.1:8080/v1` | Custom GGUF | Standalone binary |
| [LocalAI](https://localai.io) | `http://127.0.0.1:8080/v1` | Custom local models | Docker / binary |
| Custom | `http://127.0.0.1:PORT/v1` | Any OpenAI-compatible endpoint | BYOK (OpenRouter, Gemini, OpenAI) |

---

## ☁️ Zero-OAuth Cloud Onboarding (`hera-rs/src/p2p/cloud_links.rs`)

Connecting external cloud storage (Google Drive, Dropbox, OneDrive, Cloudflare R2) is **100% Zero-OAuth**:
- DJs do not need to register Google Cloud console projects or generate API keys.
- Simply paste the folder share link in HERA Desktop (`/bridge` or `/dna`) or via `POST /api/v1/dj/connect-drive`.
- HERA resolves direct lossless binary streams and bypasses the 25 MB Google Drive virus warning automatically (`&confirm=t`).

---

## 📋 Wishlist & Roadmap Board (DJ & Human-Centric)

This board tracks upcoming features designed specifically to streamline live DJ performance, mobile accessibility, and team collaboration:

```text
========================================================================================
                                 HERA WISHLIST BOARD
========================================================================================

[ 📋 ] DJ Cue Sheet Generator (_00_SET_GUIDE.txt)
       Automatically generate an easy-to-read ASCII overview file in each set folder:
       - Displays track sequence, BPM progression, Camelot harmonic mix path, and energy.
       - Optimized for instant viewing on mobile phones in the DJ booth.

[ 📱 ] Terminal Mobile QR Code Fast-Access
       Upon completing `hera sync push`, display a scannable ASCII QR code in the terminal
       for instant opening of the Google Drive set folder on your phone or tablet.

[ 💾 ] 1-Click "Gig USB" Exporter (`hera sync to-usb E:`)
       Export selected sets directly to FAT32/exFAT USB drives with clean filesystem naming,
       omitting hidden OS metadata files that can cause playback stutter on Pioneer CDJs.

[ 🔄 ] Silent Background Auto-Sync (`auto_sync: true`)
       Automatically sync new sets to Google Drive in the background as soon as DSP
       validation finishes, sending a lightweight OS notification when ready.

[ 🤝 ] B2B Shared Crate Collaboration (`hera sync b2b <drive-folder-link>`)
       Allow two DJs playing Back-to-Back to drop tracks into a shared Google Drive folder;
       Hera downloads, analyzes, and arranges them into an optimal harmonic sequence.

[ 🎧 ] Lightweight Mobile Preview Streamer
       Optionally transcode sets to low-bitrate MP3 previews in a `_previews/` folder
       for quick, data-saving track previewing on mobile connections.
========================================================================================
```

---

## 🤝 Contributing

We welcome contributions from developers, DJs, and audio engineers! Please review [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines on:
- Setting up your local development environment with Rust (`cargo`), Bun `1.4.4`, Vite+ (`vp`), and `moon 2.6.5`.
- Modular `< 350` LOC coding conventions, `ts-rs` IPC contracts (`hera-ipc`), and Tauri v2 ACL Triad parity.
- Running automated test suites and static checks (`cargo check`, `cargo test -p hera-node-rs`, `cargo test -p hera-ipc`, `bun run check`, `bun run test`).
- Submitting Pull Requests under Receipt-Driven Development (`superpowers-odd`).

---

## 📄 License

Hera is licensed under the **MIT License**. See the [LICENSE](LICENSE) file for details.

<div align="center">
  <sub>Crafted for DJs, producers, and audio curators worldwide.</sub>
</div>
