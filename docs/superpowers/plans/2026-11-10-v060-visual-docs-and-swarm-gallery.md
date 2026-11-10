# HERA v0.6.0 Visual Identity, Product Screen Gallery & Documentation Alignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the legacy Y2K banner with a new `v0.6.0` Dark Booth Studio & Symbiotic P2P Swarm visual identity, capture real Chromium screenshots of all 5 `v0.6.0` product views (`Biblioteca`, `Agente`, `Transición`, `Mi Estilo`, `Modo Mini`), illustrate the 5-Stage P2P + Federated R2 `Edge Guardian` Swarm concept, and align all documentation in `README.md`, `MANIFESTO.md`, `hera-desktop/`, `hera-rs/`, and `docs/` with `v0.6.0`.

**Architecture:** Use `generate_image` to produce the new flagship `16:9` hero banner (`assets/hera_banner_y2k.jpg` and `assets/hera_banner.jpg`) and the P2P Swarm visual concept diagram (`assets/hera_swarm_concept.jpg`). Render the real `hera-desktop` `v0.6.0` Dark Booth Studio UI (`#0c0e12` + `#14b8a6`) in headless Chromium at `1180×720` (`2x` DPR) across all 5 views (`docs/superpowers/assets/hera_v060_1..5_*.png` and `hera_live_1..4_*.png`), update all markdown guides, and enforce image-link and legacy-string hygiene in `hera-desktop/src-ipc/src/workflow_guard.rs`.

**Tech Stack:** Rust (`cargo`, `hera-ipc`), Headless Chromium (`/usr/bin/chromium`), Bun `1.4.4` + Vite+ (`vp`), Markdown / Mermaid.

**Spec:** `docs/superpowers/specs/2026-11-10-v060-visual-docs-and-swarm-gallery-design.md`

## Global Constraints

- Zero OS-specific absolute paths in any tracked file, spec, plan, or test.
- Zero Python runtime or legacy Streamlit (`localhost:8501`, `.venv/bin/hera`, `hera ui`, `hera chat`) references in active `v0.6.0` operational instructions.
- Every markdown image reference (`![...](...)` and `<img src="..." />`) across `README.md`, `MANIFESTO.md`, `hera-desktop/README.md`, `hera-rs/README.md`, and `docs/*.md` must resolve to a non-empty file on disk.
- All product UI screenshots must be captured from real Chromium rendering of the `v0.6.0` Dark Booth Studio DOM/CSS (`#0c0e12` ground, `#14b8a6` studio emerald, crisp Spanish typography).

## Review Focus

1. Broken relative image paths between root `README.md` (`docs/superpowers/assets/...`) and sub-directory docs (`../docs/superpowers/assets/...` or `superpowers/assets/...`).
2. Legacy `v0.1` Python / Streamlit commands (`hera ui`, `localhost:8501`, `.venv/bin/hera`) remaining in `docs/MODOS_DE_USO.md` or `docs/VPS_REMOTE_GUIDE.md`.
3. Missing coverage of any of the 5 product views (`Biblioteca`, `Agente`, `Transición`, `Mi Estilo`, `Modo Mini`) or the P2P Swarm visual explanation.
4. Pre-modular file paths (`hera-rs/src/mcp.rs`, `hera-rs/src/p2p.rs`) referenced as active files in `hera-rs/README.md` or `docs/FEDERATED_R2_SWARM.md`.
5. Upstream Museeks boilerplate remaining in `hera-desktop/AGENTS.md` without `hera-ipc`, `hera_agent`, and `hera_dj` plugin rules.

---

### Task 1: Generate New `v0.6.0` Hero Banner & Symbiotic P2P Swarm Concept Visual (`assets/`)

**Files:**
- Modify: `assets/hera_banner_y2k.jpg`
- Modify: `assets/hera_banner.jpg`
- Create: `assets/hera_swarm_concept.jpg`

**Interfaces:**
- Produces: `assets/hera_banner_y2k.jpg`, `assets/hera_banner.jpg`, and `assets/hera_swarm_concept.jpg` consumed by `README.md`, `docs/FEDERATED_R2_SWARM.md`, and `docs/MODOS_DE_USO.md`.

- [x] **Step 1: Generate the new `16:9` Dark Booth Studio & P2P Swarm Hero Banner**
  Use `generate_image` (`AspectRatio: "16:9"`) with the `#0c0e12` obsidian slate + `#14b8a6` studio emerald + warm gold aesthetic depicting the HERA `v0.6.0` Dark Booth Studio, 4 colored acoustic Hot Cues, Camelot harmonic rings, and interconnected global P2P Edge Guardian nodes, and write it to `assets/hera_banner_y2k.jpg` and `assets/hera_banner.jpg`.

