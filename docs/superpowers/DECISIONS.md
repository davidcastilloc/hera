# 🧠 HERA — Architectural Decisions & Distilled Memory (`DECISIONS.md`)

> **Multi-Instance Source of Truth (Windows / macOS / Linux / Antigravity)**  
> This file stores distilled architectural decisions, stack invariants, and critical root-cause findings. Update it **only** when a task reveals a critical technical constraint or architectural decision (`superpowers:multi-instance-handoff`). Never dump raw logs or OS-specific absolute paths here.

---

## 1. Detected Stack & Tooling Baseline (`hera`)

- **Python Runtime & Package Manager:** Python `>=3.11` managed deterministically via `uv` (`pyproject.toml` + `uv.lock`, build backend `hatchling`).
- **Core Python Libraries:** `pydantic>=2.0`, `mcp>=1.0.0`, `aiosqlite>=0.20.0`, `click>=8.0.0`, `httpx>=0.27.0`, `structlog>=24.0.0`, `mutagen>=1.47.0`, `streamlit>=1.35.0`, `google-antigravity>=0.1.14`, `google-genai>=2.20.0`.
- **Optional Audio DSP (`[project.optional-dependencies.analysis]`):** `librosa>=0.10.0`, `pyacoustid>=1.3.0`, `numpy>=1.26.0`, `soundfile>=0.12.0`.
- **Rust Edge/Native DSP Node (`hera-rs/Cargo.toml`):** `hera-node-rs` (`v0.2.0`, Rust Edition 2021) using `axum 0.7`, `symphonia 0.5`, `rustfft 6.2`, dual-runtime feature flags `native` (`tokio 1.40`) and `cloudflare` (`worker 0.4`).
- **Quality & Verification Commands:**
  - **Python Lint:** `uv run ruff check .`
  - **Python Format Check:** `uv run ruff format --check .`
  - **Python Typecheck:** `uv run mypy src/`
  - **Python Test Suite:** `uv run pytest`
  - **Rust Check & Tests:** `cargo check --manifest-path hera-rs/Cargo.toml` / `cargo test --manifest-path hera-rs/Cargo.toml`

---

## 2. Core Architectural Invariants

1. **Quarantine First:** All external downloads land in `quarantine/` first; tracks are promoted to `library/` only after passing zero-trust FFT spectral validation (zero synthetic sine waves or fake upscales).
2. **4-Layer Unified Holistic Synergy:** Crate sequencing combines Camelot Wheel harmonic compatibility, gradual BPM tempo curves, transient punch matching (Crest Factor), and 64D style embeddings (`u64` SimHash POPCNT).
3. **Open-Core Repository Boundary (`hera` <-> `hera-cloud`):**
   - `hera` (Public Repo): Local/VPS/Edge client, DSP forensics, MCP server, and P2P community node (`share_on_p2p = true` by default).
   - `hera-cloud` (Private Repo): SaaS control plane, Cloudflare R2 CAS vault, paid tier enforcement (`beta_mvp` vs `pro_monthly`), and private unit economics. Never leak private SaaS pricing/admin routes into `hera`.
4. **Cross-Platform Multi-Instance Portability:** All code, configs, plans, and docs must use repository-relative paths (`pathlib.Path`) and regular files in `.agent/skills/` and `.claude/skills/` (zero OS symlinks, zero hardcoded drive letters or home directories).

---

## 3. Chronological Decision Log

### [2026-11-08] ADR-001: Adoption of `superpowers-odd` Multi-Instance Workflow (ODD + RDD)
- **Context:** Engineering takes place across three distinct environments (Windows, macOS, and Linux/Antigravity). OS symlinks fail on Windows without Administrator permissions, and long sessions suffer from context rot.
- **Decision:**
  1. Forked `obra/superpowers` to `davidcastilloc/superpowers-odd` with custom `multi-instance-handoff` master skill and ODD/RDD gates.
  2. Installed skills as regular tracked files in `.agent/skills/` and `.claude/skills/` (zero OS symlinks).
  3. Established `docs/superpowers/{specs,plans,DECISIONS.md}` as the Git-backed synchronization state across all instances.
  4. Enforced Semantic Release / Conventional Commits under `git config user.name "David Castillo"` and `git config user.email "vikruzdavid@gmail.com"` (`davidcastilloc`) and removed hardcoded Windows paths from `AGENTS.md`.

