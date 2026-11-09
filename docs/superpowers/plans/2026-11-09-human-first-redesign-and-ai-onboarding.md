# HERA — Human-First Redesign & AI Onboarding Implementation Plan

**Spec Reference:** `docs/superpowers/specs/2026-11-09-human-first-redesign-and-ai-onboarding-design.md`  
**Methodology:** Organic-Driven Development (ODD) + Receipt-Driven Development (RDD)  
**Author:** David Castillo (`vikruzdavid@gmail.com`)

---

## Phases & Tasks

- [x] **Phase 1: Shared Human Translation & View Mode Store (`hera-desktop`)**
  - [x] Add `viewMode: 'simple' | 'pro'` and `toggleViewMode` to `hera-desktop` bridge state/Zustand.
  - [x] Create translation helpers for human-first labels:
    - Compatibility scores to friendly match text ("💚 Mezcla Perfecta (98%)", "Sube el ritmo suave").
    - 4 Hot Cues ("🟢 1: Entrada", "🔵 2: Calma", "🔴 3: Explosión", "🟡 4: Salida").
    - Forensics quality ("✨ Sonido de Estudio").
  - [x] Add unit tests verifying translation helpers and view mode transitions (`4 passed in 69ms`).

- [x] **Phase 2: Redesign Navigation & Studio Header (`hera-desktop`)**
  - [x] Update top bar in `hera-desktop` with mode toggle (`✨ Vista Simple` vs `🎛️ Vista Pro`).
  - [x] Add active Copilot status badge and P2P Community indicator.
  - [x] Update bottom transport bar with human-friendly cue pins and simplified tabs (`🎵 Mis Canciones`, `🧬 Mi Estilo DJ`, `🌉 Conectar & Copiloto`).
  - [x] Verify with linter, typecheck, and unit tests (`vp build: 1300 modules transformed, 0 errors`).

- [x] **Phase 3: Universal AI Connectivity Onboarding Drawer (`hera-desktop`)**
  - [x] Implement slide-out Drawer in `bridge.tsx` for AI Onboarding.
  - [x] Implement Commander Authority selector ("👑 Tú eres El Jefe", "⚡ Piloto Automático", "👀 Solo Consejos").
  - [x] Render 3 simple option cards (Detected PC IAs, Free HERA Cloud, Offline Club Mode) + Collapsible Pro Section (MCP / BYOK).
  - [x] Wire to Tauri IPC `scan_ai_connectivity` and backend report.
  - [x] Verify with tests and browser check.

- [x] **Phase 4: Conectar Canciones & Copiloto Console (`bridge.tsx`)**
  - [x] Implement Split-Screen Studio layout (56% Decks + Steps + Community Table | 44% Copilot Dock).
  - [x] Render human-first Deck cards ("Sonando Ahora", "Quiero Llegar a").
  - [x] Render 15s preview button ("🎧 Oír cómo mezcla (15s)").
  - [x] Render structured Copilot receipts with 1-click approval ("✅ Me gusta, agregar a mi Lista", "🎛️ Mandar a VirtualDJ").
  - [x] Verify with tests and build.

- [x] **Phase 5: Mi Estilo DJ View (`dna.tsx`)**
  - [x] Replace raw 24-cell Camelot matrix with Human Vibe cards ("🌌 Melódico", "🔥 Hora Pico", "☀️ Cantado") in Simple View, while keeping the full matrix in Pro View.
  - [x] Render 4 Golden Mixing Rules with toggles.
  - [x] Render conversational AI understanding summary card.
  - [x] Verify with tests and build.

- [x] **Phase 6: End-to-End Verification & Multi-Instance Git Commit**
  - [x] Run full test suite: `cargo test --manifest-path hera-rs/Cargo.toml` (17 passed, 0 failed).
  - [x] Run `cargo check --manifest-path hera-desktop/src-tauri/Cargo.toml` (0 errors).
  - [x] Run `bun run test` (13 passed, 0 failed) and `bun run build` (success in 2.18s).
  - [x] Take real browser screenshot receipts of the running app (`live_bridge_simple_screen.png`, `live_onboarding_drawer_screen.png`, `live_dna_simple_screen.png`).
  - [x] Commit atomically according to `superpowers:multi-instance-handoff`.
