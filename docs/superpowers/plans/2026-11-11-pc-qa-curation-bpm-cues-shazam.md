# HERA v0.7.0 — PC QA Overhaul Plan: 5-Layer BPM Engine (Salsa, Variable BPM & Tecnomerengue), Anti-Empty-Zone 8-Cue Merger (`>= 50%`), CD-Skip Forensics, Mixed Audio+Video Crates (`< 10m` Mixes), Shazam DNA, Cancellable Scanner, System Tray & `BETA` Onboarding

> **For agentic workers (Multi-Instance Resume on High-Resource PC):** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for atomic `git pull` / `git push` handoff tracking.

**Goal:** Resolve all 10 real-world DJ PC QA findings plus a ground-up 5-layer overhaul of HERA's BPM calculation engine (Salsa syncopated *tumbao* / clave `82–108 BPM`, sliding-window **Variable BPM** tracking where VirtualDJ fails, **Tecnomerengue** `132–160 BPM`, octave/polyrhythmic de-aliasing, and metadata/AI consensus) without breaking user disk space (`Don't break the user space`), Golden Master snapshots, or `< 350` LOC ratchets.

**Architecture:** Introduce cohesive `<= 349` LOC Pure-Rust domain submodules (`hera-rs/src/bpm_engine/`, `hera-rs/src/cue_engine.rs`, `hera-rs/src/forensics_glitch.rs`, `hera-rs/src/dj_learning/`, `hera-rs/src/shazam_id/`), export typed IPC DTOs via `hera-ipc` (`ts-rs`), extend SQLite (`06_create_dj_learning_tables.sql` + offline `.sqlx/`), wire cancellable 2-phase Audio+Video scanning and native `TrayIconBuilder` in `src-tauri`, and surface `BETA` Onboarding, `< 10m` vs `Mixes (> 10m)`, 8-Cue rails, Variable-BPM/CD-Skip badges, and `/agent` curatorial coaching in `hera-desktop`.

**Tech Stack:** Rust (`Cargo` workspace, `symphonia 0.5`, `rustfft 6.2`, `candle-core`, `sqlx 0.8`, `ts-rs 12.0.1`, `tauri 2.12`), React 19 Compiler, StyleX, Vitest, `moon 2.6.5`.

**Spec:** `docs/superpowers/specs/2026-11-11-pc-qa-curation-bpm-cues-shazam-design.md`

---

## Global Constraints & Ratchet Guardrails (Read Before Coding)

1. **`Don't break the user space`:** NEVER move, rename, or delete physical audio/video files on disk when routing `> 10m` tracks to Mixes, retiring tracks (*"Clavelito ya no"*), flagging scratched tracks (*"21. Eres tú"*), or identifying tracks via Shazam DNA. Always create a `.bak` copy before modifying `VirtualDJ/database.xml`.
2. **Immutable Golden Masters (`hera-rs/tests/golden_masters.rs`):** Keep `audit_pcm_channels`, `analyze_structure_and_style`, `optimize_crate_2opt`, `optimize_crate_with_spine`, and `list_mcp_tools` returning bit-identical outputs on existing 4-cue / 6-track fixtures (`0` diffs on `hera-rs/tests/snapshots/*.snap`; NEVER run `cargo insta accept` on legacy snapshots). Expose new V2 functions (`analyze_bpm_consensus`, `analyze_structure_8cues`, `audit_pcm_with_glitch_scan`, `optimize_crate_with_dj_rules`) or guard optional fields with `#[serde(default, skip_serializing_if = "...")]`.
3. **Architectural Ratchets (`hera-rs/tests/architectural_ratchets.rs`):**
   - Every `.rs` file in `hera-rs/src/{bpm_engine,dj_learning,shazam_id,mcp,code_mode,dj_brain,p2p,audio_engine}`, `hera-desktop/src-ipc/src`, and `hera-desktop/src-tauri/src/plugins/{hera_dj,hera_agent}` MUST stay `<= 349` lines (`register new sliced dirs in SLICED_SUBMODULE_DIRS`).
   - Keep `TrackProgress.tsx <= 220` lines (extract `<HotCueRail8 />` into `hera-desktop/src/components/HotCueRail8.tsx`), `bridge-hera.ts <= 300` lines (`<= 3` manual exported types), and `bridge-agent.ts <= 300` lines.
   - Whenever adding a `#[tauri::command]`, update all 4 ACL Triad locations (`plugins/...`, `src-tauri/build.rs`, `src-tauri/capabilities/main.json`, `src-tauri/src/main.rs`) AND bump `EXPECTED_TAURI_COMMANDS_COUNT` in `hera-rs/tests/architectural_ratchets.rs`.
   - Zero OS-specific user paths in tracked docs/tests (`ratchet_zero_os_absolute_paths_in_docs_and_plans`).
   - Zero `clippy` (`-D warnings`) or `cargo fmt --check` warnings (`ADR-030`).

