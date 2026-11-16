# AI Curation Intelligence (`Genre × Era`), Zero-IO Acoustic Harvest & 50k Booth Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver coherent `Genre × Era` AI playlist curation, single-pass Zero-IO acoustic harvesting (`LUFS`, `768 B` RGB waveforms, `64D` Sonic Twins, `8 Hot Cues` DJ learning, CD-skip glitches, Shazam ID), `< 1 ms` 50k SQLite scaling, live `VirtualDJ/History` + Ableton watching, Pre-Gig hardware auditing, and a functional `BETA` Onboarding Wizard without altering user files on disk.

**Architecture:** Additive SQLite migration `08` (preserving committed migration `07`) extends `ADR-035` Scanner v3.1 and `1078b61` `libs/tray.rs` with single-pass PCM feature extraction (`audio_metrics.rs`, `cue_engine.rs`, `shazam_id/`), a 14-family `genre_intelligence/` classifier wired into `optimize_crate_with_dj_rules` and `/agent` MCP tools, and non-blocking `<= 349` LOC Rust / StyleX React 19 modules.

**Tech Stack:** Rust (`hera-node-rs`, `hera-ipc`, `tauri 2.12`, `sqlx 0.8` dynamic `QueryBuilder`, `symphonia`, `rustfft`, `lofty`, `indexmap`), TypeScript, React 19 Compiler, `@stylexjs/stylex`, Vitest, `moon`.

**Spec:** `docs/superpowers/specs/2026-11-15-curation-intelligence-and-zero-io-booth-design.md`

## Global Constraints

- **Sacred Disk Invariant (`Don't break the user space`):** Zero physical file moves, renames, or deletions when routing `> 600s` Mixes, excluding tracks, flagging CD skips, or identifying tracks via Shazam; always write `.bak` backups before modifying external DJ XML files.
- **Immutable Migration `07`:** NEVER edit `hera-desktop/src-tauri/src/migrations/07_add_scanner_v3_columns.sql`; put all schema additions in `08_add_zero_io_and_curation_intelligence.sql` and use `sqlx::query` / `sqlx::QueryBuilder` (never `sqlx::query!`).
- **Zero New Dependencies & Zero External Routers:** Use only crates/packages already present in `Cargo.lock` and `bun.lock`; never integrate external LLM proxy routers (`9router`).
- **Architectural Ratchets (`hera-rs/tests/architectural_ratchets.rs`):** Every `.rs` file in sliced directories (`bpm_engine`, `scanner`, `genre_intelligence`, `shazam_id`, `mcp`, `code_mode`, `dj_brain`, `p2p`, `audio_engine`, `hera-ipc/src`, `plugins/hera_dj`, `plugins/hera_agent`) `<= 349` lines post-`cargo fmt`; `hera-desktop/src/lib/player.ts <= 680` lines; `TrackProgress.tsx <= 220` lines; `bridge-hera.ts <= 300` lines (`<= 3` manual exported types); `0` diffs in `hera-rs/tests/golden_masters.rs`; `0` OS user-home/drive paths in `docs/superpowers/` and `hera-rs/tests/`.

## Review Focus

- **Corrupt / zero-byte media files during background scan:** Must catch panics/timeouts, mark `scan_level = -1` (`CORRUPT_FILE`), and never re-enqueue on subsequent launches (`Task 1`).
- **Salsa (`82–108 BPM`) vs Perreo (`88–102 BPM`) and 90s Euro-Merengue vs 2026 Merenguetón:** Must never mix accidentally in standard playlists despite identical `(BPM, Camelot)` coordinates (`Task 4`).
- **VirtualDJ tracks with manual `Cue 5` and `Cue 8`:** Must preserve all manual cues (`1..8`) verbatim without `.take(4)` truncation and learn per-genre second-half (`>= 50%`) ratios (`Task 2`, `Task 3`).
- **Videoclips with silent black intros (`< -42 dBFS`) and loudness jumps (`-16 LUFS` vs `-7 LUFS`):** Must anchor `Cue 1` at/after `first_audible_sec` and clamp `replay_gain_db` so `gain_lin * peak_lin <= 0.966` (`-0.3 dBFS`, zero clipping) (`Task 1`, `Task 2`).
- **50,000-track library scroll & AI agent turns:** Cover art cache must never exceed `256` entries (`Task 1`) and `/agent` track search/playlist creation must execute indexed SQLite queries in `< 5 ms` without `db.get_all_tracks()` (`Task 3`).