### [2026-11-08] ADR-002: Test Suite Environment Requirements (`--all-extras` & Temp Dir Isolation)
- **Context:** During Phase 3 zero-regression verification:
  1. `tests/unit/test_transient_synergy_regression.py` requires `librosa`, which is declared under `[project.optional-dependencies] analysis`.
  2. Both `hera/hera-rs` and `hera-cloud` use `std::env::temp_dir().join("hera_local_p2p_test_dir")` in `test_multi_storage_r2_quota_zero_oauth_gdrive_and_local_p2p_seeder`.
- **Decision:**
  1. Always initialize the Python development environment with `uv sync --all-extras` so `dev` (`pytest`, `ruff`, `mypy`) and `analysis` (`librosa`, `soundfile`) extras are present.
  2. Run `cargo test` in `hera-rs` and `hera-cloud` sequentially when both repositories are mounted on the same host to avoid `/tmp/hera_local_p2p_test_dir` race conditions.

### [2026-11-08] ADR-003: Cloudflare P2P Swarm Water-Filling, Catalog Blinding & Custody Invariants
- **Context:** Scaling the P2P swarm across community-contributed Cloudflare Workers (`10 GB` free R2, `100,000 req/day`) and `hera-cloud` required strict billing safety caps, anti-scraping catalog protection, and zero-trust custody enforcement while keeping `hera-rs` dual-target compatible (`native` binary + `wasm32` Worker).
- **Decision:**
  1. **Constrained Water-Filling & Active Node Filtering:** Cap community Workers at `9,000 MB` R2 (`9.0 GB`) and `85,000 req/day`. Sort nodes by ascending effective headroom during Water-Filling so constrained nodes saturate first and overflow demand spills over to unconstrained nodes. Filter healthy nodes strictly by `edge_guardian_active && custody_status == "ONLINE_HEALTHY"` so revoked nodes immediately drop out of swarm capacity and P2C routing.
  2. **Single-Use 60s Signed Tickets & 4 MB Immutable Shards:** Blind R2 shard keys as `shard-blobs/{hmac}.hbin`, require HMAC-SHA256 `X-Hera-Swarm-Ticket` headers with a 60s TTL and unique `nonce` in `ticket_id`, reject ticket replays against `delivery_receipts`, and serve `4 MB` (`4,194,304` B) blocks with `Cache-Control: public, max-age=31536000, immutable` (`1 : 1.5` Turbo Credit ratio).
  3. **Dual-Target Parity (`hera-rs` <-> `hera-cloud`):** Keep `#[cfg(not(target_arch = "wasm32"))] pub fn push_federation_over_http` in `P2PMeshState` so the native `hera-node` binary (`src/main.rs`) compiles cleanly alongside the Cloudflare Worker target, while keeping `/api/v1/cloud/economics` exclusively in `hera-cloud`.

