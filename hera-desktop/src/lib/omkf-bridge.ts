import { invoke } from '@tauri-apps/api/core';

import type { Track } from '../generated/typings';
import DatabaseBridge from './bridge-database';
import type { TrackWithScanMeta } from './scanner-v3';
import useLibraryStore from './store';

export const FOLDER_CRATE_PREFIX = 'folder--';

export function isFolderPlaylistId(playlistId?: string | null): boolean {
  return Boolean(playlistId && playlistId.startsWith(FOLDER_CRATE_PREFIX));
}

export function toFolderPlaylistId(crateId: string): string {
  return crateId.startsWith(FOLDER_CRATE_PREFIX)
    ? crateId
    : `${FOLDER_CRATE_PREFIX}${crateId}`;
}

export function fromFolderPlaylistId(playlistId: string): string {
  return playlistId.startsWith(FOLDER_CRATE_PREFIX)
    ? playlistId.slice(FOLDER_CRATE_PREFIX.length)
    : playlistId;
}

export interface FolderCrateSummary {
  crate_id: string;
  folder_path: string;
  display_name: string;
  tree_seal_64: string;
  track_count: number;
  video_count: number;
  full_count: number;
  bpm_min?: number | null;
  bpm_max?: number | null;
  dominant_camelot?: string | null;
  dominant_genre?: string | null;
  era_span?: string | null;
  lufs_avg?: number | null;
  has_custom_rules: boolean;
  updated_at_unix_ms?: number;
}

export interface FolderCrateDetail {
  summary: FolderCrateSummary;
  rules_markdown: string;
  ledger_entries: string[];
  track_ids: string[];
  hydrated_from_omkf: boolean;
  disk_online: boolean;
}

export interface HydratedFolderCrate {
  detail: FolderCrateDetail;
  tracks: Track[];
}

export interface PlannedFileMove {
  track_id: string;
  source_path: string;
  target_path: string;
  reason: string;
  bucket_label: string;
}

export interface OrganizeFolderRequest {
  crate_id: string;
  strategy: 'by_tempo_zone' | 'by_genre_era' | 'quarantine_issues';
  dry_run: boolean;
  rollback_tx_id?: string | null;
}

export interface OrganizeFolderResponse {
  crate_id: string;
  dry_run: boolean;
  tx_id: string;
  moves: PlannedFileMove[];
  applied_count: number;
  rolled_back: boolean;
}

export function formatFolderCrateSubtitle(crate: FolderCrateSummary): string {
  const bpmSpan =
    crate.bpm_min != null && crate.bpm_max != null && crate.bpm_max > 0
      ? `${Math.round(crate.bpm_min)}–${Math.round(crate.bpm_max)} BPM`
      : 'BPM ···';
  const key = crate.dominant_camelot || '—';
  const pct =
    crate.track_count > 0
      ? Math.round((crate.full_count / crate.track_count) * 100)
      : 0;
  return `${bpmSpan} · ${key} · ${pct}% FULL`;
}

