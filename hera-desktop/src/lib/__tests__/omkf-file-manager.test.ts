import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock, getTracksSpy, getAllTracksSpy } = vi.hoisted(() => {
  if (
    typeof (globalThis as unknown as Record<string, unknown>).window ===
    'undefined'
  ) {
    (globalThis as unknown as Record<string, unknown>).window = globalThis;
  }
  return {
    invokeMock: vi.fn<(...args: unknown[]) => Promise<unknown>>(),
    getTracksSpy: vi.fn<(...args: unknown[]) => Promise<unknown[]>>(),
    getAllTracksSpy: vi.fn<(...args: unknown[]) => Promise<unknown[]>>(),
  };
});

vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
  invoke: invokeMock,
}));

vi.mock('../bridge-database', () => ({
  default: {
    getTracks: getTracksSpy,
    getAllTracks: getAllTracksSpy,
  },
}));

import FolderCrateTree from '../../components/FolderCrateTree';
import FolderExplorerHeader from '../../components/FolderExplorerHeader';
import TrackRow from '../../components/TrackRow';
import type {
  FolderCrateDetail,
  FolderCrateSummary,
  OmkfFsTrackDto,
} from '../../generated/hera-typings';
import {
  buildFolderBreadcrumbs,
  buildFolderTreeRows,
  mapOmkfDtoToTrack,
  pickInitialFolderCrateId,
  queryHarmonicCandidates,
  readFolderCrate,
  searchAllFoldersDeduplicated,
} from '../omkf-bridge';

function makeSummary(
  overrides: Partial<FolderCrateSummary> & {
    crate_id: string;
    folder_path: string;
    display_name: string;
  },
): FolderCrateSummary {
  return {
    tree_seal_64: '0011223344556677',
    track_count: 0,
    video_count: 0,
    full_count: 0,
    bpm_min: null,
    bpm_max: null,
    dominant_camelot: null,
    dominant_genre: null,
    era_span: null,
    lufs_avg: null,
    has_custom_rules: false,
    updated_at_unix_ms: 1700000000000,
    parent_crate_id: null,
    rel_path: overrides.display_name,
    depth: 0,
    subfolder_count: 0,
    ...overrides,
  };
}

function makeSampleDto(
  overrides: Partial<OmkfFsTrackDto> = {},
): OmkfFsTrackDto {
  return {
    id: 'trk-afro-1',
    crate_id: 'crate-afro',
    path: '/music/_REPERTORIO_DJ/01_Afro_House/Move.mp3',
    file_name: 'Move.mp3',
    title: 'Move',
    artist: 'Adam Port',
    album: '01_Afro_House',
    duration_sec: 352,
    bpm: 120.0,
    camelot: '4B',
    energy: 0.78,
    hot_cues: [
      { slot: 1, label: '1:Intro', seconds: 8.0, kind: 'intro' },
      { slot: 2, label: '2:Breakdown', seconds: 64.5, kind: 'breakdown' },
      { slot: 3, label: '3:Drop', seconds: 96.0, kind: 'drop' },
      { slot: 4, label: '4:Outro', seconds: 310.0, kind: 'outro' },
    ],
    scan_level: 2,
    is_video: false,
    is_fake_320kbps: false,
    has_audio_skips: false,
    is_variable_bpm: false,
    has_intro_vocals: true,
    has_outro_vocals: false,
    replay_gain_db: -6.4,
    waveform_peaks_b64: 'AAAA',
    copy_count: 3,
    ...overrides,
  };
}

