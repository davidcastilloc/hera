# Changelog

All notable changes to **HERA (`davidcastilloc/hera`)** are documented in this file following [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.6.0] - 2026-11-09

### Added
- **Agent-First Root Cargo Workspace, `hera-ipc` (`ts-rs` + ACL Triad Guardian), `< 350` LOC Modularization & Lock-Free `DualDeckEngine` (`ADR-024`):**
  - Consolidated `hera-rs` (`hera-node-rs` `v0.6.0`), `hera-desktop/src-ipc` (`hera-ipc` `v0.6.0`), and `hera-desktop/src-tauri` (`hera-desktop` `v0.6.0`) into a unified root `Cargo.toml` workspace (`resolver = "2"`, default headless members `["hera-rs", "hera-desktop/src-ipc"]`).
  - Introduced the zero-GTK pure-Rust `hera-ipc` crate (`acl_guard.rs`, `workflow_guard.rs`, `typegen.rs`, `dj_types.rs`, `agent_types.rs` -> `hera-desktop/src/generated/hera-typings.ts`) for `< 5s` cross-OS `ts-rs` DTO generation and automated Tauri v2 ACL Triad parity verification across all 8 inlined plugins.
  - Decomposed monolithic modules into `< 350` LOC cohesive files across `hera-rs/src/dj_brain/`, `hera-rs/src/mcp/`, `hera-rs/src/code_mode/`, `hera-desktop/src-tauri/src/plugins/hera_agent/`, `hera-desktop/src-tauri/src/plugins/hera_dj/`, `hera-desktop/src/routes/bridge.tsx` (`src/components/transition/`), and `hera-desktop/src/components/MiniOverlay.tsx` (`src/components/mini-overlay/`).
  - Added `#![forbid(unsafe_code)]` in-memory `DualDeckEngine` (`hera-rs/src/audio_engine/`) over Symphonia PCM frames driven by a lock-free `[AtomicU64; 64]` SPSC `CommandQueue` for sample-accurate 4-Hot-Cue jumping and linear tempo resampling (`0.5..=2.0`) with zero heap allocations in `render_interleaved_stereo`.
- **Autonomous `/agent` Workspace & 44-Tool Loopback HTTP MCP (`ADR-023`):**
  - Added `plugin:hera-agent` (`hera-desktop/src-tauri/src/plugins/hera_agent/`) hosting a per-launch random `Bearer`-authenticated `axum` HTTP MCP router bound strictly to `127.0.0.1:0`, combining `21` DSP/P2P/curator tools with `23` desktop SQLite, VirtualDJ, player, and host OS tools (`44` total).
  - Built the minimalist `/agent` workspace (`agent.tsx`, `AgentComposer.tsx`, `AgentToolStepRow.tsx`, `Ctrl+J` / `Cmd+J`) with live streaming tool execution (`hera:agent-stream`), `@ Contexto` track/playlist/folder attachments, SQLite session persistence, and an inline `Confirmar` approval gate.
- **Single-Window Morphing VirtualDJ `Modo Mini` Overlay (`ADR-022`):**
  - Added atomic single-window morphing (`set_mini_overlay_mode`, `Ctrl+Shift+M` / `Cmd+Shift+M`) switching between full studio (`1180x720`), `360x408` booth card (`Tarjeta de Cabina`), and `560x52` HUD pill (`Píldora HUD`) without spawning extra WebView processes.
  - Added non-blocking `push_virtualdj_live_crate` writing deduplicated `HERA En Vivo.vdjfolder` + `HERA En Vivo_vdj_database.xml` with 4 `<Poi>` Hot Cues, alongside live VirtualDJ `History/tracklist.txt` reader (`read_virtualdj_now_playing`).
- **Unified Dark Booth Studio UI, Minimalist Distillation & Non-Blocking Async Tauri IPC (`ADR-016`..`ADR-021`):**
  - Unified native and webview Dark Booth Studio tokens (`#0c0e12` booth slate + `#14b8a6` studio emerald), `216px` left studio rail, `1.5px`-stroke SVG iconography, silent inline action confirmations, and low-text `Transición` (`bridge.tsx`) and `Mi Estilo` (`dna.tsx`) views.
  - Converted Tauri IPC commands to 100% off-main-thread `async` + `spawn_blocking`, added bounded parallel `127.0.0.1` AI/CLI discovery (`< 400 ms`), enabled tag-tolerant case-insensitive Symphonia audio import, and integrated localhost `tauri-plugin-mcp-bridge` (`127.0.0.1:9223`).
