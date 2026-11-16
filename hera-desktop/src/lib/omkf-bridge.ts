import { invoke } from '@tauri-apps/api/core';

import type { Track } from '../generated/typings';
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
  dir_mtime_ns: number;
  track_count: number;
  video_count: number;
  full_count: number;
  bpm_min?: number | null;
  bpm_max?: number | null;
  dominant_camelot?: string | null;
  dominant_genre?: string | null;
  era_span?: string | null;
  lufs_avg?: number | null;
  rules_markdown: string;
  updated_at_unix_ms: number;
}

export interface FolderCrateDetail {
  summary: FolderCrateSummary;
  tracks: Track[];
  crate_markdown: string;
  rules_markdown: string;
  ledger_markdown: string;
  offline_fallback: boolean;
}

export interface PlannedFileMove {
  track_id: string;
  source_path: string;
  target_path: string;
  target_folder: string;
  reason: string;
}

export interface OrganizeFolderRequest {
  crate_id: string;
  strategy: 'by_tempo_zone' | 'by_genre_era' | 'quarantine_issues';
  dry_run: boolean;
  rollback_tx_id?: string | null;
}

export interface OrganizeFolderResponse {
  crate_id: string;
  tx_id: string;
  dry_run: boolean;
  applied: boolean;
  planned_moves: PlannedFileMove[];
  ledger_entry?: string | null;
  message: string;
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
): Promise<FolderCrateDetail | null> {
  try {
    const res = await invoke<FolderCrateDetail | null>(
      'plugin:hera-dj|omkf_read_folder_crate',
      { crateId },
    );
    if (res && res.summary) return res;
  } catch {
    // Fallback outside native Tauri IPC
  }
  const allTracks = useLibraryStore.getState().tracks;
  const matching = allTracks.filter(
    (t) => (t as TrackWithScanMeta).crate_id === crateId,
  );
  if (matching.length === 0) return null;
  return {
    summary: {
      crate_id: crateId,
      folder_path: '',
      display_name: crateId,
      tree_seal_64: '0',
      dir_mtime_ns: 0,
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
      rules_markdown: '',
      updated_at_unix_ms: Date.now(),
    },
    tracks: matching,
    crate_markdown: '',
    rules_markdown: '',
    ledger_markdown: '',
    offline_fallback: true,
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
    tx_id: `tx-preview-${Date.now()}`,
    dry_run: request.dry_run,
    applied: false,
    planned_moves: [],
    ledger_entry: null,
    message: 'Vista previa sin motor Tauri activo.',
  };
}