describe('OMKF File Manager pure DTO mapping & IPC hydration', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    getTracksSpy.mockReset();
    getAllTracksSpy.mockReset();
  });

  it('maps OmkfFsTrackDto 1:1 to Track without calling DatabaseBridge', () => {
    const mapped = mapOmkfDtoToTrack(makeSampleDto());

    expect(mapped.id).toBe('trk-afro-1');
    expect(mapped.title).toBe('Move');
    expect(mapped.album_artist).toBe('Adam Port');
    expect(mapped.artists).toEqual(['Adam Port']);
    expect(mapped.duration).toBe(352);
    expect(mapped.bpm).toBe(120.0);
    expect(mapped.camelot).toBe('4B');
    expect(mapped.scan_level).toBe(2);
    expect(mapped.copy_count).toBe(3);
    expect(mapped.crate_id).toBe('crate-afro');
    expect(mapped.is_mix_over_10m).toBe(false);
    expect(mapped.hot_cues).toHaveLength(4);
    expect(mapped.hot_cues?.[0]).toMatchObject({
      num: 1,
      name: '1:Intro',
      seconds: 8.0,
      color: '#22c55e',
      role: 'intro',
    });
    expect(mapped.hot_cues?.[2]).toMatchObject({
      num: 3,
      name: '3:Drop',
      seconds: 96.0,
      color: '#ef4444',
      role: 'drop',
    });
    expect(getTracksSpy).not.toHaveBeenCalled();
    expect(getAllTracksSpy).not.toHaveBeenCalled();
  });

  it('hydrates readFolderCrate, global search, and harmonic candidates directly from detail.tracks', async () => {
    const summary = makeSummary({
      crate_id: 'crate-afro',
      folder_path: '/music/_REPERTORIO_DJ/01_Afro_House',
      display_name: '01_Afro_House',
      track_count: 1,
      full_count: 1,
    });
    const detail: FolderCrateDetail = {
      summary,
      rules_markdown: '# Reglas Afro House',
      ledger_entries: [],
      track_ids: ['trk-afro-1'],
      hydrated_from_omkf: true,
      disk_online: true,
      subfolders: [],
      tracks: [makeSampleDto()],
    };

    invokeMock.mockResolvedValueOnce(detail);
    const hydrated = await readFolderCrate('crate-afro');
    expect(invokeMock).toHaveBeenCalledWith(
      'plugin:hera-dj|omkf_read_folder_crate',
      { crateId: 'crate-afro' },
    );
    expect(hydrated?.tracks).toHaveLength(1);
    expect(hydrated?.tracks[0]?.title).toBe('Move');
    expect(getTracksSpy).not.toHaveBeenCalled();
    expect(getAllTracksSpy).not.toHaveBeenCalled();

    invokeMock.mockResolvedValueOnce(detail);
    const searchHits = await searchAllFoldersDeduplicated('  move  ');
    expect(invokeMock).toHaveBeenCalledWith(
      'plugin:hera-dj|omkf_read_folder_crate',
      { crateId: '__search__:move' },
    );
    expect(searchHits).toHaveLength(1);
    expect(searchHits[0]?.copy_count).toBe(3);

    invokeMock.mockResolvedValueOnce(detail);
    const harmonicHits = await queryHarmonicCandidates(120, '4B');
    expect(invokeMock).toHaveBeenCalledWith(
      'plugin:hera-dj|omkf_read_folder_crate',
      { crateId: '__harmonic__:120:4B' },
    );
    expect(harmonicHits).toHaveLength(1);
    expect(getTracksSpy).not.toHaveBeenCalled();
  });
});

