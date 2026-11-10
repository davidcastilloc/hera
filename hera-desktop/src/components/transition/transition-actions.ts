import LibraryAPI from '../../api/LibraryAPI';
import PlaylistsAPI from '../../api/PlaylistsAPI';
import type { Playlist, Track } from '../../generated/typings';
import DatabaseBridge from '../../lib/bridge-database';
import {
  downloadP2PTrackToLibrary,
  exportCrateToDjSoftware,
  type HarmonicBridgeStep,
  type ResolveHarmonicBridgeResponse,
  type SwarmCatalogItem,
} from '../../lib/bridge-hera';
import { buildSyntheticTrackDescriptor } from '../../lib/hera-harmonic-utils';
import useLibraryStore from '../../lib/store';

export function deriveActiveBridgeSteps(
  customSteps: Array<HarmonicBridgeStep> | null,
  bridgeResult: ResolveHarmonicBridgeResponse | null,
  originBpm: number,
  targetBpm: number,
  targetCamelot: string,
): Array<HarmonicBridgeStep> {
  if (customSteps && customSteps.length > 0) return customSteps;
  if (bridgeResult?.bridge_steps && bridgeResult.bridge_steps.length > 0) {
    return bridgeResult.bridge_steps;
  }
  return [
    {
      step_number: 1,
      track_id: 'p2p-bridge-01',
      artist: 'Keinemusik',
      title: 'Cesonia',
      bpm: Math.round(originBpm + (targetBpm - originBpm) * 0.35),
      camelot: '9A',
      owner_dj: 'DaviDJ',
      hera_seal: 'hera_seal_9f81c2e4b0a1',
      cue_in_seconds: 15.2,
      cue_out_seconds: 168.4,
      synergy_score: 0.96,
      transition_tip: '',
      p2p_boost_free_promo: true,
      boost_credit_cost_mb: 0,
    },
    {
      step_number: 2,
      track_id: 'p2p-bridge-02',
      artist: 'Hugel',
      title: 'Morenita',
      bpm: Math.round(originBpm + (targetBpm - originBpm) * 0.72),
      camelot: targetCamelot,
      owner_dj: 'DJFoo',
      hera_seal: 'hera_seal_4d72a8f103e9',
      cue_in_seconds: 14.8,
      cue_out_seconds: 174.0,
      synergy_score: 0.95,
      transition_tip: '',
      p2p_boost_free_promo: false,
      boost_credit_cost_mb: 0,
    },
  ];
}

export function deriveSwarmCatalog(
  bridgeResult: ResolveHarmonicBridgeResponse | null,
  activeSteps: Array<HarmonicBridgeStep>,
): Array<SwarmCatalogItem> {
  if (bridgeResult?.swarm_catalog && bridgeResult.swarm_catalog.length > 0) {
    return bridgeResult.swarm_catalog;
  }
  return activeSteps.map((s) => ({
    cas_fingerprint: `cas-${s.track_id}`,
    artist: s.artist,
    title: s.title,
    bpm: s.bpm,
    camelot: s.camelot,
    subgenre: 'Club',
    hera_seal: s.hera_seal,
    opaque_blob_key: `shards/boost/${s.track_id}.shard`,
    swarm_replicas_count: 3,
    seeders: [s.owner_dj],
    p2p_boost_ready: true,
    p2p_boost_free_promo: Boolean(s.p2p_boost_free_promo),
    effective_bandwidth_hz: 21600,
    quality_score: 98.4,
  }));
}

export function matchLocalLibraryTrack(
  artist: string,
  title: string,
  trackId?: string,
): Track | undefined {
  const storeTracks = useLibraryStore.getState().tracks;
  if (trackId) {
    const byId = storeTracks.find((t) => t.id === trackId);
    if (byId) return byId;
  }
  const normTitle = title.trim().toLowerCase();
  const normArtist = artist.trim().toLowerCase();
  return storeTracks.find(
    (t) =>
      t.title.toLowerCase().includes(normTitle) ||
      (t.artists[0]?.toLowerCase().includes(normArtist) &&
        normTitle.includes(t.title.toLowerCase())),
  );
}