export function deriveFolderCratesFromTracks(
  tracks: Track[],
  persisted: FolderCrateSummary[],
): FolderCrateSummary[] {
  const byFolder = new Map<string, FolderCrateSummary>();
  for (const c of persisted) {
    const key = c.folder_path
      .replace(/\\/g, '/')
      .replace(/\/+$/, '')
      .toLowerCase();
    byFolder.set(key || c.crate_id, c);
  }
  const buckets = new Map<
    string,
    { rawFolder: string; displayName: string; crateId: string; tracks: Track[] }
  >();
  for (const t of tracks) {
    const norm = t.path.replace(/\\/g, '/');
    const lastSlash = norm.lastIndexOf('/');
    if (lastSlash <= 0) continue;
    const rawFolder = norm.slice(0, lastSlash);
    const key = rawFolder.toLowerCase();
    if (byFolder.has(key)) continue;
    const meta = t as TrackWithScanMeta;
    let b = buckets.get(key);
    if (!b) {
      const segs = rawFolder.split('/').filter(Boolean);
      b = {
        rawFolder,
        displayName: segs[segs.length - 1] ?? rawFolder,
        crateId: meta.crate_id || `dir:${key}`,
        tracks: [],
      };
      buckets.set(key, b);
    }
    b.tracks.push(t);
  }
  for (const [key, b] of buckets.entries()) {
    let bpmMin = Number.POSITIVE_INFINITY;
    let bpmMax = 0;
    let fullCount = 0;
    let videoCount = 0;
    let domCam: string | null = null;
    let bestCamCount = 0;
    const camCounts = new Map<string, number>();
    for (const t of b.tracks) {
      const meta = t as TrackWithScanMeta;
      if (t.is_video) videoCount++;
      if ((meta.scan_level ?? 0) >= 2 || (t.hot_cues && t.hot_cues.length >= 4))
        fullCount++;
      if (t.bpm && t.bpm > 0) {
        if (t.bpm < bpmMin) bpmMin = t.bpm;
        if (t.bpm > bpmMax) bpmMax = t.bpm;
      }
      const k = t.camelot?.trim();
      if (k) {
        const cnt = (camCounts.get(k) ?? 0) + 1;
        camCounts.set(k, cnt);
        if (cnt > bestCamCount) {
          bestCamCount = cnt;
          domCam = k;
        }
      }
    }
    byFolder.set(key, {
      crate_id: b.crateId,
      folder_path: b.rawFolder,
      display_name: b.displayName,
      tree_seal_64: '0',
      track_count: b.tracks.length,
      video_count: videoCount,
      full_count: fullCount,
      bpm_min: Number.isFinite(bpmMin) ? bpmMin : null,
      bpm_max: bpmMax > 0 ? bpmMax : null,
      dominant_camelot: domCam,
      dominant_genre: null,
      era_span: null,
      lufs_avg: null,
      has_custom_rules: false,
    });
  }
  return Array.from(byFolder.values()).sort((a, b) =>
    a.display_name.localeCompare(b.display_name),
  );
}

export async function listFolderCrates(): Promise<FolderCrateSummary[]> {
  try {
    const res = await invoke<FolderCrateSummary[]>(
      'plugin:hera-dj|omkf_list_folder_crates',
    );
    if (Array.isArray(res)) return res;
  } catch {
    // Fallback outside native Tauri IPC
  }
  return [];
}

export async function readFolderCrate(
  crateId: string,
): Promise<HydratedFolderCrate | null> {
  try {
    const detail = await invoke<FolderCrateDetail | null>(
      'plugin:hera-dj|omkf_read_folder_crate',
      { crateId },
    );
    if (detail && detail.summary) {
      let tracks: Track[] = [];
      if (detail.track_ids.length > 0) {
        try {
          tracks = await DatabaseBridge.getTracks(detail.track_ids);
        } catch {
          const idSet = new Set(detail.track_ids);
          tracks = useLibraryStore
            .getState()
            .tracks.filter((t) => idSet.has(t.id));
        }
      }
      return { detail, tracks };
    }
  } catch {
    // Fallback outside native Tauri IPC
  }

  const allTracks = useLibraryStore.getState().tracks;
  const matching = allTracks.filter(
    (t) => (t as TrackWithScanMeta).crate_id === crateId,
  );
  if (matching.length === 0) return null;
  return {
    detail: {
      summary: {
        crate_id: crateId,
        folder_path: '',
        display_name: crateId,
        tree_seal_64: '0',
        track_count: matching.length,
        video_count: matching.filter((t) => t.is_video).length,
        full_count: matching.filter(
          (t) => ((t as TrackWithScanMeta).scan_level ?? 0) >= 2,
        ).length,
        bpm_min: null,
        bpm_max: null,
        dominant_camelot: null,
        dominant_genre: null,
        era_span: null,
        lufs_avg: null,
        has_custom_rules: false,
        updated_at_unix_ms: Date.now(),
      },
      rules_markdown: '',
      ledger_entries: [],
      track_ids: matching.map((t) => t.id),
      hydrated_from_omkf: false,
      disk_online: false,
    },
    tracks: matching,
  };
}

export async function saveCrateRules(
  crateId: string,
  rulesMarkdown: string,
): Promise<boolean> {
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
  try {
    const res = await invoke<OrganizeFolderResponse>(
      'plugin:hera-dj|omkf_preview_or_apply_organize',
      { request },
    );
    if (res && typeof res.crate_id === 'string') return res;
  } catch {
    // Fallback outside native Tauri IPC
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