---

### Task 1: 5-Layer Multi-Strategy BPM Engine (`Salsa`, `Variable BPM`, `Tecnomerengue` & Consensus Arbiter)

**Files:**
- Create: `hera-rs/src/bpm_engine/mod.rs` (`<= 349` LOC)
- Create: `hera-rs/src/bpm_engine/multiband_onset.rs` (`<= 349` LOC)
- Create: `hera-rs/src/bpm_engine/window_tracker.rs` (`<= 349` LOC)
- Create: `hera-rs/src/bpm_engine/latin_heuristics.rs` (`<= 349` LOC)
- Create: `hera-rs/src/bpm_engine/consensus_ai.rs` (`<= 349` LOC)
- Modify: `hera-rs/src/lib.rs`
- Modify: `hera-rs/src/structure.rs` (delegate V2 BPM + `Tecnomerengue` subgenre while keeping `golden_masters` 124 BPM fixture bit-identical)
- Modify: `hera-rs/tests/architectural_ratchets.rs` (add `"hera-rs/src/bpm_engine"` to `SLICED_SUBMODULE_DIRS`)

**Interfaces:**
- `BpmAnalysisReport { bpm: f32, entry_bpm: f32, exit_bpm: f32, bpm_min_detected: f32, bpm_max_detected: f32, tempo_drift_bpm: f32, tempo_drift_pct: f32, window_iqr_bpm: f32, is_variable_bpm: bool, bpm_confidence: f32, bpm_needs_review: bool, detected_rhythm_family: String, tempo_stability_label: String, tempo_map_points: Vec<TempoAnchorPoint> }`
- `pub fn analyze_bpm_consensus(mono: &[f32], sample_rate: u32, external_hint: Option<ExternalBpmHint>) -> BpmAnalysisReport`

- [ ] **Step 1: Add failing unit tests for all 5 BPM failure modes**
  Create deterministic synthetic PCM tests in `hera-rs/tests/bpm_engine_qa.rs`:
  1. `test_salsa_tumbao_and_campana_resolves_to_82_108_bpm` (`94.0 BPM` Salsa with NO beat-1 kick, syncopated baby-bass *tumbao* on beats `2.5` & `4`, campana/martillo in `800–3500 Hz`, and `15s` rubato piano intro -> must detect `94.0 ± 0.5 BPM`, NOT `125.3 BPM` `4/3x` or `188 BPM` `2x`).
  2. `test_variable_bpm_live_band_drift_tracks_entry_and_exit_bpm` (live band accelerating smoothly from `90.0 BPM` in verse to `96.5 BPM` in montuno -> `is_variable_bpm == true`, `entry_bpm ~ 90.0`, `exit_bpm ~ 96.5`, `tempo_drift_bpm >= 5.5`, even if an erroneous static VDJ hint of `129.0` is supplied).
  3. `test_tecnomerengue_132_to_160_bpm_not_folded_in_half` (`148.0 BPM` and `158.0 BPM` binary tambora/synth brass + 16th-note `4.5–10 kHz` güira -> `bpm ~ 148.0` / `158.0`, `top_subgenre == "Tecnomerengue"`, never folded to `79.0 BPM`).
  4. `test_fractional_vinyl_tempo_not_falsely_integer_snapped` (`92.35 BPM` steady analog rip -> preserves `92.35 ± 0.08 BPM` instead of snapping to `92.0`).
  5. `test_consensus_arbiter_resolves_octave_mismatch_with_genre_prior` (`184.0 BPM` external tag on Salsa orchestra metadata -> folded to `92.0 BPM` via harmonic consensus).
