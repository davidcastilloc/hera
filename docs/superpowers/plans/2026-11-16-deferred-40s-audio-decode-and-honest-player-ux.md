# Deferred 40s Audio Decode & Honest Booth Player UX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Gate heavy full-track audio decoding and `P0Immediate` DSP promotion behind `>= 40.0s` of real played audio (or an explicit out-of-window seek), eliminate FIFO stale-click playback queues, and enforce honest Dark Booth Studio (`#0c0e12` / `#14b8a6`) loading/buffering UI that never animates playback during silence.

**Architecture:** `hera-node-rs::audio_engine` loads a `< 50 ms` `3.0s` instant prefix and a lightweight `50.0s` linear-resampled audition buffer while tracking cumulative rendered frames (`played_frames`) on `SharedAudioAtomics`; full-track (`<= 900.0s`) decoding and `rubato` resampling remain dormant until `played_frames >= 40 * sample_rate` or a seek targets `>= buffered_frames`, holding the playhead steady with `is_buffering = true` on underrun. Tauri IPC (`audio_transport.rs`) and `Player` (`player.ts`) enforce end-to-end `latest-wins` sequence cancellation, defer `scanner_notify_deck_track` to `>= 40.0s` played, strip hover `P0Immediate` spam and `__MACOSX`/`._*` junk scans, and bind transport UI indicators to real audio flow.

**Tech Stack:** Rust (`hera-node-rs`, `hera-ipc`, `tauri` v2, `symphonia`, `rubato`, `rtrb`, `ts-rs`), TypeScript, React 19, StyleX, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-16-deferred-40s-audio-decode-and-honest-player-ux-design.md`

## Global Constraints

- Every Rust source file MUST stay `<= 349` lines (`ratchet_monolith_max_lines_ceiling`); `hera-rs/src/audio_engine/hub.rs` is near ceiling, so new commit-gate state/watcher logic lives in `hera-rs/src/audio_engine/commit_gate.rs`.
- `#![forbid(unsafe_code)]`, zero `Mutex`, zero heap allocation, and zero I/O inside `DualDeckEngine::render_interleaved_stereo` and `RealtimeRenderWorker::render_stereo_block`.
- Constants pinned by spec: `PREFIX_DECODE_SECS = 3.0` (`<= 3.5`), `AUDITION_WINDOW_SECS = 50.0`, `FULL_DECODE_COMMIT_PLAYED_SECS = 40.0`, `MAX_FULL_IN_MEMORY_DECODE_SECS = 900.0`.
- Dedicated Dark Booth Studio palette (`#0c0e12` ground, `#12161d` transport bar, `#14b8a6` accent, `#edeff3` / `#8e98a8` ink), professional Spanish booth copy (`Cargando pista…`), crisp SVG icons, zero emojis, zero layout shift (`CLS = 0`), zero HTML `<audio>` elements (`ratchet_player_ts_zero_html_audio_element`).
- Zero OS-specific absolute host paths in committed code, tests, docs, or plans (`ratchet_zero_os_absolute_paths_in_docs_and_plans`).

## Review Focus

- Rapid DJ track browsing (`< 40.0s` played per track across multiple switches): must decode at most `AUDITION_WINDOW_SECS` (`50.0s`), never trigger full-track (`<= 900.0s`) `rubato` decode, and never call `scanner_notify_deck_track`.
- Explicit seek or Hot Cue jump past `50.0s` (e.g. `52.0s` on a `65.0s` track) before `40.0s` of playback: must immediately wake the commit gate, hold `playhead_frame` steady at target with `is_buffering == true` (zero silent playhead drift), and resume non-zero audio once upgraded.
- Paused wall-clock idle time (`10.0s` played + `35.0s` paused): must NOT trigger the `40.0s` commit gate in either Rust (`played_frames`) or TypeScript (`playedAccumSec`).
- Burst of 5 rapid double-clicks while disk/IPC is slow: stale loads `1..4` must drop before Symphonia decode in `native_audio_load_track` and `Player.setTrack()` must return `false` so stale `start()`/`next()`/`previous()` calls never invoke `play()`.
- macOS `__MACOSX` folders and `._Track.wav` (`188-byte` AppleDouble sidecars): must be rejected by path filter before `Symphonia` or `FFmpeg` runs.

