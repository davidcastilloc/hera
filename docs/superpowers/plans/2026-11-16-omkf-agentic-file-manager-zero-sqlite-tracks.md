# OMKF Agentic File Manager & Zero-SQLite-Tracks UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace HERA Desktop's flat SQLite `tracks` UI feed with an in-memory + live-filesystem OMKF hierarchical file manager (`Carpetas`), eliminate route-transition IPC blocking, and isolate audio playback (`0` background DSP workers while playing + `< 10 ms` Stage-3 buffer upgrade at `40s`).

**Architecture:** Rust's new `OmkfFsCatalog` indexes `Config.library_folders` hierarchically in RAM from live directory entries and `.hera/tracks.omkf` capsules, serving folder trees (`omkf_list_folder_crates`), live per-folder tracks + subfolders (`omkf_read_folder_crate`), deduplicated cross-folder search (`__search__:<query>`), and harmonic candidates (`__harmonic__:<bpm>:<camelot>`) without reading SQLite `tracks`. On the audio thread, `AUDITION_WINDOW_SECS` rises to `65.0s`, Stage 3 swaps linear-resampled audio without `rustfft`, and `AutoGigGovernor` drops background scanner threads to `0` whenever a deck is playing.

**Tech Stack:** Rust (`hera-node-rs`, `hera-ipc` `ts-rs`, `hera-desktop` Tauri v2), TypeScript / React 19 / StyleX / TanStack Router & Query / Zustand / Vitest (`Vite+` + `Bun 1.4.4`).

**Spec:** `docs/superpowers/specs/2026-11-16-omkf-agentic-file-manager-zero-sqlite-tracks-design.md`

## Global Constraints

- Every `.rs` file inside `hera-rs/src/scanner`, `hera-rs/src/audio_engine`, `hera-desktop/src-ipc/src`, and `hera-desktop/src-tauri/src/plugins/hera_dj` MUST stay `<= 349` lines (`ratchet_monolith_max_lines_ceiling`). Every new `.rs` helper under `hera-desktop/src-tauri/src/libs/` MUST also stay `<= 349` lines.
- Frontend file line ceilings enforced by `hera-rs/tests/architectural_ratchets.rs`: `hera-desktop/src/lib/omkf-bridge.ts <= 300`, `hera-desktop/src/lib/player.ts <= 680`, `hera-desktop/src/lib/native-audio-transport.ts <= 320`, `hera-desktop/src/routes/bridge.tsx <= 450`, `hera-desktop/src/components/MiniOverlay.tsx <= 450`.
- Preserve `EXPECTED_TAURI_COMMANDS_COUNT = 77` across the 8 internal Tauri plugins by extending existing `omkf_*` DTOs and sentinel queries (`__search__:<q>`, `__harmonic__:<bpm>:<cam>`) rather than adding/removing `#[tauri::command]` functions.
- `hera-rs` uses Rust 2021 edition (avoid Rust 2024 `if let ... && let ...` chains inside `hera-rs/`).
- Sacred Disk Invariant: NEVER delete user audio/video files on disk; `.hera/crate.md` organization moves must support `dry_run` preview and `# Ledger Reversible` rollback.
- Dark Booth Studio (`hera-desktop/DESIGN.md`): `#0c0e12` ground, `#10141b` rails, `#12161d` bars, `#171c24` elevated controls, `#232a36` `1px` hairlines, `#14b8a6` / `#2dd4bf` emerald accent, `tabular-nums`, Spanish DJ-booth UI copy, zero violet/purple hues, zero white surfaces, zero decorative emojis.
- Zero OS-specific absolute host paths in committed code, tests, plans, or `DECISIONS.md`. Sign all commits (`git commit -S`) as `David Castillo <vikruzdavid@gmail.com>`.

## Review Focus