- **5-Stage Zero-Config P2P Cascade, RFC 5766 TURN Relay, Universal Symphonia Streaming Decoder & Rust/WASM Hardening (`ADR-012`..`ADR-015`):**
  - Expanded zero-config P2P cascade (`hera-rs/src/p2p/`): `LAN Direct` -> `Multi-STUN` (`stun.cloudflare.com:3478` + `stun.l.google.com:19302`) -> `16-Port Prediction Spray` + persistent Desktop UDP seeder -> `Cloudflare Calls RFC 5766 TURN Relay` (`Allocate` + `ChannelBind`, `0` R2 bytes) -> `R2 P2P Boost`.
  - Added streaming multi-codec decoder `decode_audio_file_partial` (`WAV`, `FLAC`, `MP3`, `M4A`, `AAC`, `ALAC`, `OGG`, `AIFF`) in `hera-rs/src/forensics.rs`.
  - Introduced typed `HeraError` (`hera-rs/src/error.rs`), warm Cloudflare Worker `OnceLock<P2PMeshState>`, constant-time secret/ticket verification (`constant_time_eq`), `8 KiB` `Seek` UDP chunk reads, and zero-allocation 1D `2-Opt TSP` optimization.

### Fixed
- **Root Cargo Workspace Release Pipelines (`.github/workflows/release-binaries.yml`, `.github/workflows/release.yml`, `hera-ipc`):**
  - Updated cross-OS binary packaging and `Swatinem/rust-cache@v2` workspace paths to consume the root workspace `target/` directory and root `Cargo.lock`, enforced by `verify_workspace_release_workflows` in `hera-desktop/src-ipc/src/workflow_guard.rs`.

---

## [0.5.0] - 2026-11-08

### Added
- **Cloudflare Edge Optimization Primitives (`hera-rs/src/cloudflare_edge.rs`, `ADR-011`):**
  - **Cloudflare KV Global Forensic Dedup Cache (`POST /api/v1/edge/forensic-cache/lookup` & `/store`):** Resolves verified 4 Hot Cues + spectral telemetry in `< 2 ms` at the Edge (saving `420 ms` of laptop FFT CPU per hit) and credits `+5.0 MB` Pioneer Forensic Bounty for newly verified masters.
  - **Cloudflare D1 Edge SQLite DNA Sync (`POST /api/v1/edge/d1-dna-sync`):** Zero-cost (`$0.00`) synchronization of compact `DJ Style DNA` profiles and pinned Set Spine anchors (`Opening`, `Peak`, `Closing`).
  - **Cloudflare Vectorize + `u64` SimHash POPCNT Matcher (`POST /api/v1/edge/vectorize-match` & `GET /api/v1/edge/cloudflare-capabilities`):** Hybrid acoustic texture + Camelot harmonic wheel + BPM curve matching across the global P2P swarm.
- **5 Ecosystem Upgrade Pillars (`hera-desktop` & `hera-rs`):**
  - **Pillar 1 — Visual `2-Opt TSP` Set Spine Builder (`HeraDjToolbar.tsx`, `useFilteredTracks.ts`, `hera_dj.rs`):** Added `🧭 Ordenar Set (2-Opt TSP)` (`plugin:hera-dj|build_dj_set_spine`) to reorder `/library` tracks in-place with Opening/Peak/Closing anchors and live Spine Synergy score badge.
  - **Pillar 2 — Real Spectral Forensics & 4 Hot Cues on Folder Import (`track.rs` & `database.rs`):** Wired `enrich_track_with_hera_dsp` (`decode_audio_bytes` + `audit_pcm_channels` + `analyze_structure_and_style`) into `get_track_from_file` so `rayon` parallel folder scanning automatically populates `bpm`, `camelot`, `energy`, `hera_seal`, and `hot_cues` (`1:Intro`, `2:Breakdown`, `3:Drop`, `4:Outro`) in `hera.db`.
  - **Pillar 3 — Automated Multi-OS Tauri v2 Release Pipeline (`.github/workflows/release.yml`):** Added GitHub Actions matrix (`ubuntu-22.04`, `macos-latest` Universal/Apple Silicon, `windows-latest`) using `oven-sh/setup-bun@v2` + `tauri-apps/tauri-action@v0`.
  - **Pillar 4 — Live Cloudflare Edge & MoQ Swarm Radar (`bridge.tsx`):** Added the **☁️ Radar Enjambre P2P & Cloudflare Edge Optimizer** panel displaying active Guardian nodes, KV Forensic Cache hits, Pioneer Bounty (`+5 MB`), `1:1.5` Turbo Credit, and live `hera://swarm/camelot/*` MoQ Harmonic Pub/Sub events.
  - **Pillar 5 — `Rolldown` Code-Splitting (`vite.config.ts`):** Configured `rolldownOptions.output.manualChunks` (`vendor-react`, `vendor-tanstack`, `vendor-ui`, `index`), eliminating all `> 500 kB` chunk warnings.
