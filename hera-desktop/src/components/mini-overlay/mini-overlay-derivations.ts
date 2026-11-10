import LibraryAPI from '../../api/LibraryAPI';
import type { Track } from '../../generated/typings';
import {
  downloadP2PTrackToLibrary,
  type HarmonicBridgeStep,
  pushVirtualDjLiveCrate,
} from '../../lib/bridge-hera';
import {
  buildSyntheticTrackDescriptor,
  camelotDistance,
  nextHarmonicCamelot,
} from '../../lib/hera-harmonic-utils';
import useLibraryStore from '../../lib/store';
import type {
  MiniBridgeEndpoints,
  MiniSuggestionItem,
  MiniTrackSummary,
} from './MiniIcons';

export function deriveCurrentTrackInfo(
  tracks: Array<Track>,
  selectedCurrentId: string,
  vdjLiveTrack: { artist: string; title: string } | null,
  playingTrack: Track | null,
): MiniTrackSummary {
  if (selectedCurrentId) {
    const chosen = tracks.find((t) => t.id === selectedCurrentId);
    if (chosen) {
      return {
        id: chosen.id,
        artist: chosen.artists[0] || chosen.album_artist || 'Artista',
        title: chosen.title,
        bpm: Math.round(chosen.bpm ?? 126),
        camelot: chosen.camelot || '8A',
      };
    }
  }
  if (vdjLiveTrack) {
    const matched = tracks.find(
      (t) =>
        t.title.toLowerCase().includes(vdjLiveTrack.title.toLowerCase()) ||
        vdjLiveTrack.title.toLowerCase().includes(t.title.toLowerCase()),
    );
    return {
      id: matched?.id ?? 'vdj-live',
      artist: vdjLiveTrack.artist,
      title: vdjLiveTrack.title,
      bpm: Math.round(matched?.bpm ?? 126),
      camelot: matched?.camelot || '8A',
    };
  }
  if (playingTrack) {
    return {
      id: playingTrack.id,
      artist: playingTrack.artists[0] || playingTrack.album_artist || 'Artista',
      title: playingTrack.title,
      bpm: Math.round(playingTrack.bpm ?? 126),
      camelot: playingTrack.camelot || '8A',
    };
  }
  if (tracks.length > 0) {
    const first = tracks[0];
    return {
      id: first.id,
      artist: first.artists[0] || first.album_artist || 'Artista',
      title: first.title,
      bpm: Math.round(first.bpm ?? 126),
      camelot: first.camelot || '8A',
    };
  }
  return {
    id: 'default-opus',
    artist: 'Eric Prydz',
    title: 'Opus',
    bpm: 126,
    camelot: '8A',
  };
}

export function deriveMiniSuggestions(
  tracks: Array<Track>,
  currentTrackInfo: MiniTrackSummary,
  energyDeltaBpm: number,
  filterText: string,
): Array<MiniSuggestionItem> {
  const targetBpm = currentTrackInfo.bpm + energyDeltaBpm;
  const query = filterText.trim().toLowerCase();

  const localCandidates: Array<{ score: number; item: MiniSuggestionItem }> =
    [];
  for (const t of tracks) {
    if (t.id === currentTrackInfo.id) continue;
    const artist = t.artists[0] || t.album_artist || 'Artista';
    if (
      query &&
      !t.title.toLowerCase().includes(query) &&
      !artist.toLowerCase().includes(query) &&
      !(t.camelot || '').toLowerCase().includes(query)
    ) {
      continue;
    }
    const bpm = t.bpm ?? 124;
    const key = t.camelot || '8A';
    const dist = camelotDistance(currentTrackInfo.camelot, key);
    const bpmDiff = Math.abs(bpm - targetBpm);
    const score = dist * 10 + bpmDiff;
    localCandidates.push({
      score,
      item: {
        id: t.id,
        artist,
        title: t.title,
        bpm,
        camelot: key,
        cueSeconds: t.hot_cues?.[0]?.seconds ?? 0.5,
        heraSeal: `local_${t.id}`,
        localTrack: t,
      },
    });
  }

  localCandidates.sort((a, b) => a.score - b.score);
  const result = localCandidates.slice(0, 3).map((entry) => entry.item);

  const fallbackPool: Array<MiniSuggestionItem> = [
    {
      id: 'mini-p2p-cesonia',
      artist: 'Keinemusik',
      title: 'Cesonia',
      bpm: Math.max(90, currentTrackInfo.bpm - 1 + energyDeltaBpm),
      camelot: nextHarmonicCamelot(currentTrackInfo.camelot, 1),
      cueSeconds: 15.0,
      heraSeal: 'hera_seal_9f81c2e4b0a1',
    },
    {
      id: 'mini-p2p-consciousness',
      artist: 'Anyma',
      title: 'Consciousness',
      bpm: Math.max(90, currentTrackInfo.bpm - 2 + energyDeltaBpm),
      camelot: currentTrackInfo.camelot,
      cueSeconds: 14.5,
      heraSeal: 'hera_seal_8a124_anyma',
    },
    {
      id: 'mini-p2p-morenita',
      artist: 'Hugel',
      title: 'Morenita',
      bpm: Math.max(90, currentTrackInfo.bpm + 2 + energyDeltaBpm),
      camelot: nextHarmonicCamelot(currentTrackInfo.camelot, 2),
      cueSeconds: 14.8,
      heraSeal: 'hera_seal_4d72a8f103e9',
    },
  ];

  for (const fb of fallbackPool) {
    if (result.length >= 3) break;
    if (
      query &&
      !fb.title.toLowerCase().includes(query) &&
      !fb.artist.toLowerCase().includes(query) &&
      !fb.camelot.toLowerCase().includes(query)
    ) {
      continue;
    }
    if (!result.some((r) => r.title.toLowerCase() === fb.title.toLowerCase())) {
      result.push(fb);
    }
  }

  return result.length > 0 ? result : fallbackPool.slice(0, 3);
}

