import { useNavigate } from '@tanstack/react-router';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import type { Track } from '../../generated/typings';
import useInvalidate from '../../hooks/useInvalidate';
import DatabaseBridge from '../../lib/bridge-database';
import {
  type HarmonicBridgeStep,
  resolveHarmonicBridge,
  type ResolveHarmonicBridgeResponse,
  type SwarmCatalogItem,
} from '../../lib/bridge-hera';
import { startSyntheticHarmonicPreview } from '../../lib/hera-harmonic-utils';
import player from '../../lib/player';
import useLibraryStore from '../../lib/store';
import {
  acquireSwarmCandidateStep,
  createPlaylistFromBridgeSteps,
  deriveActiveBridgeSteps,
  deriveSwarmCatalog,
  exportBridgeToVirtualDjCrate,
  matchLocalLibraryTrack,
} from './transition-actions';

async function acquireSwarmCandidateStepSafe(
  item: SwarmCatalogItem,
  invalidate: () => Promise<unknown>,
): Promise<HarmonicBridgeStep | null> {
  try {
    return await acquireSwarmCandidateStep(item, invalidate);
  } catch {
    return null;
  }
}

async function createPlaylistFromBridgeStepsSafe(
  params: Parameters<typeof createPlaylistFromBridgeSteps>[0],
): Promise<{
  ok: boolean;
  playlist: Awaited<ReturnType<typeof createPlaylistFromBridgeSteps>>;
}> {
  try {
    const playlist = await createPlaylistFromBridgeSteps(params);
    return { ok: true, playlist };
  } catch {
    return { ok: false, playlist: null };
  }
}

async function exportBridgeToVirtualDjCrateSafe(
  params: Parameters<typeof exportBridgeToVirtualDjCrate>[0],
): Promise<boolean> {
  try {
    await exportBridgeToVirtualDjCrate(params);
    return true;
  } catch {
    return false;
  }
}

