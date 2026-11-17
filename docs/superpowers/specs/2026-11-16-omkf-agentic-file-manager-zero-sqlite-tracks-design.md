# OMKF Agentic File Manager & Zero-SQLite-Tracks UI Architecture — Design Spec

**Date:** 2026-11-16  
**Repository:** `davidcastilloc/hera` (`hera-rs` + `hera-ipc` + `hera-desktop`)  
**Status:** Ready for Approval (Builder-Checked)

## 1. Intent & Why

Live QA on a real 14,976-track DJ collection revealed that retaining Museeks' legacy SQLite `tracks` table as the UI data source defeats HERA's OMKF (`Open Music Knowledge Format`) architecture:
1. **Operational SQLite Rot & Duplicates:** Importing overlapping folders (e.g. root `Music/` alongside `_REPERTORIO_DJ/`, `BACKUP/`, `_USB_PIONEER/`) or moving/copying files across crates fills SQLite `tracks` with repeated songs, stale ghost rows, `SQLITE_BUSY` (`database is locked`) errors, and 7-second `getAllTracks()` JSON dumps.
2. **Blocked Route Transitions & React Churn:** Route `beforeLoad`/`loader` hooks blocked navigation for `3.7s–4.1s` awaiting SQLite IPC, while `200 ms` scanner broadcasts cloned and re-sorted `14,976` `Track` objects on the main UI thread.
3. **Playback Stalls at Deep Analysis (`40s`) & Debug Binary Penalty:** Running `target/debug/hera-desktop.exe` on `qa` made DSP 20x–40x slower; at `40s`, `commit_gate.rs` ran `compute_cues_seconds` (`rustfft`) + 128-tap `rubato` sinc while `scanner_worker` simultaneously ran `P0Immediate` on the playing file, exhausting the `10s` buffer runway.

By making **the physical filesystem + `.hera/` (`tracks.omkf` + `crate.md`) the sole source of truth for browsing and playback**, OS filesystem semantics guarantee zero duplicate files per folder, zero ghost rows, `< 1 ms` folder hydration from `.hera/tracks.omkf`, and `< 10 ms` route switching.

## 2. Goals, Non-Goals & Anti-Goals

### Goals
- **Zero `getAllTracks()` / Zero SQLite `tracks` Reads in UI:** `/library` never loads a monolithic `tracks` table into JS memory.
- **Hierarchical Agentic File Manager (`/library` = `Carpetas`):** Left tree + breadcrumbs navigate configured `Config.library_folders` hierarchically (`Root -> Subfolder -> Leaf Crate`). Selecting a folder lists its immediate child folders (clickable cards) and its physical audio/video files hydrated directly from `std::fs::read_dir` + `.hera/tracks.omkf`, with `CrateRulesBanner` (`.hera/crate.md` rules, dry-run/apply organize, reversible ledger undo) mounted at the top of every folder.
- **In-Memory `OmkfFsCatalog` (`< 15 ms` Boot & Deduplicated Global Search):** Rust indexes configured `library_folders` in RAM from `.hera/tracks.omkf` capsules + directory entries; typing in the global search bar queries `OmkfFsCatalog` in `< 2 ms`, deduplicating identical tracks across folders/backups (`partial_hash_64kb` or normalized `(artist, title, duration_sec)`) and surfacing `copy_count` (`×N carpetas`).
- **Retire `/artists` from Navigation & Eliminate Blocking Route Loaders:** Remove `/artists` from `Navigation.tsx`; remove blocking IPC `await`s from TanStack Router `beforeLoad`/`loader` hooks so route changes complete in `< 10 ms`.
- **100% Isolated Audio Playback & `beast -> qa` `--release` Binary:** Remove `compute_cues_seconds` (`rustfft`) and heavy 128-tap `rubato` sinc from `commit_gate.rs` (use fast linear resampling + `60.0s` audition window), throttle `AutoGigGovernor` to **`0` background DSP workers** (`effective_workers() == 0`) whenever `deck_playing == true`, and deploy only `--release` `hera-desktop.exe` built on `beast` to `qa`.