1. **Nested overlapping `library_folders` roots (e.g. parent `Music` configured alongside child `Music/_REPERTORIO_DJ`):** `OmkfFsCatalog::refresh_roots` must deduplicate nested child roots covered by an ancestor root so `_REPERTORIO_DJ` appears once in the hierarchy, never twice.
2. **Container DJ folders with 0 direct audio files (e.g. `_REPERTORIO_DJ` holding only subfolders `01_Afro_House`, `03_Reggaeton`):** `OmkfFsCatalog` must retain ancestor directories whose descendants have media files (`subfolder_count > 0`) and expose their immediate `subfolders` when selected.
3. **Unanalyzed audio files (`scan_level == 0`) dropped into a folder without `.hera/tracks.omkf`:** `OmkfFsCatalog::read_folder` must still return those live files immediately using filename-parsed `Artist - Title [BPM - Camelot]` metadata so the folder is never blank before DSP finishes.
4. **Cross-folder duplicate collapse in global search (`__search__:<query>`):** Identical tracks copied across `_REPERTORIO_DJ`, `BACKUP`, and `_USB_PIONEER` must collapse into a single search result preferring the active non-`BACKUP` path and reporting `copy_count == 3`.
5. **Deck playback resumption after `0`-worker hard pause:** Transitioning `ScanScheduler::set_deck_playing(true)` -> `effective_workers() == 0` -> `set_deck_playing(false)` must notify parked worker threads via `Condvar::notify_all()` so queued jobs resume immediately without deadlock.

---

### Task 1: Zero-Contention Audio Engine (`65s` Audition Window, `< 10 ms` Stage-3 Upgrade & `0`-Worker Deck Pause)

**Files:**
- Modify: `hera-rs/src/scanner/scheduler.rs`
- Modify: `hera-rs/src/scanner/scheduler_tests.rs`
- Modify: `hera-rs/src/audio_engine/prefix_loader.rs`
- Modify: `hera-rs/src/audio_engine/commit_gate.rs`
- Modify: `hera-rs/src/audio_engine/tests.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/scanner_v3.rs`

**Interfaces:**
- Produces:
  - `AutoGigGovernor::effective_workers(&self) -> usize` returning `0` when `self.is_deck_playing()` is `true`, `1` when `!is_deck_playing() && (is_external_booth_mode() || is_on_battery_power())`, and `max_workers` otherwise.
  - `ScanScheduler::set_deck_playing(&self, playing: bool)` updating `self.governor.set_deck_playing(playing)` and calling `self.cv.notify_all()` when `!playing`; `ScanScheduler::pop_next` and `ScanScheduler::wait_next` returning `None` while `self.governor.is_deck_playing()` is `true`.
  - `pub(crate) const AUDITION_WINDOW_SECS: f64 = 65.0;` in `hera-rs/src/audio_engine/prefix_loader.rs`.
  - `spawn_two_stage_deck_loader` Stage 3 in `hera-rs/src/audio_engine/commit_gate.rs` using `resample_deck_buffer_fast_prefix(&raw_full, target_sr)` and proportional/known cues (zero `compute_cues_seconds` / `rustfft`, zero `resample_deck_buffer_to_rate` `rubato` sinc).

- [x] **Step 1: Write failing unit tests in `hera-rs/src/scanner/scheduler_tests.rs` and `hera-rs/src/audio_engine/tests.rs`**

Add `test_deck_playing_hard_pauses_workers_to_zero_and_wakes_on_stop` in `hera-rs/src/scanner/scheduler_tests.rs`:
```rust
#[test]
fn test_deck_playing_hard_pauses_workers_to_zero_and_wakes_on_stop() {
    let sched = ScanScheduler::new(4);
    sched.enqueue_or_promote("t-1", ScanPriority::P0Immediate);
    sched.set_deck_playing(true);
    assert_eq!(sched.governor.effective_workers(), 0);
    assert_eq!(sched.pop_next(), None);
    assert_eq!(sched.wait_next(Duration::from_millis(10)), None);

    sched.set_deck_playing(false);
    assert_eq!(sched.governor.effective_workers(), 4);
    assert_eq!(sched.pop_next(), Some(("t-1".to_string(), ScanPriority::P0Immediate)));
}
```
And in `hera-rs/src/audio_engine/tests.rs`, update any `50.0` audition-window assertions to `65.0` (`AUDITION_WINDOW_SECS`) and add `test_audition_window_65s_and_fast_stage3_commit_swap` verifying `AUDITION_WINDOW_SECS == 65.0` and that advancing past `40.0s` upgrades a `> 70s` track to full length without triggering `is_buffering`.