- [ ] **Step 2: Run `cargo test -p hera-node-rs --test bpm_engine_qa` to capture RED receipt**
- [ ] **Step 3: Implement `multiband_onset.rs`, `window_tracker.rs` (12s sliding windows + Viterbi continuity), `latin_heuristics.rs`, and `consensus_ai.rs`**
  Ensure WASM compatibility (`wasm32-unknown-unknown --features cloudflare --no-default-features`) by gating optional `CandleLocalDjModel` tie-breaker behind `#[cfg(feature = "native")]` with a pure-Rust heuristic fallback on WASM.
- [ ] **Step 4: Run `cargo test -p hera-node-rs` (including `golden_masters` and `architectural_ratchets`) + WASM check + `clippy -D warnings`**
  Expected: `0` failed, `0` warnings, `0` golden master diffs.
- [ ] **Step 5: Mark `[x]` and commit atomically**
  Commit: `feat(dsp): add 5-layer multi-window BPM engine for Salsa, variable BPM, and Tecnomerengue`

---

### Task 2: Forensic CD-Skip / Glitch Detector (*"21. Eres tú"*) & Cancellable Full-Track Decoder (`<= 600s`)

**Files:**
- Create: `hera-rs/src/forensics_glitch.rs` (`<= 349` LOC)
- Modify: `hera-rs/src/forensics.rs`
- Modify: `hera-rs/src/lib.rs`

**Interfaces:**
- `GlitchScanReport { has_audio_skips: bool, skip_glitch_count: u32, decode_error_packets: u32, micro_dropout_count: u32, buffer_repeat_stutter_count: u32, skip_timestamps_seconds: Vec<f32>, warning_summary: Option<String> }`
- `pub fn decode_media_file_cancellable(path: &Path, max_seconds: f32, cancel_flag: Option<&AtomicBool>) -> Result<DecodedAudioStreamWithPacketStats, String>`
- `pub fn scan_pcm_for_cd_skips(mono: &[f32], sample_rate: u32, decode_error_packets: u32) -> GlitchScanReport`

- [ ] **Step 1: Add failing unit tests in `hera-rs/tests/forensics_glitch_qa.rs`**
  Test (a) clean studio audio (`has_audio_skips == false`), (b) synthetic scratched-CD track with `45ms` digital dropout + `0.72` phase cliff and `90ms` repeated stutter frame (`has_audio_skips == true`, exact timestamp within `±0.1s`), and (c) `AtomicBool` mid-stream cancellation halting decode immediately.
- [ ] **Step 2: Run `cargo test -p hera-node-rs --test forensics_glitch_qa` to capture RED receipt**
- [ ] **Step 3: Implement `forensics_glitch.rs` and cancellable packet-error-counting Symphonia stream in `forensics.rs`**
  Keep `audit_pcm_channels` signature and JSON output unchanged so `golden_forensics_and_4_hot_cues` passes untouched; add `.mp4`, `.mov`, `.m4v`, `.mkv`, `.webm` container support via Symphonia demuxers.
- [ ] **Step 4: Run `cargo test -p hera-node-rs` and `cargo clippy -p hera-node-rs -- -D warnings` for GREEN receipt**
- [ ] **Step 5: Mark `[x]` and commit atomically**
  Commit: `feat(forensics): add scratched-CD skip/stutter detector and cancellable full-track decoder`

---

### Task 3: Anti-Empty-Zone Gate & Second-Half (`>= 50%`) 8-Hot-Cue Engine + Smart DJ Merger

**Files:**
- Create: `hera-rs/src/cue_engine.rs` (`<= 349` LOC)
- Create: `hera-rs/src/dj_learning/mod.rs` (`<= 349` LOC)
- Create: `hera-rs/src/dj_learning/vdj_rekordbox_parser.rs` (`<= 349` LOC)
- Create: `hera-rs/src/dj_learning/cue_templates.rs` (`<= 349` LOC)
- Modify: `hera-rs/src/export.rs` (export slots `1..=8` preserving exact `HotCuePoint.num` and `is_video`)
- Modify: `hera-rs/tests/architectural_ratchets.rs` (add `"hera-rs/src/dj_learning"` to `SLICED_SUBMODULE_DIRS`)

