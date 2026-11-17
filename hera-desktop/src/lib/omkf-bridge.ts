import { invoke, isTauri } from '@tauri-apps/api/core';

import type {
  FolderCrateDetail,
  FolderCrateSummary,
  OmkfFsHotCueDto,
  OmkfFsTrackDto,
  OrganizeFolderRequest as GeneratedOrganizeFolderRequest,
  OrganizeFolderResponse,
  PlannedFileMove,
} from '../generated/hera-typings';
import type { Track } from '../generated/typings';
import DatabaseBridge from './bridge-database';
import {
  buildFolderBreadcrumbs,
  buildFolderTreeRows,
  deduplicateMockSearchTracks,
  deriveFolderCratesFromTracks,
  filterMockHarmonicTracks,
  FOLDER_CRATE_PREFIX,
  type FolderTreeRow,
  formatFolderCrateSubtitle,
  fromFolderPlaylistId,
  getDefaultExpandedCrateIds,
  isFolderPlaylistId,
  makeSyntheticFolderDetail,
  mapOmkfDtoToTrack,
  normalizeFolderHierarchy,
  type OmkfTrack,
  pickInitialFolderCrateId,
  toFolderPlaylistId,
} from './omkf-tree-utils';
import type { TrackWithScanMeta } from './scanner-v3';
import useLibraryStore from './store';

export type {
  FolderCrateDetail,
  FolderCrateSummary,
  FolderTreeRow,
  OmkfFsHotCueDto,
  OmkfFsTrackDto,
  OmkfTrack,
  OrganizeFolderResponse,
  PlannedFileMove,
};

export {
  buildFolderBreadcrumbs,
  buildFolderTreeRows,
  deriveFolderCratesFromTracks,
  FOLDER_CRATE_PREFIX,
  formatFolderCrateSubtitle,
  fromFolderPlaylistId,
  getDefaultExpandedCrateIds,
  isFolderPlaylistId,
  mapOmkfDtoToTrack,
  normalizeFolderHierarchy,
  pickInitialFolderCrateId,
  toFolderPlaylistId,
};

export type OrganizeStrategy =
  | 'by_tempo_zone'
  | 'by_genre_era'
  | 'quarantine_issues';

export type OrganizeFolderRequest = Omit<
  GeneratedOrganizeFolderRequest,
  'strategy'
> & {
  strategy: OrganizeStrategy;
};

export interface HydratedFolderCrate {
  detail: FolderCrateDetail;
  tracks: OmkfTrack[];
}

let mockCatalogTracks: Track[] = [];

export function syncMockCatalogTracks(tracks: Track[]): void {
  mockCatalogTracks = tracks;
}

async function resolveBrowserMockTracks(): Promise<Track[]> {
  if (mockCatalogTracks.length > 0) return mockCatalogTracks;
  const storeTracks = useLibraryStore.getState().tracks;
  if (storeTracks.length > 0) {
    mockCatalogTracks = storeTracks;
    return storeTracks;
  }
  try {
    const dbTracks = await DatabaseBridge.getAllTracks();
    if (dbTracks.length > 0) mockCatalogTracks = dbTracks;
    return dbTracks;
  } catch {
    return [];
  }
}

export async function listFolderCrates(): Promise<FolderCrateSummary[]> {
  if (isTauri()) {
    try {
      const res = await invoke<FolderCrateSummary[]>(
        'plugin:hera-dj|omkf_list_folder_crates',
      );
      return Array.isArray(res) ? res : [];
    } catch {
      return [];
    }
  }
  const mockTracks = await resolveBrowserMockTracks();
  return mockTracks.length > 0
    ? deriveFolderCratesFromTracks(mockTracks, [])
    : [];
}

export async function readFolderCrate(
  crateId: string,
): Promise<HydratedFolderCrate | null> {
  if (isTauri()) {
    try {
      const detail = await invoke<FolderCrateDetail | null>(
        'plugin:hera-dj|omkf_read_folder_crate',
        { crateId },
      );
      if (!detail || !detail.summary) return null;
      return { detail, tracks: (detail.tracks ?? []).map(mapOmkfDtoToTrack) };
    } catch {
      return null;
    }
  }

  const allTracks = await resolveBrowserMockTracks();
  if (crateId.startsWith('__search__:')) {
    const deduped = deduplicateMockSearchTracks(
      allTracks,
      crateId.slice('__search__:'.length),
    );
    return {
      detail: makeSyntheticFolderDetail(
        crateId,
        'Resultados en carpetas',
        deduped,
      ),
      tracks: deduped,
    };
  }
  if (crateId.startsWith('__harmonic__:')) {
    const cands = filterMockHarmonicTracks(
      allTracks,
      crateId.slice('__harmonic__:'.length),
    );
    return {
      detail: makeSyntheticFolderDetail(crateId, 'Candidatos armónicos', cands),
      tracks: cands,
    };
  }

  const derivedCrates = deriveFolderCratesFromTracks(allTracks, []);
  const crateSummary = derivedCrates.find((c) => c.crate_id === crateId);
  const { childrenByParent } = normalizeFolderHierarchy(derivedCrates);
  const subfolders = childrenByParent.get(crateId) ?? [];
  const normTarget = crateSummary?.folder_path
    ? crateSummary.folder_path.replace(/\\/g, '/').replace(/\/+$/, '') || '/'
    : null;

  const matching = allTracks.filter((t) => {
    const meta = t as TrackWithScanMeta;
    if (meta.crate_id === crateId) return true;
    if (!normTarget) return false;
    const p = t.path.replace(/\\/g, '/');
    const idx = p.lastIndexOf('/');
    const parent = idx <= 0 ? '/' : p.slice(0, idx);
    return parent.toLowerCase() === normTarget.toLowerCase();
  });

  if (!crateSummary && matching.length === 0) return null;
  const detail = makeSyntheticFolderDetail(
    crateId,
    crateSummary?.display_name ?? crateId,
    matching,
    crateSummary?.folder_path ?? '',
    subfolders,
  );
  if (crateSummary) {
    detail.summary = { ...crateSummary, subfolder_count: subfolders.length };
  }
  return { detail, tracks: matching };
}

export async function searchAllFoldersDeduplicated(
  query: string,
): Promise<OmkfTrack[]> {
  const res = await readFolderCrate(`__search__:${query.trim()}`);
  return res?.tracks ?? [];
}

export async function queryHarmonicCandidates(
  bpm: number,
  camelot: string,
): Promise<Track[]> {
  const res = await readFolderCrate(`__harmonic__:${bpm}:${camelot}`);
  return res?.tracks ?? [];
}

export async function saveCrateRules(
  crateId: string,
  rulesMarkdown: string,
): Promise<boolean> {
  if (!isTauri()) return true;
  try {
    const res = await invoke<boolean>('plugin:hera-dj|omkf_save_crate_rules', {
      crateId,
      rulesMarkdown,
    });
    return res ?? true;
  } catch {
    return false;
  }
}

export async function previewOrApplyOrganize(
  request: OrganizeFolderRequest,
): Promise<OrganizeFolderResponse> {
  if (isTauri()) {
    try {
      const res = await invoke<OrganizeFolderResponse>(
        'plugin:hera-dj|omkf_preview_or_apply_organize',
        { request },
      );
      if (res && typeof res.crate_id === 'string') return res;
    } catch {
      // Fallback below
    }
  }
  return {
    crate_id: request.crate_id,
    dry_run: request.dry_run,
    tx_id: `tx-preview-${Date.now()}`,
    moves: [],
    applied_count: 0,
    rolled_back: false,
  };
}