---

### Task 1: Rust Audio Engine — `40s` Played-Frames Commit Gate, `50s` Audition Window & Honest `is_buffering` Underrun

**Files:**
- Create: `hera-rs/src/audio_engine/commit_gate.rs`
- Modify: `hera-rs/src/audio_engine/mod.rs`
- Modify: `hera-rs/src/audio_engine/mixer.rs`
- Modify: `hera-rs/src/audio_engine/render_worker.rs`
- Modify: `hera-rs/src/audio_engine/dsp_chain.rs`
- Modify: `hera-rs/src/audio_engine/backend.rs`
- Modify: `hera-rs/src/audio_engine/hub.rs`
- Modify: `hera-rs/src/audio_engine/prefix_loader.rs`
- Test: `hera-rs/src/audio_engine/prefix_tests.rs`

**Interfaces:**
- Produces:
  - `hera_node_rs::audio_engine::prefix_loader::{PREFIX_DECODE_SECS, AUDITION_WINDOW_SECS, FULL_DECODE_COMMIT_PLAYED_SECS}` (`3.0`, `50.0`, `40.0`)
  - `AudioTelemetryFrame.is_buffering: bool` and `AudioTransportSnapshot.is_buffering: bool`
  - `DeckState.buffering: bool` and `DeckState.played_frames_since_load: u64` (reset on `replace_deck` / `replace_deck_prefix`, preserved on `upgrade_deck_seamless`)
  - `SharedAudioAtomics.{buffered_frames: AtomicU64, played_frames: AtomicU64, is_buffering: AtomicBool, force_full_decode: AtomicBool}`

- [x] **Step 1: Write failing engine tests for `40s` commit gate, out-of-window seek wake, and zero silent playhead drift in `hera-rs/src/audio_engine/prefix_tests.rs`**

Add three deterministic `HeadlessClockAudioBackend` tests on a `65.0s` `44.1 kHz` WAV (`write_test_pcm16_wav(&wav_path, 44_100, 65.0, 220.0)`):
1. `test_full_decode_stays_dormant_under_40s_played_and_upgrades_after_40s`:
   - Load `65.0s` WAV via `load_file_fast`, issue `Play(DeckId::DeckA)`, render `20.0s` of frames (`20 * 48_000`), sleep `60ms`, assert `!backend.is_stream_complete()` and `backend.deck_buffer_frames(DeckId::DeckA).unwrap() <= (51.0 * 48_000.0) as u64` (proving `<= 50s` audition buffer is active and full `65s` decode did NOT run even if paused/sleeping).
   - Render another `21.0s` (`total played = 41.0s >= 40.0s`), wait up to `5s` for `backend.is_stream_complete()`, assert `backend.deck_buffer_frames(DeckId::DeckA).unwrap() >= (64.9 * 48_000.0) as u64`, and render through `65.0s` EOF with non-zero samples.
2. `test_seek_beyond_audition_window_holds_playhead_with_is_buffering_and_wakes_full_decode`:
   - Load `65.0s` WAV, `Play(DeckId::DeckA)`, immediately `SeekToSample(DeckId::DeckA, 55 * 48_000)` (`55.0s > 50.0s` audition window with `0s` played).
   - Before upgrade finishes, render `4096` frames in deterministic mode without sleeping: assert rendered samples are `0.0`, `snap.is_buffering == true`, and `snap.playhead_frame == 55 * 48_000` (playhead MUST NOT drift forward while silent!).
   - Wait up to `5s` for `backend.is_stream_complete()`, render `4096` frames, assert `snap.is_buffering == false` and non-zero audio resumes from `55.0s`.