**Interfaces:**
- `CueOrigin` (`DjManual`, `DjLearnedTemplate`, `HeraAcousticSecondHalf`)
- `pub fn compute_mixable_8_hot_cues(mono: &[f32], sample_rate: u32, bpm_report: &BpmAnalysisReport, learned_template: Option<&GenreCueTemplate>) -> Vec<HotCuePoint>`
- `pub fn merge_manual_and_hera_cues(manual_cues: &[HotCuePoint], proposed_cues: &[HotCuePoint]) -> Vec<HotCuePoint>`
- `pub fn parse_virtualdj_xml_and_history(xml_content: &str, history_m3u_files: &[(String, String)]) -> DjExternalHistorySnapshot`

- [ ] **Step 1: Add failing unit tests in `hera-rs/tests/cue_learning_qa.rs`**
  1. Synthesize a `240s` track containing a beatless vocal breakdown at `120s..136s` and a beatless reverb tail at `224s..240s`: assert **zero** cues land inside `[120s..136s]` or `[224s..240s]`, `Cue 1` lands on `1.1.1`, and `Cues 5..8` all land at `>= 120.0s` (`>= 50%`) on percussive downbeats (`Cue 8` cleanly before `224s`).
  2. Parse a fixture `VirtualDJ/database.xml` containing *"Tú me vas a llorar"* with manual `<Poi Num="5" Pos="134.2" />` and `<Poi Num="8" Pos="208.6" />`: assert `merge_manual_and_hera_cues` preserves `Cue 5` and `Cue 8` bit-for-bit (`CueOrigin::DjManual`) and updates `GenreCueTemplate` ratios.
- [ ] **Step 2: Run `cargo test -p hera-node-rs --test cue_learning_qa` to capture RED receipt**
- [ ] **Step 3: Implement `cue_engine.rs` and `dj_learning/{mod,vdj_rekordbox_parser,cue_templates}.rs` + 8-cue `export.rs` support**
  Include safe atomic `.bak` backup helper (`write_virtualdj_xml_with_backup`) that never alters audio paths on disk.
- [ ] **Step 4: Run `cargo test -p hera-node-rs` + `cargo clippy -p hera-node-rs -- -D warnings` for GREEN receipt**
- [ ] **Step 5: Mark `[x]` and commit atomically**
  Commit: `feat(cues): enforce anti-empty-zone >=50% 8-cue placement and VirtualDJ/Rekordbox cue learning`

---

### Task 4: Curatorial Rules in `synergy.rs` (`< 10m` Mixes Gate, Audio+Video Crates, Golden Pairs & Track Exclusions)

**Files:**
- Create: `hera-rs/src/dj_learning/curation_rules.rs` (`<= 349` LOC)
- Modify: `hera-rs/src/synergy.rs`

**Interfaces:**
- `DjCurationRuleSet { golden_pairs: Vec<GoldenTransitionPair>, excluded_track_ids: HashSet<String>, excluded_title_patterns: Vec<String>, allow_mixed_video: bool, max_crate_track_seconds: f32 }` (default `max_crate_track_seconds = 600.0`)
- `CuratedCrateWithRulesResult { crate_plan: OptimizedCrateResult, routed_to_mixes_over_10m: Vec<String>, skipped_excluded_tracks: Vec<String>, flagged_glitch_tracks: Vec<String> }`
- `pub fn optimize_crate_with_dj_rules(...) -> CuratedCrateWithRulesResult`

- [ ] **Step 1: Add failing test `test_curation_rules_golden_pairs_exclusions_video_and_10min_mixes` in `hera-rs/tests/curation_rules_qa.rs`**
  Include:
  - Track A (*"Perdóname mi vida"*, audio) + Track B (*"Conquístame"*, `.mp4` video) registered as a `GoldenTransitionPair`,
  - *"Clavelito"* registered in `excluded_title_patterns`,
  - A `42-minute` (`2520s`) Salsa Mix.
  Assert: the `42m` file is routed to `routed_to_mixes_over_10m` (excluded from crate), *"Clavelito"* is in `skipped_excluded_tracks`, *"Perdóname mi vida"* is immediately followed by *"Conquístame"* (mixed Audio + Video crate), and `entry_bpm`/`exit_bpm` score variable-BPM transitions accurately.
