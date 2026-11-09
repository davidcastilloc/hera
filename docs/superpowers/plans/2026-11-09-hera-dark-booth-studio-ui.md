# Implementation Plan — Unified Dark Booth Studio UI (`hera-desktop`)

- [x] **Task 1: Native Dark Window, Spanish Default & Unified Studio Tokens**
  - Update `hera-desktop/src-tauri/src/plugins/config.rs`, `hera-desktop/src-tauri/src/libs/utils.rs`, `hera-desktop/src-tauri/src/main.rs`, `hera-desktop/index.html`, `hera-desktop/src/global.css`, `hera-desktop/src/themes/dark.json`, `hera-desktop/src/themes/light.json`, `hera-desktop/src/lib/bridge-config.ts`, and `hera-desktop/src/api/SettingsAPI.ts`.
- [x] **Task 2: Uniform 1.5px SVG Icon Set (`Icon.tsx`)**
  - Add `disc.svg`, `artists.svg`, `crate-list.svg`, `bridge.svg`, `dna.svg`, `sliders.svg`, `sparkles.svg`, `export.svg`, `cpu.svg`, `folder-plus.svg`, `heart.svg`, `check-circle.svg`, and `search.svg` under `hera-desktop/src/assets/icons/` and register in `hera-desktop/src/components/Icon.tsx`.
- [x] **Task 3: Left Navigation Rail, Transport Header & Contextual Crate Bar**
  - Refactor `hera-desktop/src/routes/__root.tsx`, `hera-desktop/src/components/Navigation.tsx`, `hera-desktop/src/components/TrackListStatus.tsx`, `hera-desktop/src/components/Header.tsx`, `hera-desktop/src/components/Search.tsx`, `hera-desktop/src/components/HeraDjToolbar.tsx`, `hera-desktop/src/routes/library.tsx`, and `hera-desktop/src/routes/playlists.$playlistID.tsx`.
- [x] **Task 4: Tabular Track List, Proportional 4-Cue Rail & Spanish Empty State**
  - Refactor `hera-desktop/src/components/TrackListStates.tsx`, `hera-desktop/src/components/TrackListHeader.tsx`, `hera-desktop/src/components/TrackRow.tsx`, and `hera-desktop/src/components/TrackProgress.tsx`.
- [x] **Task 5: Cohesive Studio Views (`/bridge`, `/dna`, `/settings`) & Verification Gate**
  - Refactor `hera-desktop/src/lib/hera-human-language.ts`, `hera-desktop/src/lib/hera-human-language.test.ts`, `hera-desktop/src/routes/bridge.tsx`, `hera-desktop/src/routes/dna.tsx`, `hera-desktop/src/elements/SettingsNav.tsx`, `hera-desktop/src/routes/settings.tsx`, `hera-desktop/src/routes/settings.library.tsx`, and `hera-desktop/src/routes/settings.about.tsx`.
  - Verify `vp check`, `vp test`, `vp build`, `cargo check`, `cargo test`, and standalone native `hera-desktop.exe` build.
