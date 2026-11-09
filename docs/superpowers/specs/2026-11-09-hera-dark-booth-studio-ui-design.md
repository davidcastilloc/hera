# HERA Desktop — Unified Dark Booth Studio UI Redesign

**Date:** 2026-11-09  
**Target:** `hera-desktop/` (`src/`, `src-tauri/`)

## 1. Problem Statement
- `hera-desktop` previously mixed upstream light-mode Museeks surfaces (`#ffffff` / `#f4f4f4` in English) with hardcoded pitch-black HERA components (`#070a0f` / `#090c12` in Spanish), creating a split half-white / half-black window and a two-row broken bottom bar (`Navigation.tsx` + `HeraDjToolbar.tsx`).
- Decorative emojis (`✨`, `🎛️`, `🧬`, `🐺🪙🥫`, `🟢🔵🔴🟡`) and violet badges (`rgba(139, 92, 246, 0.16)`) clashed with professional DJ booth ergonomics.

## 2. Design Decisions
1. **Dedicated Dark Booth System (`Pizarra de Cabina & Esmeralda Estudio`):**
   - Native Tauri window forced to `Theme::Dark` with `#0c0e12` background (`hera-desktop/src-tauri/src/main.rs`) and default config `theme = "dark"`, `language = "es"` (`hera-desktop/src-tauri/src/plugins/config.rs`).
   - Unified CSS custom properties across `hera-desktop/src/themes/dark.json`, `hera-desktop/src/themes/light.json`, and `hera-desktop/src/global.css`:
     - Ground: `#0c0e12`, Chrome/Rail: `#12161d` / `#10141b`, Elevated Panel: `#171c24`, Hairline: `#232a36`.
     - Ink: `#edeff3` primary, `#8e98a8` secondary slate (`>= 4.5:1` contrast).
     - Single action accent: `#14b8a6` (`#041311` ink on solid primary buttons; `#2dd4bf` for active markers). Zero violet.
     - Browser surfaces themed: scrollbars, text selection, focus rings, and `tabular-nums` for BPM, durations, and cue timestamps.
2. **Unified Left Navigation Rail + Contextual Crate Header:**
   - Replace the bottom split footer with a `216px` left rail (`hera-desktop/src/components/Navigation.tsx`) containing the HERA mark, 6 SVG-icon routes (`Biblioteca`, `Artistas`, `Playlists`, `Puente y Copiloto`, `Mi ADN`, `Ajustes`), `Simple / Pro` view switch, track/swarm status readout, and discreet `Apoyar proyecto` link to `/settings/about`.
   - Move `HeraDjToolbar` out of global navigation into a `44px` contextual crate header (`hera-desktop/src/components/HeraDjToolbar.tsx`) above track lists showing cumulative set duration (`tabular-nums`), `Ordenar Set`, export format selector, and primary `Exportar a Cabina`.
3. **SVG Iconography & Cohesive Spanish Booth Copy:**
   - Author uniform 1.5px-stroke SVG icons in `hera-desktop/src/assets/icons/` and `hera-desktop/src/components/Icon.tsx`, eliminating emoji icons across `TrackRow.tsx`, `HeraDjToolbar.tsx`, `bridge.tsx`, `dna.tsx`, and `settings.about.tsx`.
   - Upgrade empty library state (`hera-desktop/src/components/TrackListStates.tsx`) with direct folder import (`@tauri-apps/plugin-dialog` + `LibraryAPI.addLibraryFolders`).