export function deriveMiniBridgeEndpoints(
  tracks: Array<Track>,
  bridgeOriginId: string,
  bridgeTargetId: string,
  bridgeSwapped: boolean,
): MiniBridgeEndpoints {
  const first = tracks[0];
  const second = tracks.length > 1 ? tracks[tracks.length - 1] : undefined;

  const rawA = tracks.find((t) => t.id === bridgeOriginId) ??
    first ?? {
      id: 'deck-a-default',
      artists: ['Avicii'],
      album_artist: 'Avicii',
      title: 'Levels',
      bpm: 124,
      camelot: '8A',
    };
  const rawB = tracks.find((t) => t.id === bridgeTargetId) ??
    second ?? {
      id: 'deck-b-default',
      artists: ['SHM'],
      album_artist: 'SHM',
      title: "Don't You Worry",
      bpm: 128,
      camelot: '10A',
    };

  const a = bridgeSwapped ? rawB : rawA;
  const b = bridgeSwapped ? rawA : rawB;

  return {
    origin: {
      id: a.id,
      artist: a.artists[0] || a.album_artist || 'Artista',
      title: a.title,
      bpm: Math.round(a.bpm ?? 124),
      camelot: a.camelot || '8A',
    },
    target: {
      id: b.id,
      artist: b.artists[0] || b.album_artist || 'Artista',
      title: b.title,
      bpm: Math.round(b.bpm ?? 128),
      camelot: b.camelot || '10A',
    },
  };
}

export async function pushSingleMiniSuggestionToVdj(
  item: MiniSuggestionItem,
  invalidate: () => Promise<unknown>,
): Promise<void> {
  let trackToPush = item.localTrack;
  if (!trackToPush) {
    const dl = await downloadP2PTrackToLibrary({
      trackId: item.id,
      artist: item.artist,
      title: item.title,
      bpm: item.bpm,
      camelot: item.camelot,
      heraSeal: item.heraSeal,
    });
    await LibraryAPI.importAudioFiles([dl.saved_path]);
    await invalidate();
    trackToPush = buildSyntheticTrackDescriptor(
      item.id,
      item.artist,
      item.title,
      item.bpm,
      item.camelot,
      {
        savedPath: dl.saved_path,
        album: 'HERA En Vivo',
        heraSeal: item.heraSeal,
      },
    );
  }
  await pushVirtualDjLiveCrate([trackToPush], true, 'HERA En Vivo');
}

export async function pushMiniBridgeStepsToVdj(
  bridgeSteps: Array<HarmonicBridgeStep>,
): Promise<void> {
  const storeTracks = useLibraryStore.getState().tracks;
  const resolvedTracks: Array<Track> = await Promise.all(
    bridgeSteps.map(async (step) => {
      const localMatch = storeTracks.find(
        (t) =>
          t.id === step.track_id ||
          t.title.toLowerCase().includes(step.title.toLowerCase()),
      );
      if (localMatch) return localMatch;

      const dl = await downloadP2PTrackToLibrary({
        trackId: step.track_id,
        artist: step.artist,
        title: step.title,
        bpm: step.bpm,
        camelot: step.camelot,
        heraSeal: step.hera_seal,
      });
      return buildSyntheticTrackDescriptor(
        step.track_id,
        step.artist,
        step.title,
        step.bpm,
        step.camelot,
        {
          savedPath: dl.saved_path,
          album: 'HERA En Vivo',
          heraSeal: step.hera_seal,
        },
      );
    }),
  );

  await pushVirtualDjLiveCrate(resolvedTracks, false, 'HERA En Vivo');
}
