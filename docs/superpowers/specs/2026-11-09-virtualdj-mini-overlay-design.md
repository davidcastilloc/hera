# VirtualDJ Mini Overlay Mode (`Modo Mini`) — Design Specification

**Date:** `2026-11-09`  
**Scope:** `hera-desktop/` (`src-tauri/src/plugins/hera_dj.rs`, `src-tauri/build.rs`, `src-tauri/capabilities/main.json`, `src/lib/bridge-hera.ts`, `src/components/Header.tsx`, `src/components/MiniOverlay.tsx`, `src/routes/__root.tsx`)  
**Status:** Approved for implementation

---

## 1. Problem & Booth Context

When mixing live in **VirtualDJ** (or Rekordbox, Serato, Traktor) on a single laptop screen, switching to a full `1180×720` desktop window obscures the active waveforms, decks, and mixer. DJs need a compact, frameless **Always-on-Top** companion anchored in the bottom-right corner that keeps deck visibility intact while surfacing harmonic next-track suggestions, quick `A → B` bridges, headphone preview, and 1-click live delivery to VirtualDJ.

---

## 2. Single-Window Morphing Architecture (`Opción A`)

Instead of spawning a second WebView2 process, `hera-desktop` morphs its existing `main` window between three states with zero extra memory overhead and instant shared playback/library state:

1. **Full Studio (`full`, default `1180×720`, `min 960×580`):**
   - Standard window decorations, `alwaysOnTop = false`, full `Header` + `Navigation` + route `<Outlet />`.
   - Includes a compact `Modo Mini` button in `Header.tsx` (and global shortcut `Ctrl+Shift+M` / `Cmd+Shift+M`).
2. **Expanded Booth Card (`card`, `360×408` logical px):**
   - Frameless (`decorations = false`), `alwaysOnTop = true` (toggleable via Pin icon), anchored `24 px` from the right edge and `56 px` above the bottom monitor edge.
   - Displays:
     - Draggable `38 px` top bar (`HERA` + emerald dot, `[Siguientes | Puente]` segmented switch, Pin toggle, Collapse to Pill `▾`, Restore Full `⤢`).
     - **`Siguientes` tab:** `ACTUAL` strip (auto-synced from VirtualDJ `History/tracklist.txt` when present, or selectable from local library / playing track) + 3 harmonic next suggestions (`01..03`, `BPM · Tono`, 4 colored cue dots, Play/Stop preview button, `+ VDJ` -> inline `✓ En VDJ`) + bottom quick bar (`Ajustar...`, `Suave`, `+ Energía`, `Sin voz`).
     - **`Puente` tab:** Compact `Actual → Destino` selectors + 2 harmonic bridge step rows (4 colored cue dots + Play/Stop preview) + primary emerald button `Mandar puente a VirtualDJ` (`✓ Puente en VDJ` inline confirmation) + swap button.
3. **Collapsed HUD Pill (`pill`, `560×52` logical px):**
   - Frameless single-line capsule anchored to the same bottom-right corner (`Sonando <Track> → <Next> <Key> <BPM> •••• [▶] [+ VDJ] [▴] [⤢]`).

---

## 3. Live VirtualDJ File Integration (`plugin:hera-dj`)

1. **`set_mini_overlay_mode`:**
   - Captures pre-mini window size/position snapshot before entering `card` or `pill`, applies `set_decorations`, `set_always_on_top`, `set_min_size`, `set_size`, and bottom-right monitor positioning atomically in Rust, and restores the exact snapshot when returning to `full`.
2. **`push_virtualdj_live_crate`:**
   - Executes inside `spawn_blocking`. Materializes P2P/bridge audio into `library/` if needed and writes `HERA En Vivo.vdjfolder` + `_vdj_database.xml` (including all 4 `<Poi>` Hot Cues, BPM, and Camelot key) into `Documents/VirtualDJ/Folders/` (with automatic fallback copy in `exports/VirtualDJ/`).
3. **`read_virtualdj_now_playing`:**
   - Executes inside `spawn_blocking`. Parses the latest non-empty track entry from `Documents/VirtualDJ/History/tracklist.txt` (or newest `.m3u` history file) with graceful `None` fallback when VirtualDJ is not active.

---

## 4. Design & Performance Invariants

- Dark studio palette (`#0c0e12` ground, `#10141b` bars, `#171c24` controls, `1px` `#26303d` hairlines, `#14b8a6` emerald accent); zero light surfaces, zero purple/violet hues, zero thick side borders, zero floating routine toasts.
- Lazy-load `MiniOverlay.tsx` via `React.lazy` + `<Suspense>` so initial startup bundle size remains minimal.
- Zero main-thread UI stalls (`< 50 ms` frame latency across all transitions and VirtualDJ writes).