- [x] **Step 2: Run tests to verify failure**

Run: `cargo test -p hera-node-rs test_deck_playing_hard_pauses_workers_to_zero_and_wakes_on_stop`
Expected: FAIL (`set_deck_playing` missing / `effective_workers()` returns `1` instead of `0`).

- [x] **Step 3: Implement `0`-worker deck pause and fast Stage-3 upgrade**

1. In `hera-rs/src/scanner/scheduler.rs`:
   - Update `AutoGigGovernor::effective_workers(&self) -> usize` to return `0` when `self.is_deck_playing()`, `1` when `self.is_external_booth_mode() || self.is_on_battery_power()`, else `self.max_workers`.
   - Add `pub fn set_deck_playing(&self, playing: bool)` on `ScanScheduler` (calls `self.governor.set_deck_playing(playing); self.cv.notify_all();`).
   - Guard `pop_from_inner`: pass `deck_playing: bool` (or check `self.governor.is_deck_playing()` in `pop_next` and `wait_next`) so `None` is returned immediately while `is_deck_playing()` is `true`.
2. In `hera-desktop/src-tauri/src/plugins/hera_dj/scanner_v3.rs`: update `scanner_set_deck_playing` to call `global_scheduler().set_deck_playing(is_playing)`.
3. In `hera-rs/src/audio_engine/prefix_loader.rs`: change `AUDITION_WINDOW_SECS` from `50.0` to `65.0` (remove unused `compute_cues_seconds` if dead or mark `#[cfg(test)]` if only used in tests).
4. In `hera-rs/src/audio_engine/commit_gate.rs`: in Stage 3 (`lines 126-145`), compute cues from `known_cues_sec.map(sanitize_cue_seconds).unwrap_or_else(|| [0.02, 0.25, 0.50, 0.80].map(|r| full_dur_sec * r))` where `full_dur_sec = (full.interleaved_stereo.len() / 2) as f64 / f64::from(full.sample_rate.max(1))`, and call `resample_deck_buffer_fast_prefix(&raw_full, target_sr)` instead of `resample_deck_buffer_to_rate`.

- [x] **Step 4: Run Rust audio & scanner tests and architectural ratchets**

Run: `cargo test -p hera-node-rs scanner::scheduler -- --nocapture && cargo test -p hera-node-rs audio_engine -- --nocapture && cargo test -p hera-node-rs --test architectural_ratchets`
Expected: PASS (`0 failed`, all `.rs` files `<= 349` lines).

- [x] **Step 5: Mark Task 1 complete and commit**

Mark Task 1 checkboxes `[x]` in `docs/superpowers/plans/2026-11-16-omkf-agentic-file-manager-zero-sqlite-tracks.md` and commit signed (`git commit -S`) with `fix(audio-engine): pause scanner to 0 workers on play and make 40s stage-3 swap lock-free`.

---

### Task 2: `hera-ipc` OMKF Filesystem DTOs & Rust In-Memory `OmkfFsCatalog`

**Files:**
- Create: `hera-desktop/src-ipc/src/omkf_fs_types.rs`
- Modify: `hera-desktop/src-ipc/src/lib.rs`
- Modify: `hera-desktop/src-ipc/src/dj_types.rs`
- Modify: `hera-desktop/src-ipc/src/typegen.rs`
- Create: `hera-desktop/src-tauri/src/libs/omkf_fs_catalog.rs`
- Create: `hera-desktop/src-tauri/src/libs/omkf_fs_organize.rs`
- Modify: `hera-desktop/src-tauri/src/libs/mod.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/omkf_ipc.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/scanner_worker.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/db.rs`
- Modify: `hera-desktop/src/generated/hera-typings.ts`

