# VirtualDJ Mini Overlay (`Modo Mini`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a single-window Always-on-Top `Modo Mini` overlay (`360×408` `Tarjeta de Cabina` and `560×52` `Píldora HUD`) with live VirtualDJ `.vdjfolder` push and `History/tracklist.txt` detection to `hera-desktop`.

**Architecture:** Extend `plugin:hera-dj` in Rust with non-blocking VirtualDJ live-crate export, `tracklist.txt` history parsing, and atomic `WebviewWindow` morphing (`full` ↔ `card` ↔ `pill` with saved geometry snapshot). Lazy-load a dedicated StyleX `MiniOverlay.tsx` shell from `__root.tsx` when mini mode is active while adding a `Modo Mini` trigger button + `Ctrl+Shift+M` shortcut to the main `Header.tsx`.

**Tech Stack:** Rust (`tauri` v2, `hera-node-rs`), React 19 (`React.lazy`, `Suspense`, `startTransition`), TypeScript, StyleX, Zustand, Vitest (`vp check`, `vp test`, `vp build`).

**Spec:** `docs/superpowers/specs/2026-11-09-virtualdj-mini-overlay-design.md`

## Global Constraints

- Strictly preserve the dark studio palette (`#0c0e12`, `#10141b`, `#171c24`, `1px` `#26303d` borders, `#14b8a6` accent); zero light surfaces, zero purple/violet hues, zero thick side borders, zero routine floating toasts.
- Run all filesystem I/O in Rust inside `tauri::async_runtime::spawn_blocking`.
- Lazy-load `MiniOverlay` (`React.lazy`) so full-studio startup bundle does not pay for overlay UI until activated.
- Use repository-relative paths only in specs, plans, and commits.

## Review Focus

- Missing `Documents/VirtualDJ` directory: `push_virtualdj_live_crate` must still succeed by writing to `exports/VirtualDJ/HERA En Vivo.vdjfolder` (and creating `Documents/VirtualDJ/Folders` only when user `Documents` exists) without erroring, and `read_virtualdj_now_playing` must return `null` cleanly when no VirtualDJ history file exists.
- Restoring `full` mode after toggling `card` ↔ `pill`: `NormalWindowSnapshot` must only be overwritten when transitioning *from* `full`, never when switching between `card` and `pill`, so `⤢` always restores the true full-studio window dimensions (`1180×720` or `1280×800`).
- Empty library (`0 pistas`): `MiniOverlay` must display clean default track values (`Eric Prydz — Opus · 126 · 8A`) and synthesize/download valid audio if `+ VDJ` is clicked on a bridge suggestion.
- Long artist/track names inside `360 px` card and `560 px` pill: every title container must enforce `minWidth: 0`, `overflow: 'hidden'`, `textOverflow: 'ellipsis'`, and `whiteSpace: 'nowrap'` so `scrollWidth <= clientWidth` at both `360×408` and `560×52`.
- Pin toggle (`alwaysOnTop`): unpinning in `MiniOverlay` updates `set_always_on_top(false)` immediately without exiting Mini Mode.

---

### Task 1: Rust Backend Commands (`set_mini_overlay_mode`, `push_virtualdj_live_crate`, `read_virtualdj_now_playing`) & Unit Tests

**Files:**
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj.rs`
- Modify: `hera-desktop/src-tauri/build.rs`
- Modify: `hera-desktop/src-tauri/capabilities/main.json`

- [x] **Step 1: Add unit tests in `hera-desktop/src-tauri/src/plugins/hera_dj.rs` for VirtualDJ `tracklist.txt` parsing and live `.vdjfolder` generation**
- [x] **Step 2: Implement `set_mini_overlay_mode`, `push_virtualdj_live_crate`, and `read_virtualdj_now_playing` in `hera_dj.rs` and register permissions in `build.rs` and `capabilities/main.json`**
- [x] **Step 3: Run `cargo test --manifest-path hera-desktop/src-tauri/Cargo.toml` and verify green receipt**

### Task 2: TypeScript Bridge, Zustand Mini-Mode Store & Lazy-Loaded `MiniOverlay.tsx`

**Files:**
- Modify: `hera-desktop/src/lib/bridge-hera.ts`
- Create: `hera-desktop/src/lib/mini-mode-store.ts`
- Create: `hera-desktop/src/components/MiniOverlay.tsx`
- Modify: `hera-desktop/src/components/Header.tsx`
- Modify: `hera-desktop/src/routes/__root.tsx`

- [x] **Step 1: Add typed wrappers (`setMiniOverlayMode`, `pushVirtualDjLiveCrate`, `readVirtualDjNowPlaying`) in `bridge-hera.ts` and lightweight `useMiniModeStore` in `mini-mode-store.ts`**
- [x] **Step 2: Build `MiniOverlay.tsx` (`card` `360×408` with `Siguientes` & `Puente` tabs + `pill` `560×52` HUD capsule, inline `✓ En VDJ` state, headphone preview, Pin toggle, `▾`/`▴` collapse, `⤢` restore)**
- [x] **Step 3: Add `Modo Mini` button + `Ctrl+Shift+M` shortcut in `Header.tsx` and lazy-loaded `<MiniOverlay />` switch in `__root.tsx`**
- [x] **Step 4: Run `vp check --fix`, `vp check`, `vp test`, `vp build`, and craft detector (`0` findings)**

### Task 3: End-to-End Live WebView2 Verification via Tauri MCP & Atomic Commit

**Files:**
- Modify: `docs/superpowers/plans/2026-11-09-virtualdj-mini-overlay.md`
- Modify: `docs/superpowers/DECISIONS.md`

- [x] **Step 1: Build `debug` and `release` binaries and execute live Tauri MCP QA verifying `full` -> `card` (`360×408`) -> `+ VDJ` (`HERA En Vivo.vdjfolder` written on disk) -> `Puente` tab -> `pill` (`560×52`) -> `full` restoration (`0` console errors, `0` overflow)**
- [x] **Step 2: Visually inspect live WebView2 screenshots of `card` (`Siguientes` & `Puente`) and `pill`, mark `- [x]`, append `ADR-021` in `DECISIONS.md`, and commit atomically**
