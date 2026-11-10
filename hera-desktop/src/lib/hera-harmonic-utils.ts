import type { Track } from '../generated/typings';

export const CAMELOT_OPTIONS = [
  '1A',
  '2A',
  '3A',
  '4A',
  '5A',
  '6A',
  '7A',
  '8A',
  '9A',
  '10A',
  '11A',
  '12A',
  '1B',
  '2B',
  '3B',
  '4B',
  '5B',
  '6B',
  '7B',
  '8B',
  '9B',
  '10B',
  '11B',
  '12B',
] as const;

export const CUE_DOT_COLORS = [
  '#22c55e',
  '#38bdf8',
  '#ef4444',
  '#f59e0b',
] as const;

const MINOR_ROOT_HZ: Record<number, number> = {
  1: 207.65,
  2: 155.56,
  3: 233.08,
  4: 174.61,
  5: 130.81,
  6: 196.0,
  7: 146.83,
  8: 220.0,
  9: 164.81,
  10: 246.94,
  11: 185.0,
  12: 138.59,
};

export function parseCamelot(raw: string): { num: number; mode: 'A' | 'B' } {
  const clean = raw.trim().toUpperCase();
  const mode = clean.endsWith('B') ? 'B' : 'A';
  const parsed = Number.parseInt(clean.slice(0, -1), 10);
  const num =
    Number.isFinite(parsed) && parsed >= 1 && parsed <= 12 ? parsed : 8;
  return { num, mode };
}

export function camelotDistance(a: string, b: string): number {
  const ca = parseCamelot(a);
  const cb = parseCamelot(b);
  const ring = Math.min(
    Math.abs(ca.num - cb.num),
    12 - Math.abs(ca.num - cb.num),
  );
  return ring + (ca.mode === cb.mode ? 0 : 1);
}

export function nextHarmonicCamelot(raw: string, stepOffset = 1): string {
  const { num, mode } = parseCamelot(raw);
  const wrapped = ((((num - 1 + stepOffset) % 12) + 12) % 12) + 1;
  return `${wrapped}${mode}`;
}

export function camelotRootHz(camelot: string): {
  root: number;
  major: boolean;
} {
  const { num, mode } = parseCamelot(camelot);
  const major = mode === 'B';
  const base = MINOR_ROOT_HZ[num] ?? 220.0;
  return { root: major ? base * 1.1892 : base, major };
}

export type SyntheticTrackOptions = {
  savedPath?: string;
  album?: string;
  heraSeal?: string;
};

export function buildSyntheticTrackDescriptor(
  id: string,
  artist: string,
  title: string,
  bpm: number,
  camelot: string,
  options?: SyntheticTrackOptions,
): Track {
  return {
    id,
    path: options?.savedPath ?? `exports/${artist} - ${title}.wav`,
    title,
    album: options?.album ?? 'Single',
    album_artist: artist,
    artists: [artist],
    genres: ['Club'],
    year: 2026,
    duration: 210,
    track_no: 1,
    track_of: 1,
    disk_no: 1,
    disk_of: 1,
    is_compilation: false,
    bpm,
    camelot,
    energy: 0.84,
    hera_seal: options?.heraSeal ?? `hera_seal_${id}`,
    hot_cues: [
      {
        num: 1,
        name: '1:Intro',
        seconds: 15.2,
        color: '#22c55e',
        role: 'intro',
      },
      {
        num: 2,
        name: '2:Breakdown',
        seconds: 68.0,
        color: '#38bdf8',
        role: 'breakdown',
      },
      {
        num: 3,
        name: '3:Drop',
        seconds: 102.0,
        color: '#ef4444',
        role: 'drop',
      },
      {
        num: 4,
        name: '4:Outro',
        seconds: 168.4,
        color: '#f59e0b',
        role: 'outro',
      },
    ],
  };
}

export type SyntheticPreviewConfig = {
  bpm: number;
  camelot: string;
  durationSecs: number;
};

export function startSyntheticHarmonicPreview({
  bpm,
  camelot,
  durationSecs,
}: SyntheticPreviewConfig): AudioContext | null {
  try {
    if (typeof window === 'undefined') return null;
    const AudioContextCtor =
      window.AudioContext ||
      (window as unknown as { webkitAudioContext?: typeof AudioContext })
        .webkitAudioContext;
    if (!AudioContextCtor) return null;

    const ctx = new AudioContextCtor();
    const now = ctx.currentTime + 0.04;
    const beatSec = 60 / Math.max(70, Math.min(180, bpm));
    const { root, major } = camelotRootHz(camelot);
    const thirdRatio = major ? 1.259921 : 1.1892071;

    const masterGain = ctx.createGain();
    masterGain.gain.setValueAtTime(0.21, now);
    masterGain.gain.exponentialRampToValueAtTime(0.001, now + durationSecs);
    masterGain.connect(ctx.destination);

    for (const freq of [root, root * thirdRatio, root * 1.4983071]) {
      const osc = ctx.createOscillator();
      const padGain = ctx.createGain();
      osc.type = 'triangle';
      osc.frequency.setValueAtTime(freq, now);
      padGain.gain.setValueAtTime(0.001, now);
      padGain.gain.linearRampToValueAtTime(0.24, now + 0.32);
      padGain.gain.linearRampToValueAtTime(0.18, now + durationSecs - 0.5);
      padGain.gain.exponentialRampToValueAtTime(0.001, now + durationSecs);
      osc.connect(padGain);
      padGain.connect(masterGain);
      osc.start(now);
      osc.stop(now + durationSecs);
    }

    const totalBeats = Math.floor(durationSecs / beatSec);
    for (let b = 0; b < totalBeats; b += 1) {
      const bt = now + b * beatSec;
      const kickOsc = ctx.createOscillator();
      const kickGain = ctx.createGain();
      kickOsc.type = 'sine';
      kickOsc.frequency.setValueAtTime(134, bt);
      kickOsc.frequency.exponentialRampToValueAtTime(48, bt + 0.11);
      kickGain.gain.setValueAtTime(0.72, bt);
      kickGain.gain.exponentialRampToValueAtTime(0.001, bt + 0.22);
      kickOsc.connect(kickGain);
      kickGain.connect(masterGain);
      kickOsc.start(bt);
      kickOsc.stop(bt + 0.24);
    }

    return ctx;
  } catch {
    return null;
  }
}
