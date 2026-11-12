# 🛡️ Design Spec: Feature Flags and Clean Production Navigation

**Status:** Approved  
**Author:** David Castillo & Antigravity  
**Date:** 2026-11-12  
**Related QA Findings:** QA-14 (`/bridge` mockup), QA-15 (`/artists` empty state), QA-16 (Feature Flags)

---

## 🎯 Goal

Ensure HERA Desktop never exposes incomplete, simulated, or mockup features to end-users in production. All modules that are not 100% finished with real underlying data must be gated by a robust, reactive feature flag mechanism.

---

## 📐 Architecture & Principles

1. **Centralized Feature Flags (`src/lib/feature-flags.ts`):**
   - Pure TypeScript module with default production settings.
   - Reactive Zustand or LocalStorage store to allow toggling during development / testing without rebuilds.
   - Flags:
     - `enableHarmonicBridge`: default `false` (until real SQLite harmonic transition pathfinder is connected).
     - `enableStyleDna`: default `false` (until real SQLite aggregation is connected).
     - `enableExperimentalFileManager`: default `false`.
     - `showLabsBadgeInNav`: default `false`.

2. **Clean Navigation (`src/components/Navigation.tsx`):**
   - The rail only displays fully finished, production-ready items:
     - `Biblioteca` (`/library`)
     - `Agente` (`/agent`)
     - `Artistas` (`/artists`)
     - `Playlists` (`/playlists`)
     - `Ajustes` (`/settings`)
   - If a feature flag is disabled, its NavItem is completely omitted from the sidebar.

3. **Under Construction Component (`src/components/UnderConstructionView.tsx`):**
   - A unified, stylish Dark Booth card (`#0c0e12`, `#12161d`, `#14b8a6` accents).
   - Displayed if the user navigates directly to a disabled route (`/bridge`, `/dna`).
   - Features:
     - Icon: `tool` / `sparkles` with teal glow.
     - Badge: `HERA LABS · EN DESARROLLO`.
     - Feature title and clear explanation.
     - Single primary button: `Volver a la Biblioteca` (`navigate({ to: '/library' })`).

4. **Fix for QA-15 (`src/routes/artists.tsx`):**
   - Prevent passing hardcoded `tracks={[]}` to `TrackListStates` when landing on `/artists`.
   - If `artists.length > 0` and no artist is in URL, auto-redirect to first artist or show a clean selection prompt: *"Selecciona un artista de la lista lateral"*.
   - If `artists.length === 0`, show contextual notice: *"No se encontraron artistas en la biblioteca"*.

---

## 🧪 Verification Strategy

- Run `bun run check` and `bun run test` in `hera-desktop`.
- Verify that `Navigation.tsx` renders only the 5 clean items.
- Verify that visiting `/bridge` and `/dna` renders the `UnderConstructionView` with button to `/library`.
- Verify `artists.tsx` does not display "Tu biblioteca de cabina está vacía".