---

### Task 1: 50k Cover LRU, `< 80 ms` Cue/Auto-Gain Deck Load & Kernel-Prioritized Quarantine Scheduler

**Files:**
- Modify: `hera-desktop/src-tauri/src/plugins/cover.rs`
- Create: `hera-rs/src/audio_engine/decoder_ffmpeg.rs`
- Modify: `hera-rs/src/audio_engine/mod.rs`
- Modify: `hera-rs/src/audio_engine/decoder.rs`
- Modify: `hera-rs/src/audio_engine/backend.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/audio_transport.rs`
- Modify: `hera-rs/src/scanner/scheduler.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/scanner_worker.rs`

**Interfaces:**
- Produces:
  - `AudioBackend::load_file_with_cues(&self, deck: DeckId, path: &Path, known_cues_sec: Option<[f64; 4]>) -> Result<AudioTransportSnapshot, String>`
  - `native_audio_load_track(..., hot_cues_sec: Option<Vec<f64>>, replay_gain_db: Option<f32>) -> Result<NativePlayerSnapshot, String>`
  - `ScanScheduler::{seed_completed_levels, mark_completed, cancel_and_drain_background}` and `AutoGigGovernor::{set_external_booth_mode, set_on_battery_power, is_throttled_booth_mode}`

- [x] **Step 1: Write failing unit tests for bounded cover LRU (`256`), `load_file_with_cues` + `SetGain`, and `completed_level` (`-1`, `1 -> 2`) scheduler state**
  - Add `test_cover_lru_evicts_past_256_and_handles_root_path_without_panic` in `plugins/cover.rs`.
  - Add `test_load_file_with_known_cues_skips_recompute_and_applies_gain` in `hera-rs/src/audio_engine/`.
  - Add `test_fast_completed_repromotes_to_full_and_skips_quarantined_minus_one` + `test_external_booth_and_battery_governor_freezes_p3` in `hera-rs/src/scanner/scheduler.rs`.

- [x] **Step 2: Run tests to verify they fail**
  - Run: `cargo test -p hera-node-rs scanner::scheduler && cargo test -p hera-node-rs audio_engine::`
  - Expected: FAIL on missing methods/assertions.

- [x] **Step 3: Implement bounded cover LRU, safe stereo `decoder_ffmpeg.rs` (`#![forbid(unsafe_code)]`), `hot_cues_sec` + `SetGain` wiring, and `BELOW_NORMAL` / `-1` quarantine worker hardening**
  - Replace `#[memoize::memoize]` in `cover.rs` with `OnceLock<Mutex<IndexMap<String, Option<String>>>>` capped at `256`, skip `.webm`/`.mkv` in `lofty`, remove all `.unwrap()`, and run inside `spawn_blocking`.
  - Wire `known_cues_sec` and `TransportCommand::SetGain(DeckId::DeckA, 10f32.powf(db.clamp(-12.0, 6.0) / 20.0))` in `audio_transport.rs`.
  - Add `THREAD_PRIORITY_BELOW_NORMAL` / `nice(10)` (`unsafe extern` for Rust 2024 in `src-tauri`), `catch_unwind` -> `scan_level = -1`, and `< 100 ms` queue drain on cancel in `scanner_worker.rs`.

- [x] **Step 4: Run verification receipts**
  - Run: `cargo test -p hera-node-rs --lib && cargo test -p hera-desktop cover && cargo test -p hera-node-rs --test architectural_ratchets`
  - Expected: PASS (`0` errors, all modules `<= 349` LOC).

- [x] **Step 5: Mark `[x]` and commit atomically**
  - Commit: `feat(engine): bound cover LRU, wire instant cue+LUFS deck load, and harden scanner governor`