### [2026-11-08] ADR-004: `p2p_exchange.rs` Canonical Module, Swarm MCP/Code Mode Parity, Static Type Hardening & CI Guards
- **Context:** Auditing the documentation after landing the Federated R2 Swarm (`e7f38de`) revealed 5 gaps: `hera-rs/src/p2p_exchange.rs` was referenced in docs (`docs/FEDERATED_R2_SWARM.md`, `AGENTS.md`, `CLAUDE.md`) without a physical module file; the Swarm endpoints were exposed over HTTP REST but missing in MCP (`mcp.rs`), Code Mode (`code_mode.rs`), OpenAPI, and `skills/hera/SKILL.md`; `uv run mypy src/` had 42 errors (including 3 real handler signature bugs in `src/hera/agent/tools.py` and an unguarded headless `pystray` import in `src/hera/desktop/tray.py`); and `.github/workflows/ci.yml` lacked a `mypy` gate and an Open-Core leak check.
- **Decision:**
  1. **Physical `hera-rs/src/p2p_exchange.rs` Module & Process-Isolated Test Dir:** Created `hera-rs/src/p2p_exchange.rs` re-exporting all Swarm types and providing `compute_water_filling_allocations`, `compute_elastic_replication_factor`, `derive_blind_shard_key`, and `issue_custody_challenge`. Isolated `hera_local_p2p_test_dir` with `std::process::id()` so parallel `cargo test` runs across `hera` and `hera-cloud` never collide.
  2. **Full MCP, Code Mode & OpenAPI Parity for the Swarm:** Exposed `get_swarm_strength`, `swarm_heartbeat`, `swarm_route_ticket`, and `swarm_custody_audit` (`hera.getSwarmStrength`, `hera.swarmHeartbeat`, `hera.swarmRouteTicket`, `hera.swarmCustodyAudit`) across `mcp.rs`, `code_mode.rs`, `/openapi.json`, and `skills/hera/SKILL.md`.
  3. **Zero `mypy` Errors & Headless Linux Safety:** Fixed all 42 `mypy` errors across 19 files (`0 issues in 61 source files`), wired `Database(cfg.db_path)` into `src/hera/agent/tools.py` (`handle_audit_repertoire`, `handle_build_dj_crate`, `handle_analyze_batch`), and guarded `src/hera/desktop/tray.py` with `PYSTRAY_BACKEND=dummy` on headless Linux.
  4. **CI Gate & Multi-OS Binary Releases:** Added `uv run mypy src/` and an automated Open-Core Security Boundary Check to `.github/workflows/ci.yml`, plus `.github/workflows/release-binaries.yml` for `Linux`, `macOS`, and `Windows` `hera-node` builds.

### [2026-11-08] ADR-005: 5-Layer Post-P2P Swarm Evolution (Streaming Merkle Proofs, Iroh Endpoint Identity, Latency-Aware P2C Striping, Delta-CRDT & MoQ Harmonic Pub/Sub)
- **Context:** To evolve the Federated R2 Edge Swarm beyond classic P2P bottlenecks (waiting for full 4 MB/38 MB downloads before hash validation, losing node reputation on Wi-Fi-to-5G handoffs, single-node download bottlenecks, offline festival sync conflicts, and polling overhead), we designed and verified 5 incremental Post-P2P architectural layers across `hera-rs` and `hera-cloud`.
- **Decision:**
  1. **Layer 3 — Streaming Merkle Proofs (`16 x 256 KiB = 4 MB`, BitTorrent v2 BEP 52 / Iroh style):** Implemented `ShardMerkleTree` and 4-sibling `MerkleInclusionProof` (`compute_shard_merkle_tree`, `generate_merkle_inclusion_proof`, `verify_merkle_inclusion_proof`) delivered via `X-Hera-Merkle-Root` and `X-Hera-Merkle-Proof` headers on `GET /api/v1/swarm/shard/:blob_id`.
  2. **Layer 2 — Cryptographic `endpoint_pubkey_id` (`ed25519_<hex>`) + `Wi-Fi -> 5G` Roaming + Stateless CGNAT Relay:** Anchored node identity and `1:1.5` Turbo Credits to `endpoint_pubkey_id`, incrementing `network_generation` on endpoint changes without resetting reputation and providing `relay_assisted_url` (`https://relay.hera.audio/iroh-tunnel/<pubkey>`) when `behind_cgnat = true`.
  3. **Layers 1 & 2 — `Latency-Aware P2C` + Multi-Node `HTTP/3-QUIC-MULTIPLEXED` Parallel Striping (`POST /api/v1/swarm/parallel-manifest`):** Weighted `ewma_rtt_ms` alongside daily Worker request load (`compute_node_p2c_composite_cost`) and striped 4 MB blocks across distinct `Edge Guardian` nodes (`swarm_parallel_manifest` in MCP / `hera.swarmParallelManifest` in Code Mode).
  4. **Layer 4 — Conflict-Free `Delta-CRDT` State Synchronization (`POST /api/v1/swarm/crdt-sync`):** Implemented commutative, associative, and idempotent `merge_swarm_crdt_deltas` combining `GCounter` (monotonic megabytes served + play counts) and `LWW-Element-Set` (HLC-timestamped track metadata & Hot Cues) verified via `compute_crdt_state_digest_sha256`.
  5. **Layer 5 — `MoQ` Semantic Harmonic Pub/Sub (`POST /api/v1/swarm/subscribe`):** Enabled subscriptions to `hera://swarm/<camelot>/<bpm_min>-<bpm_max>` (`are_camelot_keys_harmonically_compatible`) with immediate matching events and proactive push inside `SwarmHeartbeatResponse.semantic_pubsub_events`.

