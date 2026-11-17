# Deferred 40s Audio Decode & Honest Booth Player UX — Design Spec

**Date:** 2026-11-16  
**Repository:** `davidcastilloc/hera` (`hera-rs` + `hera-ipc` + `hera-desktop`)  
**Status:** Approved for Planning

## 1. Problem Statement (Observed on `qa` Real DJ Library)

During live library scanning and rapid track auditioning on `qa`:
1. **Silent Underrun with Fake Animation (`~2.5s`):** `prefix_loader.rs` decoded only `2.5s` upfront while spawning a heavy full-track decode + `rustfft` cue analysis + `rubato` sinc resample at `t = 0s`. Under load, `2.5s` ran out before upgrade finished; `mixer.rs` output `0.0` (silence) while still incrementing `playhead_frames`, and `PlayingIndicator.tsx` / `TrackProgress.tsx` kept animating as if audio were playing.
2. **FIFO Click-Queue Lag:** Rapidly clicking multiple tracks queued uncancellable `spawn_blocking` decodes in `audio_transport.rs`, while `Player.start()` / `next()` / `previous()` / `startFromQueue()` called `await this.play()` unconditionally after `await this.setTrack(track)` even when superseded by a newer click. Hovering rows also spammed `P0Immediate` via `scheduleHoverPrewarm`.
3. **macOS Sidecar Junk Storm:** `__MACOSX` folders and `._*` AppleDouble `188-byte` files triggered doomed Symphonia + FFmpeg subprocesses during background scans.

## 2. Core Product & Engineering Invariants

1. **`40s` Played-Audio Commit Gate (`FULL_DECODE_COMMIT_PLAYED_SECS = 40.0`):**
   - Clicking a track decodes a `< 50 ms` instant prefix (`INSTANT_PREFIX_SECS = 3.0`) and immediately extends an audition window (`AUDITION_WINDOW_SECS = 50.0`) using lightweight linear resampling (`resample_deck_buffer_fast_prefix`) with **zero** `rubato` sinc resampling, **zero** `rustfft` structure analysis, and **zero** `P0Immediate` scanner promotion.
   - Heavy full-track decode (`<= 900.0s` + `resample_deck_buffer_to_rate`) and `scanner_notify_deck_track` (`P0Immediate`) run **only** after the track accumulates **`>= 40.0s` of real rendered playback** (paused wall-clock excluded) **or** when the DJ explicitly seeks / jumps to a Hot Cue beyond the currently buffered frames.
   - When full-track decode runs, `upgrade_buffer_if_generation` swaps the audio buffer into `DeckA` **before** any optional Hot Cue DSP computation.
2. **Strict Latest-Wins Cancellation (`0 ms` Stale Drop):**
   - `NativeAudioState` tracks an `AtomicU64` request sequence checked before and inside `spawn_blocking` prior to decoding.
   - `Player.setTrack(track)` returns `Promise<boolean>` (`false` when superseded) so callers never invoke `play()` for stale clicks.
   - `TrackRow` does not promote hovered rows to `P0Immediate`.
3. **Honest Dark Booth Studio UX (`#0c0e12` / `#14b8a6`, Zero Fake Motion, Zero Emojis):**
   - When `!deck.stream_complete && idx >= buffered_frames`, `mixer.rs` holds `playhead_frames` steady and sets `is_buffering = true` on `AudioTransportSnapshot` and `AudioTelemetryFrame`.
   - `TrackProgress.tsx`, `PlayingIndicator.tsx`, `PlayingBarInfo.tsx`, and `TrackRow.tsx` freeze playhead extrapolation and equalizer bounce whenever `isLoadingTrack || isBuffering`, displaying a crisp `#14b8a6` studio loading/buffering state (`Cargando pista…`) with zero layout shift (`CLS = 0`).
4. **Scanner Junk Hygiene & 3-Node Build Discipline:**
   - Reject `__MACOSX` directories and `._*` AppleDouble files in `O(1)` path checks before Symphonia/FFmpeg.
   - Compile `--release` binaries on `beast` and push ready `.exe` artifacts to `qa`.
