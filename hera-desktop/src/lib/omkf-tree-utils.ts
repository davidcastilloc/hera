import type {
  FolderCrateDetail,
  FolderCrateSummary,
  OmkfFsTrackDto,
} from '../generated/hera-typings';
import type { HotCuePin, Track } from '../generated/typings';
import type { TrackWithScanMeta } from './scanner-v3';
import { filterTracks, stripAccents } from './utils-library';

export const FOLDER_CRATE_PREFIX = 'folder--';

const HOT_CUE_COLORS: Record<number, string> = {
  1: '#22c55e',
  2: '#38bdf8',
  3: '#ef4444',
  4: '#f59e0b',
};

const NON_PRIMARY_FOLDER_RE =
  /backup|respaldo|archivo|recycle|trash|quarantine/i;

export type OmkfTrack = Track & {
  crate_id?: string | null;
  copy_count?: number;
};

export interface FolderTreeRow {
  crate: FolderCrateSummary;
  depth: number;
  hasChildren: boolean;
  isExpanded: boolean;
}

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

export function formatFolderCrateSubtitle(crate: FolderCrateSummary): string {
  if (crate.track_count === 0 && (crate.subfolder_count ?? 0) > 0) {
    const subCount = crate.subfolder_count ?? 0;
    return subCount === 1 ? '1 subcarpeta' : `${subCount} subcarpetas`;
  }
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

export function mapOmkfDtoToTrack(dto: OmkfFsTrackDto): OmkfTrack {
  const hotCues: HotCuePin[] = (dto.hot_cues ?? []).map((c) => ({
    num: c.slot,
    name: c.label,
    seconds: c.seconds,
    color: HOT_CUE_COLORS[c.slot] ?? '#14b8a6',
    role: c.kind,
    origin: 'omkf',
  }));

  return {
    id: dto.id,
    path: dto.path,
    title: dto.title,
    album: dto.album,
    album_artist: dto.artist,
    artists: dto.artist ? [dto.artist] : [],
    genres: [],
    year: null,
    duration: dto.duration_sec,
    track_no: null,
    track_of: null,
    disk_no: null,
    disk_of: null,
    is_compilation: false,
    bpm: dto.bpm ?? undefined,
    camelot: dto.camelot ?? undefined,
    energy: dto.energy ?? undefined,
    hot_cues: hotCues,
    scan_level: dto.scan_level,
    is_video: dto.is_video,
    is_fake_320kbps: dto.is_fake_320kbps,
    has_audio_skips: dto.has_audio_skips,
    is_variable_bpm: dto.is_variable_bpm,
    has_intro_vocals: dto.has_intro_vocals,
    has_outro_vocals: dto.has_outro_vocals,
    replay_gain_db: dto.replay_gain_db ?? undefined,
    waveform_peaks_b64: dto.waveform_peaks_b64 ?? undefined,
    is_mix_over_10m: dto.duration_sec > 600,
    crate_id: dto.crate_id,
    copy_count: dto.copy_count,
  };
}

function normFolderPath(raw: string): string {
  const clean = raw.replace(/\\/g, '/').replace(/\/+$/, '');
  return clean === '' && raw.startsWith('/') ? '/' : clean.toLowerCase();
}

export function normalizeFolderHierarchy(crates: FolderCrateSummary[]): {
  byId: Map<string, FolderCrateSummary>;
  childrenByParent: Map<string | null, FolderCrateSummary[]>;
  parentById: Map<string, string | null>;
  depthById: Map<string, number>;
} {
  const byId = new Map<string, FolderCrateSummary>();
  for (const c of crates) {
    byId.set(c.crate_id, c);
  }

  const hasExplicitParent = crates.some((c) => c.parent_crate_id != null);
  const parentById = new Map<string, string | null>();

  if (hasExplicitParent) {
    for (const c of crates) {
      const p = c.parent_crate_id ?? null;
      parentById.set(c.crate_id, p && byId.has(p) ? p : null);
    }
  } else {
    const normEntries = crates.map((c) => ({
      id: c.crate_id,
      norm: normFolderPath(c.folder_path),
    }));
    for (const entry of normEntries) {
      let bestParentId: string | null = null;
      let bestLen = -1;
      if (entry.norm && entry.norm !== '/') {
        for (const cand of normEntries) {
          if (cand.id === entry.id || !cand.norm) continue;
          const isPrefix =
            cand.norm === '/'
              ? entry.norm.startsWith('/')
              : entry.norm.startsWith(`${cand.norm}/`);
          if (isPrefix && cand.norm.length > bestLen) {
            bestLen = cand.norm.length;
            bestParentId = cand.id;
          }
        }
      }
      parentById.set(entry.id, bestParentId);
    }
  }

  const depthById = new Map<string, number>();
  const resolveDepth = (id: string, visited = new Set<string>()): number => {
    const cached = depthById.get(id);
    if (cached !== undefined) return cached;
    const explicit = byId.get(id)?.depth;
    if (hasExplicitParent && typeof explicit === 'number') {
      depthById.set(id, explicit);
      return explicit;
    }
    const pid = parentById.get(id) ?? null;
    if (!pid || visited.has(id)) {
      depthById.set(id, 0);
      return 0;
    }
    visited.add(id);
    const d = resolveDepth(pid, visited) + 1;
    depthById.set(id, d);
    return d;
  };

  const childrenByParent = new Map<string | null, FolderCrateSummary[]>();
  for (const c of crates) {
    resolveDepth(c.crate_id);
    const pid = parentById.get(c.crate_id) ?? null;
    const list = childrenByParent.get(pid) ?? [];
    list.push(c);
    childrenByParent.set(pid, list);
  }

  return { byId, childrenByParent, parentById, depthById };
}

export function getDefaultExpandedCrateIds(
  crates: FolderCrateSummary[],
  selectedCrateId?: string | null,
): Set<string> {
  const { parentById, depthById } = normalizeFolderHierarchy(crates);
  const expanded = new Set<string>();
  for (const c of crates) {
    if ((depthById.get(c.crate_id) ?? 0) === 0) {
      expanded.add(c.crate_id);
    }
  }
  let cur = selectedCrateId ? (parentById.get(selectedCrateId) ?? null) : null;
  let guard = 0;
  while (cur && guard++ < 32) {
    expanded.add(cur);
    cur = parentById.get(cur) ?? null;
  }
  return expanded;
}

export function buildFolderTreeRows(
  crates: FolderCrateSummary[],
  expandedIds?: ReadonlySet<string> | string[] | null,
): FolderTreeRow[] {
  if (crates.length === 0) return [];
  const { childrenByParent, depthById } = normalizeFolderHierarchy(crates);
  const expandedSet =
    expandedIds instanceof Set
      ? expandedIds
      : Array.isArray(expandedIds)
        ? new Set(expandedIds)
        : getDefaultExpandedCrateIds(crates);

  const out: FolderTreeRow[] = [];
  const visit = (parentId: string | null) => {
    const siblings = childrenByParent.get(parentId) ?? [];
    for (const crate of siblings) {
      const children = childrenByParent.get(crate.crate_id) ?? [];
      const isExpanded = expandedSet.has(crate.crate_id);
      out.push({
        crate,
        depth: depthById.get(crate.crate_id) ?? 0,
        hasChildren: children.length > 0,
        isExpanded: children.length > 0 && isExpanded,
      });
      if (children.length > 0 && isExpanded) {
        visit(crate.crate_id);
      }
    }
  };

  visit(null);
  return out;
}

export function buildFolderBreadcrumbs(
  crates: FolderCrateSummary[],
  selectedCrateId: string | null,
): FolderCrateSummary[] {
  if (!selectedCrateId || crates.length === 0) return [];
  const { byId, parentById } = normalizeFolderHierarchy(crates);
  const chain: FolderCrateSummary[] = [];
  let cur: string | null = selectedCrateId;
  const seen = new Set<string>();
  while (cur && !seen.has(cur)) {
    seen.add(cur);
    const node = byId.get(cur);
    if (!node) break;
    chain.push(node);
    cur = parentById.get(cur) ?? null;
  }
  chain.reverse();
  return chain;
}

export function pickInitialFolderCrateId(
  crates: FolderCrateSummary[],
): string | null {
  if (crates.length === 0) return null;
  const isWorkable = (c: FolderCrateSummary) =>
    !NON_PRIMARY_FOLDER_RE.test(c.folder_path) &&
    !NON_PRIMARY_FOLDER_RE.test(c.display_name);

  const activeWithTracks = crates.find(
    (c) => c.track_count > 0 && isWorkable(c),
  );
  if (activeWithTracks) return activeWithTracks.crate_id;

  const activeContainer = crates.find(isWorkable);
  if (activeContainer) return activeContainer.crate_id;

  return crates[0]?.crate_id ?? null;
}

export function deriveFolderCratesFromTracks(
  tracks: Track[],
  persisted: FolderCrateSummary[],
): FolderCrateSummary[] {
  const byFolder = new Map<string, FolderCrateSummary>();
  for (const c of persisted) {
    const key = normFolderPath(c.folder_path);
    byFolder.set(key || c.crate_id, c);
  }
  const buckets = new Map<
    string,
    { rawFolder: string; displayName: string; crateId: string; tracks: Track[] }
  >();
  for (const t of tracks) {
    const norm = t.path.replace(/\\/g, '/');
    const lastSlash = norm.lastIndexOf('/');
    if (lastSlash < 0) continue;
    const rawFolder = lastSlash === 0 ? '/' : norm.slice(0, lastSlash);
    const key = normFolderPath(rawFolder) || '/';
    if (byFolder.has(key)) continue;
    const meta = t as TrackWithScanMeta;
    let b = buckets.get(key);
    if (!b) {
      const segs = rawFolder.split('/').filter(Boolean);
      b = {
        rawFolder,
        displayName:
          segs[segs.length - 1] ?? (rawFolder === '/' ? 'Carpetas' : rawFolder),
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
      parent_crate_id: null,
      rel_path: b.displayName,
      depth: 0,
      subfolder_count: 0,
    });
  }
  return Array.from(byFolder.values()).sort((a, b) =>
    a.display_name.localeCompare(b.display_name),
  );
}

export function deduplicateMockSearchTracks(
  tracks: Track[],
  query: string,
): OmkfTrack[] {
  const cleanQ = stripAccents(query.trim());
  const matched = cleanQ ? filterTracks(tracks, cleanQ) : tracks;
  const groups = new Map<
    string,
    { best: Track; count: number; prio: number }
  >();

  for (const t of matched) {
    const artistSlug = stripAccents(
      (t.artists[0] ?? t.album_artist ?? '').toLowerCase(),
    ).trim();
    const titleSlug = stripAccents(t.title.toLowerCase()).trim();
    const durBucket = Math.round((t.duration || 0) / 10);
    const key = `${artistSlug}::${titleSlug}::${durBucket}`;
    const prio = NON_PRIMARY_FOLDER_RE.test(t.path) ? -10 : 10;
    const prev = groups.get(key);
    if (!prev) {
      groups.set(key, { best: t, count: 1, prio });
    } else {
      prev.count += 1;
      if (prio > prev.prio) {
        prev.best = t;
        prev.prio = prio;
      }
    }
  }

  return Array.from(groups.values())
    .slice(0, 200)
    .map(({ best, count }) => ({ ...best, copy_count: count }));
}

export function filterMockHarmonicTracks(
  tracks: Track[],
  spec: string,
): Track[] {
  const [bpmRaw = '', camRaw = ''] = spec.split(':');
  const targetBpm = Number.parseFloat(bpmRaw) || 0;
  const targetCam = camRaw.trim().toUpperCase();
  const workable = tracks.filter((t) => !NON_PRIMARY_FOLDER_RE.test(t.path));
  const pool = workable.length > 0 ? workable : tracks;
  if (targetBpm <= 0 && !targetCam) return pool.slice(0, 60);

  const scored = pool
    .map((t) => {
      const bpmDiff = targetBpm > 0 && t.bpm ? Math.abs(t.bpm - targetBpm) : 4;
      const sameKey =
        targetCam && t.camelot?.toUpperCase() === targetCam ? 20 : 0;
      return { track: t, score: 100 - bpmDiff * 2 + sameKey };
    })
    .sort((a, b) => b.score - a.score);

  return scored.slice(0, 60).map((s) => s.track);
}

export function makeSyntheticFolderDetail(
  crateId: string,
  displayName: string,
  tracks: Track[],
  folderPath = '',
  subfolders: FolderCrateSummary[] = [],
): FolderCrateDetail {
  return {
    summary: {
      crate_id: crateId,
      folder_path: folderPath,
      display_name: displayName,
      tree_seal_64: '0',
      track_count: tracks.length,
      video_count: tracks.filter((t) => t.is_video).length,
      full_count: tracks.filter(
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
      parent_crate_id: null,
      rel_path: displayName,
      depth: 0,
      subfolder_count: subfolders.length,
    },
    rules_markdown: '',
    ledger_entries: [],
    track_ids: tracks.map((t) => t.id),
    hydrated_from_omkf: false,
    disk_online: true,
    subfolders,
    tracks: [],
  };
}