---

### Task 2: Zero-IO Single-Pass Acoustic Harvest (`audio_metrics.rs`), 8-Cue Engine, Salsa/Tecnomerengue BPM Audit & CD-Skip Glitches

**Files:**
- Create: `hera-rs/src/scanner/audio_metrics.rs`
- Create: `hera-rs/src/cue_engine.rs`
- Modify: `hera-rs/src/lib.rs`
- Modify: `hera-rs/src/scanner/mod.rs`
- Modify: `hera-rs/src/scanner/fast_seek.rs`
- Modify: `hera-rs/src/scanner/full_pass.rs`
- Modify: `hera-rs/src/bpm_engine/latin_heuristics.rs`
- Modify: `hera-rs/src/mcp/analyze.rs`

**Interfaces:**
- Produces:
  - `build_rgb_waveform_blob(samples: &[f32], sample_rate: u32, cols: usize) -> Vec<u8>` (`768` B)
  - `compute_loudness_and_lead_in(samples: &[f32], sample_rate: u32) -> AudioLoudnessMetrics`
  - `quantize_embedding_64d(vec64: &[f32]) -> [i8; 64]`, `cosine_similarity_i8(a: &[i8; 64], b: &[i8; 64]) -> f32`, `check_stereo_sub_mono_phase(interleaved_lr: &[f32], sample_rate: u32) -> f32`
  - `merge_and_place_8_hot_cues(...) -> Vec<FullHotCue>` (`CueOrigin::DjManual` preserved, `Cue 1 >= first_audible_sec`, `Pads 5..8 >= 50%` anti-empty-zone)
  - Extended `FullPassResult` with `entry_bpm`, `exit_bpm`, `has_intro_vocals`, `has_outro_vocals`, `embedding_64d_q8`, `rgb_waveform_blob`, `loudness`, and `glitch_report: GlitchScanReport`

- [x] **Step 1: Write failing unit tests for `768 B` RGB waveform, `-11 LUFS` anti-clip gain + lead-in silence trim, 8-cue `>= 50%` anti-empty-zone merger, Salsa `4/3x` tag cross-audit, and `full_pass` CD-skip detection**
  - Pin exact assertions: `rgb_blob.len() == 768`, `gain_lin * peak_lin <= 0.967`, `cues[0].seconds >= 2.0` on a `2.0s` silent video intro, manual `Cue 5` & `Cue 8` preserved verbatim, and synthetic `60ms` buffer stutter / `80ms` dropout flagged in `FullPassResult.glitch_report.has_audio_skips`.

- [x] **Step 2: Run tests to verify they fail**
  - Run: `cargo test -p hera-node-rs scanner::`
  - Expected: FAIL on missing modules/fields.

- [x] **Step 3: Implement `audio_metrics.rs`, `cue_engine.rs`, `fast_seek.rs` (`entry_bpm`/`exit_bpm` without 3rd STFT), `full_pass.rs` (`scan_pcm_for_cd_skips` + Zero-IO fields), and parallel streaming `mcp/analyze.rs`**
  - Keep `structure::analyze_structure_and_style` untouched so `golden_masters.rs` stays bit-identical.

- [x] **Step 4: Run verification receipts**
  - Run: `cargo test -p hera-node-rs && cargo test -p hera-node-rs --test golden_masters && cargo test -p hera-node-rs --test architectural_ratchets`
  - Expected: PASS (`0` snapshot diffs, all `.rs <= 349` LOC).

- [x] **Step 5: Mark `[x]` and commit atomically**
  - Commit: `feat(dsp): add zero-io RGB/LUFS/64D/glitch harvest, 8-cue engine, and Salsa BPM cross-audit`

---

### Task 3: Additive SQLite Migration `08`, 8-Cue VDJ Import, DB-First Workable Discovery, `< 1 ms` Agent SQL & Shazam Constellation Engine