3. Update `test_prefix_loader_starts_fast_and_seamlessly_upgrades_past_prefix_to_eof` and `test_prefix_loader_cancels_stale_generation_on_rapid_track_switch` so tracks `<= 50.0s` (`30.0s` WAV) complete within the `50.0s` audition window (`is_stream_complete() == true`) while a `65.0s` switch test cancels stale generation cleanly.

- [x] **Step 2: Run `hera-node-rs` prefix tests to verify they fail**

Run: `cargo test -p hera-node-rs prefix_tests`
Expected: FAIL (`is_buffering` field missing and `65.0s` track decodes immediately before `40.0s` played).

- [x] **Step 3: Implement honest underrun (`buffering` + held playhead) and `played_frames_since_load` in `mod.rs`, `mixer.rs`, `dsp_chain.rs`, `backend.rs`, and `render_worker.rs`**

1. In `hera-rs/src/audio_engine/mod.rs`: add `buffering: bool` and `played_frames_since_load: u64` to `DeckState` (reset both to `false` / `0` in `replace_deck` and `replace_deck_prefix`; set `buffering = false` while preserving `played_frames_since_load` in `upgrade_deck_seamless`; expose `pub fn is_deck_buffering(&self, deck: DeckId) -> bool` and `pub fn deck_played_frames(&self, deck: DeckId) -> u64`).
2. In `hera-rs/src/audio_engine/mixer.rs`: when `idx >= total_frames` (or `idx0 >= total_frames`) and `!deck.stream_complete`, set `deck.buffering = true` and **keep `deck.playhead_frames` unchanged** (remove `(idx + rem)` / `+ rem * step` silent drift!). When real frames `rendered > 0` are mixed, set `deck.buffering = false` and increment `deck.played_frames_since_load = deck.played_frames_since_load.saturating_add(rendered as u64)`.
3. Add `pub is_buffering: bool` to `AudioTelemetryFrame` (`dsp_chain.rs`) and `AudioTransportSnapshot` (`backend.rs`).
4. In `render_worker.rs`: store `buffered_frames`, `played_frames`, and `is_buffering` on `SharedAudioAtomics` and pass `is_buffering` into `analyzer.analyze_block(...)`.

- [x] **Step 4: Implement `commit_gate.rs` + two-stage loader (`3.0s` instant -> `50.0s` audition -> `>= 40.0s` or out-of-window seek full upgrade) in `hub.rs` and `prefix_loader.rs`**

1. Create `hera-rs/src/audio_engine/commit_gate.rs` (`<= 140` LOC) containing `wait_for_commit_or_cancel(atomics: &SharedAudioAtomics, generation: u64, cancel: &AtomicBool, threshold_frames: u64) -> bool` which wakes via `Condvar` (or `15 ms` bounded park loop checking `cancel`, `load_generation != generation`, `force_full_decode`, or `played_frames >= threshold_frames`).
2. In `hub.rs` (keep `<= 349` LOC): add `buffered_frames`, `played_frames`, `is_buffering`, `force_full_decode` to `SharedAudioAtomics`; in `send_command` for `SeekToSample(DeckA, frame)` / `JumpToHotCue(DeckA, cue)`, if `!stream_complete && target >= buffered_frames`, set `force_full_decode.store(true, Ordering::Release)` and `is_buffering.store(true, Ordering::Release)`; add `upgrade_audition_buffer_if_generation` (sets `buffered_frames`; marks `stream_complete = true` if `eof_reached`).
3. In `prefix_loader.rs`:
   - Set `PREFIX_DECODE_SECS = 3.0`, `AUDITION_WINDOW_SECS = 50.0`, `FULL_DECODE_COMMIT_PLAYED_SECS = 40.0`.
   - Stage 1 (sync): decode `3.0s`, fast-linear resample, install prefix.
   - Background worker (`hera-deck-loader`):
     - Stage 2A (audition window): decode up to `50.0s`, fast-linear resample (`resample_deck_buffer_fast_prefix`), call `upgrade_audition_buffer_if_generation`. If `audition.eof_reached`, mark `stream_complete = true` and return immediately (zero `rubato` / `rustfft`!).
     - Stage 2B (`40s` commit gate): call `wait_for_commit_or_cancel(..., (40.0 * f64::from(target_sr)) as u64)`. Return immediately if cancelled/superseded.
     - Stage 3 (full decode): decode up to `MAX_FULL_IN_MEMORY_DECODE_SECS` (`900.0s`), `resample_deck_buffer_to_rate`, apply known or proportional cues, and call `upgrade_buffer_if_generation` **immediately** before any optional `compute_cues_seconds` so audio never waits on `rustfft`.

