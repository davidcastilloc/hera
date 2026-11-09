import { describe, expect, it } from 'vitest';

import {
  formatSecondsToMinutes,
  getHumanAudioQuality,
  getHumanCueInfo,
  getHumanMatchLabel,
} from './hera-human-language';

describe('hera-human-language (Zero Jargon translation layer)', () => {
  it('formats seconds to mm:ss correctly', () => {
    expect(formatSecondsToMinutes(15)).toBe('00:15');
    expect(formatSecondsToMinutes(108)).toBe('01:48');
    expect(formatSecondsToMinutes(282)).toBe('04:42');
  });

  it('translates 4 Hot Cues into human-first DJ booth language', () => {
    const cue1 = getHumanCueInfo(1, 16);
    expect(cue1.simpleName).toBe('Entrada');
    expect(cue1.color).toBe('#10b981');
    expect(cue1.timeFormatted).toBe('00:16');

    const cue2 = getHumanCueInfo(2, 108);
    expect(cue2.simpleName).toBe('Calma');
    expect(cue2.color).toBe('#3b82f6');

    const cue3 = getHumanCueInfo(3, 154);
    expect(cue3.simpleName).toBe('Explosión');
    expect(cue3.color).toBe('#f43f5e');

    const cue4 = getHumanCueInfo(4, 312);
    expect(cue4.simpleName).toBe('Salida limpia');
    expect(cue4.color).toBe('#f59e0b');
  });

  it('translates harmonic compatibility into friendly percentages and advice', () => {
    const sameKey = getHumanMatchLabel('8A', '8A', 124.0, 124.0);
    expect(sameKey.compatibilityLabel).toContain('Mezcla Perfecta');
    expect(sameKey.tempoAdvice).toBe('Mismo ritmo exacto');

    const neighborKey = getHumanMatchLabel('8A', '9A', 124.0, 125.5);
    expect(neighborKey.compatibilityLabel).toContain('Mezcla Fluida');
    expect(neighborKey.tempoAdvice).toBe('Sube suave (+1.5 BPM)');

    const modeShift = getHumanMatchLabel('8A', '8B', 125.0, 124.0);
    expect(modeShift.compatibilityLabel).toContain('Cambio de Brillo');
    expect(modeShift.tempoAdvice).toBe('Baja suave (-1.0 BPM)');
  });

  it('translates forensics FFT cutoff into clean studio badges', () => {
    const hiRes = getHumanAudioQuality(21.8);
    expect(hiRes.label).toBe('✨ Sonido de Estudio');
    expect(hiRes.isLossless).toBe(true);

    const fallback = getHumanAudioQuality(null);
    expect(fallback.label).toContain('Audio Limpio');
  });
});