describe('OMKF File Manager hierarchical tree, breadcrumbs & initial folder selection', () => {
  const backupRoot = makeSummary({
    crate_id: 'crate-backup',
    folder_path: '/music/BACKUP_2024',
    display_name: 'BACKUP_2024',
    track_count: 250,
    depth: 0,
  });
  const musicRoot = makeSummary({
    crate_id: 'crate-music',
    folder_path: '/music/DJ_ROOT',
    display_name: 'DJ_ROOT',
    track_count: 0,
    depth: 0,
    subfolder_count: 1,
  });
  const repertorio = makeSummary({
    crate_id: 'crate-repertorio',
    folder_path: '/music/DJ_ROOT/_REPERTORIO_DJ',
    display_name: '_REPERTORIO_DJ',
    parent_crate_id: 'crate-music',
    rel_path: 'DJ_ROOT/_REPERTORIO_DJ',
    track_count: 0,
    depth: 1,
    subfolder_count: 2,
  });
  const afroLeaf = makeSummary({
    crate_id: 'crate-afro',
    folder_path: '/music/DJ_ROOT/_REPERTORIO_DJ/01_Afro_House',
    display_name: '01_Afro_House',
    parent_crate_id: 'crate-repertorio',
    rel_path: 'DJ_ROOT/_REPERTORIO_DJ/01_Afro_House',
    track_count: 48,
    depth: 2,
    subfolder_count: 0,
  });
  const reggaetonLeaf = makeSummary({
    crate_id: 'crate-reggaeton',
    folder_path: '/music/DJ_ROOT/_REPERTORIO_DJ/03_Reggaeton',
    display_name: '03_Reggaeton',
    parent_crate_id: 'crate-repertorio',
    rel_path: 'DJ_ROOT/_REPERTORIO_DJ/03_Reggaeton',
    track_count: 64,
    depth: 2,
    subfolder_count: 0,
  });

  const crates = [backupRoot, musicRoot, repertorio, afroLeaf, reggaetonLeaf];

  it('groups roots when collapsed and expands nested children recursively', () => {
    const collapsedRows = buildFolderTreeRows(crates, new Set());
    expect(collapsedRows.map((r) => r.crate.crate_id)).toEqual([
      'crate-backup',
      'crate-music',
    ]);
    expect(collapsedRows[1]?.hasChildren).toBe(true);
    expect(collapsedRows[1]?.isExpanded).toBe(false);

    const expandedRootRows = buildFolderTreeRows(
      crates,
      new Set(['crate-music']),
    );
    expect(expandedRootRows.map((r) => [r.crate.crate_id, r.depth])).toEqual([
      ['crate-backup', 0],
      ['crate-music', 0],
      ['crate-repertorio', 1],
    ]);

    const fullyExpandedRows = buildFolderTreeRows(
      crates,
      new Set(['crate-music', 'crate-repertorio']),
    );
    expect(fullyExpandedRows.map((r) => [r.crate.crate_id, r.depth])).toEqual([
      ['crate-backup', 0],
      ['crate-music', 0],
      ['crate-repertorio', 1],
      ['crate-afro', 2],
      ['crate-reggaeton', 2],
    ]);
  });

  it('builds ancestor breadcrumbs from root down to selected leaf folder', () => {
    const crumbs = buildFolderBreadcrumbs(crates, 'crate-afro');
    expect(crumbs.map((c) => c.display_name)).toEqual([
      'DJ_ROOT',
      '_REPERTORIO_DJ',
      '01_Afro_House',
    ]);
  });

  it('picks the first workable non-BACKUP folder with tracks on boot', () => {
    expect(pickInitialFolderCrateId(crates)).toBe('crate-afro');
    expect(pickInitialFolderCrateId([backupRoot])).toBe('crate-backup');
    expect(pickInitialFolderCrateId([])).toBeNull();
  });

  it('renders hierarchical FolderCrateTree, FolderExplorerHeader breadcrumbs/subfolders, and TrackRow copy_count pill', () => {
    const treeHtml = renderToStaticMarkup(
      createElement(FolderCrateTree, {
        crates,
        selectedCrateId: 'crate-afro',
        onSelectCrate: () => {},
        collapsed: false,
        onToggleCollapsed: () => {},
      }),
    );
    expect(treeHtml).toContain('RAÍCES Y CARPETAS');
    expect(treeHtml).toContain('_REPERTORIO_DJ');
    expect(treeHtml).toContain('01_Afro_House');
    expect(treeHtml).not.toContain('Todas las carpetas');

    const headerHtml = renderToStaticMarkup(
      createElement(FolderExplorerHeader, {
        crates,
        selectedCrateId: 'crate-repertorio',
        subfolders: [afroLeaf, reggaetonLeaf],
        trackCount: 0,
        searchQuery: '',
        onSelectCrate: () => {},
        onClearSearch: () => {},
      }),
    );
    expect(headerHtml).toContain('Ruta:');
    expect(headerHtml).toContain('DJ_ROOT');
    expect(headerHtml).toContain('_REPERTORIO_DJ');
    expect(headerHtml).toContain('.hera sincronizado');
    expect(headerHtml).toContain('Subcarpetas:');
    expect(headerHtml).toContain('01_Afro_House');
    expect(headerHtml).toContain('03_Reggaeton');

    const searchHeaderHtml = renderToStaticMarkup(
      createElement(FolderExplorerHeader, {
        crates,
        selectedCrateId: 'crate-afro',
        subfolders: [afroLeaf],
        trackCount: 2,
        searchQuery: 'move',
        onSelectCrate: () => {},
        onClearSearch: () => {},
      }),
    );
    expect(searchHeaderHtml).toContain(
      'Resultados en todas las carpetas (sin duplicados)',
    );
    expect(searchHeaderHtml).toContain('2 coincidencias');
    expect(searchHeaderHtml).toContain('Limpiar');

    const mappedTrack = mapOmkfDtoToTrack(makeSampleDto({ copy_count: 3 }));
    const rowHtml = renderToStaticMarkup(
      createElement(TrackRow, {
        track: mappedTrack,
        index: 0,
        selected: false,
        onTrackSelect: () => {},
        onContextMenu: () => {},
        onPlaybackStart: () => {},
      }),
    );
    expect(rowHtml).toContain('×3');
  });
});
