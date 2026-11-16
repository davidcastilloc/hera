import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';

import type {
  DiscoveredMusicRoot,
  HotCuePin,
  Track,
} from '../generated/typings';
import useLibraryStore from './store';

export type TrackWithScanMeta = Omit<
  Track,
  | 'scan_level'
  | 'is_transition_candidate'
  | 'is_fake_320kbps'
  | 'waveform_peaks_blob'
> & {
  scan_level?: number | null;
  is_transition_candidate?: boolean | null;
  is_fake_320kbps?: boolean | null;
  waveform_peaks_blob?: number[] | null;
  crate_id?: string | null;
};

export interface ScannerRowPatch {
  track_id: string;
  bpm?: number | null;
  camelot?: string | null;
  energy?: number | null;
  duration?: number | null;
  hot_cues?: HotCuePin[] | null;
  scan_level?: number | null;
  is_transition_candidate?: boolean | null;
  is_fake_320kbps?: boolean | null;
  waveform_peaks_b64?: string | null;
  has_audio_skips?: boolean | null;
  is_variable_bpm?: boolean | null;
  has_intro_vocals?: boolean | null;
  has_outro_vocals?: boolean | null;
  replay_gain_db?: number | null;
  crate_id?: string | null;
}

export type PreGigTargetProfile = 'virtualdj' | 'usb_cdj' | 'denon_cd_mp3';

export interface PreGigTrackIssue {
  track_id: string;
  title: string;
  artist: string;
  path: string;
  issue_kind: string;
  severity: string;
  message: string;
}

export interface PreGigAuditReport {
  target_profile: string;
  total_tracks_checked: number;
  ready_count: number;
  warning_count: number;
  blocking_count: number;
  issues: PreGigTrackIssue[];
}

export interface SonicTwinMatch {
  track_id: string;
  title: string;
  artist: string;
  bpm: number;
  camelot: string;
  similarity: number;
  inferred_genre_family?: string | null;
  inferred_era_year?: number | null;
}

export interface SonicTwinSession {
  sourceTrackId: string;
  sourceTitle: string;
  matches: SonicTwinMatch[];
}

export interface ShazamIdentifyResult {
  matched: boolean;
  track_id: string;
  title?: string | null;
  artist?: string | null;
  confidence: number;
  source: string;
}

export interface ScannerTelemetry {
  phase?: 'indexing' | 'analyzing' | 'idle';
  indexedDone?: number;
  fastDone: number;
  fullDone: number;
  total: number;
  tracksPerSec: number;
  etaSec: number;
  governorActive: boolean;
  externalBoothMode?: boolean;
}

interface ScannerStoreState extends ScannerTelemetry {
  isPointerInsideTable: boolean;
  externalBoothMode: boolean;
  sonicTwins: SonicTwinSession | null;
  pendingFrozenPatches: ScannerRowPatch[];
  recentlyUpdatedIds: Record<string, number>;
  setTelemetry: (t: Partial<ScannerTelemetry>) => void;
  setPointerInsideTable: (inside: boolean) => void;
  setExternalBoothMode: (enabled: boolean) => void;
  setSonicTwins: (session: SonicTwinSession | null) => void;
  queueFrozenPatches: (patches: ScannerRowPatch[]) => void;
  clearFrozenPatches: () => ScannerRowPatch[];
  markTracksRecentlyUpdated: (ids: string[]) => void;
}

let recentGlowCleanupTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleRecentGlowCleanup(
  get: () => ScannerStoreState,
  set: (updater: (s: ScannerStoreState) => Partial<ScannerStoreState>) => void,
) {
  if (recentGlowCleanupTimer) clearTimeout(recentGlowCleanupTimer);
  recentGlowCleanupTimer = setTimeout(() => {
    const currentNow = Date.now();
    const currentMap = get().recentlyUpdatedIds;
    const nextMap: Record<string, number> = {};
    let changed = false;

    for (const [id, ts] of Object.entries(currentMap)) {
      if (currentNow - ts < 750) {
        nextMap[id] = ts;
      } else {
        changed = true;
      }
    }

    if (changed) {
      set((state) => ({ ...state, recentlyUpdatedIds: nextMap }));
    }

    if (Object.keys(nextMap).length > 0) {
      scheduleRecentGlowCleanup(get, set);
    } else {
      recentGlowCleanupTimer = null;
    }
  }, 780);
}