- [x] **Step 5: Run `hera-node-rs` tests and architectural ratchets**

Run: `cargo test -p hera-node-rs prefix_tests` and `cargo test -p hera-node-rs --test architectural_ratchets`
Expected: PASS (`0 failed`, all Rust files `<= 349` LOC).

- [x] **Step 6: Commit Task 1**

```bash
git add hera-rs/src/audio_engine/
git commit -S -m "feat(audio): gate full-track decode behind 40s played audio and hold playhead on underrun"
```

---

### Task 2: Tauri IPC Latest-Wins Load Gate, `is_buffering` DTOs & `__MACOSX` / `._*` Scanner Hygiene

**Files:**
- Modify: `hera-desktop/src-ipc/src/audio_types.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/audio_transport.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/audio_telemetry.rs`
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj/workability.rs`
- Modify: `hera-desktop/src-tauri/src/libs/fast_library_scan.rs`
- Modify: `hera-desktop/src/generated/hera-typings.ts`
- Test: `hera-desktop/src-tauri/src/plugins/hera_dj/tests.rs`

**Interfaces:**
- Consumes: `AudioTelemetryFrame.is_buffering: bool`, `AudioTransportSnapshot.is_buffering: bool` from Task 1.
- Produces:
  - `NativeAudioTelemetryFrame.is_buffering: bool` and `NativePlayerSnapshot.is_buffering: bool` (`ts-rs` exported to `hera-typings.ts`)
  - `NativeAudioState { pub backend: Arc<dyn AudioBackend>, pub load_seq: Arc<AtomicU64> }` (with `NativeAudioState::new(Arc<dyn AudioBackend>)`)
  - `pub fn is_junk_audio_path(path: &Path) -> bool` in `workability.rs`

- [ ] **Step 1: Write failing tests for latest-wins IPC cancellation and `__MACOSX` / `._*` sidecar filtering in `hera-desktop/src-tauri/src/plugins/hera_dj/tests.rs`**

1. Add `test_is_junk_audio_path_rejects_macosx_and_appledouble_sidecars`:
   - Assert `workability::is_junk_audio_path(Path::new("Music/Pack/__MACOSX/Pack/._Fill_01.wav")) == true`.
   - Assert `workability::is_junk_audio_path(Path::new("Music/Pack/._Kick_128.mp3")) == true`.
   - Assert `workability::is_junk_audio_path(Path::new("Music/Pack/Kick_128.wav")) == false`.
2. Add `test_latest_wins_load_gate_skips_superseded_load_before_decode`:
   - Create `NativeAudioState::new(...)`, reserve `seq_1 = state.next_load_seq()` and `seq_2 = state.next_load_seq()`, call `state.load_track_if_current(seq_1, Path::new("missing_never_touched.wav"), None, None, None)`: assert it returns `Ok(None)` (superseded in `0 ms` without attempting file decode!).
3. Assert `map_player_snapshot` propagates `is_buffering`.

- [ ] **Step 2: Run `hera-desktop` unit tests to verify they fail**

Run: `cargo test -p hera-desktop --bins test_is_junk_audio_path_rejects_macosx`
Expected: FAIL (`is_junk_audio_path` and `next_load_seq` not defined).

- [ ] **Step 3: Add `is_buffering` to `hera-ipc`, implement `load_seq` gate in `audio_transport.rs`, and reject `__MACOSX` / `._*` in `workability.rs` + `fast_library_scan.rs`**

1. In `hera-desktop/src-ipc/src/audio_types.rs`: add `#[serde(default)] pub is_buffering: bool` to `NativeAudioTelemetryFrame` and `NativePlayerSnapshot`.
2. Update `map_telemetry_frame`, `map_player_snapshot` (`audio_transport.rs`), and `synthesize_telemetry_frame` (`audio_telemetry.rs`) to populate `is_buffering`.
3. In `audio_transport.rs`: add `load_seq: Arc<AtomicU64>` to `NativeAudioState` (`pub fn new(backend: Arc<dyn AudioBackend>) -> Self`, `pub fn next_load_seq(&self) -> u64`, `pub fn load_track_if_current(...) -> Result<Option<NativePlayerSnapshot>, String>`). In `native_audio_load_track`, increment `seq` **before** `spawn_blocking`; inside `spawn_blocking`, if `seq != load_seq.load(Ordering::Acquire)` before or after `load_file_fast`, return `Ok(map_player_snapshot(backend.snapshot(), None))` without touching `SetGain` or decoding stale files. Update `NativeAudioState(...)` construction site in `plugins/hera_dj/mod.rs`.
4. In `workability.rs`: extend `is_junk_system_directory` with `"__macosx"` and add `pub fn is_junk_audio_path(path: &Path) -> bool` returning `true` when `is_junk_system_directory(path)` or `path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("._") || n.eq_ignore_ascii_case(".ds_store"))`. Apply `is_junk_audio_path` in `fast_library_scan.rs` directory walk and `plugins/db.rs` track import filter.

