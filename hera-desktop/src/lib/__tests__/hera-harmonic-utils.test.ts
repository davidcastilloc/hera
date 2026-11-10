import { describe, expect, it, vi } from 'vitest';

import {
  buildSyntheticTrackDescriptor,
  CAMELOT_OPTIONS,
  camelotDistance,
  camelotRootHz,
  CUE_DOT_COLORS,
  nextHarmonicCamelot,
  parseCamelot,
  startSyntheticHarmonicPreview,
} from '../hera-harmonic-utils';

describe('hera-harmonic-utils', () => {
  it('exposes all 24 Camelot keys and 4 canonical Hot Cue dot colors', () => {
    expect(CAMELOT_OPTIONS).toHaveLength(24);
    expect(CAMELOT_OPTIONS[0]).toBe('1A');
    expect(CAMELOT_OPTIONS[11]).toBe('12A');
    expect(CAMELOT_OPTIONS[12]).toBe('1B');
    expect(CAMELOT_OPTIONS[23]).toBe('12B');
    expect(CUE_DOT_COLORS).toEqual([
      '#22c55e',
      '#38bdf8',
      '#ef4444',
      '#f59e0b',
    ]);
  });

  it('parses valid, lowercase, padded, and invalid Camelot strings safely', () => {
    expect(parseCamelot('8A')).toEqual({ num: 8, mode: 'A' });
    expect(parseCamelot(' 12b ')).toEqual({ num: 12, mode: 'B' });
    expect(parseCamelot('1a')).toEqual({ num: 1, mode: 'A' });
    expect(parseCamelot('')).toEqual({ num: 8, mode: 'A' });
    expect(parseCamelot('99B')).toEqual({ num: 8, mode: 'B' });
    expect(parseCamelot('0A')).toEqual({ num: 8, mode: 'A' });
  });

  it('computes circular Camelot wheel distance across 12->1 wrap and A/B modes', () => {
    expect(camelotDistance('8A', '8A')).toBe(0);
    expect(camelotDistance('12A', '1A')).toBe(1);
    expect(camelotDistance('1A', '12A')).toBe(1);
    expect(camelotDistance('8A', '8B')).toBe(1);
    expect(camelotDistance('12A', '1B')).toBe(2);
    expect(camelotDistance('2A', '8B')).toBe(7);
  });

  it('steps around the 12-key Camelot ring in both directions', () => {
    expect(nextHarmonicCamelot('8A', 1)).toBe('9A');
    expect(nextHarmonicCamelot('12A', 1)).toBe('1A');
    expect(nextHarmonicCamelot('11B', 2)).toBe('1B');
    expect(nextHarmonicCamelot('1A', -1)).toBe('12A');
  });

  it('resolves minor and relative-major root frequencies in Hz', () => {
    const aMinor = camelotRootHz('8A');
    expect(aMinor.major).toBe(false);
    expect(aMinor.root).toBeCloseTo(220.0, 2);

    const cMajor = camelotRootHz('8B');
    expect(cMajor.major).toBe(true);
    expect(cMajor.root).toBeCloseTo(220.0 * 1.1892, 2);

    const gSharpMinor = camelotRootHz('1a');
    expect(gSharpMinor.major).toBe(false);
    expect(gSharpMinor.root).toBeCloseTo(207.65, 2);
  });

  it('builds synthetic Track descriptors with 4 structural Hot Cues and optional overrides', () => {
    const defaultTrack = buildSyntheticTrackDescriptor(
      'p2p-01',
      'Keinemusik',
      'Cesonia',
      124,
      '9A',
    );
    expect(defaultTrack.id).toBe('p2p-01');
    expect(defaultTrack.path).toBe('exports/Keinemusik - Cesonia.wav');
    expect(defaultTrack.album).toBe('Single');
    expect(defaultTrack.artists).toEqual(['Keinemusik']);
    expect(defaultTrack.bpm).toBe(124);
    expect(defaultTrack.camelot).toBe('9A');
    expect(defaultTrack.hera_seal).toBe('hera_seal_p2p-01');
    expect(defaultTrack.hot_cues).toHaveLength(4);
    expect(defaultTrack.hot_cues?.map((c) => c.role)).toEqual([
      'intro',
      'breakdown',
      'drop',
      'outro',
    ]);

    const liveCrateTrack = buildSyntheticTrackDescriptor(
      'mini-01',
      'Hugel',
      'Morenita',
      126,
      '10A',
      {
        savedPath: 'exports/live/Hugel - Morenita.wav',
        album: 'HERA En Vivo',
        heraSeal: 'hera_seal_custom',
      },
    );
    expect(liveCrateTrack.path).toBe('exports/live/Hugel - Morenita.wav');
    expect(liveCrateTrack.album).toBe('HERA En Vivo');
    expect(liveCrateTrack.hera_seal).toBe('hera_seal_custom');
    expect(liveCrateTrack.hot_cues).toHaveLength(4);
  });

  it('returns null gracefully when WebAudio AudioContext is unavailable', () => {
    vi.stubGlobal('window', {});
    const ctx = startSyntheticHarmonicPreview({
      bpm: 125,
      camelot: '8A',
      durationSecs: 4.5,
    });
    expect(ctx).toBeNull();
    vi.unstubAllGlobals();
  });
});