**Files:**
- Create: `hera-desktop/src-tauri/src/migrations/08_add_zero_io_and_curation_intelligence.sql`
- Create: `hera-rs/src/shazam_id/mod.rs`
- Create: `hera-rs/src/shazam_id/constellation.rs`
- Create: `hera-rs/src/shazam_id/matcher.rs`
- Modify: `hera-rs/tests/architectural_ratchets.rs` (add `shazam_id` to `SLICED_SUBMODULE_DIRS`)
- Modify: `hera-desktop/src-tauri/src/plugins/db.rs` (`PRAGMA mmap_size = 268435456`)
- Modify: `hera-desktop/src-tauri/src/libs/track.rs`
- Modify: `hera-desktop/src-tauri/src/libs/fast_library_scan.rs` (preserve `1..8` VDJ cues)
- Modify: `hera-desktop/src-tauri/src/libs/db_scanner_queries.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/workability.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_agent/tools_library.rs`

**Interfaces:**
- Consumes: `FullPassResult`, `AudioLoudnessMetrics`, `GlitchScanReport` from `Task 2`.
- Produces:
  - `extract_constellation_landmarks(samples: &[f32], sr: u32) -> Vec<LandmarkHash>` & `match_landmarks_against_index(...) -> Option<ShazamMatch>`
  - `discover_workable_roots_from_existing_dbs() -> Vec<DiscoveredMusicRoot>`
  - `query_agent_tracks_filtered(...) -> AnyResult<Vec<Track>>` (`< 1 ms` indexed SQLite query)

- [x] **Step 1: Write failing tests for Migration `08` + `< 5 ms` filtered SQL query, 8-cue VDJ import preservation, DB-First root extraction, and Shazam landmark identification**
  - Test that an untagged/UUID copy of an indexed audio signal matches the canonical track via `shazam_id::matcher` even with a `2.5s` offset, and that `query_agent_tracks_filtered` excludes `scan_level < 0`, `has_audio_skips = 1`, and `duration > 600`.

- [x] **Step 2: Run tests to verify they fail**
  - Run: `cargo test -p hera-node-rs shazam_id && cargo test -p hera-desktop`
  - Expected: FAIL.

- [x] **Step 3: Implement `08_add_zero_io_and_curation_intelligence.sql` (leaving `07` untouched), concurrent WAL `SqlitePool` read path (`ADR-036` handoff), `shazam_id/`, 8-cue preservation in `fast_library_scan.rs`, `discover_workable_roots_from_existing_dbs`, and `query_agent_tracks_filtered` in `tools_library.rs`**
  - Split read-only frontend/agent queries onto a WAL reader `SqlitePool` in `plugins/db.rs` so Level 0 batch upserts never stall interactive reads (`getArtistTracks`, `getAllPlaylists`, `query_agent_tracks_filtered`).
  - Annotate any new optional `i64` `ts-rs` fields on `Track` with `#[ts(optional, type = "number")]`.

- [x] **Step 4: Run verification receipts**
  - Run: `cd hera-desktop && bun run gen:types && cd .. && cargo test -p hera-node-rs && cargo test -p hera-desktop && cargo test -p hera-node-rs --test architectural_ratchets`
  - Expected: PASS (`0` errors).

- [x] **Step 5: Mark `[x]` and commit atomically**
  - Commit: `feat(db,shazam): add migration 08, 8-cue VDJ preservation, DB-first discovery, fast agent SQL, and Shazam ID`

---

### Task 4: 14-Family `Genre × Era` Classifier, `entry_bpm`/`exit_bpm` Synergy Bridges & DJ Curation Super-Prompt

**Files:**
- Create: `hera-rs/src/genre_intelligence/mod.rs`
- Create: `hera-rs/src/genre_intelligence/taxonomy.rs`
- Create: `hera-rs/src/genre_intelligence/era_detector.rs`
- Create: `hera-rs/src/genre_intelligence/acoustic_classifier.rs`
- Modify: `hera-rs/src/lib.rs`
- Modify: `hera-rs/src/synergy.rs`
- Modify: `hera-rs/src/curation_rules.rs`
- Modify: `hera-rs/src/dj_brain/prompts.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_agent/turn_agy.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_agent/tools_playlist_vdj.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_agent/tools_curation.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_agent/smart_intent.rs`
- Modify: `hera-rs/tests/architectural_ratchets.rs` (add `genre_intelligence` to `SLICED_SUBMODULE_DIRS`)