export const useScannerStore = create<ScannerStoreState>((set, get) => ({
  phase: 'idle',
  indexedDone: 0,
  fastDone: 0,
  fullDone: 0,
  total: 0,
  tracksPerSec: 0,
  etaSec: 0,
  governorActive: false,
  isPointerInsideTable: false,
  externalBoothMode: false,
  sonicTwins: null,
  pendingFrozenPatches: [],
  recentlyUpdatedIds: {},
  setTelemetry: (t) =>
    set((state) => ({
      ...state,
      ...t,
      externalBoothMode:
        t.externalBoothMode !== undefined
          ? t.externalBoothMode
          : state.externalBoothMode,
    })),
  setPointerInsideTable: (inside) => set({ isPointerInsideTable: inside }),
  setExternalBoothMode: (enabled) =>
    set({ externalBoothMode: enabled, governorActive: enabled }),
  setSonicTwins: (sonicTwins) => set({ sonicTwins }),
  queueFrozenPatches: (patches) =>
    set((state) => ({
      pendingFrozenPatches: [...state.pendingFrozenPatches, ...patches],
    })),
  clearFrozenPatches: () => {
    const pending = get().pendingFrozenPatches;
    set({ pendingFrozenPatches: [] });
    return pending;
  },
  markTracksRecentlyUpdated: (ids) => {
    if (!ids || ids.length === 0) return;
    const now = Date.now();
    set((state) => {
      const next = { ...state.recentlyUpdatedIds };
      for (const id of ids) {
        next[id] = now;
      }
      return { ...state, recentlyUpdatedIds: next };
    });
    scheduleRecentGlowCleanup(get, set);
  },
}));

export function markTracksRecentlyUpdated(ids: string[]): void {
  useScannerStore.getState().markTracksRecentlyUpdated(ids);
}

let hoverTimer: ReturnType<typeof setTimeout> | null = null;
let viewportTimer: ReturnType<typeof setTimeout> | null = null;

/** Hover Pre-Warming (300 ms): promueve el track bajo el cursor a P0Immediate. */
export function scheduleHoverPrewarm(trackId: string): void {
  if (hoverTimer) clearTimeout(hoverTimer);
  hoverTimer = setTimeout(() => {
    void invoke<number>('plugin:hera-dj|scanner_prewarm_track', {
      trackId,
    }).catch(() => {});
  }, 300);
}

export function cancelHoverPrewarm(): void {
  if (hoverTimer) {
    clearTimeout(hoverTimer);
    hoverTimer = null;
  }
}

/** Viewport Lookahead (120 ms): promueve los tracks visibles a P1Viewport. */
export function scheduleViewportBoost(trackIds: string[]): void {
  if (viewportTimer) clearTimeout(viewportTimer);
  viewportTimer = setTimeout(() => {
    if (trackIds.length > 0) {
      void invoke<number>('plugin:hera-dj|scanner_boost_viewport', {
        trackIds,
      }).catch(() => {});
    }
  }, 120);
}

export function notifyDeckPlaying(playing: boolean): void {
  void invoke<number>('plugin:hera-dj|scanner_set_deck_playing', {
    playing,
  }).catch(() => {});
}

export function notifyDeckTrack(
  trackId: string,
  camelotNeighbors: string[],
): void {
  void invoke<number>('plugin:hera-dj|scanner_notify_deck_track', {
    trackId,
    camelotNeighbors,
  }).catch(() => {});
}

export interface TrackLoadMeta {
  replayGainDb?: number;
  hotCuesSec?: number[];
}

