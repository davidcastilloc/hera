# HERA v0.7.0 — AI Curation Intelligence (`Genre × Era`), Zero-IO Single-Pass Acoustic Harvest (`LUFS`, RGB, `64D`, `8 Cues`, Glitches, Shazam), Live VirtualDJ/Ableton Bridge & 50k Booth Hardening

**Date:** 2026-11-15  
**Repository:** `davidcastilloc/hera` (`hera-rs`, `hera-desktop/src-ipc`, `hera-desktop/src-tauri`, `hera-desktop/src`)  
**Baseline Audited:** `1078b61` (`v0.6.1` post-`ADR-035` Scanner v3.1 + Native System Tray `libs/tray.rs`)  
**Status:** Approved Multi-Instance Design Specification (`superpowers-odd`)

---

## 1. Intent, Empirical Findings & `HEAD` Delta Audit

Live DJ QA against a real 50,000-track Latin + Open-Format Audio/Video library (`VirtualDJ/History/tracklist.txt`, `VirtualDJ/database.xml`, and active crates) combined with a line-by-line delta audit against `HEAD` (`1078b61`) surfaced two complementary engineering imperatives:

### 1.1 Root Causes of Questionable AI Playlist Selections
1. **Prompt Blindness & Disconnected Curation Rules:**
   - `hera-desktop/src-tauri/src/plugins/hera_agent/turn_agy.rs` injects a 2-line generic hint with zero DJ curation constraints.
   - `tools_playlist_vdj.rs` (`create_playlist`) and `smart_intent.rs` bypass `hera_node_rs::synergy::optimize_crate_with_dj_rules` completely: when `track_ids` is omitted they pull the entire library via `db.get_all_tracks()` and sort lexicographically (`1A..12B`, then ascending BPM) via `sort_tracks_harmonically`.
2. **BPM/Camelot Numeric Collision Across Incompatible Genres & Eras:**
   - *Salsa Baúl / Romántica* (`82–98 BPM`, `7A/8A/8B`) shares the exact numeric `(BPM, Camelot)` zone with *Reggaetón / Perreo Actual* (`88–102 BPM`, `7A/8A`), causing pure `(Camelot, BPM)` sorting to drop Salsa tracks in the middle of Perreo blocks.
   - *90s Euro-Merengue / Merengue House* (e.g., Sandy & Papo at `~126 BPM`) collides numerically with *2025–2026 Venezuelan Merenguetón* (e.g., Alleh & Yorghaki at `122–126 BPM`), creating jarring 30-year aesthetic clashes unless an explicit crossover bridge is requested.