- [x] **Step 2: Generate the `16:9` Symbiotic P2P Swarm Concept Visual (`assets/hera_swarm_concept.jpg`)**
  Use `generate_image` (`AspectRatio: "16:9"`) illustrating the 5-Stage Zero-Config P2P Cascade (`LAN -> STUN -> Spray -> TURN -> R2`) and the Federated R2 `Edge Guardian` Swarm (`Water-Filling 9 GB Cap`, `4 MB Blind Shards`, `1:1.5 Turbo Credits`), and save to `assets/hera_swarm_concept.jpg`.

---

### Task 2: Capture Real `v0.6.0` Chromium Screenshots of All 5 Product Screens (`docs/superpowers/assets/`)

**Files:**
- Create: `docs/superpowers/assets/hera_v060_1_library_1180x720.png`
- Create: `docs/superpowers/assets/hera_v060_2_agent_1180x720.png`
- Create: `docs/superpowers/assets/hera_v060_3_transition_1180x720.png`
- Create: `docs/superpowers/assets/hera_v060_4_dna_1180x720.png`
- Create: `docs/superpowers/assets/hera_v060_5_mini_overlay_1180x720.png`
- Modify: `docs/superpowers/assets/hera_live_1_library_1100x680.png`
- Modify: `docs/superpowers/assets/hera_live_2_dna_1100x680.png`
- Modify: `docs/superpowers/assets/hera_live_3_bridge_1100x680.png`
- Modify: `docs/superpowers/assets/hera_live_4_library_compact_900x550.png`

**Interfaces:**
- Produces: 5 high-resolution (`1180×720`) `v0.6.0` product screenshots and updated `hera_live_1..4_*.png` assets consumed by `README.md`, `hera-desktop/README.md`, and `docs/MODOS_DE_USO.md`.

- [x] **Step 1: Render and capture all 5 `v0.6.0` Dark Booth Studio views via headless Chromium**
  Render the exact `v0.6.0` Dark Booth Studio screens (`Biblioteca`, `Agente`, `Transición`, `Mi Estilo`, and `Modo Mini` `Card 360×408` + `HUD Pill 560×52`) with `/usr/bin/chromium --headless --window-size=1180,720` and inspect every PNG via `view_file` to verify crisp typography and layout fidelity.

---

### Task 3: Update Root `README.md`, `MANIFESTO.md`, `hera-desktop/`, `hera-rs/` & `docs/` Guides + CI Guard (`hera-ipc`)

**Files:**
- Modify: `hera-desktop/src-ipc/src/workflow_guard.rs`
- Modify: `README.md`
- Modify: `MANIFESTO.md`
- Modify: `hera-desktop/README.md`
- Modify: `hera-desktop/AGENTS.md`
- Modify: `hera-rs/README.md`
- Modify: `docs/README.md`
- Modify: `docs/MODOS_DE_USO.md`
- Modify: `docs/VPS_REMOTE_GUIDE.md`
- Modify: `docs/Hera_Guia_para_Agentes_AI.md`
- Modify: `docs/FEDERATED_R2_SWARM.md`

**Interfaces:**
- Consumes: `assets/hera_banner_y2k.jpg`, `assets/hera_swarm_concept.jpg`, `docs/superpowers/assets/hera_v060_1..5_*.png`
- Produces: `verify_documentation_assets_and_v060_alignment` test in `hera-desktop/src-ipc/src/workflow_guard.rs`.

- [x] **Step 1: Write the failing documentation & image asset guard test in `hera-desktop/src-ipc/src/workflow_guard.rs`**
  Add `verify_documentation_assets_and_v060_alignment` asserting that:
  - `docs/superpowers/assets/hera_v060_1_library_1180x720.png` through `hera_v060_5_mini_overlay_1180x720.png` and `assets/hera_swarm_concept.jpg` exist and are non-empty.
  - `README.md` and `hera-desktop/README.md` embed all 5 `v0.6.0` product screens and `assets/hera_swarm_concept.jpg`.
  - `docs/MODOS_DE_USO.md`, `docs/VPS_REMOTE_GUIDE.md`, `docs/README.md`, and `docs/Hera_Guia_para_Agentes_AI.md` contain zero legacy `localhost:8501` or `.venv/bin/hera` strings.

- [x] **Step 2: Update `README.md`, `MANIFESTO.md`, `hera-desktop/README.md`, `hera-desktop/AGENTS.md`, `hera-rs/README.md`, and `docs/*.md`**
  Embed the new banner, the complete 5-screen visual tour (`Biblioteca`, `Agente`, `Transición`, `Mi Estilo`, `Modo Mini`), and the visual + architectural walkthrough of the Symbiotic P2P Swarm (*"Por el bien del enjambre"*), and align all `docs/` guides with `v0.6.0`.

- [x] **Step 3: Run full workspace verification and commit**
  Run `cargo check`, `cargo test -p hera-node-rs`, `cargo test -p hera-ipc`, and `cd hera-desktop && bun run check && bun run test && bun run build`, mark all tasks `- [x]`, update `docs/superpowers/DECISIONS.md` (`ADR-026`), and commit/push atomically.