### [2026-11-08] ADR-006: DJ Laptop-First Product Alignment, Cloudflare AI Free Default, DJ Brain & Museeks-Inspired Desktop UI
- **Context:** Product & business logic review revealed that DJs execute Hera on the same laptop where they run VirtualDJ, Rekordbox, or Traktor, and require tracks physically downloaded to local disk (`library/`, `sets/`) rather than locked inside cloud buckets. Additionally, non-technical DJs (including users in Venezuela/LATAM without USD credit cards) need zero-friction access both offline and online.
- **Decision:**
  1. **Laptop-First Storage & Native DJ Exporters:** Physical tracks always land on the DJ's local disk; extend `src/hera/domain/export.py` to support **VirtualDJ** (`.vdjfolder` + `<Poi>` 4 Hot Cues) and **Traktor** (`collection.nml` `<CUE_V2>`) alongside Rekordbox XML, M3U8, and Ableton Live.
  2. **Deterministic Offline UI + Cloudflare Workers AI Free Default:** When offline, provide 100% deterministic UI buttons (zero fake chat heuristics). When online, default out-of-the-box to **Cloudflare Workers AI (`10,000 Neurons/day` free using `@cf/meta/llama-3.1-8b-instruct-fp8-fast` / `llama-3.2-3b-instruct` = ~1,400 turns/day at `$0.00` + AI Gateway caching/fallbacks)** via `hera-cloud`, plus a clean visual selector for BYOK (`OpenRouter`, `Gemini`, `OpenAI`) and optional local engines (`Ollama`, `LM Studio`).
  3. **Persistent DJ Brain & Automated Style Onboarding (`$0.00` D1 Sync):** Auto-compute the DJ's musical DNA (`bpm_zones`, `dominant_camelot`, `artist_dna_profile`) from local folders during onboarding, persist preferences (`dj_preferences`) and conversations locally in `hera.db`, and sync compressed session summaries to Cloudflare D1 (`5 GB` free = ~14M messages).
  4. **Desktop UI & Full-Stack Architecture (Clon Directo de `martpie/museeks` MIT `v0.23.4` + Instalador 100% Rust `~8 MB` sin Python Empaquetado):**
     - **Frontend (`HERA Desktop` — Base `martpie/museeks`):** Reemplazar el reproductor pesado Dual-Deck en Streamlit (`src/hera/ui/dj_player.py`) clonando directamente el código base real de **`martpie/museeks` (MIT)** sobre **Tauri v2 (`src-tauri`) + React 19 (`babel-plugin-react-compiler` 1.0) + StyleX (Zero-Runtime Atomic CSS) + `@tanstack/react-virtual` + `@tanstack/react-router` + `Zustand` + `@dnd-kit` + `ts-rs`**. Esto conserva al 100% sus `+40` componentes UI/UX ya curados (`TrackList.tsx`, `TrackRow.tsx`, `Header.tsx`, `PlayingBar.tsx`, `SideNav.tsx`, temas oscuros, atajos de teclado y bridge IPC en `src/lib/bridge-*.ts`) sin reescribir nada desde cero, y los extiende con las columnas y vistas de HERA (`BPM`, `Camelot Key`, `Energy`, `4 Hot Cues` clicables `🟢 Cue 1 Intro`, `🔵 Cue 2 Phrase`, `🔴 Cue 3 Drop`, `🟡 Cue 4 Mix Out`, `Hera Seal`, `Mi ADN DJ`, `Puente Harmónico P2P` y `Hera AI Copilot`).
     - **Local Desktop Backend (`100% Pure Rust — Cero Python en el Instalador ~8 MB`):** Todo lo que necesita la aplicación de escritorio vive en **Rust nativo (`hera-rs` + `src-tauri`: `forensics.rs`, `structure.rs`, `synergy.rs`, `p2p.rs`, `p2p_exchange.rs`, `export.rs`, `dj_brain.rs`, `sqlx`, `lofty`, `axum`, `ts-rs`)**. Así el instalador `.exe`/`.dmg`/`.deb` pesa apenas **`~8 MB`** (en lugar de `~500 MB` si empaquetara Python/`librosa`). El paquete Python (`src/hera/`) queda únicamente como CLI/SDK opcional en el repositorio para desarrolladores.
     - **Cloud Backend (`hera-cloud` on Cloudflare Edge):** **Cloudflare Workers (Rust -> `wasm32`)** + **Cloudflare Workers AI (`10,000 Neurons/day` free default)** + **AI Gateway** + **Cloudflare D1 (`5 GB` free SQLite sync)** + **Cloudflare Vectorize** + **Federated R2 (`Zero-Egress`)**.

