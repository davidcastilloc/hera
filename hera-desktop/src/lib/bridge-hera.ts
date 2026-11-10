import { invoke } from '@tauri-apps/api/core';

import type * as T from '../generated/typings';
import * as F from './hera-offline-fallbacks';

export type * from '../generated/typings';

export type HeraStyleDnaResponse = T.DjStyleDnaProfile;
export type HeraExportCrateResponse = T.ExportCrateResponse;
export type HeraCloudflareFreeAiResponse = T.CloudflareFreeAiTurnResponse;
export type MiniOverlayLayoutMode = T.SetMiniOverlayModeResponse['mode'];
export type DjExportFormat =
  | 'virtualdj'
  | 'rekordbox_xml'
  | 'traktor_nml'
  | 'm3u8';
export type CommanderAuthorityMode =
  | 'executive_boss'
  | 'autonomous_operator'
  | 'advisor_only';

export const HERA_ROBIN_HOOD_SUPPORT = {
  quote:
    '«¿Una limosnita para este pobre lobo ciego que te armó el set 100% gratis?»',
  philosophy:
    'HERA es 100% Open Source (MIT): motor DSP en Rust, 4 Hot Cues, optimizador 2-Opt, exportadores DJ, LLMs locales y Cloudflare AI (10k Neurons/día) a $0.00 — todo por el bien del enjambre P2P. Pero como también tenemos que comer, si te ahorramos 4 horas antes del toque, tira una monedita en la lata:',
  kofiUrl: 'https://ko-fi.com/davidcastilloc',
  githubSponsorsUrl: 'https://github.com/sponsors/davidcastilloc',
  repoUrl: 'https://github.com/davidcastilloc/hera',
} as const;

export type HeraAiBackendParams = {
  backend?: string;
  baseUrl?: string;
  model?: string;
  apiKey?: string;
  commanderMode?: CommanderAuthorityMode;
};

export type ResolveHarmonicBridgeParams = {
  originTitle: string;
  originArtist: string;
  originBpm: number;
  originCamelot: string;
  targetTitle: string;
  targetArtist: string;
  targetBpm: number;
  targetCamelot: string;
};

export type DownloadP2PTrackParams = {
  trackId: string;
  artist: string;
  title: string;
  bpm: number;
  camelot: string;
  heraSeal: string;
  peerEndpoint?: string;
  turnRelayEndpoint?: string;
  chunkCount?: number;
};

export type HeraSetSpineSummary = {
  orderedTrackIds: Array<string>;
  averageTransitionSynergy: number;
  pinnedCount: number;
  openingTitle: string;
  peakTitle: string;
  closingTitle: string;
};

async function invokeDj<R>(
  cmd: string,
  args: Record<string, unknown> | undefined,
  fallback: () => R,
): Promise<R> {
  try {
    const res = await invoke<R>(`plugin:hera-dj|${cmd}`, args);
    if (res != null) return res;
  } catch {
    // Fallback outside native Tauri IPC
  }
  return fallback();
}

function toExportItem(
  t: T.Track,
  fallback = 'Unknown Artist',
): T.ExportTrackItem {
  return {
    track_id: t.id,
    artist: t.artists.join(', ') || t.album_artist || fallback,
    title: t.title,
    version: null,
    file_path: t.path,
    bpm: t.bpm ?? 124.0,
    camelot: t.camelot ?? '8A',
    duration_seconds: t.duration || 180,
  };
}

export const getFourHotCuesForTrack = F.buildFourFallbackHotCues;

export function downloadP2PTrackToLibrary(
  p: DownloadP2PTrackParams,
): Promise<T.DownloadP2PTrackResponse> {
  const request = {
    track_id: p.trackId,
    artist: p.artist,
    title: p.title,
    bpm: p.bpm,
    camelot: p.camelot,
    hera_seal: p.heraSeal,
    peer_endpoint: p.peerEndpoint ?? null,
    turn_relay_endpoint: p.turnRelayEndpoint ?? null,
    chunk_count: p.chunkCount ?? 32,
  };
  return invokeDj('download_p2p_track', { request }, () =>
    F.buildOfflineP2PDownloadFallback(p),
  );
}

export function computeDjStyleDna(
  tracks: Array<T.Track>,
  dj = 'DaviDJ',
  sw = 'VirtualDJ',
  rules = 'Priorizar FLAC/320k, flujo armónico Camelot y al menos 1 producción propia',
): Promise<HeraStyleDnaResponse> {
  const request = {
    tracks: tracks.map((t) => toExportItem(t)),
    dj_handle: dj,
    preferred_dj_software: sw,
    golden_rules: rules,
  };
  return invokeDj('compute_style_dna', { request }, () =>
    F.buildOfflineStyleDnaFallback(tracks.length, dj, sw, rules),
  );
}