### Non-Goals
- Deleting SQLite entirely: `hera.db` remains for `/agent` sessions (`agent_sessions`, `agent_messages`), curatorial learning (`dj_golden_pairs`, `dj_excluded_tracks`, `genre_cue_templates`), and legacy `.m3u` user playlists if explicitly opened in `/playlists`.
- Cloud sync schema changes (`hera-cloud` v0.7.0 endpoints remain unchanged).

### Anti-Goals (Forbidden)
- Loading all tracks from all folders into a single flat React array on boot.
- Running `cargo run` / `tauri dev` (`debug` profile) on `qa`.
- Running `rustfft` cue detection or background scanner DSP threads while `DeckA`/`DeckB` is actively playing audio.

## 3. Constraints

- Every sliced Rust file `<= 349` lines; frontend files within `architectural_ratchets.rs` ceilings (`omkf-bridge.ts <= 300`, `player.ts <= 680`, `native-audio-transport.ts <= 320`).
- Preserve `EXPECTED_TAURI_COMMANDS_COUNT = 77` (extend existing `omkf_list_folder_crates` and `omkf_read_folder_crate` payloads via `hera-ipc` DTOs rather than proliferating redundant IPC commands, or update ACL triad atomically if any command is added/removed).
- Dark Booth Studio visual identity (`#0c0e12` ground, `#10141b` rail, `#12161d` header, `#14b8a6` accent, crisp SVG icons, Spanish copy, zero emojis, zero layout shift).
- Zero OS-specific absolute paths in committed source, specs, plans, or `DECISIONS.md`.

## 4. Approaches Considered

1. **Option A (Chosen — Pure Filesystem + `.hera/tracks.omkf` In-Memory Catalog, Retire `/artists`):**
   - *How it works:* `OmkfFsCatalog` in Rust walks `Config.library_folders` (skipping `__MACOSX`, `._*`, `$RECYCLE.BIN`, `.hera`), reads `.hera/tracks.omkf` headers/slots into compact RAM structs, serves hierarchical folder trees via `omkf_list_folder_crates`, hydrates a single folder's live files + `.hera/tracks.omkf` slots via `omkf_read_folder_crate(crate_id)` (returning `tracks: Vec<OmkfFsTrackDto>` + `subfolders: Vec<FolderCrateSummary>`), and serves deduplicated global search via `omkf_read_folder_crate("__search__:<query>")`.
   - *Trade-off:* Retires `/artists` album-grid view in favor of DJ folder hierarchy + instant search. Eliminates 100% of SQLite `tracks` bloat, lock contention, ghost rows, and duplicate clutter.
2. **Option B (Rejected — Hybrid Folder Browser + SQLite `tracks` Sync):**
   - *Trade-off:* Keeps `/artists` backed by SQLite `tracks`, but retains `SQLITE_BUSY` write contention, startup/sync drift, and duplicate rows in search.
3. **Option C (Rejected — Deduplicate SQLite `tracks` via SQL `GROUP BY` Triggers):**
   - *Trade-off:* Still dumps thousands of rows over IPC on boot and hides valid per-crate DJ edits when paths collide.

## 5. Architecture & Data Flow Decided

### 5.1 Rust `OmkfFsCatalog` & IPC DTOs (`hera-ipc` + `src-tauri/src/libs/omkf_fs_catalog.rs`)
- New DTO module `hera-desktop/src-ipc/src/omkf_fs_types.rs` exported through `hera-ipc`:
  - `OmkfFsTrackDto`: `{ id, path, file_name, title, artist, album, duration_sec, bpm, camelot, energy, hot_cues: Vec<HotCuePin>, scan_level, is_video, is_fake_320kbps, has_audio_skips, is_variable_bpm, has_intro_vocals, has_outro_vocals, replay_gain_db, waveform_peaks_b64, copy_count: usize }` (mapped 1:1 to frontend `Track` in `omkf-bridge.ts`).
  - Extend `FolderCrateSummary` (`#[serde(default)]`): `parent_crate_id: Option<String>`, `rel_path: String`, `depth: usize`, `subfolder_count: usize`.
  - Extend `FolderCrateDetail` (`#[serde(default)]`): `subfolders: Vec<FolderCrateSummary>`, `tracks: Vec<OmkfFsTrackDto>`.