### [2026-11-08] ADR-007: `HERA Desktop` UI QA Incident — Root `.gitignore` `lib/` Trap, Column Alignment & Tauri v2 ACL
- **Context:** During live browser/Playwright QA of `hera-desktop` on Linux after pulling `a05b217`:
  1. `hera-desktop/src/lib/` was completely missing from Git because the root `.gitignore` contained an unanchored `lib/` rule (legacy Python template), which silently ignored `hera-desktop/src/lib/` (`bridge-hera.ts`, `bridge-database.ts`, `bridge-config.ts`, `player.ts`, `store.ts`, etc.) during `git add`.
  2. Visual inspection of `/library` revealed a column order & width mismatch between `TrackListHeader.tsx` (`Title | BPM | Key | 4 Hot Cues | Duration | Artist | Album | Genre`) and `TrackRow.tsx` (`Title | Duration | Artist | Album | Genre | BPM | Key | 4 Hot Cues`), plus zero contrast on the purple `camelotBadge` when a row was selected (`--active-item-bg` purple over `--main-color` purple text).
  3. At the minimum window size (`900x550`), fixed percentage widths (`30%` / `40%`) in `Navigation.tsx` squeezed the track counter into 3 vertical lines alongside `HeraDjToolbar.tsx`.
  4. `SettingsAPI.ts` ignored `ConfigBridge.get('theme')` when `getCurrentWindow().theme()` returned `null`, falling back to blinding `light` mode instead of DJ-booth `dark` mode.
  5. Custom Tauri plugin `hera-dj` (`src-tauri/src/plugins/hera_dj.rs`) was registered in `main.rs` but missing from `src-tauri/build.rs` and `src-tauri/capabilities/main.json`, which causes Tauri v2 ACL to reject `invoke('plugin:hera-dj|...')`.
- **Decision (Mandatory Rules for All Coding Agents):**
  1. **Root `.gitignore` Anchoring:** Keep `/lib/` anchored to the repository root in `.gitignore` so nested frontend directories like `hera-desktop/src/lib/` are never silently ignored by Git.
  2. **`TrackListHeader` <-> `TrackRow` 1:1 Column Invariant:** Any change to columns in `hera-desktop/src/components/TrackListHeader.tsx` MUST be mirrored in exact DOM order and flex widths (`width`, `minWidth`, `flexShrink: 0`) in `hera-desktop/src/components/TrackRow.tsx`, and badges inside rows MUST define a high-contrast `selected` style (`camelotBadgeSelected`).
  3. **Tauri v2 Plugin ACL Triad:** Every custom Tauri command added to `hera-desktop/src-tauri/src/plugins/*.rs` MUST be registered in all 3 places: `src/main.rs` (`.plugin(...)`), `build.rs` (`tauri_build::InlinedPlugin::new().commands(&[...])`), and `capabilities/main.json` (`"<plugin>:allow-<command>"`).
  4. **DJ Booth Dark Mode Default:** Default `hera-desktop` to `dark` theme (`DEFAULT_THEME = themes.dark` and `ConfigBridge.get('theme')` priority in `SettingsAPI.ts`) and verify layout at both `1100x680` and minimum `900x550` with `vp check` before pushing.