- **Swarm-First Invariant & Robin Hood Open-Source Sustenance (`ADR-010`, `839e3e8`):**
  - Enforced `p2p_sharing_enabled = true` (`can_disable_p2p_sharing = false`) across all free/local/beta tiers and added the `🐺🕶️🥫 Limosnita` Robin Hood modal on 1-click export.

---

## [0.4.1] - 2026-11-08

### Fixed
- **Complete `HERA` Desktop Branding (`hera-desktop`, `audio.hera.app`):**
  - Replaced leftover `Museeks` / `museeks` / `io.museeks.app` branding with **`HERA`** / **`hera-desktop`** / **`audio.hera.app`** across `hera-desktop/package.json`, `hera-desktop/index.html`, `hera-desktop/src-tauri/tauri.conf.json`, `hera-desktop/src-tauri/Cargo.toml`, `hera-desktop/src-tauri/Info.plist`, `src-tauri/src/main.rs`, `src-tauri/src/plugins/{app_menu,config,db}.rs`, `src-tauri/src/libs/{error,init}.rs`, `src/routes/settings.{about,library}.tsx`, `src/api/SettingsAPI.ts`, `src/components/GlobalErrors.tsx`, and Lingui translation catalogs.

---

## [0.4.0] - 2026-11-08

### Added
- **Hybrid `Bun 1.4.4` + `Vite+` (`Rolldown` / `Oxlint` / `Oxfmt`) + `moon 2.6.5` Monorepo Orchestration (`ADR-009`):**
  - Migrated `hera-desktop` package management and script execution from `pnpm` to **`Bun 1.4.4`** (`bun.lock`, `~1.07s` clean install for 297 packages) while preserving 100% of `Vite+` (`Rolldown` + `Oxlint` + `Oxfmt`) and Tauri v2 plugins (`@stylexjs/unplugin`, `babel-plugin-react-compiler`, `@lingui/vite-plugin`, `@tanstack/router-plugin`, `vite-plugin-svgr`).
  - Configured **`moon 2.6.5`** (`.moon/workspace.yml`, `hera-rs/moon.yml`, `hera-desktop/moon.yml`) with cross-project dependency graph (`hera-desktop` -> `hera-rs`) and content-addressed task caching (`moon run :check :test hera-desktop:build` completes 5 tasks in `128ms` on cache hit).
- **Pure-Rust Multi-Turn ReAct Agent Harness (`HeraAgentHarness`) & 12-Backend LLM Registry (`ADR-008`, `7fd1bcf`):**
  - Expanded `list_supported_ai_backends` to 12 backends (`cloudflare_workers_ai_free`, `lmstudio_local`, `ollama_local`, `jan_local`, `llamacpp_local`, `vllm_local`, `openrouter_free`, `groq_lpu`, `deepseek_r1`, `gemini_flash`, `openai_gpt4o`, `anthropic_claude`).
  - Added live local `/v1/models` auto-discovery (`POST /api/v1/ai/probe`, Tauri `probe_ai_backend`) and a multi-turn ReAct tool-calling loop (`evaluate_agent_harness_turn`) executing against HERA's 20 operational MCP tools with dual OpenAI `tool_calls` + `<tool_call>` GGUF parsing and self-healing error recovery.

---

## [0.3.0] - 2026-11-08

