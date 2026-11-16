// Ensure window is defined before importing any modules that touch window (like player.ts)
if (
  typeof (globalThis as unknown as Record<string, unknown>).window ===
  'undefined'
) {
  (globalThis as unknown as Record<string, unknown>).window = globalThis;
}

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';

vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual =
    await importOriginal<typeof import('@tanstack/react-router')>();
  return {
    ...actual,
    useNavigate: () => () => {},
    useRouter: () => ({ invalidate: () => Promise.resolve() }),
  };
});

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => false,
  invoke: vi.fn<(...args: unknown[]) => Promise<number>>().mockResolvedValue(0),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn<(...args: unknown[]) => Promise<unknown>>(),
  ask: vi.fn<(...args: unknown[]) => Promise<boolean>>(),
}));

import type { HotCuePin, Track } from '../../generated/typings';
import type { ScannerRowPatch, TrackWithScanMeta } from '../../lib/scanner-v3';

const {
  applyRowPatchesToTracks,
  mergeIncomingTrackBatch,
  removeTrackIds,
  shouldFreezeSortPatches,
} = await import('../LibraryEvents');
const { default: TrackListStates } = await import('../TrackListStates');
const { default: TrackRow } = await import('../TrackRow');
const { default: useLibraryStore } = await import('../../lib/store');

const testQueryClient = new QueryClient();

const mockTrack1: Track = {
  id: 'track-1',
  path: 'C:/Music/Track1.mp3',
  title: 'Pista Uno',
  album: 'Album Uno',
  album_artist: 'Artista Uno',
  artists: ['Artista Uno'],
  genres: ['House'],
  year: 2024,
  duration: 180,
  track_no: 1,
  track_of: 10,
  disk_no: 1,
  disk_of: 1,
  is_compilation: false,
  bpm: 124.0,
  camelot: '8A',
};

const mockTrack2: Track = {
  id: 'track-2',
  path: 'C:/Music/Track2.mp3',
  title: 'Pista Dos',
  album: 'Album Dos',
  album_artist: 'Artista Dos',
  artists: ['Artista Dos'],
  genres: ['Techno'],
  year: 2024,
  duration: 210,
  track_no: 2,
  track_of: 10,
  disk_no: 1,
  disk_of: 1,
  is_compilation: false,
};

describe('scanner-realtime-ux pure helpers', () => {
  it('mergeIncomingTrackBatch deduplicates existing and appends new tracks in order', () => {
    const existing = [mockTrack1];
    const incoming: Track[] = [
      { ...mockTrack1, title: 'Pista Uno (Remastered)' },
      mockTrack2,
    ];

    const merged = mergeIncomingTrackBatch(existing, incoming);
    expect(merged).toHaveLength(2);
    expect(merged[0].id).toBe('track-1');
    expect(merged[0].title).toBe('Pista Uno (Remastered)');
    expect(merged[1].id).toBe('track-2');
    expect(merged[1].title).toBe('Pista Dos');
  });

  it('removeTrackIds prunes tracks by ID cleanly', () => {
    const existing = [mockTrack1, mockTrack2];
    const filtered = removeTrackIds(existing, ['track-1']);
    expect(filtered).toHaveLength(1);
    expect(filtered[0].id).toBe('track-2');
  });

  it('applyRowPatchesToTracks updates bpm, camelot, hot_cues, scan_level, duration', () => {
    const initialTracks = [mockTrack2];
    const cues: HotCuePin[] = [
      { num: 1, name: 'Intro', seconds: 0, color: '#22c55e', role: 'intro' },
      { num: 2, name: 'Break', seconds: 32, color: '#38bdf8', role: 'break' },
      { num: 3, name: 'Drop', seconds: 64, color: '#ef4444', role: 'drop' },
      { num: 4, name: 'Outro', seconds: 190, color: '#f59e0b', role: 'outro' },
    ];
    const patches: ScannerRowPatch[] = [
      {
        track_id: 'track-2',
        bpm: 128.5,
        camelot: '4A',
        duration: 245,
        scan_level: 2,
        hot_cues: cues,
        is_transition_candidate: true,
        is_fake_320kbps: false,
      },
    ];

    const updated = applyRowPatchesToTracks(initialTracks, patches);
    expect(updated).toHaveLength(1);
    const t = updated[0];
    expect(t.bpm).toBe(128.5);
    expect(t.camelot).toBe('4A');
    expect(t.duration).toBe(245);
    expect(t.hot_cues).toEqual(cues);
    const meta = t as TrackWithScanMeta;
    expect(meta.scan_level).toBe(2);
    expect(meta.is_transition_candidate).toBe(true);
    expect(meta.is_fake_320kbps).toBe(false);
  });

  it('shouldFreezeSortPatches returns true only when pointer inside AND sorted by Bpm or Camelot', () => {
    expect(shouldFreezeSortPatches(true, 'Bpm')).toBe(true);
    expect(shouldFreezeSortPatches(true, 'Camelot')).toBe(true);
    expect(shouldFreezeSortPatches(false, 'Bpm')).toBe(false);
    expect(shouldFreezeSortPatches(false, 'Camelot')).toBe(false);
    expect(shouldFreezeSortPatches(true, 'Title')).toBe(false);
    expect(shouldFreezeSortPatches(true, 'Artist')).toBe(false);
    expect(shouldFreezeSortPatches(true, undefined)).toBe(false);
  });
});