### [2026-11-08] ADR-008: Pure-Rust Multi-Turn ReAct Agent Harness (`HeraAgentHarness`), 12-Backend Registry & Dual Local LLM Tool Protocol
- **Context:** `evaluate_cloudflare_free_ai_turn` in `hera-rs/src/dj_brain.rs` was previously a single-turn static string synthesizer without live HTTP/TCP connectivity to local LLMs (`LM Studio` `:1234`, `Ollama` `:11434`, `Jan` `:1337`, `llama.cpp` `:8080`, `vLLM` `:8000`) or multi-turn tool execution against the node's 20 operational MCP tools.
- **Decision:**
  1. **100% Pure-Rust Multi-Turn ReAct Loop (`evaluate_agent_harness_turn`):** Exported `pub fn dispatch_tool_call` in `mcp.rs` and wired `dj_brain.rs` to convert all 20 operational MCP tools (excluding `chat_with_dj_copilot` and `ensure_hera_updated` to prevent recursion/network side effects) into OpenAI `tools: [{"type": "function", ...}]` schemas, executing up to `max_tool_iterations` (`1..=10`, default `6`) against live `P2PMeshState`.
  2. **Zero-Dependency Local HTTP/1.1 + Dual Tool-Call Parser:** Implemented `http_get_json_local` and `http_post_chat_completions` over `std::net::TcpStream` with strict connect/read timeouts for local/LAN `http://` servers (`LM Studio`, `Ollama`) and `curl` fallback for `https://`, plus a dual parser (`parse_assistant_tool_calls`) that handles both standard `message.tool_calls` arrays and `<tool_call>{...}</tool_call>` blocks emitted by quantized GGUF models.
  3. **Self-Healing Tool Error Recovery & UI Traceability:** Malformed JSON arguments or tool execution errors are returned as structured `role: "tool"` error observations with `self_heal_hint` so the LLM self-corrects on the next iteration, while every step is recorded in `CloudflareFreeAiTurnResponse.tool_calls_trace` (`Vec<AgentToolExecutionStep>`) and rendered in `hera-desktop/src/routes/bridge.tsx` alongside the `probe_ai_backend` command (registered across all 3 Tauri v2 ACL files per `ADR-007`).

### [2026-11-08] ADR-009: Hybrid `Bun 1.4.4` + `Vite+` (`Rolldown` / `Oxlint` / `Oxfmt`) + `moon 2.6.5` Polyglot Monorepo Orchestration
- **Context:** Evaluating whether to replace `pnpm` with `Bun` and `moon` (`moonrepo`) across the polyglot (`Rust` + `TypeScript/React 19`) monorepo raised a critical architectural question: what happens to the build-time compiler plugins (`@stylexjs/unplugin`, `babel-plugin-react-compiler`, `@lingui/vite-plugin`, `@tanstack/router-plugin`, `vite-plugin-svgr`) and Tauri v2 native plugins (`plugin-dialog`, `plugin-fs`, `plugin-clipboard-manager`, `plugin-notification`, `plugin-os`, `plugin-process`, `plugin-hera-dj`)?
- **Decision:**
  1. **Hybrid Toolchain (Best of All Worlds — Zero Plugin Loss):**
     - **`Bun 1.4.4` (`bun install`, `bun.lock`, `bun run`):** Replaces `pnpm` (`pnpm-lock.yaml` / `pnpm-workspace.yaml` removed) as the ultra-fast Zig package manager (`~1.07s` install for 297 packages) and script runner in `hera-desktop/package.json` and `hera-desktop/src-tauri/tauri.conf.json` (`beforeDevCommand: "bun run dev"`, `beforeBuildCommand: "bun run build"`).
     - **`Vite+` (`vp` = Rust `Rolldown` + `Oxlint` + `Oxfmt`):** Retained as the compiler/bundler invoked by `bun run` so 100% of the Vite/Rollup and Tauri v2 plugins in `vite.config.ts` (`@stylexjs/unplugin`, `babel-plugin-react-compiler`, `@lingui/vite-plugin`, `@tanstack/router-plugin`, `vite-plugin-svgr`) continue to execute identically (`~4.63s` production bundle).
     - **`moon 2.6.5` (`.moon/workspace.yml`, `hera-rs/moon.yml`, `hera-desktop/moon.yml`):** Orchestrates the dependency graph (`hera-desktop` depends on `hera-rs`) and content-addressed task caching (`moon run :check :test hera-desktop:build` completes 5 tasks in **`128ms`** on cache hit).
  2. **Clean `overrides` Pinning for `Vite+` under `Bun`:** Pin `"vite": "npm:@voidzero-dev/vite-plus-core@0.1.22"` and `"vitest": "npm:@voidzero-dev/vite-plus-test@0.1.22"` in `devDependencies` and `overrides` without `workspaces.catalog` `@latest` so `bun install` hoists a single singleton instance of `@voidzero-dev/vite-plus-test` for `vp test run --project unit`.