export function useTransitionStudio() {
  const tracks = useLibraryStore((state) => state.tracks);
  const invalidate = useInvalidate();
  const navigate = useNavigate();

  const [originTrackId, setOriginTrackId] = useState<string>('');
  const [originTitle, setOriginTitle] = useState('Levels');
  const [originArtist, setOriginArtist] = useState('Avicii');
  const [originBpm, setOriginBpm] = useState(124);
  const [originCamelot, setOriginCamelot] = useState('8A');

  const [targetTrackId, setTargetTrackId] = useState<string>('');
  const [targetTitle, setTargetTitle] = useState("Don't You Worry Child");
  const [targetArtist, setTargetArtist] = useState('Swedish House Mafia');
  const [targetBpm, setTargetBpm] = useState(128);
  const [targetCamelot, setTargetCamelot] = useState('10A');

  const [bridgeResult, setBridgeResult] =
    useState<ResolveHarmonicBridgeResponse | null>(null);
  const [customSteps, setCustomSteps] =
    useState<Array<HarmonicBridgeStep> | null>(null);
  const [activeAuditionId, setActiveAuditionId] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [inlineBadge, setInlineBadge] = useState<string | null>(null);
  const [addedFingerprints, setAddedFingerprints] = useState<Array<string>>([]);
  const [showMoreSuggestions, setShowMoreSuggestions] = useState<boolean>(true);

  const audioCtxRef = useRef<AudioContext | null>(null);
  const previewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const badgeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const flashInlineBadge = (code: string) => {
    if (badgeTimerRef.current) clearTimeout(badgeTimerRef.current);
    setInlineBadge(code);
    badgeTimerRef.current = setTimeout(() => {
      setInlineBadge(null);
    }, 2400);
  };

  const stopWebAudioPreview = useCallback(() => {
    if (previewTimerRef.current) {
      clearTimeout(previewTimerRef.current);
      previewTimerRef.current = null;
    }
    if (audioCtxRef.current) {
      void audioCtxRef.current.close().catch(() => {});
      audioCtxRef.current = null;
    }
    setActiveAuditionId(null);
  }, []);

  useEffect(() => {
    return () => {
      stopWebAudioPreview();
      if (badgeTimerRef.current) clearTimeout(badgeTimerRef.current);
    };
  }, [stopWebAudioPreview]);

  useEffect(() => {
    if (tracks.length === 0) {
      void DatabaseBridge.getAllTracks()
        .then((dbTracks) => {
          if (dbTracks.length > 0) {
            useLibraryStore.setState({ tracks: dbTracks });
          }
        })
        .catch(() => {});
    }
  }, [tracks.length]);

  useEffect(() => {
    let cancelled = false;
    void resolveHarmonicBridge({
      originTitle,
      originArtist,
      originBpm,
      originCamelot,
      targetTitle,
      targetArtist,
      targetBpm,
      targetCamelot,
    })
      .then((res) => {
        if (!cancelled) {
          setBridgeResult(res);
        }
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [
    originArtist,
    originBpm,
    originCamelot,
    originTitle,
    targetArtist,
    targetBpm,
    targetCamelot,
    targetTitle,
  ]);

  const handleSelectDeckTrack = (deck: 'A' | 'B', trackId: string) => {
    const found = tracks.find((t) => t.id === trackId);
    if (!found) return;
    const artist = found.artists[0] || found.album_artist || 'Artista';
    const bpm = Math.round(found.bpm ?? 124);
    const camelot = found.camelot || '8A';
    setCustomSteps(null);
    if (deck === 'A') {
      setOriginTrackId(found.id);
      setOriginTitle(found.title);
      setOriginArtist(artist);
      setOriginBpm(bpm);
      setOriginCamelot(camelot);
    } else {
      setTargetTrackId(found.id);
      setTargetTitle(found.title);
      setTargetArtist(artist);
      setTargetBpm(bpm);
      setTargetCamelot(camelot);
    }
  };

  const handleSwapDecks = () => {
    setCustomSteps(null);
    setOriginTrackId(targetTrackId);
    setTargetTrackId(originTrackId);
    setOriginTitle(targetTitle);
    setTargetTitle(originTitle);
    setOriginArtist(targetArtist);
    setTargetArtist(originArtist);
    setOriginBpm(targetBpm);
    setTargetBpm(originBpm);
    setOriginCamelot(targetCamelot);
    setTargetCamelot(originCamelot);
  };

  const activeSteps = useMemo(
    () =>
      deriveActiveBridgeSteps(
        customSteps,
        bridgeResult,
        originBpm,
        targetBpm,
        targetCamelot,
      ),
    [bridgeResult, customSteps, originBpm, targetBpm, targetCamelot],
  );

  const swarmCatalog = useMemo(
    () => deriveSwarmCatalog(bridgeResult, activeSteps),
    [activeSteps, bridgeResult],
  );

  const findMatchingLocalTrack = matchLocalLibraryTrack;

  const handleAuditionStep = async (
    auditionId: string,
    bpm: number,
    camelot: string,
    cueSeconds: number,
    localCandidate?: Track,
  ) => {
    if (activeAuditionId === auditionId) {
      stopWebAudioPreview();
      player.pause();
      return;
    }
    stopWebAudioPreview();

    if (localCandidate) {
      await player.start([localCandidate], localCandidate.id, {
        type: 'library',
      });
      player.setCurrentTime(Math.max(0, cueSeconds));
      setActiveAuditionId(auditionId);
      return;
    }

    const durationSecs = 5.5;
    const ctx = startSyntheticHarmonicPreview({ bpm, camelot, durationSecs });
    if (!ctx) return;
    audioCtxRef.current = ctx;
    setActiveAuditionId(auditionId);
    previewTimerRef.current = setTimeout(() => {
      stopWebAudioPreview();
    }, durationSecs * 1000);
  };

  const handleAcquireAndInsertCandidate = async (item: SwarmCatalogItem) => {
    if (busyAction) return;
    setBusyAction(item.cas_fingerprint);
    const updatedStep = await acquireSwarmCandidateStepSafe(item, invalidate);
    if (updatedStep) {
      setCustomSteps([updatedStep, ...activeSteps.slice(1)]);
      setAddedFingerprints((prev) => [...prev, item.cas_fingerprint]);
    }
    setBusyAction(null);
  };

  const handleApproveAndCreatePlaylist = async () => {
    if (busyAction) return;
    setBusyAction('approve_set');
    const result = await createPlaylistFromBridgeStepsSafe({
      activeSteps,
      originTrackId,
      targetTrackId,
      originCamelot,
      targetCamelot,
      invalidate,
    });
    if (result.ok) {
      flashInlineBadge('saved');
      if (result.playlist) {
        void navigate({
          to: '/playlists/$playlistID',
          params: { playlistID: result.playlist.id },
        });
      }
    }
    setBusyAction(null);
  };

  const handleExportBridgeVirtualDj = async () => {
    if (busyAction) return;
    setBusyAction('export_vdj');
    const exported = await exportBridgeToVirtualDjCrateSafe({
      originTrackId,
      originArtist,
      originTitle,
      originBpm,
      originCamelot,
      targetTrackId,
      targetArtist,
      targetTitle,
      targetBpm,
      targetCamelot,
      activeSteps,
    });
    if (exported) {
      flashInlineBadge('exported');
    }
    setBusyAction(null);
  };

  return {
    tracks,
    navigate,
    originTrackId,
    originTitle,
    originArtist,
    originBpm,
    originCamelot,
    targetTrackId,
    targetTitle,
    targetArtist,
    targetBpm,
    targetCamelot,
    activeSteps,
    swarmCatalog,
    activeAuditionId,
    busyAction,
    inlineBadge,
    addedFingerprints,
    showMoreSuggestions,
    setCustomSteps,
    setShowMoreSuggestions,
    handleSelectDeckTrack,
    handleSwapDecks,
    handleOriginBpmChange: (bpm: number) => {
      setCustomSteps(null);
      setOriginBpm(bpm);
    },
    handleOriginCamelotChange: (camelot: string) => {
      setCustomSteps(null);
      setOriginCamelot(camelot);
    },
    handleTargetBpmChange: (bpm: number) => {
      setCustomSteps(null);
      setTargetBpm(bpm);
    },
    handleTargetCamelotChange: (camelot: string) => {
      setCustomSteps(null);
      setTargetCamelot(camelot);
    },
    findMatchingLocalTrack,
    handleAuditionStep,
    handleAcquireAndInsertCandidate,
    handleApproveAndCreatePlaylist,
    handleExportBridgeVirtualDj,
  };
}