- `OmkfFsCatalog` (`RwLock` managed state in `hera-dj`):
  - Builds folder tree from `Config.library_folders` (`< 15 ms` reading directory entries + `.hera/tracks.omkf` `64 B` headers).
  - `read_folder(crate_id)`: reads live `std::fs::read_dir(folder_path)` filtered by supported audio/video extensions and `!is_junk_audio_path`, joins by filename against `.hera/tracks.omkf` slots (`O(N)`), parses `.hera/crate.md`, and returns `FolderCrateDetail`. Any unanalyzed file (`scan_level == 0` not yet in `.hera/tracks.omkf`) is returned immediately with filename-parsed title/BPM/key hints (`e.g. [96 BPM - 12B]`) and queued for background OMKF capsule analysis when the deck is idle!
  - `search_deduplicated(query, limit = 200)`: searches in-memory folder entries matching `query` (case/accent-insensitive across filename, artist, title, folder name), groups identical tracks by `partial_hash_64kb` (when non-zero) or canonical `(artist_slug, title_slug, round(duration))`, preferring `_REPERTORIO_DJ` / active crates over `BACKUP` / `_ARCHIVO_*`, and sets `copy_count`.

### 5.2 Background OMKF Capsule Scanner & Zero-Contention Audio Engine
- **Deck-Playing Hard Pause (`hera-rs/src/scanner/scheduler.rs`):** When `governor.deck_playing == true`, `effective_workers()` returns **`0`** (down from `1`), and `scanner_worker` skips disk/capsule flushes while audio is playing so 100% of disk + CPU belongs to the DJ's deck.
- **Audio Engine Stage-3 Fast Upgrade (`hera-rs/src/audio_engine/commit_gate.rs` & `prefix_loader.rs`):**
  - Raise `AUDITION_WINDOW_SECS` from `50.0` to `65.0` (`25.0s` of safety runway past the `40.0s` commit threshold!).
  - In Stage 3 (`>= 40.0s` played or out-of-window seek): remove `compute_cues_seconds` (`rustfft` NEVER runs on the audio loader thread; preserve existing known/proportional cues) and use `resample_deck_buffer_fast_prefix` (linear stereo resampling, `< 8 ms` for a 5-min track in release, `< 60 ms` in debug) so full-track buffer swap completes instantaneously with **zero** audible pause or `is_buffering` stall at `40s`–`50s`.
  - Defer scanner queue promotion for an unanalyzed track until playback pauses/stops or switches away, writing results directly to that folder's `.hera/tracks.omkf` capsule.

### 5.3 Hierarchical File Manager UX (`/library` = `Carpetas`) & Instant Routing
- **Left Navigation Rail (`Navigation.tsx`):** `Carpetas` (`/library`), `Agente` (`/agent`), `Playlists` (`/playlists` for `.m3u` / saved sets), `Ajustes` (`/settings`). Remove `/artists`.
- **Zero Blocking Route Loaders:** Remove `await queryClient.prefetchQuery(allTracksQuery)` and `await DatabaseBridge.getAllPlaylists()` from `library.tsx` and `playlists.tsx` `beforeLoad`/`loader`; fetch via cached `useQuery` (`staleTime: 30_000`) inside components so route switches render in `< 10 ms`.
- **Explorer Layout in `/library`:**
  ```
  ┌────────────────────┬─────────────────────────────────────────────────────────┐
  │ Raíces y Carpetas  │ Breadcrumb: Music ▸ _REPERTORIO_DJ ▸ 01_Afro_House      │
  │ ▾ _REPERTORIO_DJ   ├─────────────────────────────────────────────────────────┤
  │   ▸ 00_CRATES...   │ [OMKF .hera] 120–124 BPM · 4B · Reglas · Organizar IA   │
  │   ● 01_Afro_House  ├─────────────────────────────────────────────────────────┤
  │   ▸ 03_Reggaeton   │ [Subcarpetas (si tiene): 📁 Edits  📁 Instrumentales ]  │
  │ ▸ _USB_PIONEER     ├─────────────────────────────────────────────────────────┤
  │                    │ # │ Título            │ BPM   │ Key │ 4 Hot Cues │ Dur  │
  │                    │ ▶ │ Move              │ 120.0 │ 4B  │ ● ● ● ●    │ 5:52 │
  └────────────────────┴─────────────────────────────────────────────────────────┘
  ```
  - By default on boot, selects the first workable DJ root/folder (preferring non-`BACKUP` folders) instead of dumping 14,976 tracks.
  - Typing in the top `Search` box switches the main pane to `Resultados en todas las carpetas (deduplicados)` (`<= 200` results, `< 2 ms`, with `×N carpetas` badge when copies exist in `BACKUP` or other sets).