### Added
- **`HERA Desktop` (`hera-desktop/` — Tauri v2 + React 19 Compiler + StyleX):**
  - Cloned `martpie/museeks` (`v0.23.4`, MIT) and adapted it into a `~8 MB` 100% Pure-Rust + React 19 DJ pre-gig command center (`59a4da4`, `a05b217`, `d876eb7`).
  - Added top `TrackProgress.tsx` bar with **4 clickable acoustic Hot Cue pins** (`🟢 1:Intro`, `🔵 2:Breakdown`, `🔴 3:Drop`, `🟡 4:Outro`) and virtualized DJ columns (`BPM`, `Camelot Key`, `4 Hot Cues`) in `TrackListHeader.tsx` and `TrackRow.tsx`.
  - Added deterministic offline toolbar (`HeraDjToolbar.tsx` + `bridge-hera.ts`) for 1-click export to **VirtualDJ (`.vdjfolder`)**, **Rekordbox (`.xml`)**, **Traktor Pro (`.nml`)**, and **M3U8**.
  - Added `/dna` view (`dna.tsx`) with the full 24-key Camelot Wheel (`1A..12A` and `1B..12B`), DJ software selector, and Golden Rules switches backed by `hera-rs/src/dj_brain.rs`.
  - Added `/bridge` view (`bridge.tsx`) with Deck A ➔ Deck B P2P Harmonic Bridge resolution, 15-second transition preview (`Cue 4 Mix Out ➔ Cue 1 Intro`), and **Cloudflare Workers AI Free (`10,000 Neurons/day` at `$0.00`)** copilot.
- **Constrained `2-Opt TSP` Set Spine Builder (`hera-rs/src/synergy.rs`):**
  - Added `SetSpineConfig`, `EnergyAnchor`, `PinnedTrackConstraint`, and `build_set_spine` (`POST /api/v1/crates/set-spine`, MCP tool `build_set_spine`, and Code Mode `hera.buildSetSpine()`), raising the native Rust MCP server to **22 tools**.
- **Pure-Rust Native DJ Exporters & `DJ Style DNA` Engine (`hera-rs/src/export.rs`, `hera-rs/src/dj_brain.rs`):**
  - Exposed `export_dj_crate`, `compute_dj_style_dna`, and `chat_with_dj_copilot` over HTTP REST, MCP (`stdio` + HTTP), Tauri v2 IPC (`plugin:hera-dj`), and TypeScript Code Mode.
- **Live Playwright UI/UX QA Gallery (`docs/superpowers/assets/hera_live_*.png` & `hera-desktop/README.md`):**
  - Versioned 4 production screenshots (`1100x680` and `900x550`) and the 5 DJ Curator Superpowers guide (`39e920f`).

### Fixed
- **Root `.gitignore` Anchoring & `HERA Desktop` UI QA (`ADR-007`, `d876eb7`, `627079a`, `dcf404e`):**
  - Anchored `/lib/` to the root `.gitignore` so `hera-desktop/src/lib/` is tracked in Git.
  - Enforced `1:1` column order and flex widths between `TrackListHeader.tsx` and `TrackRow.tsx`, high-contrast `camelotBadgeSelected`, and default DJ-booth dark mode.
  - Registered `hera-dj` commands across the Tauri v2 ACL triad (`main.rs`, `build.rs`, `capabilities/main.json`).
  - Repositioned `Toasts.tsx` (`top: 56px, right: 12px`) with click-to-dismiss so playback alerts never overlap the `🤖 Consultar Copiloto` button in `/bridge`, and synchronized `useLibraryStore.setState({ tracks })` on library load.

### Removed
- **BREAKING (`c08677f`):** Purged legacy Python runtime and Streamlit dual-deck scripts in favor of the 100% Pure-Rust (`hera-rs`) + Tauri v2 (`hera-desktop`) architecture (`~8 MB` native binary with zero Python dependency).

---

## [0.2.0] - 2026-11-08

### Added
- **5-Layer Post-P2P Federated R2 Swarm Orchestrator (`hera-rs/src/p2p.rs`, `hera-rs/src/p2p_exchange.rs`, `ADR-003`, `ADR-004`, `ADR-005`):**
  - Constrained Water-Filling shard allocation respecting Cloudflare Free Tier safety caps (`9,000 MB` R2, `85,000 req/day`).
  - Latency-Aware Power-of-Two-Choices (`P2C`), single-use `60s` HMAC-signed `X-Hera-Swarm-Ticket` tokens, `1 : 1.5` Turbo Credit rewards, and 64-byte `Proof-of-Custody` range audits.
  - Streaming Merkle Inclusion Proofs (`16 x 256 KiB = 4 MB`), `ed25519` endpoint identity roaming (`Wi-Fi -> 5G`), Multi-Node `HTTP/3-QUIC` parallel striping, `Delta-CRDT` sync, and `MoQ` Semantic Harmonic Pub/Sub.
