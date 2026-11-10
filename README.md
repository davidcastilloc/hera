<div align="center">

<img src="assets/hera_banner_y2k.jpg" alt="HERA v0.6.0 — Pure-Rust AI DJ Studio & Global P2P Swarm Banner" width="100%" />

# 🎧 HERA (`v0.6.0`)
### *The Autonomous, Local-First AI DJ Studio & Symbiotic P2P Swarm*

**«Todo es posible si lo imaginas: De la intención musical al booth del DJ en segundos»**  
**🐝 Lema del Enjambre: *«Mientras más nodos de Hera, más fuerte es el sistema de enjambres»***

[![Pure Rust](https://img.shields.io/badge/Rust-100%25%20Pure%20Rust%20v0.6.0-dea584.svg?style=flat&logo=rust)](hera-rs/)
[![Tauri v2](https://img.shields.io/badge/Desktop-Tauri%20v2%20%2B%20React%2019-24C8D8.svg?style=flat&logo=tauri)](hera-desktop/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Protocol: MCP](https://img.shields.io/badge/protocol-MCP%2044%20Tools%20%7C%20Code%20Mode-14b8a6.svg)](https://modelcontextprotocol.io/)
[![Cloud: Cloudflare Edge](https://img.shields.io/badge/Cloud-D1%20%7C%20R2%20%7C%20KV%20%7C%20Vectorize%20%7C%20Workers%20AI-orange.svg)](https://workers.cloudflare.com/)
[![P2P: 5-Stage Zero-Config Cascade](https://img.shields.io/badge/P2P-LAN%20%7C%20STUN%20%7C%20Spray%20%7C%20TURN%20%7C%20R2%20Swarm-cyan.svg)](docs/FEDERATED_R2_SWARM.md)
[![Philosophy: Local--First](https://img.shields.io/badge/philosophy-Swarm--First%20%26%20Local--First-14b8a6.svg)](MANIFESTO.md)

<p align="center">
  <b>HERA (`v0.6.0`)</b> es un estudio de curaduría acústica, copiloto autónomo y enjambre P2P simbiótico diseñado para DJs reales.<br/>
  Motor nativo <b>100% Pure Rust + Tauri v2 (`~8 MB`, cero Python)</b>. Analiza audio en <code>~0.08s</code> con <code>Symphonia + rustfft</code> (BPM, Rueda Camelot de 24 llaves, 4 Hot Cues acústicos, fase de subgraves), ordena sets con <code>2-Opt TSP</code>, se transforma en un overlay en vivo para VirtualDJ (<code>Modo Mini</code>), exporta en 1 clic a VirtualDJ / Rekordbox / Traktor y comparte Masters Lossless verificados a través de una cascada P2P Zero-Config de 5 etapas y nodos comunitarios <code>Edge Guardian</code>.
</p>

[📸 Tour Visual (5 Pantallas)](#-tour-visual-del-producto-las-5-pantallas-de-hera-v060) • [🐝 Concepto de Enjambres](#-el-concepto-de-los-enjambres-p2p-por-el-bien-del-enjambre) • [✨ Manifiesto](MANIFESTO.md) • [📖 Modos de Uso](docs/MODOS_DE_USO.md) • [🚀 Quickstart](#-quickstart--installation)

---

</div>

## 📑 Table of Contents
- [📸 Tour Visual del Producto: Las 5 Pantallas de HERA `v0.6.0`](#-tour-visual-del-producto-las-5-pantallas-de-hera-v060)
- [🐝 El Concepto de los Enjambres P2P (*«Por el bien del enjambre»*)](#-el-concepto-de-los-enjambres-p2p-por-el-bien-del-enjambre)
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

## 📸 Tour Visual del Producto: Las 5 Pantallas de HERA `v0.6.0`

HERA Desktop (`hera-desktop`) está construido bajo el sistema visual **Dark Booth Studio (`#0c0e12` pizarra de cabina + `#14b8a6` esmeralda estudio)** para operar sin fatiga visual en cabinas oscuras, con menos de 50 palabras de ruido de interfaz por pantalla y respuesta instantánea (`< 50 ms`).

### 1. 🎧 Biblioteca DJ + 4 Pines de Hot Cues + Exportador de Cabina (`/library`)
Importa tus carpetas locales (`WAV`, `FLAC`, `MP3`, `M4A`, `AAC`, `ALAC`, `OGG`, `AIFF`), salta en 1 clic a los **4 Hot Cues acústicos** en el reproductor superior (`🟢 Cue 1 Intro`, `🔵 Cue 2 Breakdown`, `🔴 Cue 3 Drop`, `🟡 Cue 4 Outro`), ordena la mezcla entera con **`2-Opt TSP`** y exporta con todos los cues inyectados directo a **VirtualDJ (`.vdjfolder`)**, **Rekordbox (`.xml`)**, **Traktor (`.nml`)** o **M3U8**.

![HERA v0.6.0 — Pantalla 1: Biblioteca DJ y 4 Hot Cues](docs/superpowers/assets/hera_v060_1_library_1180x720.png)

---

### 2. ✨ Agente Autónomo con 44 Herramientas MCP (`/agent` · `Ctrl+J`)
Un espacio conversacional centrado y sin distracciones conectado a un servidor **MCP HTTP Loopback (`127.0.0.1:0` con Bearer secreto)** que expone **44 herramientas nativas en Rust** (análisis DSP, curaduría `2-Opt TSP`, búsqueda/descarga en el enjambre P2P, control del reproductor, escritura en vivo a VirtualDJ e inspección del sistema). Adjunta pistas o playlists con **`@ Contexto`**, dispara flujos rápidos con **`/set`** o **`/virtualdj`**, e inspecciona cada llamada en filas colapsables de `28 px` con modo **`Auto`** o **`Confirmar`**.

![HERA v0.6.0 — Pantalla 2: Agente Autónomo con 44 Tools MCP](docs/superpowers/assets/hera_v060_2_agent_1180x720.png)

---

### 3. ⇄ Transición y Puente Armónico con Enjambre P2P (`/bridge`)
¿Necesitas cruzar de un groove a `122 BPM en 8A` (*Baiana*) a un himno de hora pico a `128 BPM en 10A` (*Pepas VIP*) sin forzar el pitch ni chocar las voces? El flujo horizontal **`Actual → Puente sugerido → Destino`** calcula los 2 eslabones armónicos exactos tanto en tu disco local como en el **Enjambre P2P global**, te permite **pre-escuchar 15 segundos del cruce (`Cue 4 Mix Out → Cue 1 Intro`)** y ajustar la transición en un clic (`Más suave`, `Más energía`, `Sin voces`).

![HERA v0.6.0 — Pantalla 3: Transición y Puente Armónico P2P](docs/superpowers/assets/hera_v060_3_transition_1180x720.png)

---

### 4. ◈ Mi Estilo y ADN Musical (`/dna`)
Escanea tu colección en Rust (`0%` de internet requerido) y sintetiza tu identidad musical: programa principal de cabina (`VirtualDJ`, `Rekordbox`, `Traktor`, `Serato`), **Ritmo habitual (`125 BPM`)**, **Tonos principales (`8A · 9A · 10A`)**, barra de **Balance musical (`Melódico · Hora pico · Vocal`)**, distribución detallada de las 24 llaves de la **Rueda Camelot** y tus 3 interruptores de **Preferencias al ordenar** (*Evitar choque de voces*, *Mantener graves continuos*, *Ocultar audio de baja calidad*).

![HERA v0.6.0 — Pantalla 4: Mi Estilo y ADN Musical](docs/superpowers/assets/hera_v060_4_dna_1180x720.png)

---

### 5. ⧉ Modo Mini Always-on-Top para Cabina en Vivo (`Ctrl+Shift+M`)
Cuando mezclas en vivo en una sola pantalla de laptop con **VirtualDJ**, una ventana grande tapa las ondas y los decks. Al pulsar `Ctrl+Shift+M`, la misma ventana de HERA se transforma sin consumir un solo megabyte extra de RAM en dos formatos flotantes anclados abajo a la derecha:
- **Tarjeta de Cabina (`360×408 px`):** Detecta automáticamente qué pista está sonando en VirtualDJ (`History/tracklist.txt`), sugiere las 3 siguientes canciones armónicas o un puente en 2 pasos, permite pre-escuchar en audífonos (`▶`) y al pulsar **`+ VDJ`** inyecta la pista al instante en `Documents/VirtualDJ/Folders/HERA En Vivo.vdjfolder` con sus 4 Hot Cues `<Poi>`.
- **Cápsula HUD Ultracompacta (`560×52 px`):** Una sola línea flotante (`Sonando → Siguiente · Tono · BPM · 4 Cues · [+ VDJ]`).

![HERA v0.6.0 — Pantalla 5: Modo Mini Tarjeta 360x408 y Cápsula HUD 560x52](docs/superpowers/assets/hera_v060_5_mini_overlay_1180x720.png)

---

## 🐝 El Concepto de los Enjambres P2P (*«Por el bien del enjambre»*)

> **Regla de Oro de HERA:** *«La mejor decisión es por el bien del enjambre siempre. Mientras más nodos de Hera, más fuerte es el sistema de enjambres.»*

![Concepto y Arquitectura del Enjambre P2P Simbiótico de HERA v0.6.0](assets/hera_swarm_concept.jpg)

### ¿Cuál es la idea detrás de los Enjambres de HERA?

En las redes P2P tradicionales (Soulseek, BitTorrent), la mayoría de los usuarios descargan archivos de baja calidad o transcodes falsos y cierran el programa (*leeching*), mientras que los servidores centrales en la nube quiebran por costos de ancho de banda y almacenamiento. **HERA resuelve ambos problemas con un ecosistema simbiótico de costo marginal `$0.00`:**

#### 1. 🤝 El Trueque Simbiótico (`100% Gratis` a cambio de Nutrir el Enjambre)
- Todo el motor acústico en Rust (`Symphonia + rustfft`), los 4 Hot Cues automáticos, el optimizador `2-Opt TSP`, los exportadores para VirtualDJ/Rekordbox/Traktor y el Copiloto IA con **Cloudflare Workers AI (`10,000 Neurons/día` gratis)** cuestan **`$0.00`** para cualquier DJ.
- A cambio, en todos los nodos gratuitos y locales (`free`, `beta_mvp`, `local_unlimited`), **compartir en el enjambre viene activado por defecto (`p2p_sharing_enabled = true`)**. Cada vez que un DJ analiza y valida un Master de estudio en su laptop, esa pista fortalece el catálogo global para los demás DJs. *(Solo las cuentas `pro_monthly` con bóveda privada de `50 GB` pueden apagar el P2P en dubplates o edits exclusivos).*

#### 2. 🛡️ Cuarentena Forense Zero-Trust y Doble Sello Criptográfico
- Ningún archivo entra al enjambre ni a tu carpeta `library/` sin pasar primero por `quarantine/` y superar la auditoría espectral de [`hera-rs/src/forensics.rs`](hera-rs/src/forensics.rs):
  - **Detector de Falsos Lossless (`rustfft`):** Rechaza MP3 de `128 kbps` inflados a `.flac`/`.wav` detectando cortes de ladrillo por debajo de `20 kHz`.
  - **Auditoría de Fase Sub-Bass (`< 120 Hz`):** Garantiza correlación estéreo `r >= 0.25` para que los subwoofers del club nunca se cancelen en mono.
  - **Doble Sello Anti-Poisoning (`hera_seal` + `audio_sha256`):** Protege el enjambre contra ataques *Bait-and-Switch*. Si un nodo intenta servir un bloque alterado, es aislado de inmediato en `quarantined_nodes`.

#### 3. 🌐 Cascada P2P Zero-Config de 5 Etapas (Sin abrir puertos en el router)
Cuando pides una pista o un puente armónico, [`hera-rs/src/p2p/`](hera-rs/src/p2p/) conecta los nodos automáticamente a través de 5 niveles:
1. **Etapa 1 — `LAN Direct Subnet`:** Transferencia directa por Wi-Fi/Ethernet local en cabina (`0 ms` WAN, `0` bytes de nube).
2. **Etapa 2 — `Multi-STUN + UDP Hole Punching`:** Perforación simultánea RFC 5389 (`stun.cloudflare.com:3478` + `stun.l.google.com:19302`) para NAT cónico.
3. **Etapa 3 — `16-Port Prediction Spray`:** Rocío determinista de 16 puertos con apretón de manos `HERA_PUNCH_SYN` / `HERA_PUNCH_ACK` para atravesar CGNAT móvil.
4. **Etapa 3.5 — `Cloudflare Managed TURN Relay (RFC 5766)`:** Retransmisión binaria (`ChannelBind 0x4000`) que salva CGNAT simétrico estricto consumiendo **`0` bytes de R2**.
5. **Etapa 4 — `Enjambre R2 Federado & P2P Boost`:** Descarga paralela multi-nodo (`Swarm Striping` sobre HTTP/3 QUIC) desde los nodos comunitarios `Edge Guardian`.

#### 4. 🦀 Nodos Comunitarios `Edge Guardian` y Matemática `Constrained Water-Filling` (`$0.00/mes`)
Cualquier DJ o desarrollador puede desplegar [`hera-rs`](hera-rs/README.md) en su cuenta gratuita de **Cloudflare Workers + R2 (`$0.00/mes`)** ([guía completa en `docs/FEDERATED_R2_SWARM.md`](docs/FEDERATED_R2_SWARM.md)):
- **Protección Matemática de Facturación `$0.00` (`Constrained Water-Filling`):** El enjambre reparte los fragmentos más demandados imponiendo techos estrictos de seguridad en cada Worker comunitario: **`9.0 GB` de R2** (dejando `1 GB` libre bajo los `10 GB` gratuitos) y **`85,000 req/día`** (dejando `15k` libres bajo las `100,000 req/día` gratuitas). Ningún colaborador paga jamás un centavo.
- **Fragmentos Ciegos de `4 MB` (`8.4x` Eficiencia) + Árboles Merkle (`16×256 KiB`):** Los archivos se dividen en bloques inmutables `shard-blobs/{hmac}.hbin` de `4 MB` sin nombres de artistas ni canciones en claro, protegidos por tickets firmados de `60s` (`X-Hera-Swarm-Ticket`) y pruebas Merkle estilo BitTorrent v2 (BEP 52) / Iroh.
- **Meritocracia `1 : 1.5` y Auditoría `Proof-of-Custody` (64 Bytes):** Mientras el nodo pasa los desafíos aleatorios de 64 bytes en R2, disfruta de **carril rápido ilimitado (`EDGE_GUARDIAN_UNLIMITED_FAST_LANE`)** y gana **`1.5 MB` de Crédito Turbo por cada `1.0 MB` servido** a otros DJs (`ProofOfDeliveryReceipt`).
- **Deduplicación Global CAS y Bounty al Pionero (`+38 MB`):** Si 20 DJs tienen el mismo Master, la bóveda guarda **1 sola copia física** (`0.0 MB` extra) y acredita **`+38.0 MB` de cuota libre** al curador pionero que descubrió la pista.

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

> 💡 **Guías en español:** Consulta la [Guía Completa de Modos de Uso](docs/MODOS_DE_USO.md) y la [Guía de HERA en VPS Remoto + Enjambre Edge](docs/VPS_REMOTE_GUIDE.md).

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
  <sub>Crafted for DJs, producers, and audio curators worldwide · <b>«Por el bien del enjambre»</b></sub>
</div>
