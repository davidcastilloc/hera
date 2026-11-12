# 📋 Implementation Plan: Feature Flags, Incomplete Feature Guard & Clean Navigation

**Status:** In Progress  
**Spec Reference:** `docs/superpowers/specs/2026-11-12-feature-flags-and-clean-navigation-design.md`  
**Related QA Findings:** QA-14, QA-15, QA-16

---

### Task List

- [x] **Task 1: Feature Flags Store & Under Construction Component**
  - Create `hera-desktop/src/lib/feature-flags.ts` with reactive Zustand store and local storage persistence.
  - Create `hera-desktop/src/components/UnderConstructionView.tsx` with Dark Booth aesthetic, badge, description, and "Volver a la Biblioteca" button.
  - Verify with unit test `hera-desktop/src/lib/__tests__/feature-flags.test.ts`.

- [x] **Task 2: Clean Navigation Integration**
  - Update `hera-desktop/src/components/Navigation.tsx` to read `useFeatureFlags()`:
    - Hide `/bridge` if `enableHarmonicBridge` is false.
    - Hide `/dna` if `enableStyleDna` is false.
  - Wrap `/bridge` in `hera-desktop/src/routes/bridge.tsx` with `UnderConstructionView` if flag is false.
  - Wrap `/dna` in `hera-desktop/src/routes/dna.tsx` with `UnderConstructionView` if flag is false.

- [x] **Task 3: Fix QA-15 False Empty Library in `/artists`**
  - In `hera-desktop/src/routes/artists.tsx`, replace `<TrackListStates isLoading={false} tracks={[]} />` when `pathname === '/artists'`.
  - Show clean selection prompt: *"Selecciona un artista de la barra lateral para ver sus pistas"* or notice if no artists exist.

- [x] **Task 4: Typecheck, Unit Tests & Git Commit**
  - Run `cd hera-desktop && bun run check && bun run test`.
  - Commit changes with conventional commit `feat(ui): implement feature flags, under construction view, and clean navigation`.