const trackLoadMetaByPath = new Map<string, TrackLoadMeta>();

export function registerTrackLoadMetadata(
  filePath: string,
  replayGainDb?: number | null,
  hotCuesSec?: number[] | null,
): void {
  const gain =
    replayGainDb != null && Number.isFinite(replayGainDb)
      ? replayGainDb
      : undefined;
  const cues =
    hotCuesSec && hotCuesSec.length >= 4 ? hotCuesSec.slice(0, 4) : undefined;
  trackLoadMetaByPath.set(filePath, { replayGainDb: gain, hotCuesSec: cues });
  if (trackLoadMetaByPath.size > 256) {
    const oldest = trackLoadMetaByPath.keys().next().value;
    if (oldest !== undefined) trackLoadMetaByPath.delete(oldest);
  }
}

export function getTrackLoadMetadata(
  filePath: string,
): TrackLoadMeta | undefined {
  return trackLoadMetaByPath.get(filePath);
}

export async function cancelBackgroundScan(): Promise<boolean> {
  try {
    const res = await invoke<boolean>('plugin:hera-dj|scanner_cancel_scan');
    useScannerStore.setState({ phase: 'idle' });
    return res ?? true;
  } catch {
    try {
      const res = await invoke<boolean>('plugin:hera-dj|cancel_library_scan');
      useScannerStore.setState({ phase: 'idle' });
      return res ?? true;
    } catch {
      useScannerStore.setState({ phase: 'idle' });
      return false;
    }
  }
}

export async function setExternalBoothMode(enabled: boolean): Promise<boolean> {
  useScannerStore.getState().setExternalBoothMode(enabled);
  try {
    const res = await invoke<boolean>(
      'plugin:hera-dj|scanner_set_external_booth_mode',
      { enabled },
    );
    return res ?? enabled;
  } catch {
    return enabled;
  }
}

export async function discoverWorkableRoots(): Promise<DiscoveredMusicRoot[]> {
  try {
    const res = await invoke<DiscoveredMusicRoot[]>(
      'plugin:hera-dj|scanner_discover_workable_roots',
    );
    return Array.isArray(res) ? res : [];
  } catch {
    return [];
  }
}

export async function promoteCrateFull(
  crateId: string,
  trackIds?: string[],
): Promise<number> {
  try {
    const res = await invoke<number>(
      'plugin:hera-dj|scanner_promote_crate_full',
      {
        crateId,
        trackIds: trackIds ?? null,
      },
    );
    return typeof res === 'number' ? res : 0;
  } catch {
    return 0;
  }
}

export function buildLocalPreGigAuditFallback(
  targetProfile: PreGigTargetProfile,
  tracks: Track[],
): PreGigAuditReport {
  const issues: PreGigTrackIssue[] = [];
  let warningCount = 0;
  let blockingCount = 0;

  for (const t of tracks) {
    const artist = t.artists.join(', ') || t.album_artist || 'Artista';
    const meta = t as TrackWithScanMeta;
    if ((meta.scan_level ?? 0) < 0) {
      blockingCount++;
      issues.push({
        track_id: t.id,
        title: t.title,
        artist,
        path: t.path,
        issue_kind: 'corrupt_file',
        severity: 'error',
        message: 'Archivo en cuarentena (corrupto o ilegible).',
      });
      continue;
    }
    if (t.has_audio_skips) {
      blockingCount++;
      issues.push({
        track_id: t.id,
        title: t.title,
        artist,
        path: t.path,
        issue_kind: 'cd_skip_glitch',
        severity: 'error',
        message: 'Posible salto de audio / CD rayado detectado.',
      });
    }
    if (
      (targetProfile === 'usb_cdj' || targetProfile === 'denon_cd_mp3') &&
      t.is_video
    ) {
      blockingCount++;
      issues.push({
        track_id: t.id,
        title: t.title,
        artist,
        path: t.path,
        issue_kind: 'video_on_cdj',
        severity: 'error',
        message: 'Pista de video incompatible con reproductores CDJ/Denon.',
      });
    }
    if (
      targetProfile === 'denon_cd_mp3' &&
      t.sample_rate_hz != null &&
      t.sample_rate_hz !== 44100
    ) {
      blockingCount++;
      issues.push({
        track_id: t.id,
        title: t.title,
        artist,
        path: t.path,
        issue_kind: 'sample_rate_incompatible',
        severity: 'error',
        message: `Sample rate ${t.sample_rate_hz} Hz incompatible (Denon CD/MP3 requiere 44.1 kHz).`,
      });
    }
    if (t.is_fake_320kbps) {
      warningCount++;
      issues.push({
        track_id: t.id,
        title: t.title,
        artist,
        path: t.path,
        issue_kind: 'fake_320kbps',
        severity: 'warning',
        message: 'Posible transcodificación / falso 320 kbps (< 16 kHz).',
      });
    }
  }

  const flaggedIds = new Set(issues.map((i) => i.track_id));
  return {
    target_profile: targetProfile,
    total_tracks_checked: tracks.length,
    ready_count: Math.max(0, tracks.length - flaggedIds.size),
    warning_count: warningCount,
    blocking_count: blockingCount,
    issues,
  };
}

