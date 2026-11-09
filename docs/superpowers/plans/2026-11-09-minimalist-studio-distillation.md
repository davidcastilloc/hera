# Minimalist Studio Distillation Implementation Plan

**Goal:** Distill `hera-desktop` views (`Navigation`, `Biblioteca` toolbar/rows, `Transición`, `Mi Estilo`, and `Toasts`) into a low-text, zero-jargon, single-hierarchy dark studio UI while preserving 100% of Tauri v2 IPC workflows.

**Architecture:** Retain all Tauri v2 commands and Zustand/React Query stores (`bridge-hera.ts`, `bridge-database.ts`, `LibraryAPI.ts`, `PlaylistsAPI.ts`) while refactoring presentation components in `hera-desktop/src/` to eliminate nested cards, multi-sentence explanations, technical enum/engine labels, and routine floating toast popups.

**Tech Stack:** React 19, TypeScript, StyleX, TanStack Router, Zustand, Vitest (`vp check`, `vp test`, `vp build`), Rust / Tauri v2 (`cargo test`, `cargo build`).

**Spec:** `docs/superpowers/specs/2026-11-09-minimalist-studio-distillation-design.md`

## Global Constraints

- Keep dark studio palette (`#0c0e12` ground, `#10141b` rail, `#12161d` bars, `#171c24` controls, `#232a36` `1px` hairlines, `#edeff3` / `#8e98a8` ink, `#14b8a6` accent); zero violet/purple hues and zero light/white surfaces.
- Zero colored `border-left` / `border-right` thicker than `1px`, zero zero-offset glow halos, zero nested cards, zero kickers/eyebrows above headings, and zero emoji icons.
- Under 50 words of static UI copy per screen on `Transición` and `Mi Estilo`, free of internal technical terms (`2-Opt TSP`, `ReAct`, `executive_boss`, `10k Neurons`, `FFT`, `Quarantine-First`).
- Preserve all existing `bridge-hera.ts` and `hera-human-language.ts` exports covered by unit tests.

## Review Focus

- Empty library state (`0 pistas`): `Mi Estilo` and `Transición` must still render valid default BPM/key values (`124 BPM`, `8A · 9A · 10A`) without `NaN` or empty labels.
- Long track/playlist titles: `Actual`/`Destino` selectors in `Transición` and playlist headers in `HeraDjToolbar` must truncate cleanly with `textOverflow: 'ellipsis'` without horizontal overflow (`scrollWidth <= clientWidth`).
- Popover dismissal & focus: clicking outside or pressing `Escape` closes the `Asistente` popover in `Transición`, and opening `Conexión avanzada…` reveals the custom local URL input inline without overflowing the viewport.
- Routine vs. error notifications: routine actions (`Exportar`, `Ordenar mezcla`, `Guardar playlist`, `+ Añadir`) show inline button feedback and never spawn floating toast overlays, while real `warning` / `danger` audio/import errors still render in `Toasts.tsx`.
- Async playlist creation from `Transición`: clicking `Guardar playlist` downloads/imports any missing bridge tracks into SQLite and navigates cleanly to `/playlists/$playlistID`.

---

### Task 1: Quiet Navigation Rail, Silent Inline Crate Toolbar, Minimal Cue Dots & Error-Only Toasts

**Files:**
- Modify: `hera-desktop/src/components/Navigation.tsx`
- Modify: `hera-desktop/src/components/HeraDjToolbar.tsx`
- Modify: `hera-desktop/src/components/TrackListHeader.tsx`
- Modify: `hera-desktop/src/components/TrackRow.tsx`
- Modify: `hera-desktop/src/components/Toasts.tsx`

- [x] **Step 1: Simplify `Navigation.tsx`, `Toasts.tsx`, `HeraDjToolbar.tsx`, `TrackListHeader.tsx`, and `TrackRow.tsx`**
  - Update `Navigation.tsx` labels to `Biblioteca`, `Artistas`, `Playlists`, `Transición` (`/bridge`), `Mi Estilo` (`/dna`), `Ajustes`; remove the `Simple / Pro` segmented switch, the `Motor IA · P2P` button, and the `0 0 0 3px` status-dot halo; keep `<Status />` and `Apoyar proyecto` in the footer.
  - Update `Toasts.tsx` to filter `toasts` to `warning` and `danger` types only, position at `bottom: '16px', right: '16px'`, `limit={1}`, and use `1px` crisp borders instead of `borderLeftWidth: '5px'`.
  - Update `HeraDjToolbar.tsx` to clean labels (`Importar`, `Ordenar mezcla`, compact format selector, and primary button `Exportar` transitioning inline to `✓ Exportado a <Formato>` for `2.2s` without `toastManager.add`).
  - Update `TrackListHeader.tsx` (`4 Cues` → `Puntos`) and `TrackRow.tsx` (`cueRail` simplified to 4 crisp `6px` semantic dots without border box or `1 2 3 4` text clutter).