describe('scanner-realtime-ux UI components', () => {
  it('TrackListStates keeps children mounted when refreshing === true and tracks.length > 0', () => {
    useLibraryStore.setState({ refreshing: true, search: '' });

    const html = renderToStaticMarkup(
      <QueryClientProvider client={testQueryClient}>
        <TrackListStates isLoading={false} tracks={[mockTrack1]}>
          <div data-testid="live-table">TABLA_EN_VIVO</div>
        </TrackListStates>
      </QueryClientProvider>,
    );

    expect(html).toContain('data-testid="live-table"');
    expect(html).toContain('TABLA_EN_VIVO');
    expect(html).not.toContain('Cargando biblioteca');
  });

  it('TrackRow displays ··· (NOT 8A) and --:-- when bpm, camelot and duration are pending', () => {
    const pendingTrack: Track = {
      id: 'pending-1',
      path: 'C:/Music/Pending.mp3',
      title: 'Cancion En Cola',
      album: 'Sin Album',
      album_artist: 'Varios',
      artists: ['Varios'],
      genres: [],
      year: null,
      duration: 0,
      track_no: null,
      track_of: null,
      disk_no: null,
      disk_of: null,
      is_compilation: false,
      bpm: undefined,
      camelot: undefined,
      scan_level: 0,
    };

    const html = renderToStaticMarkup(
      <TrackRow
        track={pendingTrack}
        index={0}
        selected={false}
        onTrackSelect={() => {}}
        onContextMenu={() => {}}
        onPlaybackStart={() => {}}
      />,
    );

    expect(html).toContain('--:--');
    expect(html).toContain('···');
    expect(html).not.toContain('8A');
  });

  it('TrackRow displays real BPM, Camelot and full hot cues when patched', () => {
    const cues: HotCuePin[] = [
      { num: 1, name: 'Intro', seconds: 0, color: '#22c55e', role: 'intro' },
      { num: 2, name: 'Break', seconds: 32, color: '#38bdf8', role: 'break' },
      { num: 3, name: 'Drop', seconds: 64, color: '#ef4444', role: 'drop' },
      { num: 4, name: 'Outro', seconds: 190, color: '#f59e0b', role: 'outro' },
    ];
    const analyzedTrack: Track = {
      ...mockTrack1,
      bpm: 126.4,
      camelot: '11B',
      duration: 215,
      scan_level: 2,
      hot_cues: cues,
    };

    const html = renderToStaticMarkup(
      <TrackRow
        track={analyzedTrack}
        index={0}
        selected={false}
        onTrackSelect={() => {}}
        onContextMenu={() => {}}
        onPlaybackStart={() => {}}
      />,
    );

    expect(html).toContain('126.4');
    expect(html).toContain('11B');
    expect(html).not.toContain('--:--');
  });

  it('formatScannerPillStatus formats sub-1/s speeds cleanly and humanizes large ETAs instead of 0/s ~30197s', async () => {
    const { formatScannerPillStatus } = await import('../ScannerStatusPill');

    const slowStatus = formatScannerPillStatus({
      phase: 'analyzing',
      indexedDone: 11487,
      fastDone: 9857,
      fullDone: 8806,
      total: 11487,
      tracksPerSec: 0.04,
      etaSec: 30197,
      governorActive: false,
    });
    expect(slowStatus).not.toContain('0/s');
    expect(slowStatus).not.toContain('30197s');

    const steadyStatus = formatScannerPillStatus({
      phase: 'analyzing',
      indexedDone: 1000,
      fastDone: 400,
      fullDone: 100,
      total: 1000,
      tracksPerSec: 0.5,
      etaSec: 720,
      governorActive: false,
    });
    expect(steadyStatus).toContain('0.5/s');
    expect(steadyStatus).toContain('~12m');
  });
});
