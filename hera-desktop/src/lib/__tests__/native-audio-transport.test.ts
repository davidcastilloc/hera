import { afterEach, describe, expect, it, vi } from 'vitest';

import { NativeRustAudioTransport } from '../native-audio-transport';

describe('NativeRustAudioTransport', () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('drives deterministic clock fallback when Tauri runtime is absent', async () => {
    vi.useFakeTimers();
    const transport = new NativeRustAudioTransport({
      playbackRate: 1,
      volume: 0.8,
      muted: false,
    });

    const events: string[] = [];
    transport.on('loadstart', () => events.push('loadstart'));
    transport.on('play', () => events.push('play'));
    transport.on('pause', () => events.push('pause'));
    transport.on('ended', () => events.push('ended'));
    transport.on('volumechange', () => events.push('volumechange'));

    await transport.loadTrack('/music/test-track.flac', 0.25);
    expect(transport.hasTrack()).toBe(true);
    expect(transport.paused).toBe(true);

    await transport.play();
    expect(transport.paused).toBe(false);

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
});