- [ ] **Step 2: Run `cargo test -p hera-node-rs --test curation_rules_qa` to capture RED receipt**
- [ ] **Step 3: Implement `curation_rules.rs` and `optimize_crate_with_dj_rules` in `synergy.rs`**
  Keep `optimize_crate_2opt` and `optimize_crate_with_spine` untouched so `golden_2opt_tsp_and_spine_routes` passes with `0` diffs.
- [ ] **Step 4: Run `cargo test -p hera-node-rs` for GREEN receipt**
- [ ] **Step 5: Mark `[x]` and commit atomically**
  Commit: `feat(curator): enforce <10m mixes separation, mixed audio+video crates, golden pairs, and track exclusions`

---

### Task 5: Shazam-Like Constellation Fingerprinting + Hybrid Track DNA (`hera-rs/src/shazam_id/`)

**Files:**
- Create: `hera-rs/src/shazam_id/mod.rs` (`<= 349` LOC)
- Create: `hera-rs/src/shazam_id/constellation.rs` (`<= 349` LOC)
- Create: `hera-rs/src/shazam_id/matcher.rs` (`<= 349` LOC)
- Modify: `hera-rs/src/lib.rs`
- Modify: `hera-rs/tests/architectural_ratchets.rs` (add `"hera-rs/src/shazam_id"` to `SLICED_SUBMODULE_DIRS`)

**Interfaces:**
- `ConstellationFingerprint { landmark_hashes: Vec<(u64, u32)>, duration_seconds: f32 }`
- `TrackDnaIdentificationReport { matched: bool, match_source: String, confidence: f32, artist: Option<String>, title: Option<String>, version: Option<String>, genre: Option<String>, bpm_report: BpmAnalysisReport, camelot_key: String, glitch_report: GlitchScanReport, hot_cues_8: Vec<HotCuePoint>, disk_file_untouched: bool }`

- [ ] **Step 1: Add failing test `test_shazam_constellation_matches_noisy_offset_snippet_and_extracts_full_dna` in `hera-rs/tests/shazam_id_qa.rs`**
  Index 5 synthetic tracks; query a `15s` mid-track slice with `-18 dB` additive noise and untagged filename `Track_01_Unknown.mp3`; assert exact track match (`confidence >= 0.85`), full 8-cue + BPM DNA extraction, and `disk_file_untouched == true`.
- [ ] **Step 2: Run `cargo test -p hera-node-rs --test shazam_id_qa` to capture RED receipt**
- [ ] **Step 3: Implement `constellation.rs`, `matcher.rs` (Local Landmark Index + Swarm CAS + optional AcoustID/MusicBrainz HTTP resolver), and `mod.rs`**
- [ ] **Step 4: Run `cargo test -p hera-node-rs` + WASM check for GREEN receipt**
- [ ] **Step 5: Mark `[x]` and commit atomically**
  Commit: `feat(shazam): add spectral constellation fingerprinting and hybrid full-DNA identifier`

---

### Task 6: Hexagonal IPC Contracts (`hera-ipc`), SQLite Migration `06`, Workable Directory Agent & 2-Phase Cancellable Scanner

**Files:**
- Modify: `hera-desktop/src-ipc/src/dj_types.rs` & `hera-desktop/src-ipc/src/typegen.rs` (`<= 349` LOC; split `learning_types.rs` if needed)
- Create: `hera-desktop/src-tauri/src/migrations/06_create_dj_learning_tables.sql`
- Modify: `hera-desktop/src-tauri/src/libs/track.rs` (600s decode, Audio+Video extensions, `is_video`, `is_mix_over_10m`, `has_audio_skips`, `is_variable_bpm`, `entry_bpm`, `exit_bpm`, 8 Hot Cues)
- Modify: `hera-desktop/src-tauri/src/libs/database.rs` & `.sqlx/*.json`
- Modify: `hera-desktop/src-tauri/src/plugins/db.rs` (`SCAN_CANCEL_FLAG: AtomicBool`, `cancel_library_scan`, `evaluate_music_folder_workability`, 2-phase scan)
- Modify: `hera-desktop/src-tauri/src/plugins/config.rs` (`onboarding_completed: bool`, `dj_software: String`)