**Interfaces:**
- Consumes: `query_agent_tracks_filtered` from `Task 3`, `entry_bpm`/`exit_bpm`/`embedding_64d` from `Task 2`.
- Produces:
  - `classify_track_genre_and_era(artist: &str, title: &str, tags_genre: &[String], folder: &str, year_hint: Option<u16>, acoustic: Option<&AcousticRhythmSignature>) -> GenreEraProfile`
  - `compute_genre_era_compatibility(a: &GenreEraProfile, b: &GenreEraProfile, allow_crossover: bool) -> f32`
  - `build_desktop_dj_curator_system_prompt(...) -> String`

- [ ] **Step 1: Write failing tests proving zero Salsa-in-Perreo contamination, zero 90s Euro-Merengue + 2026 Merenguetón clash in standard mode, Golden Pair locking (`17 -> 20`), `< 10m` Mix routing, and `entry_bpm -> exit_bpm` bridge scoring**
  - Construct a mixed candidate pool containing *Tony Vega* (`88 BPM 7A` Salsa Baúl), *Feid / Bad Bunny* (`90 BPM 7A` Reggaetón Actual), *Sandy & Papo* (`126 BPM 8A` 1996), *Alleh & Yorghaki* (`126 BPM 4A` 2025), a `720s` live mix, an excluded *"Clavelito"* track, and a `92 -> 126 BPM` transition edit; assert clean single-family crate selection, `routed_to_mixes_over_10m.len() == 1`, and `excluded_tracks.len() == 1`.

- [ ] **Step 2: Run tests to verify they fail**
  - Run: `cargo test -p hera-node-rs genre_intelligence && cargo test -p hera-desktop hera_agent`
  - Expected: FAIL.

- [ ] **Step 3: Implement `genre_intelligence/`, wire `Genre × Era` + `entry_bpm`/`exit_bpm` into `synergy.rs` / `optimize_crate_with_dj_rules` (preserving `golden_masters.rs` when `genre_family` / `entry_bpm` are `None`), connect `create_playlist` & `smart_intent.rs`, add genre-cue learning tool in `tools_curation.rs`, and inject the DJ Curation Super-Prompt in `turn_agy.rs` & `prompts.rs`**

- [ ] **Step 4: Run verification receipts**
  - Run: `cargo test -p hera-node-rs && cargo test -p hera-node-rs --test golden_masters && cargo test -p hera-desktop && cargo test -p hera-node-rs --test architectural_ratchets`
  - Expected: PASS (`0` snapshot diffs, all `.rs <= 349` LOC).

- [ ] **Step 5: Mark `[x]` and commit atomically**
  - Commit: `feat(curator): add 14-family Genre x Era intelligence, DJ super-prompt, and rules-backed create_playlist`

---

### Task 5: Live `VirtualDJ/History` Bridge (`2.5 s` + Auto-Golden-Pairs), Ableton Watcher (`1.5 s` Club Mono Check), Intelligence IPC & Now-Playing Tray Sync

**Files:**
- Create: `hera-desktop/src-tauri/src/plugins/hera_dj/folder_watcher.rs`
- Create: `hera-desktop/src-tauri/src/plugins/hera_dj/scanner_intelligence.rs`
- Modify: `hera-desktop/src-ipc/src/dj_types.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/scanner_v3.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/virtualdj_live.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/mod.rs`
- Modify: `hera-desktop/src-tauri/src/libs/tray.rs`
- Modify: `hera-desktop/src-tauri/build.rs`
- Modify: `hera-desktop/src-tauri/capabilities/main.json`
- Modify: `hera-rs/tests/architectural_ratchets.rs` (`EXPECTED_TAURI_COMMANDS_COUNT`)

