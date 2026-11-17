import { useNavigate } from '@tanstack/react-router';
import { isTauri } from '@tauri-apps/api/core';
import {
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import type { Track } from '../../generated/typings';
import useInvalidate from '../../hooks/useInvalidate';
import usePlayingTrack from '../../hooks/usePlayingTrack';
import {
  type HarmonicBridgeStep,
  readVirtualDjNowPlaying,
  resolveHarmonicBridge,
} from '../../lib/bridge-hera';
import { startSyntheticHarmonicPreview } from '../../lib/hera-harmonic-utils';
import { useMiniModeStore } from '../../lib/mini-mode-store';
import {
  queryHarmonicCandidates,
  searchAllFoldersDeduplicated,
} from '../../lib/omkf-bridge';
import player from '../../lib/player';
import useLibraryStore from '../../lib/store';
import {
  deriveCurrentTrackInfo,
  deriveMiniBridgeEndpoints,
  deriveMiniSuggestions,
  pushMiniBridgeStepsToVdj,
  pushSingleMiniSuggestionToVdj,
} from './mini-overlay-derivations';
import type { MiniSuggestionItem } from './MiniIcons';

async function pushSingleMiniSuggestionToVdjSafe(
  item: MiniSuggestionItem,
  invalidate: () => Promise<unknown>,
): Promise<boolean> {
  try {
    await pushSingleMiniSuggestionToVdj(item, invalidate);
    return true;
  } catch {
    return false;
  }
}

async function pushMiniBridgeStepsToVdjSafe(
  steps: Array<HarmonicBridgeStep>,
): Promise<boolean> {
  try {
    await pushMiniBridgeStepsToVdj(steps);
    return true;
  } catch {
    return false;
  }
}

export function useMiniOverlayController() {
  const layoutMode = useMiniModeStore((s) => s.layoutMode);
  const pinned = useMiniModeStore((s) => s.pinned);
  const activeTab = useMiniModeStore((s) => s.activeTab);
  const collapseToPill = useMiniModeStore((s) => s.collapseToPill);
  const expandToCard = useMiniModeStore((s) => s.expandToCard);
  const restoreFullWindow = useMiniModeStore((s) => s.restoreFullWindow);
  const togglePinned = useMiniModeStore((s) => s.togglePinned);
  const setActiveTab = useMiniModeStore((s) => s.setActiveTab);

  const tracks = useLibraryStore((s) => s.tracks);
  const playingTrack = usePlayingTrack();
  const invalidate = useInvalidate();
  const navigate = useNavigate();

  const handleOpenAgentFull = useCallback(async () => {
    await restoreFullWindow();
    void navigate({ to: '/agent' });
  }, [navigate, restoreFullWindow]);

  const [selectedCurrentId, setSelectedCurrentId] = useState<string>('');
  const [vdjLiveTrack, setVdjLiveTrack] = useState<{
    artist: string;
    title: string;
  } | null>(null);
  const [filterText, setFilterText] = useState('');
  const [energyDeltaBpm, setEnergyDeltaBpm] = useState(0);
  const [activePreset, setActivePreset] = useState<string | null>(null);
  const [sentIds, setSentIds] = useState<Set<string>>(() => new Set());
  const [busyId, setBusyId] = useState<string | null>(null);
  const [bridgeSent, setBridgeSent] = useState(false);
  const [auditionId, setAuditionId] = useState<string | null>(null);

  const [bridgeOriginId, setBridgeOriginId] = useState<string>('');
  const [bridgeTargetId, setBridgeTargetId] = useState<string>('');
  const [bridgeSwapped, setBridgeSwapped] = useState(false);
  const [bridgeSteps, setBridgeSteps] = useState<Array<HarmonicBridgeStep>>([]);
  const [harmonicPool, setHarmonicPool] = useState<Track[]>([]);

  const audioCtxRef = useRef<AudioContext | null>(null);
  const previewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const bridgeBadgeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(
    null,
  );

  const stopPreview = useCallback(() => {
    if (previewTimerRef.current) {
      clearTimeout(previewTimerRef.current);
      previewTimerRef.current = null;
    }
    if (audioCtxRef.current) {
      void audioCtxRef.current.close().catch(() => {});
      audioCtxRef.current = null;
    }
    setAuditionId(null);
  }, []);

  useEffect(() => {
    return () => {
      stopPreview();
      if (bridgeBadgeTimerRef.current)
        clearTimeout(bridgeBadgeTimerRef.current);
    };
  }, [stopPreview]);

  useEffect(() => {
    let active = true;
    const poll = () => {
      void readVirtualDjNowPlaying().then((now) => {
        if (active && now) {
          setVdjLiveTrack((prev) =>
            prev?.artist === now.artist && prev?.title === now.title
              ? prev
              : { artist: now.artist, title: now.title },
          );
        }
      });
    };
    poll();
    const id = setInterval(poll, 4000);
    return () => {
      active = false;
      clearInterval(id);
    };
  }, []);

  const currentTrackInfo = useMemo(
    () =>
      deriveCurrentTrackInfo(
        tracks,
        selectedCurrentId,
        vdjLiveTrack,
        playingTrack,
      ),
    [playingTrack, selectedCurrentId, tracks, vdjLiveTrack],
  );

  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    void queryHarmonicCandidates(currentTrackInfo.bpm, currentTrackInfo.camelot)
      .then((cands) =>
        cands.length > 0 ? cands : searchAllFoldersDeduplicated(''),
      )
      .then((pool) => {
        if (!cancelled) {
          setHarmonicPool(pool);
        }
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [currentTrackInfo.bpm, currentTrackInfo.camelot]);

  const candidateTracks = useMemo(() => {
    if (harmonicPool.length === 0) return tracks;
    const seen = new Set(tracks.map((t) => t.id));
    const merged = [...tracks];
    for (const item of harmonicPool) {
      if (!seen.has(item.id)) {
        seen.add(item.id);
        merged.push(item);
      }
    }
    return merged;
  }, [harmonicPool, tracks]);

  const suggestions = useMemo(
    () =>
      deriveMiniSuggestions(
        candidateTracks,
        currentTrackInfo,
        energyDeltaBpm,
        filterText,
      ),
    [candidateTracks, currentTrackInfo, energyDeltaBpm, filterText],
  );

  const bridgeEndpoints = useMemo(
    () =>
      deriveMiniBridgeEndpoints(
        candidateTracks,
        bridgeOriginId,
        bridgeTargetId,
        bridgeSwapped,
      ),
    [bridgeOriginId, bridgeSwapped, bridgeTargetId, candidateTracks],
  );

  useEffect(() => {
    if (activeTab !== 'puente') return;
    let cancelled = false;
    void resolveHarmonicBridge({
      originTitle: bridgeEndpoints.origin.title,
      originArtist: bridgeEndpoints.origin.artist,
      originBpm: bridgeEndpoints.origin.bpm,
      originCamelot: bridgeEndpoints.origin.camelot,
      targetTitle: bridgeEndpoints.target.title,
      targetArtist: bridgeEndpoints.target.artist,
      targetBpm: bridgeEndpoints.target.bpm,
      targetCamelot: bridgeEndpoints.target.camelot,
    }).then((res) => {
      if (!cancelled) {
        setBridgeSteps(res.bridge_steps.slice(0, 2));
      }
    });
    return () => {
      cancelled = true;
    };
  }, [activeTab, bridgeEndpoints]);

  const handleToggleAudition = useCallback(
    async (item: MiniSuggestionItem) => {
      if (auditionId === item.id) {
        stopPreview();
        player.pause();
        return;
      }
      stopPreview();

      if (item.localTrack) {
        await player.start([item.localTrack], item.localTrack.id, {
          type: 'library',
        });
        player.setCurrentTime(Math.max(0, item.cueSeconds));
        setAuditionId(item.id);
        return;
      }

      const durationSecs = 4.5;
      const ctx = startSyntheticHarmonicPreview({
        bpm: item.bpm,
        camelot: item.camelot,
        durationSecs,
      });
      if (!ctx) return;
      audioCtxRef.current = ctx;
      setAuditionId(item.id);
      previewTimerRef.current = setTimeout(() => {
        stopPreview();
      }, durationSecs * 1000);
    },
    [auditionId, stopPreview],
  );

  const handlePushSingleTrackToVdj = useCallback(
    async (item: MiniSuggestionItem) => {
      if (busyId) return;
      setBusyId(item.id);
      const pushed = await pushSingleMiniSuggestionToVdjSafe(item, invalidate);
      if (pushed) {
        setSentIds((prev) => {
          const next = new Set(prev);
          next.add(item.id);
          return next;
        });
      }
      setBusyId(null);
    },
    [busyId, invalidate],
  );

  const handlePushBridgeToVdj = useCallback(async () => {
    if (busyId) return;
    setBusyId('bridge-push');
    const pushed = await pushMiniBridgeStepsToVdjSafe(bridgeSteps);
    if (pushed) {
      setBridgeSent(true);
      if (bridgeBadgeTimerRef.current)
        clearTimeout(bridgeBadgeTimerRef.current);
      bridgeBadgeTimerRef.current = setTimeout(() => {
        setBridgeSent(false);
      }, 2500);
    }
    setBusyId(null);
  }, [bridgeSteps, busyId]);

  const handleApplyPreset = (preset: 'suave' | 'energia' | 'sin_voz') => {
    startTransition(() => {
      if (activePreset === preset) {
        setActivePreset(null);
        setEnergyDeltaBpm(0);
        return;
      }
      setActivePreset(preset);
      if (preset === 'suave') setEnergyDeltaBpm(-1);
      else if (preset === 'energia') setEnergyDeltaBpm(2);
      else setEnergyDeltaBpm(0);
    });
  };

  const handleFilterChange = (next: string) => {
    startTransition(() => {
      setFilterText(next);
    });
  };

  return {
    layoutMode,
    pinned,
    activeTab,
    collapseToPill,
    expandToCard,
    restoreFullWindow,
    togglePinned,
    setActiveTab,
    handleOpenAgentFull,
    tracks: candidateTracks,
    selectedCurrentId,
    setSelectedCurrentId,
    filterText,
    handleFilterChange,
    activePreset,
    handleApplyPreset,
    sentIds,
    busyId,
    bridgeSent,
    auditionId,
    bridgeOriginId,
    setBridgeOriginId,
    bridgeTargetId,
    setBridgeTargetId,
    toggleBridgeSwapped: () => setBridgeSwapped((v) => !v),
    bridgeSteps,
    currentTrackInfo,
    suggestions,
    bridgeEndpoints,
    handleToggleAudition,
    handlePushSingleTrackToVdj,
    handlePushBridgeToVdj,
  };
}