### [2026-11-08] ADR-010: Swarm-First Invariant ("Por el Bien del Enjambre") & Robin Hood Open-Source Sustenance
- **Context:** To maximize organic DJ adoption under Open Source (MIT) philosophy while ensuring the global P2P network never suffers from free-riding ("leeching") and the project remains financially sustainable.
- **Decision:**
  1. **Swarm-First Invariant ("Por el bien del enjambre siempre"):** All local DSP forensics, 4 Hot Cues, 2-Opt TSP crate curation, native DJ exporters (`VirtualDJ`, `Rekordbox`, `Traktor`, `M3U8`), local LLMs (`LM Studio`, `Ollama`), and `Cloudflare Workers AI Free` (`10k Neurons/day`) remain 100% free (`$0.00`). In exchange, **every free, beta, and local node (`beta_mvp`, `free`, `local_unlimited`) MUST keep P2P sharing ON (`p2p_sharing_enabled = true`, `can_disable_p2p_sharing = false`)** so every active DJ strengthens the swarm. Only `pro_monthly` / `founder_unlimited` accounts can disable P2P sharing (`tier_allows_private_vault`).
  2. **Robin Hood Open-Source Sustenance (`HeraDjToolbar.tsx` & `settings.about.tsx`):** Added the non-intrusive Robin Hood blind-wolf modal (`🐺🕶️🥫 Limosnita`) triggered on 1-click set export and accessible from the toolbar/About settings with direct links to Ko-fi and GitHub Sponsors.

### [2026-11-08] ADR-011: `v0.5.0` — 5 Ecosystem Pillars & Cloudflare Edge Optimization (`KV` Forensic Dedup + `D1` DNA Sync + `Vectorize` SimHash + `Rolldown` Code-Splitting)
- **Context:** To maximize performance on modest DJ laptops and scale the global P2P swarm at `$0.00` operational cost, we integrated Cloudflare's full Edge stack (`Workers WASM`, `KV`, `D1`, `Vectorize`, `R2 Zero-Egress`, `Workers AI`) with 5 end-to-end improvements across `hera` and `hera-cloud`.
- **Decision:**
  1. **Cloudflare KV Global Forensic Dedup Cache (`src/cloudflare_edge.rs`, `POST /api/v1/edge/forensic-cache/lookup` & `/store`):** Before running heavy local `Symphonia` + `rustfft` spectral forensics (`~420 ms` CPU/track), HERA queries the Cloudflare KV Edge Cache (`< 2 ms`) by `cas_fingerprint`. On a cache hit, the laptop skips FFT decoding and receives the verified 4 Hot Cues + `hera_seal`. On a cache miss, the pioneer node computes FFT locally, stores the result in KV, and earns `+5.0 MB` of Pioneer Forensic Bounty (`EDGE_KV_PIONEER_FORENSIC_BOUNTY_MB`).
  2. **Cloudflare D1 Edge SQLite DNA Sync & Vectorize `u64` SimHash Matcher (`POST /api/v1/edge/d1-dna-sync` & `/api/v1/edge/vectorize-match`):** Synchronizes compact `~250-token` `DJ Style DNA` profiles and pinned Set Spine anchors (`Opening`, `Peak`, `Closing`) to Cloudflare D1 (`5 GB` free at `$0.00`), and ranks swarm tracks using hybrid 64-bit `style_simhash_u64` POPCNT + Camelot compatibility.
  3. **Real Spectral Forensics on Folder Import (`hera-desktop/src-tauri/src/libs/track.rs` & `database.rs`):** Wired `enrich_track_with_hera_dsp` into `get_track_from_file` so `rayon` parallel folder scanning automatically populates `bpm`, `camelot`, `energy`, `hera_seal`, and `hot_cues` (`1:Intro`, `2:Breakdown`, `3:Drop`, `4:Outro`) in `hera.db`.
  4. **Visual `2-Opt TSP` Set Spine Builder & Cloudflare Edge Radar (`HeraDjToolbar.tsx` & `bridge.tsx`):** Added `🧭 Ordenar Set (2-Opt TSP)` (`plugin:hera-dj|build_dj_set_spine`) to reorder `/library` in-place with Opening/Peak/Closing anchors, and added the live **☁️ Radar Enjambre P2P & Cloudflare Edge Optimizer** panel (`plugin:hera-dj|get_cloudflare_edge_radar`) in `/bridge`.
  5. **`Rolldown` Vendor Code-Splitting & Multi-OS Release Automation (`vite.config.ts` & `.github/workflows/release.yml`):** Split the frontend bundle into `vendor-react` (`59.7 kB` gzip), `vendor-tanstack` (`46.7 kB` gzip), `vendor-ui` (`17.3 kB` gzip), and `index` (`101.7 kB` gzip) with zero `> 500 kB` chunk warnings, and added `.github/workflows/release.yml` (`oven-sh/setup-bun@v2` + `tauri-apps/tauri-action@v0`) for automated Windows/macOS/Linux native installer releases.


