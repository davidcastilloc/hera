import { afterEach, describe, expect, it, vi } from 'vitest';

import type {
  NativeAudioTelemetryFrame,
  NativePlayerSnapshot,
} from '../../generated/typings';
import * as nativeIpc from '../native-audio-ipc';
import {
  NativeRustAudioTransport,
  registerTrackLoadMetadata,
} from '../native-audio-transport';
import {
  type SpectrumBackendMode,
  SpectrumCanvasRenderer,
} from '../spectrum-renderer';

describe('NativeRustAudioTransport & SpectrumCanvasRenderer', () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('drives deterministic clock fallback and emits 32-bin telemetry frames when Tauri runtime is absent', async () => {
    vi.useFakeTimers();
    const transport = new NativeRustAudioTransport({
      playbackRate: 1,
      volume: 0.8,
      muted: false,
    });

    const events: string[] = [];
    const telemetryFrames: NativeAudioTelemetryFrame[] = [];
    transport.on('loadstart', () => events.push('loadstart'));
    transport.on('play', () => events.push('play'));
    transport.on('pause', () => events.push('pause'));
    transport.on('ended', () => events.push('ended'));
    transport.on('volumechange', () => events.push('volumechange'));
    transport.on('telemetry', (frame) => telemetryFrames.push(frame));

    await transport.loadTrack('/music/test-track.flac', 0.25);
    expect(transport.hasTrack()).toBe(true);
    expect(transport.paused).toBe(true);

    await transport.play();
    expect(transport.paused).toBe(false);
    expect(telemetryFrames.length).toBeGreaterThanOrEqual(1);
    expect(transport.getLatestTelemetry()?.spectrum_bins).toHaveLength(32);

    transport.seek(0.1);
    expect(transport.currentTime).toBeCloseTo(0.1, 2);

    transport.setVolume(0.5);
    expect(transport.volume).toBeCloseTo(0.5, 2);
    transport.setMuted(true);
    expect(transport.muted).toBe(true);

    await vi.advanceTimersByTimeAsync(300);
    expect(transport.paused).toBe(true);
    expect(events).toEqual([
      'loadstart',
      'play',
      'volumechange',
      'volumechange',
      'pause',
      'ended',
    ]);
  });

  it('ingests 32-bin telemetry frames into SpectrumCanvasRenderer without React state churn', () => {
    const fakeCanvas = {
      width: 320,
      height: 12,
      getContext: () => null,
    } as unknown as HTMLCanvasElement;

    const renderer = new SpectrumCanvasRenderer(fakeCanvas);
    const mode: SpectrumBackendMode = renderer.getBackendMode();
    expect(['webgpu', 'webgl2', 'canvas2d', 'headless']).toContain(mode);
    expect(renderer.isUsingOffscreenCanvas()).toBe(false);

    const sampleFrame: NativeAudioTelemetryFrame = {
      sequence: 1,
      sample_rate: 48_000,
      current_time_sec: 4.2,
      duration_sec: 180.0,
      is_playing: true,
      is_buffering: false,
      rms_left: 0.45,
      rms_right: 0.44,
      peak_db: -6.9,
      sub_bass: 0.72,
      low_mid: 0.54,
      high_mid: 0.38,
      air_presence: 0.21,
      spectrum_bins: Array.from({ length: 32 }, (_, idx) =>
        Math.max(0.05, 1 - idx / 32),
      ),
    };

    renderer.pushTelemetryFrame(sampleFrame);
    renderer.renderNow();

    const smoothed = renderer.getSmoothedBins();
    expect(smoothed).toHaveLength(32);
    expect(smoothed[0]).toBeGreaterThan(0.5);
    expect(smoothed[0]).toBeGreaterThan(smoothed[31]);

    renderer.dispose();
  });

  it('forwards >= 4 hot cues (sliced to 4 deck pads) and finite replayGainDb to native_audio_load_track', async () => {
    const mockSnap: NativePlayerSnapshot = {
      backend_kind: 'headless_clock',
      sample_rate: 48_000,
      track_loaded: true,
      loaded_path: '/music/eight-cues.flac',
      is_playing: false,
      is_buffering: false,
      ended: false,
      current_time_sec: 0,
      duration_sec: 210,
      playhead_frame: 0,
      total_frames: 10_080_000,
      volume: 0.8,
      playback_rate: 1,
      hot_cue_frames: [48_000, 768_000, 1_536_000, 3_072_000],
      hot_cue_seconds: [1.0, 16.0, 32.0, 64.0],
      latest_telemetry: null,
    };
    const spy = vi
      .spyOn(nativeIpc, 'invokeNativeAudio')
      .mockResolvedValue(mockSnap);

    const transport = new NativeRustAudioTransport({
      playbackRate: 1,
      volume: 0.8,
      muted: false,
    });

    await transport.loadTrack(
      '/music/eight-cues.flac',
      210,
      [1.0, 16.0, 32.0, 64.0, 96.0, 128.0, 160.0, 192.0],
      -3.5,
    );

    expect(spy).toHaveBeenCalledWith('native_audio_load_track', {
      filePath: '/music/eight-cues.flac',
      durationHintSec: 210,
      hotCuesSec: [1.0, 16.0, 32.0, 64.0],
      replayGainDb: -3.5,
    });

    await transport.loadTrack(
      '/music/no-gain.flac',
      180,
      [1.0, 2.0],
      Number.NaN,
    );
    expect(spy).toHaveBeenCalledWith('native_audio_load_track', {
      filePath: '/music/no-gain.flac',
      durationHintSec: 180,
      hotCuesSec: undefined,
      replayGainDb: undefined,
    });
  });

  it('forwards registered replayGainDb and 8-cue fallback when loadTrack omits optional args', async () => {
    const spy = vi
      .spyOn(nativeIpc, 'invokeNativeAudio')
      .mockResolvedValue(null);
    const transport = new NativeRustAudioTransport({
      playbackRate: 1,
      volume: 0.9,
      muted: false,
    });
    registerTrackLoadMetadata(
      '/music/registered.mp3',
      -4.2,
      [2.5, 32, 64, 128, 160, 180, 195, 210],
    );

    await transport.loadTrack('/music/registered.mp3', 240);

    expect(spy).toHaveBeenCalledWith('native_audio_load_track', {
      filePath: '/music/registered.mp3',
      durationHintSec: 240,
      hotCuesSec: [2.5, 32, 64, 128],
      replayGainDb: -4.2,
    });
  });

  it('cancels stale start() clicks so only the final clicked track invokes native_audio_play', async () => {
    const { default: player } = await import('../player');
    let resolveLoad1!: (v: NativePlayerSnapshot | null) => void;
    let resolveLoad2!: (v: NativePlayerSnapshot | null) => void;
    let resolveLoad3!: (v: NativePlayerSnapshot | null) => void;
    const playCalls: number[] = [];

    vi.spyOn(nativeIpc, 'invokeNativeAudio').mockImplementation((cmd, args) => {
      if (cmd === 'native_audio_load_track') {
        const p = (args as { filePath?: string } | undefined)?.filePath;
        if (p === '/music/t1.mp3') {
          return new Promise((r) => {
            resolveLoad1 = r;
          });
        }
        if (p === '/music/t2.mp3') {
          return new Promise((r) => {
            resolveLoad2 = r;
          });
        }
        return new Promise((r) => {
          resolveLoad3 = r;
        });
      }
      if (cmd === 'native_audio_play') {
        playCalls.push(playCalls.length + 1);
      }
      return Promise.resolve(null);
    });

    const makeTrack = (id: string, path: string) => ({
      id,
      path,
      title: id,
      album: 'Club',
      album_artist: 'DJ',
      artists: ['DJ'],
      genres: [],
      year: 2026,
      duration: 180,
      track_no: 1,
      track_of: 1,
      disk_no: 1,
      disk_of: 1,
      is_compilation: false,
    });
    const tracks = [
      makeTrack('t1', '/music/t1.mp3'),
      makeTrack('t2', '/music/t2.mp3'),
      makeTrack('t3', '/music/t3.mp3'),
    ];

    const p1 = player.start(tracks, 't1', { type: 'library' });
    const p2 = player.start(tracks, 't2', { type: 'library' });
    const p3 = player.start(tracks, 't3', { type: 'library' });

    resolveLoad1(null);
    resolveLoad2(null);
    resolveLoad3(null);
    await Promise.all([p1, p2, p3]);

    expect(playCalls).toHaveLength(1);
    expect(player.getTrack()?.id).toBe('t3');
    expect(player.getState().isLoadingTrack).toBe(false);
    expect(player.getState().loadingTrackId).toBeNull();
    player.stop();
  });

  it('never pokes scanner_notify_deck_track during active playback (even past 40s), freezes currentTime while buffering, and commits unanalyzed track only on pause/stop', async () => {
    const scannerV3 = await import('../scanner-v3');
    const notifySpy = vi
      .spyOn(scannerV3, 'notifyDeckTrack')
      .mockImplementation(() => {});
    const playingSpy = vi
      .spyOn(scannerV3, 'notifyDeckPlaying')
      .mockImplementation(() => {});
    const { default: player } = await import('../player');

    let telemetryCb: ((frame: NativeAudioTelemetryFrame) => void) | null = null;
    vi.spyOn(nativeIpc, 'subscribeNativeTelemetryChannel').mockImplementation(
      (cb) => {
        telemetryCb = cb;
        return true;
      },
    );
    const snap: NativePlayerSnapshot = {
      backend_kind: 'cpal_shared',
      sample_rate: 48_000,
      track_loaded: true,
      loaded_path: '/music/commit40.mp3',
      is_playing: true,
      is_buffering: false,
      ended: false,
      current_time_sec: 0,
      duration_sec: 200,
      playhead_frame: 0,
      total_frames: 9_600_000,
      volume: 1,
      playback_rate: 1,
      hot_cue_frames: [0, 0, 0, 0],
      hot_cue_seconds: [0, 0, 0, 0],
      latest_telemetry: null,
    };
    vi.spyOn(nativeIpc, 'invokeNativeAudio').mockResolvedValue(snap);

    await player.start(
      [
        {
          id: 'commit-40',
          path: '/music/commit40.mp3',
          title: 'Commit 40',
          album: 'Club',
          album_artist: 'DJ',
          artists: ['DJ'],
          genres: [],
          year: 2026,
          duration: 200,
          track_no: 1,
          track_of: 1,
          disk_no: 1,
          disk_of: 1,
          is_compilation: false,
        },
      ],
      'commit-40',
      { type: 'library' },
    );

    // Must NOT call notifyDeckTrack immediately on click/load, and MUST never emit notifyDeckPlaying(false) while loading/playing!
    expect(notifySpy).not.toHaveBeenCalled();
    expect(playingSpy).toHaveBeenCalledWith(true);
    expect(playingSpy).not.toHaveBeenCalledWith(false);
    expect(telemetryCb).not.toBeNull();

    const emitAt = (sec: number, isBuffering = false) => {
      telemetryCb!({
        sequence: Math.floor(sec * 10),
        sample_rate: 48_000,
        current_time_sec: sec,
        duration_sec: 200,
        is_playing: true,
        is_buffering: isBuffering,
        rms_left: 0.4,
        rms_right: 0.4,
        peak_db: -6,
        sub_bass: 0.5,
        low_mid: 0.4,
        high_mid: 0.3,
        air_presence: 0.2,
        spectrum_bins: Array.from({ length: 32 }, () => 0.3),
      });
    };

    // Simulate 39.0s of real 1.0s playback steps (< 40.0s threshold)
    for (let s = 1; s <= 39; s++) {
      emitAt(s, false);
    }
    expect(notifySpy).not.toHaveBeenCalled();

    // When buffering is true at 39.0s, currentTime must stay frozen at 39.0s without extrapolation
    emitAt(39, true);
    expect(player.getState().isBuffering).toBe(true);
    expect(player.getCurrentTime()).toBe(39);

    // Resume real playback past 40.0s -> active playback MUST NOT poke scanner!
    emitAt(39.5, false);
    emitAt(40.5, false);
    emitAt(55.0, false);
    expect(player.getState().isBuffering).toBe(false);
    expect(notifySpy).not.toHaveBeenCalled();

    // Pausing/stopping an unanalyzed track (scan_level === 0) commits it once on idle and notifies deck idle
    player.pause();
    expect(playingSpy).toHaveBeenCalledWith(false);
    expect(notifySpy).toHaveBeenCalledTimes(1);
    expect(notifySpy).toHaveBeenCalledWith('commit-40', []);
    player.stop();
    expect(notifySpy).toHaveBeenCalledTimes(1);
  });
});