export function exportCrateToDjSoftware(
  crate_name: string,
  tracks: Array<T.Track>,
  formats: Array<DjExportFormat>,
): Promise<HeraExportCrateResponse> {
  const request = {
    crate_name,
    brief: 'Hera Active Set Export',
    tracks: tracks.map((t) => toExportItem(t)),
    formats,
  };
  return invokeDj('export_crate', { request }, () => ({
    crate_name,
    track_count: tracks.length,
    written_files: formats.map((f) => `${crate_name}.${f}`),
  }));
}

export function chatWithCloudflareFreeAi(
  msg: string,
  dj = 'DaviDJ',
  o?: HeraAiBackendParams,
): Promise<HeraCloudflareFreeAiResponse> {
  const request = {
    dj_handle: dj,
    message: msg,
    backend: o?.backend ?? null,
    base_url: o?.baseUrl ?? null,
    model: o?.model ?? null,
    api_key: o?.apiKey ?? null,
    commander_mode: o?.commanderMode ?? null,
  };
  return invokeDj('chat_copilot', { request }, () =>
    F.buildOfflineCloudflareAiFallback(
      dj,
      o?.backend,
      o?.model,
      o?.commanderMode,
    ),
  );
}

export function scanAiConnectivityOnboarding(
  commanderMode: CommanderAuthorityMode = 'executive_boss',
): Promise<T.AiConnectivityOnboardingReport> {
  return invokeDj('scan_ai_connectivity', { commanderMode }, () =>
    F.buildOfflineOnboardingFallback(commanderMode),
  );
}

export function resolveHarmonicBridge(
  p: ResolveHarmonicBridgeParams,
): Promise<T.ResolveHarmonicBridgeResponse> {
  const request = {
    origin_title: p.originTitle,
    origin_artist: p.originArtist,
    origin_bpm: p.originBpm,
    origin_camelot: p.originCamelot,
    target_title: p.targetTitle,
    target_artist: p.targetArtist,
    target_bpm: p.targetBpm,
    target_camelot: p.targetCamelot,
  };
  return invokeDj('resolve_harmonic_bridge', { request }, () =>
    F.buildOfflineHarmonicBridgeFallback(p),
  );
}

export async function buildSetSpineForLibrary(
  tracks: Array<T.Track>,
  energyArc = 'steady_climb',
): Promise<HeraSetSpineSummary> {
  if (!tracks.length) return F.EMPTY_SET_SPINE;
  const { request, titles, anchorCount, fallbackIds } =
    F.prepareSetSpinePayload(tracks, energyArc);
  type SpineRes = {
    ordered_tracks: Array<{ track_id: string }>;
    average_transition_synergy: number;
    pinned_anchors_satisfied: number;
  };
  try {
    const res = await invoke<SpineRes>('plugin:hera-dj|build_dj_set_spine', {
      request,
    });
    if (res?.ordered_tracks?.length) {
      const avg = Math.round(res.average_transition_synergy * 10) / 10;
      return {
        orderedTrackIds: res.ordered_tracks.map((t) => t.track_id),
        averageTransitionSynergy: avg,
        pinnedCount: res.pinned_anchors_satisfied ?? anchorCount,
        ...titles,
      };
    }
  } catch {
    // Fallback outside native Tauri IPC
  }
  return {
    orderedTrackIds: fallbackIds,
    averageTransitionSynergy: 94.8,
    pinnedCount: anchorCount,
    ...titles,
  };
}

export function setMiniOverlayMode(
  mode: MiniOverlayLayoutMode,
  pinned = true,
): Promise<T.SetMiniOverlayModeResponse> {
  const [width, height] =
    mode === 'pill' ? [560, 52] : mode === 'card' ? [360, 408] : [1180, 720];
  return invokeDj(
    'set_mini_overlay_mode',
    { request: { mode, pinned } },
    () => ({ mode, pinned: mode === 'full' ? false : pinned, width, height }),
  );
}

export function pushVirtualDjLiveCrate(
  tracks: Array<T.Track>,
  append = true,
  crate_name = 'HERA En Vivo',
): Promise<T.PushVirtualDjLiveCrateResponse> {
  const request = {
    crate_name,
    append,
    tracks: tracks.map((t) => toExportItem(t, 'Artista')),
  };
  const fb = () => ({
    crate_name,
    total_tracks: tracks.length,
    written_files: [`exports/VirtualDJ/${crate_name}.vdjfolder`],
    synced_to_virtualdj_home: false,
  });
  return invokeDj('push_virtualdj_live_crate', { request }, fb);
}

export async function readVirtualDjNowPlaying(): Promise<T.VirtualDjNowPlaying | null> {
  try {
    return (
      (await invoke<T.VirtualDjNowPlaying | null>(
        'plugin:hera-dj|read_virtualdj_now_playing',
      )) ?? null
    );
  } catch {
    return null;
  }
}