export async function auditPreGigCrate(
  targetProfile: PreGigTargetProfile,
  trackIds?: string[],
): Promise<PreGigAuditReport> {
  const storeTracks = useLibraryStore.getState().tracks;
  const scopedTracks =
    trackIds && trackIds.length > 0
      ? storeTracks.filter((t) => trackIds.includes(t.id))
      : storeTracks;
  try {
    const res = await invoke<PreGigAuditReport>(
      'plugin:hera-dj|scanner_audit_pre_gig_crate',
      {
        targetProfile,
        trackIds: trackIds ?? null,
      },
    );
    if (res && typeof res.total_tracks_checked === 'number') {
      return res;
    }
  } catch {
    // Fallback outside native Tauri IPC
  }
  return buildLocalPreGigAuditFallback(targetProfile, scopedTracks);
}

export async function findSonicTwins(
  trackId: string,
  limit = 15,
): Promise<SonicTwinMatch[]> {
  try {
    const res = await invoke<SonicTwinMatch[]>(
      'plugin:hera-dj|scanner_find_sonic_twins',
      { trackId, limit },
    );
    if (Array.isArray(res)) return res;
  } catch {
    // Fallback outside native Tauri IPC
  }
  const all = useLibraryStore.getState().tracks;
  const src = all.find((t) => t.id === trackId);
  if (!src) return [];
  const srcBpm = src.bpm ?? 120;
  return all
    .filter((t) => t.id !== trackId && (t.bpm ?? 0) > 0)
    .map((t) => {
      const diff = Math.abs((t.bpm ?? srcBpm) - srcBpm);
      const sameKey =
        t.camelot && src.camelot && t.camelot === src.camelot ? 0.15 : 0;
      const sim = Math.max(0, Math.min(0.99, 0.84 - diff * 0.02 + sameKey));
      return {
        track_id: t.id,
        title: t.title,
        artist: t.artists.join(', ') || t.album_artist || 'Artista',
        bpm: t.bpm ?? srcBpm,
        camelot: t.camelot ?? '—',
        similarity: sim,
        inferred_genre_family: t.inferred_genre_family ?? null,
        inferred_era_year: t.inferred_era_year ?? t.year ?? null,
      };
    })
    .sort((a, b) => b.similarity - a.similarity)
    .slice(0, limit);
}

export async function identifyTrackShazam(
  trackId: string,
  path?: string,
): Promise<ShazamIdentifyResult | null> {
  try {
    const res = await invoke<ShazamIdentifyResult | null>(
      'plugin:hera-dj|identify_track_shazam',
      { trackId, path: path ?? null },
    );
    return res ?? null;
  } catch {
    return null;
  }
}
