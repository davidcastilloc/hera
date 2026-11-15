# Plan: Scanner v3.1 Ultra-Fast Progressive Indexing & Real-Time Library UX

**Goal:** Eliminate the blocking full-screen spinner and sequential 15ms-sleep bottlenecks in `hera-desktop` so newly scanned audio and Video-DJ files stream into the `/library` table within `< 200 ms`, transition visually from `INDEXED` (`···`) -> `FAST` (BPM + Camelot) -> `FULL` (4 Hot Cues + Waveform Peaks) every `200 ms` via a background Rust worker daemon, and give clear live feedback across the toolbar and Settings.

**Execution Strategy:** Native Swarm (`/swarm`) using fast/cheap worker models with strictly disjoint file ownership across 3 parallel author workers (`indexer-fast`, `dsp-daemon`, `realtime-ux`) + 1 QA/release verifier (`qa-verifier`) + `monitor`.

---

## Shared Contracts

- **IPC Events:**
  - `hera:scanner-tracks-added` (`Vec<Track>`, `waveform_peaks_blob: None`): first micro-batch at `60` tracks, then batches of `250` tracks.
  - `hera:scanner-tracks-removed` (`Vec<String>`): removes short SFX (`duration > 0 && duration < 25s`) discovered during Level 1 audio decode.
  - `hera:scanner-rows-updated` (`Vec<ScannerRowPatch>`): emitted every `200 ms` by `scanner_worker.rs` with `bpm`, `camelot`, `energy`, `duration`, `hot_cues`, `scan_level`, `is_transition_candidate`, `is_fake_320kbps`.
  - `hera:scanner-telemetry` (`ScannerTelemetry`): `phase`, `indexedDone`, `fastDone`, `fullDone`, `total`, `tracksPerSec`, `etaSec`, `governorActive`.
- **SQLite `NOT NULL` Sentinel (`04_add_hera_dj_columns.sql`):** Store `bpm = 0.0` and `camelot = ""` for unanalyzed `scan_level = 0` tracks; normalize `bpm <= 0.0 -> None` and `camelot == "" -> None` on read in `DB`.
- **Ratchets:** Keep all Rust files in `hera-rs/src/{bpm_engine, scanner}` and `hera-desktop/src-tauri/src/plugins/hera_dj` at `<= 349` lines; keep `Tauri` command count at `63`; dynamic `sqlx::query` only; zero OS-specific paths in `docs/superpowers/`.

---

## Checkpoints

- [ ] **Repo Hygiene & Level 0 Streaming Indexer (`indexer-fast`):**
  - Untrack `hera-node.exe` (`git rm --cached hera-node.exe`) and ignore `/hera-node.exe` in `.gitignore`.
  - Fix `lofty` compilation issues in `hera-desktop/src-tauri/src/libs/track.rs` and skip `lofty` probing on `.webm`/`.mkv`.
  - Upgrade `hera-desktop/src-tauri/src/plugins/hera_dj/workability.rs` (`<= 349` lines) to carry VirtualDJ `author`, `title`, `duration_sec`, `bitrate_kbps`, and `<Poi>` cues with `normalize_lookup_path`.
  - Fix `hera-desktop/src-tauri/src/libs/database.rs`: keep `insert_tracks` strict, add batched `upsert_scanned_tracks_batch`, exclude `waveform_peaks_blob` (`NULL AS waveform_peaks_blob`) in `get_all_tracks`, and normalize `0.0` / `""` sentinels to `None`.
  - Implement `hera-desktop/src-tauri/src/libs/fast_library_scan.rs` (`WalkDir` streaming filter + `rayon::par_iter` Level 0 + VirtualDJ cache hydration + `60`/`250` progressive SQLite WAL upserts + `hera:scanner-tracks-added` / `hera:scanner-telemetry`) wired into `hera-desktop/src-tauri/src/plugins/db.rs`.
  - Fix `Track { ... }` struct literals in `hera-desktop/src-tauri/src/libs/tests/database_tests.rs` and `hera-desktop/src-tauri/src/plugins/hera_agent/tests.rs`.
- [ ] **Background DSP Daemon & `Condvar` Scheduler (`dsp-daemon`):**
  - Clean unused import warnings in `hera-rs/src/bpm_engine/v2_core/queue.rs`.
  - Upgrade `hera-rs/src/scanner/scheduler.rs` (`<= 349` lines) with `Condvar` wakeup (`wait_next`, `enqueue_batch`, `clear_background_queues`).
  - Extend `hera-desktop/src-tauri/src/libs/db_scanner_queries.rs` with transactional `apply_scanner_batch_updates`, `duration_sec`, `load_pending_full_pass_jobs`, `count_scanner_levels`, and `delete_short_sfx_tracks`.
  - Implement `hera-desktop/src-tauri/src/plugins/hera_dj/scanner_worker.rs` (`<= 349` lines) spawned from `hera-desktop/src-tauri/src/plugins/hera_dj/mod.rs` with `AutoGigGovernor`, `P0`/`P1`/`P2`/`P3` workers, and `200 ms` WAL commits + `hera:scanner-rows-updated` / `hera:scanner-telemetry`.
- [ ] **Real-Time Progressive UX/UI (`realtime-ux`):**
  - Unblock `hera-desktop/src/components/TrackListStates.tsx` whenever `tracks.length > 0` and include video extensions in single-file import filter.
  - Extend `hera-desktop/src/lib/scanner-v3.ts` with `recentlyUpdatedIds`, `phase`, `indexedDone`, `hot_cues`, `duration`.
  - Handle `hera:scanner-tracks-added`, `hera:scanner-tracks-removed`, and `hera:scanner-rows-updated` (with Sort-Freeze + `useLibraryStore` sync) in `hera-desktop/src/components/LibraryEvents.tsx` and `hera-desktop/src/api/LibraryAPI.ts`.
  - Connect visible virtual rows to `scheduleViewportBoost` and table hover to `setPointerInsideTable` in `hera-desktop/src/components/TrackListDefault.tsx`.
  - Remove fake `'8A'` fallback in `hera-desktop/src/components/TrackRow.tsx`, show `···` pending states + `--:--` duration + animated Hot Cue dot illumination + `#14b8a6` fresh-update highlight.
  - Update `hera-desktop/src/components/ScannerStatusPill.tsx`, trigger immediate scan after `Importar` in `hera-desktop/src/components/HeraDjToolbar.tsx`, and add live scan banner + library link in `hera-desktop/src/routes/settings.library.tsx`.
  - Add unit tests in `hera-desktop/src/components/__tests__/scanner-realtime-ux.test.tsx`.
- [ ] **Integration, Full QA & Native `.exe` Release Launch (`qa-verifier`):**
  - Merge all worker commits to `main`, record decision in `docs/superpowers/DECISIONS.md`, regenerate TS typings if needed.
  - Run `cargo test -p hera-node-rs` (including `architectural_ratchets` and `golden_masters`), `cargo test -p hera-ipc`, `cargo test -p hera-desktop`, WASM check, `bun run test`, `bun run build`.
  - Stop any locked `hera-desktop.exe`, run `bun run tauri build`, and launch `hera-desktop.exe` live.