- [ ] **Step 4: Regenerate TypeScript bindings and run `hera-ipc` + `hera-desktop` tests**

Run: `cargo test -p hera-ipc` and `cargo test -p hera-desktop --bins -- --skip export_bindings`
Expected: PASS (`hera-typings.ts` updated with `is_buffering: boolean`, all unit tests green).

- [ ] **Step 5: Commit Task 2**

```bash
git add hera-desktop/src-ipc/src/audio_types.rs hera-desktop/src/generated/hera-typings.ts hera-desktop/src-tauri/src/
git commit -S -m "fix(ipc,scanner): enforce latest-wins track load gate and skip __MACOSX/._ sidecars"
```

---

### Task 3: Frontend Transport & Honest Dark-Booth Player UX (`>= 40s` Scanner Commit, Stale-Play Guard & Buffering UI)

**Files:**
- Modify: `hera-desktop/src/lib/native-audio-ipc.ts`
- Modify: `hera-desktop/src/lib/native-audio-transport.ts`
- Modify: `hera-desktop/src/lib/spectrum-renderer.ts`
- Modify: `hera-desktop/src/lib/player.ts`
- Modify: `hera-desktop/src/components/PlayingIndicator.tsx`
- Modify: `hera-desktop/src/components/PlayingBarInfo.tsx`
- Modify: `hera-desktop/src/components/TrackProgress.tsx`
- Modify: `hera-desktop/src/components/TrackRow.tsx`
- Test: `hera-desktop/src/lib/__tests__/native-audio-transport.test.ts`
- Test: `hera-desktop/src/components/__tests__/scanner-realtime-ux.test.tsx`

**Interfaces:**
- Consumes: `NativePlayerSnapshot.is_buffering: boolean` and `NativeAudioTelemetryFrame.is_buffering: boolean` from Task 2.
- Produces:
  - `PlayerState.{isLoadingTrack: boolean, loadingTrackId: string | null, isBuffering: boolean}`
  - `Player.setTrack(track: Track): Promise<boolean>` (`true` iff this call is still the latest `trackLoadSeq`)
  - `NativeRustAudioTransport.{isBuffering: boolean, isLoading: boolean}` + `bufferingchange: (buffering: boolean) => void` event