### 5.4 Release Build Pipeline (`beast` -> `qa`)
- Compile frontend (`bun run build`) + Rust release binary (`cargo build --release -p hera-desktop`, embedding `hera-desktop/dist`) on `beast`.
- Kill `tauri dev` (`qa/t1q`) on `qa`, transfer the standalone release `hera-desktop.exe` to `qa`, and launch it natively via `hc` so `qa` runs 100% release machine code (`AVX2`, zero Vite dev server overhead).

## 6. Left to the Builder
- Exact tree indentation / expand-collapse chevron SVGs in `FolderCrateTree.tsx` (within Dark Booth Studio tokens).
- Internal helper module split under `src-tauri/src/libs/` to keep every `.rs` file `<= 349` lines.

## 7. Builder-Check Settlements (Cross-Cutting Integration Decisions)

1. **Full Backend + Tool Resolution via `OmkfFsCatalog` (Builder Call):**
   - Remove `ensureQueryData(allTracksQuery)` from `src/routes/__root.tsx` and `LibraryAPI.ts`.
   - `omkf_save_crate_rules` and `omkf_preview_or_apply_organize` operate directly on the folder's live filesystem + `.hera/crate.md` + `.hera/tracks.omkf` + `OmkfFsCatalog` in RAM without requiring pre-existing SQLite `tracks` or `folder_crates` rows.
   - `scanner_worker` scans unanalyzed files discovered by `OmkfFsCatalog` (when `deck_playing == false`) and writes directly into `<folder>/.hera/tracks.omkf` + updates `OmkfFsCatalog` in RAM, emitting throttled (`>= 2.0s`) `hera:omkf-folder-updated` events scoped to the modified `crate_id` instead of cloning a 14,976-row array in JS every `200 ms`.
   - `db::get_tracks(ids)` resolves track IDs from `OmkfFsCatalog` first (falling back to SQLite for legacy `.m3u` IDs) so `.m3u` playlists and `/agent` tools keep working seamlessly.
2. **Cross-Library Surfaces (`Modo Mini`, `/agent` `@` Mentions, `Transición`, `Mi Estilo` — Builder Call):**
   - `useLibraryStore.tracks` holds the active folder's tracks (`30–200` items).
   - `omkf_read_folder_crate` supports two fast in-memory sentinel queries on `OmkfFsCatalog` (`< 2 ms`):
     - `"__search__:<query>"` -> up to `200` deduplicated matches across all folders (used by global `Search` and `/agent` `@` mentions).
     - `"__harmonic__:<bpm>:<camelot>"` -> up to `60` compatible non-`BACKUP` tracks across all folders (used by `Modo Mini` over VirtualDJ and `Transición`).
3. **Folder Context Menu Safety (`TrackList.tsx` — Builder Call):**
   - Retire Museeks' SQLite-only `Remove from library` and `/tracks/$trackID` SQLite edit route from live folder views; keep safe non-destructive actions (`Añadir a cola`, `Añadir a playlist`, `Excluir de curación` via `dj_excluded_tracks`) honoring the Sacred Disk Invariant (never deleting DJ master files from disk).
4. **Clean `/playlists` Separation (Builder Call):**
   - Because `/library` is now `Carpetas` (the hierarchical `.hera` Agentic File Manager), `/playlists` drops its duplicate `Carpetas (.hera)` tab and `folder--*` auto-redirect, focusing strictly on saved `.m3u` / custom playlists (updating `hera-studio-qa.test-e2e.tsx` accordingly).