### [2026-11-08] ADR-012: 4-Stage Cascading Zero-Config P2P Engine, Desktop UDP Seeder Daemon & Mandatory 2-Machine / 2-ISP Field Test Gate
- **Context:** Standard P2P transfers between DJ terminals must work out-of-the-box without opening router ports (`router_port_forwarding_required = false`) and without storing standard P2P audio payloads in Cloudflare R2 (`r2_storage_bytes_used = 0`), while `HERA Desktop` (`src-tauri`) needs a persistent background UDP seeder and live swarm catalog visibility in both the UI and AI Copilot.
- **Decision:**
  1. **Live Swarm Catalog Radar & HERA Hybrid P2P UI (`dj_brain.rs`, `HeraDjToolbar.tsx`, `bridge.tsx`):** Injected `build_broker_swarm_index()` (`summarize_swarm_downloadable_catalog`) into the AI Copilot system prompt and deterministic turn evaluator (marking local tracks as `[SKIP]`), and added the HERA Hybrid P2P search table, 4 filters (`Todo el Enjambre`, `Compatibles en Camelot`, `Con P2P Boost`, `Promo Pool 0 MB`), and active transfer queue in `HERA Desktop` (zero "Modo Ares" branding).
  2. **4-Stage Cascading Zero-Config P2P Engine (`hera-rs/src/p2p.rs`, `hera-desktop/src-tauri/src/plugins/hera_dj.rs`):**
     - **Stage 1 (`LAN Direct Subnet`):** Direct local Wi-Fi/Ethernet transfer (`0 ms` WAN, `0` R2 bytes).
     - **Stage 2 (`Multi-STUN NAT Classifier` + `Simultaneous UDP Hole-Punch`):** `diagnose_multi_stun_nat_on_socket` probes both `stun.cloudflare.com:3478` and `stun.l.google.com:19302` on the same UDP socket to classify `Cone NAT` (`observed_port_delta == 0`) vs `Symmetric CGNAT`.
     - **Stage 3 (`16-Port Prediction Spray` + Persistent Desktop UDP Seeder Daemon):** `spawn_desktop_udp_p2p_daemon` (`shared_desktop_udp_daemon` + `shared_desktop_p2p_state` in `hera_dj.rs`) serves `8 KiB` chunks with 32-byte SHA-256 headers in a background thread, while `request_chunk_from_peer_with_spray` sprays `base_port..base_port+15` with 3x automatic retry and control ACK draining.
     - **Stage 4 (`Peer Supernode / R2 P2P Boost Fallback`):** Automatic fallback to `zk-ctr-sha256-v1` encrypted shards when both peers are behind strict random Symmetric CGNAT.
  3. **Mandatory 2-Machine / 2-ISP Physical Field Test Gate (Pending Real-World Execution):** While `test_live_udp_hole_punch_direct_p2p_zero_r2_and_stun_nat_probe` verifies live Multi-STUN against Cloudflare/Google STUN (`108.213.120.101`) and `262,144` B (`32 x 8 KiB`) UDP hole-punch transfer on a single host, we MUST execute a physical field test with **2 separate computers across 2 different ISPs** (e.g., PC on Home Fiber Wi-Fi + Laptop on 4G/5G Mobile Hotspot) to validate live `hera-cloud` rendezvous, real router NAT hole-punching / 16-port CGNAT spray, and open-Internet UDP packet loss recovery.