**Interfaces:**
- Consumes: `cosine_similarity_i8`, `check_stereo_sub_mono_phase` (`Task 2`), `shazam_id`, `discover_workable_roots_from_existing_dbs` (`Task 3`).
- Produces:
  - Tauri commands registered across the 4-part ACL Triad: `scanner_promote_crate_full`, `scanner_get_telemetry`, `scanner_get_doctor_issues`, `scanner_cancel_scan`, `scanner_discover_workable_roots`, `scanner_set_external_booth_mode`, `scanner_find_sonic_twins`, `scanner_configure_producer_folders`, `scanner_audit_pre_gig_crate`, `identify_track_shazam`.

- [ ] **Step 1: Write failing tests for `VirtualDJ/History` live transition auto-learning (`45s..420s` -> `dj_golden_pairs`), `PreGigAuditReport` (`denon_cd_mp3` `44.1 kHz` / video rejection), `64D` Sonic Twins ranking, and ACL Triad parity**

- [ ] **Step 2: Run tests to verify they fail**
  - Run: `cargo test -p hera-ipc && cargo test -p hera-desktop`
  - Expected: FAIL.

- [ ] **Step 3: Implement `folder_watcher.rs` (`2.5 s` VDJ History + Battery check + `1,500 ms` Ableton Club Mono watcher), `scanner_intelligence.rs`, Now-Playing tooltip updates on `libs/tray.rs`, and full ACL Triad registration**

- [ ] **Step 4: Run verification receipts**
  - Run: `cd hera-desktop && bun run gen:types && cd .. && cargo test -p hera-ipc && cargo test -p hera-node-rs --test architectural_ratchets && cargo test -p hera-desktop`
  - Expected: PASS (`verify_tauri_v2_acl_triad` green).

- [ ] **Step 5: Mark `[x]` and commit atomically**
  - Commit: `feat(vdj,ableton): add live VDJ history golden-pair learner, Ableton mono watcher, Sonic Twins, and Pre-Gig IPC`

---

### Task 6: Tricolor RGB Waveforms, `Check Pre-Toque` Popover, Vocal Clash / Glitch Badges & 50k DJ Search Tokens

**Files:**
- Modify: `hera-desktop/src/components/WaveformPeaksCanvas.tsx`
- Modify: `hera-desktop/src/components/TrackProgress.tsx` (`<= 220` LOC)
- Modify: `hera-desktop/src/lib/native-audio-transport.ts` (`<= 320` LOC)
- Modify: `hera-desktop/src/lib/scanner-v3.ts`
- Modify: `hera-desktop/src/lib/utils-library.ts`
- Create: `hera-desktop/src/components/PreGigAuditPopover.tsx`
- Modify: `hera-desktop/src/components/ScannerStatusPill.tsx`
- Modify: `hera-desktop/src/components/HeraDjToolbar.tsx`
- Modify: `hera-desktop/src/components/TrackRow.tsx`
- Modify: `hera-desktop/src/components/TrackList.tsx`
- Modify: `hera-desktop/src/components/transition/TransitionBridgeStepsCard.tsx`

**Interfaces:**
- Consumes: Generated `hera-typings.ts` + IPC commands from `Task 5`.
- Produces: `768 B` RGB (`Sub/Mid/Air`) canvas rendering, `replayGainDb` + `hotCuesSec` forwarding, `PreGigAuditPopover`, context-menu *Sonic Twins* + *Shazam ID*, Cancel Scan button, and search tokens (`skip`, `diff`, `propia`, `mix`).

- [ ] **Step 1: Write failing Vitest unit specs in `hera-desktop/src/lib/__tests__/` for `768 B` RGB decoding fallback (`512 B` vs `768 B`), new DJ search tokens (`skip`, `diff`, `propia`, `mix`), and `PreGigAuditPopover` rendering**

- [ ] **Step 2: Run Vitest to verify failure**
  - Run: `cd hera-desktop && bun run test`
  - Expected: FAIL.

- [ ] **Step 3: Implement `WaveformPeaksCanvas.tsx` `768 B` RGB rendering, `replayGainDb` forwarding in `native-audio-transport.ts` (touching `0` lines in `player.ts`), `PreGigAuditPopover.tsx`, `ScannerStatusPill.tsx` Cancel + External Booth controls, Vocal Clash / Glitch / Variable-BPM badges, and context-menu actions**