3. **Truncated Manual Hot Cues & Blind Trust in External BPM:**
   - `hera-desktop/src-tauri/src/libs/fast_library_scan.rs` truncates imported VirtualDJ cues with `.take(4)` (discarding the DJ's manual `Cue 5` and `Cue 8` in tracks like *"Tú me vas a llorar"*) and copies cached VirtualDJ BPM values unconditionally even when VirtualDJ misfires on *Salsa* (`4/3x` *tumbao* aliases at `124–142 BPM`) or *Variable-BPM* live recordings.

### 1.2 `Inspect First, Merge Deltas Only` Audit Against `HEAD` (`1078b61`)
- **Already Landed in `HEAD` (Do Not Duplicate or Break):**
  - `v2_core/audio_ingest.rs` (`#[cfg(not(target_arch = "wasm32"))]` Symphonia decode + `ffmpeg_hash.rs`), `forensics.rs` (`spec.rate`), 16 Audio+Video formats, `vdj_xml_parser.rs`, `fast_library_scan.rs` + `scanner_worker.rs` base `200 ms` WAL daemon (`ADR-035`), `EXPECTED_TAURI_COMMANDS_COUNT = 63`, and `hera-desktop/src-tauri/src/libs/tray.rs` (`1078b61`).
  - `hera-desktop/src-tauri/src/migrations/07_add_scanner_v3_columns.sql` **is already committed and applied to existing user databases**. Modifying `07` in-place would trigger `sqlx::migrate::MigrateError::VersionMismatch(7)`. All new columns and tables MUST go into **`08_add_zero_io_and_curation_intelligence.sql`**.
- **Pending High-Value Deltas Verified Missing in `HEAD`:**
  - `plugins/cover.rs` still uses unbounded `#[memoize::memoize]` (`> 1 GB` RAM leak over 50k tracks) and panicking `.unwrap()` calls on root paths.
  - `plugins/hera_dj/audio_transport.rs` still discards `let _ = hot_cues_sec;` (forcing ~2 s full-track cue recomputation on deck load) and lacks lock-free `TransportCommand::SetGain` Auto-Gain application.
  - `plugins/hera_agent/{tools_library.rs, tools_playlist_vdj.rs}` still call `db.get_all_tracks().await` on every turn instead of `< 1 ms` indexed SQLite queries.
  - `hera-rs/src/scanner/full_pass.rs` still omits `scan_pcm_for_cd_skips` (*"21. Eres tú"*), discards `has_intro_vocals`, `has_outro_vocals`, and `embedding_64d` (`64 B` `i8` Sonic Twins), and lacks Zero-IO `768 B` RGB waveforms, ITU-R BS.1770 `LUFS` Auto-Gain (`-11 LUFS`) + Lead-In silence trim, and `< 120 Hz` Club Mono sub-bass phase correlation.
  - Missing `folder_watcher.rs` (`2.5 s` live `VirtualDJ/History` bridge with automatic `dj_golden_pairs` learning + `1,500 ms` stabilized Ableton bounce watcher), `scanner_intelligence.rs` + `PreGigAuditPopover.tsx` (`Check Pre-Toque` for VirtualDJ / USB CDJ / Denon `44.1 kHz`), `shazam_id/` acoustic constellation identifier, DB-First workable directory auto-discovery, and a functional 3-step `OnboardingWizardModal.tsx` mounted in `__root.tsx`.

---

## 2. Goals, Non-Goals & Anti-Goals

### Goals
1. **Zero Accidental Genre/Era Contamination:** Isolate standard playlists/crates by *Genre Family × Era Window × Groove* while supporting intentional *Crossover / Hora Loca* sets and `entry_bpm -> exit_bpm` transition bridges.
2. **Single-Pass Zero-IO Acoustic Harvest:** Compute BPM consensus (Salsa `82–108`, Tecnomerengue `130–160`, Variable BPM `entry_bpm`/`exit_bpm`), 8 Hot Cues (`CueOrigin::DjManual` preserved + `>= 50%` structural pads `5..8`), `768 B` RGB waveforms, `-11 LUFS` Auto-Gain, `64D` `i8` embeddings, Vocal Clash flags, Club Mono sub-phase, CD-skip glitches, and Shazam landmarks from the single decoded PCM buffer.
3. **50k Instant Responsiveness & Safe Booth Operation:** `< 1 ms` SQLite agent queries, `256`-entry cover LRU, `< 80 ms` deck loads, kernel `BELOW_NORMAL`/`nice(10)` worker priority, Battery/External Booth throttling, `< 100 ms` scan cancellation, and DB-First workable folder discovery.
4. **Live DJ Learning & BETA Studio Completeness:** Learn passively from `VirtualDJ/History` (`2.5 s`) and actively from `/agent` feedback (*Golden Pairs*, *Excluded Tracks*, *Genre Cue Templates*), wire Now-Playing updates into `libs/tray.rs`, and ship a functional first-run `OnboardingWizardModal.tsx` with permanent `BETA` badging.

### Non-Goals & Anti-Goals
- **Sacred Disk Invariant (`Don't break the user space`):** NEVER move, rename, re-encode, or delete physical user media files or folders when routing `> 10m` Mixes, excluding tracks, flagging CD skips, or identifying untagged tracks via Shazam. Always write `.bak` backups before updating external DJ XML files.
- **Zero External LLM Proxy Routers:** External proxy routers (such as `decolua/9router`) are strictly excluded as unnecessary and insecure. All AI routing uses HERA's native Rust backends.
- **Zero New Dependencies & Zero Golden Master Regressions:** Use only crates/packages already locked in `Cargo.lock` and `bun.lock`. Preserve bit-identical outputs on all 4 `insta` snapshots in `hera-rs/tests/golden_masters.rs` and obey every ceiling in `hera-rs/tests/architectural_ratchets.rs` (`<= 349` LOC in sliced Rust modules, `player.ts <= 680` with `0` new feature logic, `TrackProgress.tsx <= 220`, `bridge-hera.ts <= 300`).

---

## 3. Architectural Approaches Considered

1. **Approach A — Prompt-Only Patch in `turn_agy.rs` (Rejected):**
   - *Trade-off:* Cheap (`~30` LOC), but leaves `create_playlist` sorting `1A..12B` over `db.get_all_tracks()`, still drops `Cue 5..8`, misses CD skips, leaks RAM in `cover.rs`, and fails offline/deterministic workflows (`hera-mind` Law 2).
2. **Approach B — Blind Overwrite from Web Plan `v3.0` (Rejected):**
   - *Trade-off:* Covers 50k scaling and Zero-IO DSP, but mutating `07_add_scanner_v3_columns.sql` crashes existing `hera.db` installations, conflicts with `ADR-035` / `1078b61` (`libs/tray.rs`), and lacks `Genre × Era` curation intelligence, 8-Cue learning, and Shazam identification.
3. **Approach C — Unified Delta Architecture (`v2.0` Selected):**
   - *Why Selected:* Preserves `07` via additive migration `08`, builds cleanly on `ADR-035` + `libs/tray.rs`, unifies Zero-IO DSP harvest with the 14-family `Genre × Era` curator and `optimize_crate_with_dj_rules` across both deterministic UI buttons and `/agent` MCP tools.

---

## 4. Technical Design by Subsystem

### 4.1 50k Memory Safety, `< 80 ms` Deck Load + Lock-Free Auto-Gain & Kernel Governor
- **`hera-desktop/src-tauri/src/plugins/cover.rs`:** Replace `#[memoize::memoize]` with a `256`-entry bounded LRU (`OnceLock<Mutex<IndexMap<String, Option<String>>>>`), skip `.webm`/`.mkv` in `lofty`, replace all `.unwrap()` calls with fallible `?`/`and_then`, and execute inside `spawn_blocking`.
- **`hera-rs/src/audio_engine/{decoder.rs, decoder_ffmpeg.rs, backend.rs}` & `plugins/hera_dj/audio_transport.rs`:**
  - Add safe stereo `48 kHz` `ffmpeg` fallback (`CREATE_NO_WINDOW`, `#![forbid(unsafe_code)]`) for `.webm`/`.mkv`.
  - Expose `load_file_with_cues(deck, path, known_cues_sec)` on `AudioBackend` so `native_audio_load_track` maps precomputed cues in `O(1)` (`< 80 ms`) and applies `replay_gain_db` via `TransportCommand::SetGain(DeckId::DeckA, linear_gain)`.
- **`hera-rs/src/scanner/scheduler.rs` & `plugins/hera_dj/scanner_worker.rs`:**
  - Track `completed_level: HashMap<String, i8>` (`-1` quarantined, `0` indexed, `1` fast, `2` full) so `FAST -> FULL` promotions re-enqueue cleanly in `O(1)` while skipping `-1` and `>= target`.
  - Add `external_booth_mode` and `on_battery_power` flags to `AutoGigGovernor` (`1` worker, freezes `P3FullElastic`), set worker OS priority to `THREAD_PRIORITY_BELOW_NORMAL` / `nice(10)`, wrap decode passes in `catch_unwind` writing `scan_level = -1` (`CORRUPT_FILE`), and clear background queues in `< 100 ms` on `cancel_library_scan`.

### 4.2 Zero-IO Single-Pass Acoustic Harvest, 8 Hot Cues, BPM Cross-Audit & Shazam ID
- **`hera-rs/src/scanner/audio_metrics.rs` (`<= 349` LOC):**
  - `build_rgb_waveform_blob(samples, sr, 256) -> Vec<u8>` (`768` bytes `[sub, mid, air]` via `180 Hz` / `3.2 kHz` IIR split, 98th-percentile normalized).
  - `compute_loudness_and_lead_in(samples, sr) -> AudioLoudnessMetrics` (ITU-R BS.1770 K-weighted `loudness_lufs`, `true_peak_dbfs`, `-11.0 LUFS` `replay_gain_db` clamped `[-9.0, +6.0] dB` with `-0.3 dBFS` peak ceiling, and `first_audible_sec > -42.0 dBFS`).
  - `quantize_embedding_64d(&[f32]) -> [i8; 64]`, `cosine_similarity_i8` (`< 1 ms`), and `check_stereo_sub_mono_phase` (`< 120 Hz` Pearson correlation, flagging `< 0.25` as `CLUB_MONO_PHASE_RISK`).
- **`hera-rs/src/cue_engine.rs` (`<= 349` LOC) & `fast_library_scan.rs`:**
  - Remove `.take(4)` in `fast_library_scan.rs` to preserve up to `8` cues (`CueOrigin::DjManual` vs `CueOrigin::HeraAuto`).
  - Anchor `Cue 1` at the first percussive downbeat `>= first_audible_sec`; reject empty breakdown troughs; place structural cues on `Pads 5..8` (`>= 50%` duration) calibrated by per-genre learned ratios (`GenreCueTemplate`, honoring the DJ's `Cue 5` & `Cue 8` placement).
- **BPM Consensus Cross-Audit (`latin_heuristics.rs`, `fast_seek.rs`, `full_pass.rs`):**
  - Cross-check external `122–145 BPM` tags on Salsa/tumbao signatures to fold `4/3x` aliases back into `82–108 BPM`, protect `Tecnomerengue` (`130–160 BPM`) from half-time folding, populate `entry_bpm` / `exit_bpm` without a 3rd STFT in `fast_seek.rs`, and invoke `scan_pcm_for_cd_skips` in `full_pass.rs`.
- **`hera-rs/src/shazam_id/{mod.rs, constellation.rs, matcher.rs}` (`<= 349` LOC each):**
  - Extract spectrogram peak landmark hashes `(f1, f2, dt)` -> `u64` and match against (1) local `acoustic_landmarks` + VirtualDJ index, (2) HERA P2P Swarm fingerprint index, and (3) optional online AcoustID/MusicBrainz lookup, updating SQLite metadata with zero disk renames.

### 4.3 SQLite Migration `08`, DB-First Discovery & `< 1 ms` Agent Queries
- **`hera-desktop/src-tauri/src/migrations/08_add_zero_io_and_curation_intelligence.sql`:**
  - Add columns (`sample_rate_hz`, `bitrate_kbps`, `legacy_bpm`, `has_bpm_trust_diff`, `has_intro_vocals`, `has_outro_vocals`, `is_own_production`, `loudness_lufs`, `true_peak_dbfs`, `replay_gain_db`, `sub_phase_corr`, `waveform_peaks_b64`, `embedding_64d_b64`, `inferred_genre_family`, `inferred_era_year`, `genre_confidence`), index `idx_tracks_harmonic (camelot, bpm)`, and tables `producer_watch_folders` + `acoustic_landmarks`.
  - Apply `PRAGMA mmap_size = 268435456;` in `plugins/db.rs::setup()` and use dynamic `sqlx::QueryBuilder` / `sqlx::query` exclusively (preserving `.sqlx/*.json` offline cache).
- **DB-First Workable Discovery (`plugins/hera_dj/workability.rs`):**
  - `discover_workable_roots_from_existing_dbs()` extracts parent media roots directly from `VirtualDJ/database.xml`, `Rekordbox`, and `Traktor` in `< 1 s`, reporting counts of workable Audio/Video tracks (`25s..=600s`), logical Mixes (`> 600s`), and skipped `< 25s` SFX.
- **Indexed Agent Queries (`libs/db_scanner_queries.rs`):**
  - `query_agent_tracks_filtered` replaces `db.get_all_tracks()` in `tools_library.rs` and `tools_playlist_vdj.rs`, enforcing `scan_level >= 0 AND has_audio_skips = 0` and filtering by genre family, era range, BPM window, Camelot compatibility, and duration (`<= 600s` for standard crates).

### 4.4 `Genre × Era` Curation Engine & DJ Super-Prompt (`hera-rs/src/genre_intelligence/`)
- **14 Booth Genre Families (`taxonomy.rs`, `era_detector.rs`, `acoustic_classifier.rs`):**
  - `Tecnomerengue`, `MerenguetonModerno`, `MerengueClasico`, `SalsaBaulRomantica`, `SalsaBravaChoque`, `ReggaetonOldSchool`, `ReggaetonActualPerreo`, `DembowReparto`, `AfroHouse`, `GuarachaAleteoTech`, `Bachata`, `CumbiaVallenato`, `HoraLocaGaitas`, `LlaneraDespecho`.
  - 3-tier classification: `< 1 ms` curated artist/folder/tag lexicon + 4-band rhythmic/Candle prior for untagged files + optional batch agent enrichment in SQLite.
- **Holistic Synergy & Crate Wiring (`synergy.rs`, `turn_agy.rs`, `prompts.rs`, `tools_playlist_vdj.rs`):**
  - Evaluate `effective_out_bpm_a = a.exit_bpm.unwrap_or(a.bpm)` against `effective_in_bpm_b = b.entry_bpm.unwrap_or(b.bpm)` (leaving legacy `None` inputs bit-identical for `golden_masters.rs`).
  - Apply strict genre-family isolation and `> 15-year` cross-era clash penalties unless `crossover_mode == true` or bridged via a `is_transition_candidate` track.
  - Wire `create_playlist` and `smart_intent.rs` through `optimize_crate_with_dj_rules` + active SQLite `dj_golden_pairs` / `dj_excluded_tracks`, and inject the structured DJ Curation Super-Prompt into `turn_agy.rs` and `dj_brain/prompts.rs`.

### 4.5 Live `VirtualDJ/History` Bridge, Ableton Watcher, Pre-Gig Audit & BETA Onboarding UI
- **`plugins/hera_dj/folder_watcher.rs` & `scanner_intelligence.rs` (`<= 349` LOC each):**
  - Poll `VirtualDJ/History/*.m3u` every `2.5 s`: enable `external_booth_mode` (`360 s` TTL), promote active track (`P0`) + 15 harmonic neighbors (`P1`), update `libs/tray.rs` Now-Playing tooltip + `MiniOverlay`, and auto-upsert `dj_golden_pairs` when consecutive live tracks play `45s..420s`.
  - Stabilized `1,500 ms` folder watcher indexing closed files, marking `producer_watch_folders` / `Bounces` exports as `is_own_production = 1`, running `check_stereo_sub_mono_phase`, and promoting to `P0 FULL`.
  - Expose and ACL-register (`plugins/hera_dj/mod.rs`, `build.rs`, `capabilities/main.json`, `EXPECTED_TAURI_COMMANDS_COUNT`) commands for crate promotion, doctor issues, external booth mode, `64D` Sonic Twins (`< 1 ms`), producer folders, `Check Pre-Toque` (`VirtualDJ` / `USB CDJ` / `Denon 44.1 kHz`), DB-First root discovery, and Shazam identification.
- **Studio UI (`hera-desktop/src/`):**
  - Complete `WaveformPeaksCanvas.tsx` `768 B` tricolor RGB rendering + `native-audio-transport.ts` `replayGainDb` forwarding (zero feature additions in `player.ts`), `PreGigAuditPopover.tsx` in `HeraDjToolbar.tsx`, Vocal Clash & Transition BPM chips in `TransitionBridgeStepsCard.tsx`, context-menu *Sonic Twins* & *Shazam ID* in `TrackList.tsx`, permanent `BETA` badge in `Navigation.tsx`, and mount the interactive 3-step `OnboardingWizardModal.tsx` in `routes/__root.tsx`.