- [ ] **Step 1: Write failing Vitest tests for rapid-click stale `play()` cancellation, `40s` scanner commit gate, and honest buffering UI**

1. In `hera-desktop/src/lib/__tests__/native-audio-transport.test.ts`:
   - Add test `cancels stale start() clicks so only the final clicked track invokes native_audio_play`: mock `native_audio_load_track` with deferred resolvers for `track1`, `track2`, `track3`; fire `void player.start([t1, t2, t3], t1.id, origin)`, `void player.start([t1, t2, t3], t2.id, origin)`, `const p3 = player.start([t1, t2, t3], t3.id, origin)`; resolve `t1`, then `t2`, then `t3`; await `p3`; assert `native_audio_play` was called **exactly once** (only for `t3`).
   - Add test `defers scanner_notify_deck_track until 40s of real non-paused playback and freezes currentTime when is_buffering is true`: emit telemetry frames at `10s`, `25s`, `39s` -> assert `scanner_notify_deck_track` NOT called; emit `is_buffering: true` at `39s` and advance `performance.now()` by `500ms` -> assert `transport.currentTime === 39` (zero fake drift); emit `is_buffering: false` at `40.5s` -> assert `scanner_notify_deck_track` called once for `t3.id`.
2. In `hera-desktop/src/components/__tests__/scanner-realtime-ux.test.tsx`:
   - Assert hovering `TrackRow` does NOT invoke `plugin:hera-dj|scanner_prewarm_track`.
   - Assert `PlayingIndicator` renders `aria-label="Cargando pista"` (spinner SVG, not bouncing bars) when `isLoadingTrack` or `isBuffering` is `true`.

- [ ] **Step 2: Run Vitest to verify the new tests fail**

Run: `cd hera-desktop && bun run test -- --run src/lib/__tests__/native-audio-transport.test.ts src/components/__tests__/scanner-realtime-ux.test.tsx`
Expected: FAIL.

- [ ] **Step 3: Implement `NativeRustAudioTransport` buffering truth + `Player` latest-wins `setTrack(): Promise<boolean>` and `40s` `notifyDeckTrack` gate**

1. In `native-audio-ipc.ts` & `spectrum-renderer.ts`: add `bufferingchange: (isBuffering: boolean) => void` to `NativeTransportEvents` and `is_buffering: false` to `buildFallbackTelemetryFrame`.
2. In `native-audio-transport.ts`:
   - Track `loadingState = false` and `bufferingState = false` (`get isLoading()`, `get isBuffering()`).
   - In `get currentTime()`: when `this.loadingState || this.bufferingState`, return `this.currentTimeSec` directly with **zero** `performance.now()` extrapolation.
   - In `loadTrack`: set `loadingState = true` (`emit('bufferingchange', true)`), return `boolean` (`false` when `seq !== this.loadSeq`), and clear `loadingState = false` for the winning `seq`.
   - Update `bufferingState` from `snap.is_buffering` and `frame.is_buffering`, emitting `bufferingchange` on transitions.
3. In `player.ts`:
   - Add `isLoadingTrack: boolean`, `loadingTrackId: string | null`, and `isBuffering: boolean` to `PlayerState`.
   - Change `async setTrack(track: Track): Promise<boolean>`: set `loadingTrackId = track.id`, do **not** call `notifyDeckTrack` on load; await `transport.loadTrack(...)`; if `seq !== this.trackLoadSeq`, return `false`; clear `loadingTrackId = null`, emit state change, and return `true`.
   - In `start()`, `next()`, `previous()`, and `startFromQueue()`: write `const ok = await this.setTrack(track); if (!ok) return; await this.play();`.
   - Track cumulative real playback time (`playedAccumSec`, incremented only from positive `timeupdate` deltas `0 < dt <= 2.0` while `!isPaused && !isBuffering && !isLoadingTrack`, reset in `setTrack`); fire `notifyDeckTrack(track.id, [])` once when `playedAccumSec >= 40.0`.