- [ ] **Step 1: Define new IPC DTOs in `hera-ipc` (`FolderWorkabilityReport`, `BpmUiBadgeInfo`, `DjCurationFeedbackRequest`, `ShazamDnaResponse`, `ScanPhaseProgress`) and regenerate `hera-desktop/src/generated/hera-typings.ts` via `cargo test -p hera-ipc`**
- [ ] **Step 2: Create `06_create_dj_learning_tables.sql` adding `is_video`, `is_mix_over_10m`, `has_audio_skips`, `is_variable_bpm`, `entry_bpm`, `exit_bpm`, `bpm_min`, `bpm_max`, `excluded_from_curation`, `constellation_digest` on `tracks`, plus tables `dj_golden_pairs`, `dj_excluded_tracks`, `dj_learned_cue_rules`, and `dj_feedback_memory`**
- [ ] **Step 3: Implement `evaluate_music_folder_workability` (classifying `< 30s` SFX, `30s..=600s` workable Audio/Video, `> 600s` Mixes, ignoring junk/system dirs) and 2-Phase Cancellable `scan_library` + `cancel_library_scan` (`AtomicBool`)**
- [ ] **Step 4: Refresh `.sqlx/` offline query metadata (`cargo sqlx prepare` in `hera-desktop/src-tauri`), update Tauri ACL Triad + `EXPECTED_TAURI_COMMANDS_COUNT`, and run `cargo test -p hera-ipc` + `cargo check --workspace`**
- [ ] **Step 5: Mark `[x]` and commit atomically**
  Commit: `feat(desktop-db): add SQLite DJ learning schema, mixed video/mixes classification, and cancellable 2-phase scanner`

---

### Task 7: Native System Tray Tooltip (`TrayIconBuilder`) & `/agent` Conversational Coaching Tools