export async function acquireSwarmCandidateStep(
  item: SwarmCatalogItem,
  invalidate: () => Promise<unknown>,
): Promise<HarmonicBridgeStep> {
  const dl = await downloadP2PTrackToLibrary({
    trackId: item.cas_fingerprint,
    artist: item.artist,
    title: item.title,
    bpm: item.bpm,
    camelot: item.camelot,
    heraSeal: item.hera_seal,
  });

  await LibraryAPI.importAudioFiles([dl.saved_path]);
  await invalidate();

  return {
    step_number: 1,
    track_id: item.cas_fingerprint,
    artist: item.artist,
    title: item.title,
    bpm: item.bpm,
    camelot: item.camelot,
    owner_dj: item.seeders[0] || 'HERA',
    hera_seal: item.hera_seal,
    cue_in_seconds: 15.0,
    cue_out_seconds: 170.0,
    synergy_score: 0.97,
    transition_tip: '',
    p2p_boost_free_promo: item.p2p_boost_free_promo,
    boost_credit_cost_mb: 0,
  };
}

export async function createPlaylistFromBridgeSteps(params: {
  activeSteps: Array<HarmonicBridgeStep>;
  originTrackId: string;
  targetTrackId: string;
  originCamelot: string;
  targetCamelot: string;
  invalidate: () => Promise<unknown>;
}): Promise<Playlist | null> {
  const downloadedPaths: Array<string> = [];
  for (const step of params.activeSteps) {
    const existing = matchLocalLibraryTrack(
      step.artist,
      step.title,
      step.track_id,
    );
    if (!existing) {
      const dl = await downloadP2PTrackToLibrary({
        trackId: step.track_id,
        artist: step.artist,
        title: step.title,
        bpm: step.bpm,
        camelot: step.camelot,
        heraSeal: step.hera_seal,
      });
      downloadedPaths.push(dl.saved_path);
    }
  }

  if (downloadedPaths.length > 0) {
    await LibraryAPI.importAudioFiles(downloadedPaths);
  }

  const refreshedTracks = await DatabaseBridge.getAllTracks();
  useLibraryStore.setState({ tracks: refreshedTracks });

  const orderedIds: Array<string> = [];
  if (params.originTrackId) orderedIds.push(params.originTrackId);
  for (const step of params.activeSteps) {
    const matched = matchLocalLibraryTrack(
      step.artist,
      step.title,
      step.track_id,
    );
    if (matched && !orderedIds.includes(matched.id)) {
      orderedIds.push(matched.id);
    }
  }
  if (params.targetTrackId && !orderedIds.includes(params.targetTrackId)) {
    orderedIds.push(params.targetTrackId);
  }

  const playlistName = `Transición ${params.originCamelot}→${params.targetCamelot}`;
  const playlist = await PlaylistsAPI.create(playlistName, orderedIds, true);
  await params.invalidate();
  return playlist ?? null;
}

export async function exportBridgeToVirtualDjCrate(params: {
  originTrackId: string;
  originArtist: string;
  originTitle: string;
  originBpm: number;
  originCamelot: string;
  targetTrackId: string;
  targetArtist: string;
  targetTitle: string;
  targetBpm: number;
  targetCamelot: string;
  activeSteps: Array<HarmonicBridgeStep>;
}): Promise<void> {
  const deckATrack =
    matchLocalLibraryTrack(
      params.originArtist,
      params.originTitle,
      params.originTrackId,
    ) ??
    buildSyntheticTrackDescriptor(
      params.originTrackId || 'deck-a',
      params.originArtist,
      params.originTitle,
      params.originBpm,
      params.originCamelot,
    );
  const stepTracks = params.activeSteps.map(
    (s) =>
      matchLocalLibraryTrack(s.artist, s.title, s.track_id) ??
      buildSyntheticTrackDescriptor(
        s.track_id,
        s.artist,
        s.title,
        s.bpm,
        s.camelot,
      ),
  );
  const deckBTrack =
    matchLocalLibraryTrack(
      params.targetArtist,
      params.targetTitle,
      params.targetTrackId,
    ) ??
    buildSyntheticTrackDescriptor(
      params.targetTrackId || 'deck-b',
      params.targetArtist,
      params.targetTitle,
      params.targetBpm,
      params.targetCamelot,
    );

  await exportCrateToDjSoftware(
    `Hera_Transicion_${params.originCamelot}_${params.targetCamelot}`,
    [deckATrack, ...stepTracks, deckBTrack],
    ['virtualdj', 'm3u8'],
  );
}