- [ ] **Step 4: Run verification receipts**
  - Run: `cd hera-desktop && bun run knip && bun run check && bun run test`
  - Expected: PASS (`0` lint/type/knip errors).

- [ ] **Step 5: Mark `[x]` and commit atomically**
  - Commit: `feat(ui): add RGB 3-band waveforms, Pre-Gig hardware audit, Vocal Clash/Glitch badges, and Sonic Twins menu`

---

### Task 7: Functional 3-Step `BETA` Onboarding Wizard & Permanent Studio `BETA` Badge

**Files:**
- Modify: `hera-desktop/src-tauri/src/plugins/config.rs` (`onboarding_completed: bool`, `dj_handle: String` with `#[serde(default)]`)
- Modify: `hera-desktop/src/components/OnboardingWizardModal.tsx`
- Modify: `hera-desktop/src/components/Navigation.tsx`
- Modify: `hera-desktop/src/routes/__root.tsx`
- Test: `hera-desktop/src/__tests__/onboarding-wizard.test.tsx`

**Interfaces:**
- Consumes: `scanner_discover_workable_roots`, `scanner_cancel_scan` (`Task 5`), `ConfigBridge`.
- Produces: Interactive first-run onboarding persisting `onboarding_completed = true`, `dj_handle`, selected workable Audio/Video folders (`Don't break the user space` guarantee + `< 10m` vs `> 10m` Mixes preview), and permanent `BETA` badge in `Navigation.tsx`.

- [ ] **Step 1: Write failing component test `onboarding-wizard.test.tsx` verifying 3-step configuration flow, DB-First root auto-selection, `BETA` disclosure, and completion persistence**

- [ ] **Step 2: Run test to verify failure**
  - Run: `cd hera-desktop && bun run test onboarding-wizard`
  - Expected: FAIL.

- [ ] **Step 3: Upgrade `OnboardingWizardModal.tsx` from static text to interactive 3-step configuration, add `onboarding_completed` to `Config`, mount in `routes/__root.tsx`, and render permanent `BETA` pill in `Navigation.tsx`**

- [ ] **Step 4: Run verification receipts**
  - Run: `cd hera-desktop && bun run gen:types && bun run knip && bun run check && bun run test && bun run build`
  - Expected: PASS (`0` errors).

- [ ] **Step 5: Mark `[x]` and commit atomically**
  - Commit: `feat(onboarding): wire interactive 3-step BETA onboarding wizard with DB-first folder discovery`

---

### Task 8: End-to-End Workspace Ratchets, WASM Parity & Live Booth QA Sign-Off

**Files:**
- Modify: `hera-desktop/src/__tests__/hera-studio-qa.test-e2e.tsx`
- Modify: `docs/superpowers/DECISIONS.md`
- Modify: `docs/superpowers/plans/2026-11-15-curation-intelligence-and-zero-io-booth.md`

- [ ] **Step 1: Extend E2E QA coverage for `BETA` badge, Pre-Gig Audit popover, and Mixes (`< 10m` vs `> 10m`) + Audio/Video crate controls**

- [ ] **Step 2: Run full polyglot verification suite and capture green receipts**
  - Run:
    ```bash
    cargo fmt --all -- --check
    cargo test -p hera-ipc
    cargo test -p hera-node-rs --test architectural_ratchets
    cargo test -p hera-node-rs --test golden_masters
    cargo test --workspace
    cargo clippy --workspace -- -D warnings
    cargo check --manifest-path hera-rs/Cargo.toml --target wasm32-unknown-unknown --features cloudflare --no-default-features
    cd hera-desktop && bun run gen:types && bun run knip && bun run check && bun run test && bun run build
    ```
  - Expected: `exit code 0` across every command with `0` warnings and `0` snapshot diffs.

- [ ] **Step 3: Record final architectural notes in `docs/superpowers/DECISIONS.md`, mark all tasks `[x]`, and commit atomically**
  - Commit: `test(qa): complete end-to-end curation intelligence and zero-io 50k booth verification`