- [ ] **Step 4: Implement Honest Dark Booth Studio UX in `PlayingIndicator.tsx`, `PlayingBarInfo.tsx`, `TrackProgress.tsx`, and `TrackRow.tsx`**

1. `PlayingIndicator.tsx`: read `isLoadingTrack` and `isBuffering` from `usePlayerState`; when `isLoadingTrack || isBuffering`, render a crisp `12px` `#14b8a6` rotating SVG arc (`aria-label="Cargando pista"`) instead of the 3-bar equalizer animation.
2. `PlayingBarInfo.tsx`: when `isLoadingTrack || isBuffering`, render a compact inline studio status pill (`Cargando pista…`, `#2dd4bf` ink on `rgba(20, 184, 166, 0.12)` background, `11px` tabular, zero layout shift) next to the title.
3. `TrackProgress.tsx`: pause the `requestAnimationFrame` loop whenever `isPaused || isLoadingTrack || isBuffering`, and apply a subtle `opacity: 0.65` `#14b8a6` pulse on `Slider.Indicator` while buffering.
4. `TrackRow.tsx`: remove `onMouseEnter={() => scheduleHoverPrewarm(track.id)}` and `onMouseLeave={cancelHoverPrewarm}`; display `<PlayingIndicator />` when `props.isPlaying || loadingTrackId === track.id` so double-clicking gives `< 16 ms` visual feedback on the exact row clicked.

- [ ] **Step 5: Run Vitest, TypeScript check, Knip, and Impeccable mechanical detector**

Run: `cd hera-desktop && bun run check && bun run knip && bun run test -- --run` and `.agents/skills/impeccable/scripts/impeccable.cmd detect --json hera-desktop/src/components/PlayingIndicator.tsx hera-desktop/src/components/PlayingBarInfo.tsx hera-desktop/src/components/TrackProgress.tsx hera-desktop/src/components/TrackRow.tsx`
Expected: PASS (`exit code 0`, zero detector violations).

- [ ] **Step 6: Commit Task 3**

```bash
git add hera-desktop/src/
git commit -S -m "feat(player-ui): defer scanner P0 to 40s played, drop stale clicks, and show honest buffering UX"
```

---

### Task 4: Full Polyglot Verification & Release Binary Push from `beast` to `qa`

**Files:**
- Modify: `docs/superpowers/DECISIONS.md` (record `40s` audio/scanner commit-gate & honest player UX decision)

**Interfaces:**
- Consumes: Tasks 1–3 deliverables.
- Produces: Verified `--release` `hera-desktop.exe` built on `beast` and running live against `qa`'s music library.

- [ ] **Step 1: Run full workspace verification on `beast`**

Run:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p hera-ipc`
- `cargo test -p hera-node-rs --test architectural_ratchets`
- `cargo test -p hera-node-rs -- --test-threads=4`
- `cargo test -p hera-desktop --bins -- --skip export_bindings`
- `cd hera-desktop && bun run gen:types && bun run check && bun run test -- --run && bun run build`
Expected: All commands exit `0`.

- [ ] **Step 2: Record architectural decision in `docs/superpowers/DECISIONS.md` and commit/push**

Append a concise entry documenting the `40s` played-audio commit gate, latest-wins IPC cancellation, and honest `is_buffering` transport contract (zero OS-specific paths), run `cargo test -p hera-node-rs --test architectural_ratchets`, commit signed, and `git push origin main`.

- [ ] **Step 3: Build `--release` `hera-desktop.exe` on `beast` and launch on `qa` via `hc`**

1. Build native release binary + frontend dist on `beast`: `cd hera-desktop && bun run build` and `cargo build --release -p hera-desktop`.
2. Stop the slow `debug` `tauri dev` session on `qa` (`qa/t1q`), sync `qa` (`git pull`) + copy `target/release/hera-desktop.exe` to `qa`, and launch the optimized release `hera-desktop.exe` on `qa` via `hc` for live DJ verification.
