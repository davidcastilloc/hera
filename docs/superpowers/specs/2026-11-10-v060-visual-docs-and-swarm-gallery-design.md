# HERA v0.6.0 — Visual Identity, Full Product Screen Gallery, P2P Swarm Concept & Documentation Alignment

**Date:** 2026-11-10  
**Target Version:** `v0.6.0`  
**Repository:** `davidcastilloc/hera`

---

## 1. Intent & Motivation

Following the completion of HERA `v0.6.0` — including the Unified Dark Booth Studio (`#0c0e12` slate + `#14b8a6` studio emerald), Minimalist Studio Distillation, Autonomous `/agent` tab (`Ctrl+J`, 44 MCP tools), VirtualDJ `Modo Mini` (`Ctrl+Shift+M`, `360×408` Booth Card & `560×52` HUD Pill), Lock-Free `DualDeckEngine`, Hexagonal `hera-ipc` crate, 5-Stage Zero-Config P2P Cascade, and 5-Layer Federated R2 `Edge Guardian` Swarm — the repository's visual assets and secondary documentation files still reflect earlier iterations (`v0.1`–`v0.3`):

1. **Outdated Hero Banner (`assets/hera_banner_y2k.jpg`):** Uses a neon-pink Y2K rave flyer aesthetic (`Y2K TECHNO / TRANCE / FUTURE BEATS`) instead of HERA's actual Dark Booth Studio & Global Symbiotic P2P Swarm identity.
2. **Incomplete & Pre-Redesign Product Screenshots:**
   - Root `README.md` lacks embedded screenshots of the product screens and a visual diagram of the P2P Swarm concept.
   - `hera-desktop/README.md` still displays pre-distillation `v0.3.0` captures with the old bottom bar and omits `/agent` (`Agente`) and `Modo Mini` (`Card 360×408` + `HUD Pill 560×52`).
3. **Legacy Python / Streamlit References in `docs/` & Sub-Crate Docs:**
   - `docs/MODOS_DE_USO.md`, `docs/VPS_REMOTE_GUIDE.md`, `docs/Hera_Guia_para_Agentes_AI.md`, `docs/README.md`, and `MANIFESTO.md` still reference Python `3.11+`, `.venv`, `hera ui` (`http://localhost:8501`), `hera chat`, `librosa`, `FFmpeg`, and `slskd`.
   - `hera-rs/README.md` and `hera-desktop/AGENTS.md` still reference pre-`v0.6.0` monolithic paths (`src/mcp.rs`, `src/p2p.rs`) or upstream Museeks boilerplate.

---

## 2. Goals, Non-Goals & Anti-Goals

### Goals
- Replace `assets/hera_banner_y2k.jpg` (and `assets/hera_banner.jpg`) with a new flagship `v0.6.0` **Dark Booth Studio & Symbiotic P2P Swarm** banner, plus a dedicated visual concept diagram `assets/hera_swarm_concept.jpg` illustrating the 5-Stage P2P Cascade and Federated R2 `Edge Guardian` Swarm (*"Por el bien del enjambre"*).
- Capture real, pixel-accurate Chromium renders of all 5 `v0.6.0` product views from the compiled `hera-desktop` bundle (`docs/superpowers/assets/`):
  1. `hera_v060_1_library_1180x720.png` (and update `hera_live_1_library_1100x680.png`) — **`Biblioteca` (`/`)**: Left navigation rail, top player with 4 Hot Cue pins, `HeraDjToolbar` (`Ordenar Set`, `Exportar a Cabina`, `Modo Mini`), and track table (`BPM`, `Key`, `Puntos`).
  2. `hera_v060_2_agent_1180x720.png` — **`Agente` (`/agent`, `Ctrl+J`)**: Autonomous 44-tool MCP workspace with `@ Contexto` chips, collapsible `1px` tool rows, `/set` workflow bar, and `Auto / Confirmar` policy toggle.
  3. `hera_v060_3_transition_1180x720.png` (and update `hera_live_3_bridge_1100x680.png`) — **`Transición` (`/bridge`)**: Horizontal `Actual → Puente sugerido → Destino` progression, P2P bridge candidates, 15s transition preview, bottom quick-adjust bar, and anchored `Asistente` popover.
  4. `hera_v060_4_dna_1180x720.png` (and update `hera_live_2_dna_1100x680.png`) — **`Mi Estilo` (`/dna`)**: Segmented software switch (`VirtualDJ | Rekordbox | Traktor | Serato`), borderless typographic stats (`Ritmo habitual`, `Tonos principales`, `Mezcla`), `Balance musical` bar, 3 preference switches, and 24-key Camelot grid.
  5. `hera_v060_5_mini_overlay_1180x720.png` (and update `hera_live_4_library_compact_900x550.png`) — **`Modo Mini` (`Ctrl+Shift+M`)**: Side-by-side showcase of the `360×408` Booth Card (`Siguientes | Puente` + `+ VDJ` live `.vdjfolder` push) and `560×52` Collapsed HUD Pill.
- Embed the product screen gallery and P2P Swarm visual explanation across `README.md`, `hera-desktop/README.md`, `docs/MODOS_DE_USO.md`, and `docs/FEDERATED_R2_SWARM.md`.
- Align `MANIFESTO.md`, `hera-rs/README.md`, `hera-desktop/AGENTS.md`, `docs/README.md`, `docs/MODOS_DE_USO.md`, `docs/VPS_REMOTE_GUIDE.md`, and `docs/Hera_Guia_para_Agentes_AI.md` with the `v0.6.0` 100% Pure-Rust + Tauri v2 + P2P Swarm architecture.
- Add an automated test in `hera-desktop/src-ipc/src/workflow_guard.rs` verifying that all markdown image links in top-level and `docs/` markdown files resolve to existing non-empty files and that legacy `localhost:8501` / `.venv/bin/hera` strings do not reappear in operational guides.

### Non-Goals
- Modifying Rust DSP algorithms, P2P protocol wire formats, or React component runtime behavior.

### Anti-Goals
- Never use AI text-diffusion images as fake UI screenshots (all UI screenshots must come from real Chromium rendering of the `hera-desktop` CSS/DOM).
- Never commit OS-specific absolute paths in any documentation, spec, plan, or test.

---

## 3. Approaches Considered

1. **Hybrid Real Chromium Screen Captures + AI Hero Banner & Swarm Visual (Selected):**
   - Uses `generate_image` for the artistic hero banner (`assets/hera_banner_y2k.jpg`) and Swarm concept visual (`assets/hera_swarm_concept.jpg`), while rendering the actual `hera-desktop` `v0.6.0` UI in headless Chromium (`1180×720` @ `2x` DPR) for crystal-clear Spanish typography across all 5 screens.
2. **AI-Generated Mockups for Product Screens (Rejected):**
   - Produces garbled text labels and diverges from the actual StyleX `#0c0e12` + `#14b8a6` Dark Booth Studio components.