**Interfaces:**
- Consumes: `ScanScheduler::set_deck_playing` / `governor.is_deck_playing()` from Task 1; `hera_node_rs::scanner::omkf_capsule::{read_omkf_capsule, write_omkf_capsule, encode_omkf_capsule, OmkfHeader, OmkfTrackEntry, OmkfTrackSlot, code_to_camelot, camelot_to_code}`.
- Produces:
  - `hera_ipc::omkf_fs_types` (re-exported via `hera_ipc::dj_types::*`):
    - `OmkfFsHotCueDto { pub slot: u8, pub label: String, pub seconds: f64, pub kind: String }`
    - `OmkfFsTrackDto { pub id: String, pub crate_id: String, pub path: String, pub file_name: String, pub title: String, pub artist: String, pub album: String, pub duration_sec: u32, pub bpm: Option<f64>, pub camelot: Option<String>, pub energy: Option<f64>, pub hot_cues: Vec<OmkfFsHotCueDto>, pub scan_level: i8, pub is_video: bool, pub is_fake_320kbps: bool, pub has_audio_skips: bool, pub is_variable_bpm: bool, pub has_intro_vocals: bool, pub has_outro_vocals: bool, pub replay_gain_db: Option<f64>, pub waveform_peaks_b64: Option<String>, pub copy_count: usize }`
    - Extended `FolderCrateSummary` adding `#[serde(default)] #[ts(optional = nullable)] pub parent_crate_id: Option<String>`, `#[serde(default)] pub rel_path: String`, `#[serde(default)] #[ts(type = "number")] pub depth: usize`, `#[serde(default)] #[ts(type = "number")] pub subfolder_count: usize`.
    - Extended `FolderCrateDetail` adding `#[serde(default)] pub subfolders: Vec<FolderCrateSummary>`, `#[serde(default)] pub tracks: Vec<OmkfFsTrackDto>`.
  - `crate::libs::omkf_fs_catalog::global_omkf_catalog() -> &'static OmkfFsCatalog` with:
    - `list_folders(&self, roots: &[PathBuf]) -> Vec<FolderCrateSummary>`
    - `read_folder(&self, query_or_crate_id: &str, roots: &[PathBuf]) -> (Option<FolderCrateDetail>, Vec<(String, String)>)` (supporting `"__search__:<q>"` and `"__harmonic__:<bpm>:<camelot>"` sentinels plus `(track_id, path)` list of unanalyzed `scan_level == 0` files)
    - `get_tracks_by_ids(&self, ids: &[String]) -> Vec<crate::libs::track::Track>`
    - `invalidate_folder_path(&self, dir: &Path)`