- [x] **Step 2: Run typecheck and unit tests**
  - Run: `cd hera-desktop && bun run check && bun run test`

### Task 2: Distill `Mi Estilo` (`/dna`) Screen

**Files:**
- Modify: `hera-desktop/src/routes/dna.tsx`

- [x] **Step 1: Replace multi-card essay layout in `dna.tsx` with centered `Mi Estilo` studio layout**
  - Header: `Mi Estilo` on the left, segmented software selector (`VirtualDJ`, `Rekordbox`, `Traktor`, `Serato`) on the right (no toast spam on click).
  - Row 1: 3 borderless typographic stats separated by `1px` vertical hairlines (`Ritmo habitual` -> `125 BPM`, `Tonos principales` -> `8A · 9A · 10A`, `Mezcla` -> `Progresiva`).
  - Row 2 (`Balance musical`): single `10px` horizontal segmented bar (`62%` `#14b8a6`, `24%` `#38bdf8`, `14%` `#f59e0b`) with inline legend `Melódico 62% · Hora pico 24% · Vocal 14%` and optional disclosure button `Tonos detallados` toggling the compact `1A–12A` grid.
  - Row 3 (`Preferencias al ordenar`): single clean list with `1px` dividers and accessible pill switches (`role="switch"`, `aria-checked`) for `Evitar choque de voces`, `Mantener graves continuos`, and `Ocultar audio de baja calidad`.
- [x] **Step 2: Verify `vp check` passes**

### Task 3: Distill `Transición` (`/bridge`) & Compact `Asistente` Popover

**Files:**
- Modify: `hera-desktop/src/routes/bridge.tsx`

- [x] **Step 1: Rebuild `bridge.tsx` around the single horizontal flow + Spotlight bar + anchored `Asistente` popover**
  - Top bar: `Transición` heading on the left; `Exportar` (inline `✓ Exportado` confirmation) and primary `Guardar playlist` (`handleApproveAndCreatePlaylist`) on the right.
  - Center horizontal progression: `Actual` selector/card (`Deck A` library picker + compact BPM/key controls) `→` `Puente sugerido` clean rows (`01`, `02` with `Artist — Title`, `BPM · Tono`, 4 cue dots, and `Escuchar` / `Detener` button) `→` `Destino` selector/card (`Deck B` library picker + compact BPM/key controls), with swap button between decks.
  - Progressive disclosure section `Ver más sugerencias (N)` rendering clean single-line P2P rows with `+ Añadir` (`✓ Añadida` inline state).
  - Bottom command bar: `Ajustar transición...` input + quick chips (`Más suave`, `Más energía`, `Sin voces`) + compact quiet status line when updated + sliders icon button toggling the anchored `290px` `Asistente` popover (`Conexión: Nube | Sin internet`, `Modo: Sugerir | Automático`, expandable `Conexión avanzada…` URL input, `Escape` / click-outside close).
- [x] **Step 2: Run formatting, lint, unit tests, design detector, and production build**
  - Run: `cd hera-desktop && bunx vp check --fix && bun run check && bun run test && bun run build`

### Task 4: End-to-End Live WebView2 QA via Tauri MCP & Atomic Commit

**Files:**
- Modify: `docs/superpowers/plans/2026-11-09-minimalist-studio-distillation.md`
- Modify: `docs/superpowers/DECISIONS.md`

- [x] **Step 1: Rebuild `hera-desktop` (`debug` + `release`), run Rust tests, and execute live Tauri MCP QA across all views**
- [x] **Step 2: Visually inspect fresh WebView2 screenshots (`Transición`, `Asistente` popover, `Mi Estilo`, `Biblioteca`, `Playlists`, `Ajustes`), update plan checkboxes & `DECISIONS.md` (`ADR-020`), and commit atomically**
