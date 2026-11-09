# HERA Desktop — Minimalist Studio Distillation Design

**Date:** 2026-11-09  
**Target Package:** `hera-desktop` (`hera-desktop/src/`)  
**Status:** Approved

---

## 1. Intent & Problem Statement

Live WebView2 captures of `hera-desktop` showed excessive textual density (~260–310 words per screen), nested cards, technical internal terminology (`2-Opt TSP`, `Consola ReAct`, `executive_boss`, `10k Neurons/día`, `FFT > 20 kHz`, `Quarantine-First`, `Invariante colaborativo`), duplicated controls in the left navigation rail (`Simple / Pro` toggle + `Motor IA · P2P` button), and stacked top-right toast popups obscuring contextual headers.

The goal is to distill every primary screen (`Transición`, `Mi Estilo`, `Biblioteca`, `Asistente`, and `Navigation`) down to a calm, low-cognitive-load studio tool with under 50 words of UI copy per screen, flat structural hierarchy (zero nested cards), and silent inline feedback on interactive controls.

---

## 2. Goals, Non-Goals & Anti-Goals

- **Goals:**
  - Unify `Navigation` into a quiet left rail (`Biblioteca`, `Artistas`, `Playlists`, `Transición`, `Mi Estilo`, `Ajustes`) with only the track/duration readout and discreet project link in the footer.
  - Replace the two-column `bridge.tsx` layout (`Deck A/B` + `P2P table` + `ReAct Console` essay) with a single spacious horizontal progression (`Actual → Puente sugerido → Destino`), a collapsible `Ver más sugerencias` list, a single-line bottom command bar (`Ajustar transición...` + `Más suave`, `Más energía`, `Sin voces`), and an anchored `290px` `Asistente` popover (`Conexión: Nube | Sin internet`, `Modo: Sugerir | Automático`, `Conexión avanzada…`).
  - Replace the multi-card `dna.tsx` layout and AI synthesis paragraph with a centered `Mi Estilo` workspace featuring a segmented software switch (`VirtualDJ`, `Rekordbox`, `Traktor`, `Serato`), three borderless typographic stats (`Ritmo habitual`, `Tonos principales`, `Mezcla`), one horizontal segmented `Balance musical` bar (`Melódico`, `Hora pico`, `Vocal`), three single-line preference switches (`Evitar choque de voces`, `Mantener graves continuos`, `Ocultar audio de baja calidad`), and an optional collapsible `Ver tonos detallados (1A–12A)` disclosure.
  - Simplify `HeraDjToolbar.tsx`, `TrackListHeader.tsx`, `TrackRow.tsx`, and `Toasts.tsx`: replace `1·2·3·4` text-heavy cue pills with clean 4-dot cue indicators (`Puntos`), replace routine success/info floating toasts with `2.2s` inline button confirmations (`✓ Exportado a VirtualDJ`, `✓ Mezcla ordenada`), and restrict floating toasts strictly to `warning` / `danger` notices at bottom-right with `1px` borders.
- **Non-Goals:**
  - Changing any Rust backend IPC command signatures in `hera-desktop/src-tauri` or `hera-rs`.
- **Anti-Goals:**
  - Removing real functionality (local folder/file import, 2-Opt crate ordering, multi-format DJ export, Deck A/B track selection, real audio/beat preview, P2P WAV download/import, SQLite playlist creation, or live style DNA recomputation).

---

## 3. Approaches Considered

1. **Cosmetic Copy Trimming Only (Rejected):** Shortening strings inside the existing 2-column nested-card layouts leaves structural visual noise, competing boxes, and redundant sidebar switches intact.
2. **Unified Minimalist Studio Distillation (Selected):** Flatten container hierarchy across `Navigation.tsx`, `HeraDjToolbar.tsx`, `TrackRow.tsx`, `Toasts.tsx`, `bridge.tsx`, and `dna.tsx` while keeping 100% of Tauri IPC bridges wired to clean, single-purpose controls.
3. **Separate Simple vs. Pro Route Trees (Rejected):** Doubles maintenance surface and forces users to manage a global mode toggle for information better handled via progressive disclosure.