- [x] **Step 1: Write failing tests for `hera-ipc` bindings and `OmkfFsCatalog` (Review Focus #1–#4)**

1. In `hera-desktop/src-ipc/src/typegen.rs` `test_export_hera_typescript_bindings`, assert `export type OmkfFsTrackDto =`, `subfolders: Array<FolderCrateSummary>`, and `parent_crate_id?: string | null`.
2. In `hera-desktop/src-tauri/src/libs/omkf_fs_catalog.rs` `#[cfg(test)] mod tests`, write unit tests using `tempfile::tempdir()` covering:
   - Nested overlapping roots (`root` + `root/_REPERTORIO_DJ`) deduplicate cleanly so `_REPERTORIO_DJ` appears once with `depth == 1`, `subfolder_count == 2`, and container folders with `0` direct audio files remain in the tree.
   - `read_folder` merges `.hera/tracks.omkf` slots (`bpm = 120.0`, `camelot = "4B"`, 4 Hot Cues) with live unanalyzed files (`Artist - Song [96 BPM - 12B].mp3`) while ignoring `._junk.mp3` and `__MACOSX`.
   - `read_folder("__search__:mirala bien", ...)` collapses 3 identical copies across `_REPERTORIO_DJ/03_Reggaeton`, `BACKUP/Old`, and `_USB_PIONEER` into 1 result pointing at `_REPERTORIO_DJ/03_Reggaeton` with `copy_count == 3`.
   - `organize_folder_fs` (`dry_run = true`, apply `dry_run = false`, and `rollback_tx_id`) moves files into tempo/genre subfolders, writes `.hera/crate.md` `# Ledger Reversible`, and restores files on rollback without SQLite rows.

- [x] **Step 2: Run tests to verify failure**

Run: `cargo test -p hera-ipc && cargo test -p hera-desktop --bins omkf_fs_catalog`
Expected: FAIL (missing `omkf_fs_types` / `omkf_fs_catalog`).

- [x] **Step 3: Implement `omkf_fs_types.rs`, `omkf_fs_catalog.rs`, `omkf_fs_organize.rs`, and wire `omkf_ipc.rs` + `scanner_worker.rs` + `db.rs`**

1. Create `hera-desktop/src-ipc/src/omkf_fs_types.rs` (`<= 220` lines), re-export in `lib.rs` and `dj_types.rs`, register `OmkfFsHotCueDto` and `OmkfFsTrackDto` in `typegen.rs`, and regenerate `hera-desktop/src/generated/hera-typings.ts` (`cd hera-desktop && bun run gen:types`).
2. Create `hera-desktop/src-tauri/src/libs/omkf_fs_catalog.rs` (`<= 340` lines) and `hera-desktop/src-tauri/src/libs/omkf_fs_organize.rs` (`<= 280` lines), registered in `libs/mod.rs` (and add both paths to `FILE_LINE_CEILINGS` in `hera-rs/tests/architectural_ratchets.rs` at `349`).
3. Update `hera-desktop/src-tauri/src/plugins/hera_dj/omkf_ipc.rs` (`<= 240` lines) to read `ConfigManager` (`State<'_, RwLock<ConfigManager>>` or `app_handle.state::<ConfigManager>()`) for `library_folders`, serve `omkf_list_folder_crates` and `omkf_read_folder_crate` from `global_omkf_catalog()`, enqueue unanalyzed `scan_level == 0` files into `scanner_worker::enqueue_fs_discovered_tracks` when `!global_scheduler().governor.is_deck_playing()`, and run `omkf_save_crate_rules` / `omkf_preview_or_apply_organize` via `omkf_fs_organize`.
4. Update `hera-desktop/src-tauri/src/plugins/hera_dj/scanner_worker.rs` (`<= 349` lines):
   - Expose `enqueue_fs_discovered_tracks(items: Vec<(String, String)>)` pushing into a static channel drained into `cache` + `P2FastBackground`.
   - When `sched.governor.is_deck_playing()` is `true`, skip capsule disk flushes and background DB polling.
   - When DSP outcomes arrive for filesystem tracks, upsert their slot directly into `<parent>/.hera/tracks.omkf` (or via dirty-folder flush), call `global_omkf_catalog().invalidate_folder_path(parent)`, and emit `"hera:omkf-folder-updated"` with affected `crate_ids` throttled to at most once every `2000 ms`.
5. Update `hera-desktop/src-tauri/src/plugins/db.rs` `get_tracks` to resolve matching IDs from `global_omkf_catalog().get_tracks_by_ids(&ids)` first and query SQLite only for unresolved IDs.

- [x] **Step 4: Run `hera-ipc`, `hera-desktop`, and `architectural_ratchets` tests**

Run: `cargo test -p hera-ipc && cargo test -p hera-desktop --bins omkf_fs -- --nocapture && cargo test -p hera-node-rs --test architectural_ratchets`
Expected: PASS (`0 failed`, `EXPECTED_TAURI_COMMANDS_COUNT == 77`, all file line ceilings respected).

- [x] **Step 5: Mark Task 2 complete and commit**

Mark Task 2 checkboxes `[x]` in `docs/superpowers/plans/2026-11-16-omkf-agentic-file-manager-zero-sqlite-tracks.md` and commit signed (`git commit -S`) with `feat(omkf-catalog): add in-memory OmkfFsCatalog, live folder hydration, and deduplicated search`.

---

### Task 3: Hierarchical Agentic File Manager UI (`Carpetas`), Zero-SQLite Boot & Instant Routing

**Files:**
- Modify: `hera-desktop/src/lib/omkf-bridge.ts`
- Modify: `hera-desktop/src/routes/__root.tsx`
- Modify: `hera-desktop/src/api/LibraryAPI.ts`
- Modify: `hera-desktop/src/components/LibraryEvents.tsx`
- Modify: `hera-desktop/src/lib/player.ts`
- Modify: `hera-desktop/src/components/Navigation.tsx`
- Modify: `hera-desktop/src/components/PlayingBarInfo.tsx`
- Modify: `hera-desktop/src/components/FolderCrateTree.tsx`
- Create: `hera-desktop/src/components/FolderExplorerHeader.tsx`
- Modify: `hera-desktop/src/routes/library.tsx`
- Modify: `hera-desktop/src/routes/playlists.tsx`
- Modify: `hera-desktop/src/components/TrackList.tsx`
- Modify: `hera-desktop/src/components/MiniOverlay.tsx`
- Modify: `hera-desktop/src/routes/dna.tsx`
- Modify: `hera-desktop/src/components/transition/useTransitionStudio.ts`
- Modify: `hera-desktop/src/components/transition/transition-actions.ts`
- Modify: `hera-desktop/src/lib/agent-store.ts`
- Create: `hera-desktop/src/lib/__tests__/omkf-file-manager.test.ts`
- Modify: `hera-desktop/src/__tests__/hera-studio-qa.test-e2e.tsx`
- Modify: `hera-desktop/src/__tests__/library.test-e2e.tsx`

**Interfaces:**
- Consumes: `FolderCrateSummary`, `FolderCrateDetail`, `OmkfFsTrackDto` from `hera-desktop/src/generated/hera-typings.ts` (Task 2); `plugin:hera-dj|omkf_list_folder_crates`, `plugin:hera-dj|omkf_read_folder_crate` (`crateId`, `"__search__:<q>"`, `"__harmonic__:<bpm>:<cam>"`), `hera:omkf-folder-updated` event.
- Produces:
  - `mapOmkfDtoToTrack(dto: OmkfFsTrackDto): Track & { copy_count?: number }` in `omkf-bridge.ts`
  - `searchAllFoldersDeduplicated(query: string): Promise<Array<Track & { copy_count?: number }>>`
  - `queryHarmonicCandidates(bpm: number, camelot: string): Promise<Track[]>`
  - Collapsible hierarchical `FolderCrateTree` (`Raíces y Carpetas`) + `FolderExplorerHeader` (`Ruta:` breadcrumbs + clickable `Subcarpetas:` pills) in `/library` (`Carpetas`).

- [x] **Step 1: Write failing unit tests in `hera-desktop/src/lib/__tests__/omkf-file-manager.test.ts`**

Write Vitest unit tests covering:
- `mapOmkfDtoToTrack` maps `OmkfFsTrackDto` (`bpm`, `camelot`, 4 `hot_cues`, `copy_count`, `scan_level`, `duration_sec`) to `Track` without calling `DatabaseBridge.getTracks`.
- Hierarchical tree builder (`buildFolderTreeRows`) groups roots (`parent_crate_id == null`), collapses/expands children (`depth > 0`), and picks the first workable non-`BACKUP` folder as initial selection (`pickInitialFolderCrateId`).

- [x] **Step 2: Run unit test to verify failure**

Run: `cd hera-desktop && bun run test -- src/lib/__tests__/omkf-file-manager.test.ts`
Expected: FAIL (`mapOmkfDtoToTrack` / `buildFolderTreeRows` / `pickInitialFolderCrateId` not exported).

- [x] **Step 3: Implement OMKF bridge, instant routing, hierarchical `Carpetas` explorer, and cross-surface hooks**

1. `hera-desktop/src/lib/omkf-bridge.ts` (`<= 300` lines):
   - Import generated DTOs from `../generated/hera-typings`.
   - Implement `mapOmkfDtoToTrack`, `buildFolderTreeRows`, `pickInitialFolderCrateId`, `searchAllFoldersDeduplicated`, and `queryHarmonicCandidates`. When `detail.tracks.length > 0`, `readFolderCrate` maps `detail.tracks` directly in RAM with zero `DatabaseBridge.getTracks` IPC call; when `!isTauri()`, preserve mock store fallback for Vitest browser E2E.
2. Eliminate SQLite `tracks` boot/event churn & blocking route loaders:
   - `src/routes/__root.tsx`: remove `queryClient.ensureQueryData(allTracksQuery)` on boot.
   - `src/api/LibraryAPI.ts`: remove `DatabaseBridge.getAllTracks()` calls after scan/import; invalidate `['omkf-folder-crates']` and `['omkf-folder-crate-detail']` instead.
   - `src/components/LibraryEvents.tsx`: subscribe to `hera:omkf-folder-updated` (`{ crate_ids?: string[] }`) to invalidate `['omkf-folder-crates']` and matching `['omkf-folder-crate-detail']` queries; stop mutating `allTracksQuery` on `200 ms` intervals.
   - `src/lib/player.ts` (`<= 680` lines): remove `notifyDeckTrack` from the `40s` playing timer so active playback never pokes the scanner; trigger `notifyDeckTrack` only inside `pause()`/`stop()` if the track had `scan_level === 0`.
   - `src/routes/library.tsx` & `src/routes/playlists.tsx`: remove `beforeLoad` / `loader` blocking `await`s (`useQuery` inside components with `staleTime: 30_000`); remove the duplicate `Carpetas (.hera)` tab from `playlists.tsx`.
3. Navigation & Hierarchical File Manager UI (`DESIGN.md` Dark Booth Studio):
   - `src/components/Navigation.tsx`: rename `/library` item label from `"Biblioteca"` to `"Carpetas"` and remove `<NavItem to="/artists" label="Artistas" icon="artists" />`.
   - `src/components/PlayingBarInfo.tsx`: render artist & album as plain studio text spans instead of `<Link to="/artists/...">`.
   - `src/components/FolderCrateTree.tsx`: render `RAÍCES Y CARPETAS` collapsible hierarchy (`▾` / `▸` SVG chevrons, `depth * 14px` indent, emerald active state `#14b8a6`, tabular `track_count` or `subfolder_count` pill).
   - `src/components/FolderExplorerHeader.tsx`: render `Ruta:` clickable breadcrumb trail (`Music ▸ _REPERTORIO_DJ ▸ 01_Afro_House` + right-aligned `N pistas · .hera sincronizado`) and clickable `Subcarpetas:` dark elevated cards (`#171c24`, `1px solid #232a36`, folder SVG icon + `display_name · count`) when `subfolders.length > 0`.
   - `src/routes/library.tsx`: auto-select `pickInitialFolderCrateId(folderCrates)` when `selectedCrateId === null`; mount `HeraDjToolbar`, `FolderCrateTree`, `FolderExplorerHeader`, `CrateRulesBanner`, and `TrackList`; when global `search` (`useLibraryStore(s => s.search)`) is non-empty, query `searchAllFoldersDeduplicated(search)` and show deduplicated results banner (`Resultados en carpetas · sin duplicados`); sync active folder tracks into `useLibraryStore.setState({ tracks: activeTracks })`.
   - `src/components/TrackList.tsx`: hide `Remove from library` and `/tracks/$trackID` edit item when `queueOrigin.type === 'library'`.
   - `src/components/MiniOverlay.tsx`, `src/routes/dna.tsx`, `src/components/transition/useTransitionStudio.ts`, `src/components/transition/transition-actions.ts`, `src/lib/agent-store.ts`: when `isTauri()` and `useLibraryStore.getState().tracks` needs cross-folder candidates, call `queryHarmonicCandidates` / `searchAllFoldersDeduplicated('')` instead of `DatabaseBridge.getAllTracks()`.
4. Update translations (`bun run gen:translations` if needed) and update `hera-studio-qa.test-e2e.tsx` + `library.test-e2e.tsx` to match `"Carpetas"` nav/heading and the streamlined `/playlists` view.

- [x] **Step 4: Run frontend typecheck, lint, unit tests, E2E tests, build, and architectural ratchets**

Run: `cd hera-desktop && bun run check && bun run test && bun run test:e2e && bun run build && cd .. && cargo test -p hera-node-rs --test architectural_ratchets`
Expected: PASS (`0` lint/type errors, all unit & E2E suites green, all line-count ceilings respected).

- [x] **Step 5: Mark Task 3 complete and commit**

Mark Task 3 checkboxes `[x]` in `docs/superpowers/plans/2026-11-16-omkf-agentic-file-manager-zero-sqlite-tracks.md` and commit signed (`git commit -S`) with `feat(desktop-ui): launch hierarchical OMKF Carpetas file manager with instant routing and zero SQLite tracks`.

---

### Task 4: Full Polyglot Verification, `DECISIONS.md`, `beast` `--release` Build & Live Deploy to `qa`

**Files:**
- Modify: `docs/superpowers/DECISIONS.md`
- Modify: `docs/superpowers/plans/2026-11-16-omkf-agentic-file-manager-zero-sqlite-tracks.md`

**Interfaces:**
- Consumes: Tasks 1–3 completed and verified.
- Produces: Signed `ADR-039` entry in `docs/superpowers/DECISIONS.md`, optimized `target/release/hera-desktop.exe` built on `beast` embedding `hera-desktop/dist`, pushed `main` branch, and live standalone `--release` `hera-desktop.exe` running on `qa` via `hc`.

- [x] **Step 1: Run full workspace polyglot verification + Impeccable anti-pattern gate**

Run:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p hera-ipc`
- `cargo test -p hera-node-rs --test architectural_ratchets`
- `cargo test -p hera-node-rs -- --test-threads=4`
- `cargo test -p hera-desktop --bins -- --skip export_bindings`
- `cd hera-desktop && bun run gen:types && bun run knip && bun run check && bun run test && bun run build`
Expected: Exit code `0` across every command.

- [x] **Step 2: Record `ADR-040` in `docs/superpowers/DECISIONS.md`, mark plan complete, and push signed commits**

Append concise `ADR-040` (`OMKF Agentic File Manager, Zero-SQLite-Tracks UI & 0-Worker Deck Isolation`) to `docs/superpowers/DECISIONS.md` (zero OS-specific absolute paths), mark Task 4 `[x]`, verify `git status` leaves untracked skill folders untouched, commit signed (`git commit -S`), and push to `origin/main`.

- [x] **Step 3: Compile `hera-desktop.exe --release` on `beast` and deploy/launch on `qa` via `hc`**

1. Build `cargo build --release -p hera-desktop` on `beast` (`target/release/hera-desktop.exe` embeds `hera-desktop/dist` via `tauri::generate_context!()`).
2. Remove stale `qa/t1q` session (`hc rm qa/t1q`), stop any lingering `hera-desktop` process on `qa`, `git pull --ff-only` on `qa`, transfer `target/release/hera-desktop.exe` to `qa`, launch the native `--release` binary in an interactive `qa` session via `hc`, and verify startup logs & live process health.