**Files:**
- Modify: `hera-desktop/src-tauri/src/main.rs` (`TrayIconBuilder::with_id("hera-tray")` + menu + click-to-focus)
- Create: `hera-desktop/src-tauri/src/plugins/hera_dj/tray_and_learning.rs` (`<= 349` LOC: `update_tray_tooltip`, `sync_dj_external_history`, `identify_track_shazam_dna`, `record_dj_curation_feedback`)
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/mod.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_agent/` (add loopback MCP tools `coach_track_cues`, `save_golden_pair`, `exclude_track_from_crates`, `identify_track_dna`, `evaluate_folder_workability` with 1-click confirmation gate before bulk library rewrites)
- Modify: `hera-desktop/src-tauri/build.rs`, `hera-desktop/src-tauri/capabilities/main.json`, `hera-rs/tests/architectural_ratchets.rs`

- [ ] **Step 1: Wire `TrayIconBuilder` in `src-tauri/src/main.rs` with default tooltip `"HERA BETA · Copiloto DJ Listo"` and context menu (`Mostrar HERA`, `Modo Mini`, `Cancelar Escaneo`, `Salir`)**
- [ ] **Step 2: Implement `update_tray_tooltip` + DJ Learning / Shazam / Feedback commands in `plugins/hera_dj/tray_and_learning.rs` and expose the 5 coaching tools in `plugins/hera_agent/`**
  Verify natural Spanish prompts in `smart_intent.rs`:
  - *"En 'Tú me vas a llorar' el cue 5 y el cue 8 que puse yo fue mejor"*
  - *"17. Perdóname mi vida y 20. Conquístame juntas suenan excelente"*
  - *"Clavelito ya no"*
- [ ] **Step 3: Run `cargo test -p hera-ipc`, `cargo test -p hera-node-rs --test architectural_ratchets`, and `cargo test -p hera-desktop` for GREEN receipt**
- [ ] **Step 4: Mark `[x]` and commit atomically**
  Commit: `feat(desktop-agent): add native system tray live tooltip and conversational DJ coaching tools`

---

### Task 8: Frontend Studio UI — `BETA` Badge, `OnboardingWizardModal`, `Mixes (> 10m)` Area, 8-Cue Rail, Cancel Scan & Alerts

**Files:**
- Create: `hera-desktop/src/components/OnboardingWizardModal.tsx` (`<= 420` LOC)
- Create: `hera-desktop/src/components/HotCueRail8.tsx` (`<= 220` LOC, keeping `TrackProgress.tsx <= 220` LOC)
- Modify: `hera-desktop/src/components/TrackProgress.tsx`
- Modify: `hera-desktop/src/components/Navigation.tsx` (`BETA` badge + live `Cancelar` scan button + `Mixes (> 10m)` filter count)
- Modify: `hera-desktop/src/components/HeraDjToolbar.tsx` & `hera-desktop/src/components/TrackRow.tsx` (`[VIDEO]` badge, `⚠️ Saltos / CD rayado` warning pill, `BPM Variable (A -> B)` badge, `Pistas (< 10m)` vs `Mixes (> 10m)` toggle, 1-click `Identificar ADN`)
- Modify: `hera-desktop/src/lib/player.ts` (push live track title/BPM/Camelot to `update_tray_tooltip`)
- Modify: `hera-desktop/src/routes/dna.tsx` & `hera-desktop/src/routes/agent.tsx` (render Golden Pairs, Excluded Tracks, Genre Cue Templates, and `Sincronizar VirtualDJ / Rekordbox`)

- [ ] **Step 1: Build `HotCueRail8.tsx` and wire into `TrackProgress.tsx` & `TrackRow.tsx` (`1:1` header/row column parity per `ADR-007`, `tabular-nums` per `ADR-028`)**
  Distinguish `DJ Manual` cues (solid emerald/gold pin) from `HERA >= 50%` cues (`Pads 5..8`).
- [ ] **Step 2: Add `BETA` badge + `Cancelar` scan button in `Navigation.tsx` and `Pistas (< 10m)` / `Mixes (> 10m)` segmented view + `[VIDEO]` / `⚠️ Saltos` / `BPM Variable` badges in `HeraDjToolbar.tsx` & `TrackRow.tsx`**
- [ ] **Step 3: Build 3-step `OnboardingWizardModal.tsx` (with explicit `BETA` phase banner and *"HERA nunca mueve ni renombra tus archivos en disco"* guarantee) and wire into `__root.tsx` + `Ajustes`**
- [ ] **Step 4: Surface Golden Pairs, Retired Tracks, and Learned Genre Cue Templates in `dna.tsx` and `/agent` quick actions**
- [ ] **Step 5: Run `cd hera-desktop && bun run gen:types && bun run check && bun run test && bun run build` + `cargo test -p hera-node-rs --test architectural_ratchets`**
  Expected: `0` TypeScript/Oxlint/StyleX errors, `TrackProgress.tsx <= 220` LOC, all unit tests green.
- [ ] **Step 6: Mark `[x]` and commit atomically**
  Commit: `feat(ui): add BETA onboarding wizard, 8-cue rail, <10m mixes view, cancel scan, and BPM/glitch badges`

---

### Task 9: End-to-End QA Verification & `DECISIONS.md` Final Sign-Off on PC

**Files:**
- Modify: `hera-desktop/src/__tests__/hera-studio-qa.test-e2e.tsx`
- Modify: `docs/superpowers/DECISIONS.md`

- [ ] **Step 1: Add E2E assertions in `hera-studio-qa.test-e2e.tsx` covering `BETA` badge, `OnboardingWizardModal`, `Pistas (< 10m)` vs `Mixes (> 10m)` tab, `Cancelar` scan button, 8-Cue pins, and `BPM Variable` / `⚠️ Saltos` badges**
- [ ] **Step 2: Run full canonical monorepo verification suite on PC**
  - `cargo check --workspace`
  - `cargo test -p hera-node-rs`
  - `cargo test -p hera-ipc`
  - `cargo check --manifest-path hera-rs/Cargo.toml --target wasm32-unknown-unknown --features cloudflare --no-default-features`
  - `cd hera-desktop && bun run gen:types && bun run check && bun run test && bun run build`
  - `moon run :check :test hera-desktop:build`
- [ ] **Step 3: Validate against real PC music library (Salsa, Tecnomerengue, *"Tú me vas a llorar"*, *"Perdóname mi vida -> Conquístame"*, *"Eres tú"*, *"Clavelito"*, mixed Audio+Video crate export to VirtualDJ)**
- [ ] **Step 4: Mark `[x]`, update `DECISIONS.md` with empirical PC benchmark receipts, and commit atomically**
  Commit: `test(e2e): verify v0.7.0 PC QA curation, 5-layer BPM engine, 8-cue merger, and BETA onboarding`
