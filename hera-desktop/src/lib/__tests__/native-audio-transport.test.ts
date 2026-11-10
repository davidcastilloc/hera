import { afterEach, describe, expect, it, vi } from 'vitest';

import type { NativeAudioTelemetryFrame } from '../../generated/typings';
import { NativeRustAudioTransport } from '../native-audio-transport';
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
});
