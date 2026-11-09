import * as stylex from '@stylexjs/stylex';
import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import LibraryAPI from '../api/LibraryAPI';
import PlaylistsAPI from '../api/PlaylistsAPI';
import Icon from '../components/Icon';
import View from '../elements/View';
import type { Track } from '../generated/typings';
import useInvalidate from '../hooks/useInvalidate';
import DatabaseBridge from '../lib/bridge-database';
import {
  chatWithCloudflareFreeAi,
  downloadP2PTrackToLibrary,
  exportCrateToDjSoftware,
  resolveHarmonicBridge,
  scanAiConnectivityOnboarding,
  type CommanderAuthorityMode,
  type HarmonicBridgeStep,
  type ResolveHarmonicBridgeResponse,
  type SwarmCatalogItem,
} from '../lib/bridge-hera';
import player from '../lib/player';
import useLibraryStore from '../lib/store';

export const Route = createFileRoute('/bridge')({
  component: ViewHarmonicBridgeP2P,
});

const CAMELOT_OPTIONS = [
  '1A',
  '2A',
  '3A',
  '4A',
  '5A',
  '6A',
  '7A',
  '8A',
  '9A',
  '10A',
  '11A',
  '12A',
  '1B',
  '2B',
  '3B',
  '4B',
  '5B',
  '6B',
  '7B',
  '8B',
  '9B',
  '10B',
  '11B',
  '12B',
];

const CUE_DOT_COLORS = ['#22c55e', '#38bdf8', '#ef4444', '#f59e0b'] as const;

function camelotRootHz(camelot: string): { root: number; major: boolean } {
  const num = Number.parseInt(camelot, 10) || 8;
  const major = camelot.toUpperCase().endsWith('B');
  const minorRoots: Record<number, number> = {
    1: 207.65,
    2: 155.56,
    3: 233.08,
    4: 174.61,
    5: 130.81,
    6: 196.0,
    7: 146.83,
    8: 220.0,
    9: 164.81,
    10: 246.94,
    11: 185.0,
    12: 138.59,
  };
  const base = minorRoots[num] ?? 220.0;
  return { root: major ? base * 1.1892 : base, major };
}

function buildSyntheticTrackDescriptor(
  id: string,
  artist: string,
  title: string,
  bpm: number,
  camelot: string,
): Track {
  return {
    id,
    path: `exports/${artist} - ${title}.wav`,
    title,
    album: 'Single',
    album_artist: artist,
    artists: [artist],
    genres: ['Club'],
    year: 2026,
    duration: 210,
    track_no: 1,
    track_of: 1,
    disk_no: 1,
    disk_of: 1,
    is_compilation: false,
    bpm,
    camelot,
    energy: 0.84,
    hera_seal: `hera_seal_${id}`,
    hot_cues: [
      {
        num: 1,
        name: '1:Intro',
        seconds: 15.2,
        color: '#22c55e',
        role: 'intro',
      },
      {
        num: 2,
        name: '2:Breakdown',
        seconds: 68.0,
        color: '#38bdf8',
        role: 'breakdown',
      },
      {
        num: 3,
        name: '3:Drop',
        seconds: 102.0,
        color: '#ef4444',
        role: 'drop',
      },
      {
        num: 4,
        name: '4:Outro',
        seconds: 168.4,
        color: '#f59e0b',
        role: 'outro',
      },
    ],
  };
}

function ViewHarmonicBridgeP2P() {
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
  const [customSteps, setCustomSteps] = useState<HarmonicBridgeStep[] | null>(
    null,
  );
  const [activeAuditionId, setActiveAuditionId] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [inlineBadge, setInlineBadge] = useState<string | null>(null);
  const [addedFingerprints, setAddedFingerprints] = useState<string[]>([]);
  const [showMoreSuggestions, setShowMoreSuggestions] = useState<boolean>(true);

  // Command bar + compact Asistente popover state
  const [chatPrompt, setChatPrompt] = useState('');
  const [quietNote, setQuietNote] = useState<string>('');
  const [isAiLoading, setIsAiLoading] = useState<boolean>(false);
  const [isAssistantOpen, setIsAssistantOpen] = useState<boolean>(false);
  const [showAdvancedUrl, setShowAdvancedUrl] = useState<boolean>(false);
  const [connectionScope, setConnectionScope] = useState<'cloud' | 'offline'>(
    'cloud',
  );
  const [commanderMode, setCommanderMode] =
    useState<CommanderAuthorityMode>('executive_boss');
  const [customBaseUrl, setCustomBaseUrl] = useState<string>(
    'http://127.0.0.1:11434/v1',
  );
  const [offlineBackendId, setOfflineBackendId] = useState<string>('ollama');

  const audioCtxRef = useRef<AudioContext | null>(null);
  const previewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const badgeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const noteTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const popoverRef = useRef<HTMLDivElement | null>(null);

  const flashInlineBadge = (code: string) => {
    if (badgeTimerRef.current) clearTimeout(badgeTimerRef.current);
    setInlineBadge(code);
    badgeTimerRef.current = setTimeout(() => {
      setInlineBadge(null);
    }, 2400);
  };

  const flashQuietNote = (message: string) => {
    if (noteTimerRef.current) clearTimeout(noteTimerRef.current);
    setQuietNote(message);
    noteTimerRef.current = setTimeout(() => {
      setQuietNote('');
    }, 2600);
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
      if (noteTimerRef.current) clearTimeout(noteTimerRef.current);
    };
  }, [stopWebAudioPreview]);

  useEffect(() => {
    if (!isAssistantOpen) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsAssistantOpen(false);
      }
    };
    const onPointerDown = (e: MouseEvent) => {
      if (
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node)
      ) {
        setIsAssistantOpen(false);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('mousedown', onPointerDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('mousedown', onPointerDown);
    };
  }, [isAssistantOpen]);

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
    void scanAiConnectivityOnboarding(commanderMode)
      .then((rep) => {
        const localServer = rep.local_servers.find((s) => s.reachable);
        if (localServer) {
          setOfflineBackendId(localServer.id);
        }
      })
      .catch(() => {});
  }, [commanderMode]);

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

  const activeSteps: HarmonicBridgeStep[] = useMemo(() => {
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
  }, [bridgeResult, customSteps, originBpm, targetBpm, targetCamelot]);

  const swarmCatalog: SwarmCatalogItem[] = useMemo(() => {
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
  }, [activeSteps, bridgeResult]);

  const findMatchingLocalTrack = useCallback(
    (artist: string, title: string, trackId?: string): Track | undefined => {
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
    },
    [],
  );

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

    try {
      const AudioContextCtor =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext })
          .webkitAudioContext;
      const ctx = new AudioContextCtor();
      audioCtxRef.current = ctx;
      const durationSecs = 5.5;
      const now = ctx.currentTime + 0.04;
      const beatSec = 60 / Math.max(70, Math.min(180, bpm));
      const { root, major } = camelotRootHz(camelot);
      const thirdRatio = major ? 1.259921 : 1.1892071;

      const masterGain = ctx.createGain();
      masterGain.gain.setValueAtTime(0.22, now);
      masterGain.gain.exponentialRampToValueAtTime(0.001, now + durationSecs);
      masterGain.connect(ctx.destination);

      [root, root * thirdRatio, root * 1.4983071].forEach((freq) => {
        const osc = ctx.createOscillator();
        const padGain = ctx.createGain();
        osc.type = 'triangle';
        osc.frequency.setValueAtTime(freq, now);
        padGain.gain.setValueAtTime(0.001, now);
        padGain.gain.linearRampToValueAtTime(0.25, now + 0.35);
        padGain.gain.linearRampToValueAtTime(0.18, now + durationSecs - 0.5);
        padGain.gain.exponentialRampToValueAtTime(0.001, now + durationSecs);
        osc.connect(padGain);
        padGain.connect(masterGain);
        osc.start(now);
        osc.stop(now + durationSecs);
      });

      const totalBeats = Math.floor(durationSecs / beatSec);
      for (let b = 0; b < totalBeats; b += 1) {
        const bt = now + b * beatSec;
        const kickOsc = ctx.createOscillator();
        const kickGain = ctx.createGain();
        kickOsc.type = 'sine';
        kickOsc.frequency.setValueAtTime(135, bt);
        kickOsc.frequency.exponentialRampToValueAtTime(48, bt + 0.11);
        kickGain.gain.setValueAtTime(0.75, bt);
        kickGain.gain.exponentialRampToValueAtTime(0.001, bt + 0.22);
        kickOsc.connect(kickGain);
        kickGain.connect(masterGain);
        kickOsc.start(bt);
        kickOsc.stop(bt + 0.24);
      }

      setActiveAuditionId(auditionId);
      previewTimerRef.current = setTimeout(() => {
        stopWebAudioPreview();
      }, durationSecs * 1000);
    } catch {
      setActiveAuditionId(null);
    }
  };

  const handleAcquireAndInsertCandidate = async (item: SwarmCatalogItem) => {
    if (busyAction) return;
    setBusyAction(item.cas_fingerprint);
    try {
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

      const updatedStep: HarmonicBridgeStep = {
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
      setCustomSteps([updatedStep, ...activeSteps.slice(1)]);
      setAddedFingerprints((prev) => [...prev, item.cas_fingerprint]);
    } finally {
      setBusyAction(null);
    }
  };

  const handleApproveAndCreatePlaylist = async () => {
    if (busyAction) return;
    setBusyAction('approve_set');
    try {
      const downloadedPaths: string[] = [];
      for (const step of activeSteps) {
        const existing = findMatchingLocalTrack(
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

      const orderedIds: string[] = [];
      if (originTrackId) orderedIds.push(originTrackId);
      for (const step of activeSteps) {
        const matched = findMatchingLocalTrack(
          step.artist,
          step.title,
          step.track_id,
        );
        if (matched && !orderedIds.includes(matched.id)) {
          orderedIds.push(matched.id);
        }
      }
      if (targetTrackId && !orderedIds.includes(targetTrackId)) {
        orderedIds.push(targetTrackId);
      }

      const playlistName = `Transición ${originCamelot}→${targetCamelot}`;
      const playlist = await PlaylistsAPI.create(
        playlistName,
        orderedIds,
        true,
      );
      await invalidate();
      flashInlineBadge('saved');

      if (playlist) {
        void navigate({
          to: '/playlists/$playlistID',
          params: { playlistID: playlist.id },
        });
      }
    } finally {
      setBusyAction(null);
    }
  };

  const handleExportBridgeVirtualDj = async () => {
    if (busyAction) return;
    setBusyAction('export_vdj');
    try {
      const deckATrack =
        findMatchingLocalTrack(originArtist, originTitle, originTrackId) ??
        buildSyntheticTrackDescriptor(
          originTrackId || 'deck-a',
          originArtist,
          originTitle,
          originBpm,
          originCamelot,
        );
      const stepTracks = activeSteps.map(
        (s) =>
          findMatchingLocalTrack(s.artist, s.title, s.track_id) ??
          buildSyntheticTrackDescriptor(
            s.track_id,
            s.artist,
            s.title,
            s.bpm,
            s.camelot,
          ),
      );
      const deckBTrack =
        findMatchingLocalTrack(targetArtist, targetTitle, targetTrackId) ??
        buildSyntheticTrackDescriptor(
          targetTrackId || 'deck-b',
          targetArtist,
          targetTitle,
          targetBpm,
          targetCamelot,
        );

      await exportCrateToDjSoftware(
        `Hera_Transicion_${originCamelot}_${targetCamelot}`,
        [deckATrack, ...stepTracks, deckBTrack],
        ['virtualdj', 'm3u8'],
      );
      flashInlineBadge('exported');
    } finally {
      setBusyAction(null);
    }
  };

  const handleSendAdjustment = async (presetText?: string) => {
    const promptToSend = (presetText ?? chatPrompt).trim();
    if (!promptToSend || isAiLoading) return;
    if (!presetText) {
      setChatPrompt('');
    }
    setIsAiLoading(true);
    try {
      const lower = promptToSend.toLowerCase();
      if (lower.includes('energ') || lower.includes('arriba')) {
        setCustomSteps(
          activeSteps.map((step, i) => ({
            ...step,
            bpm: Math.round((step.bpm + (i === 0 ? 1.5 : 2.0)) * 10) / 10,
          })),
        );
      } else if (lower.includes('suave') || lower.includes('lenta')) {
        setCustomSteps(
          activeSteps.map((step, i) => ({
            ...step,
            bpm:
              Math.round(
                (originBpm +
                  ((targetBpm - originBpm) * (i + 1)) /
                    (activeSteps.length + 1)) *
                  10,
              ) / 10,
          })),
        );
      }

      const activeBackend =
        connectionScope === 'cloud'
          ? 'cloudflare_free'
          : showAdvancedUrl
            ? 'custom'
            : offlineBackendId;

      await chatWithCloudflareFreeAi(promptToSend, 'DaviDJ', {
        backend: activeBackend,
        baseUrl:
          connectionScope === 'offline' && showAdvancedUrl
            ? customBaseUrl
            : undefined,
        commanderMode,
      });

      flashQuietNote(`Ajuste aplicado: ${promptToSend}`);
    } catch {
      flashQuietNote(`Ajuste local aplicado: ${promptToSend}`);
    } finally {
      setIsAiLoading(false);
    }
  };

  return (
    <View aria-label="Transición">
      <div {...stylex.props(styles.pageContainer)}>
        <div {...stylex.props(styles.scrollCanvas)}>
          <div {...stylex.props(styles.mainSheet)}>
            <header {...stylex.props(styles.headerRow)}>
              <h1 {...stylex.props(styles.pageTitle)}>Transición</h1>

              <div {...stylex.props(styles.headerActions)}>
                <button
                  type="button"
                  disabled={Boolean(busyAction)}
                  onClick={() => void handleExportBridgeVirtualDj()}
                  {...stylex.props(
                    styles.secondaryBtn,
                    inlineBadge === 'exported' && styles.secondaryBtnDone,
                  )}
                >
                  <Icon
                    name={inlineBadge === 'exported' ? 'checkCircle' : 'export'}
                    size={14}
                    color={inlineBadge === 'exported' ? '#2dd4bf' : undefined}
                  />
                  <span>
                    {busyAction === 'export_vdj'
                      ? 'Exportando...'
                      : inlineBadge === 'exported'
                        ? 'Exportado'
                        : 'Exportar'}
                  </span>
                </button>

                <button
                  type="button"
                  disabled={Boolean(busyAction)}
                  onClick={() => void handleApproveAndCreatePlaylist()}
                  {...stylex.props(styles.primaryBtn)}
                >
                  <Icon name="checkCircle" size={14} />
                  <span>
                    {busyAction === 'approve_set'
                      ? 'Guardando...'
                      : inlineBadge === 'saved'
                        ? 'Guardada'
                        : 'Guardar playlist'}
                  </span>
                </button>
              </div>
            </header>

            <section
              {...stylex.props(styles.flowGrid)}
              aria-label="Ruta de transición"
            >
              {/* Actual (Deck A) */}
              <div {...stylex.props(styles.endpointBox)}>
                <div {...stylex.props(styles.endpointHeader)}>
                  <span {...stylex.props(styles.endpointLabel)}>Actual</span>
                  <button
                    type="button"
                    onClick={handleSwapDecks}
                    title="Intercambiar pistas"
                    aria-label="Intercambiar pista actual y destino"
                    {...stylex.props(styles.swapIconBtn)}
                  >
                    <Icon name="shuffle" size={14} />
                  </button>
                </div>

                {tracks.length > 0 && (
                  <select
                    value={originTrackId}
                    onChange={(e) => handleSelectDeckTrack('A', e.target.value)}
                    aria-label="Seleccionar pista actual"
                    {...stylex.props(styles.librarySelect)}
                  >
                    <option value="">Elegir de biblioteca...</option>
                    {tracks.map((t) => (
                      <option key={t.id} value={t.id}>
                        {(t.artists[0] || 'Artista') + ' — ' + t.title}
                      </option>
                    ))}
                  </select>
                )}

                <div {...stylex.props(styles.endpointTrackPill)}>
                  {originArtist} — {originTitle}
                </div>

                <div {...stylex.props(styles.endpointMetaRow)}>
                  <input
                    type="number"
                    min={60}
                    max={200}
                    step={1}
                    aria-label="BPM pista actual"
                    value={originBpm}
                    onChange={(e) => {
                      setCustomSteps(null);
                      setOriginBpm(Number(e.target.value) || 124);
                    }}
                    {...stylex.props(styles.inlineNumber)}
                  />
                  <span>BPM ·</span>
                  <select
                    aria-label="Tono pista actual"
                    value={originCamelot}
                    onChange={(e) => {
                      setCustomSteps(null);
                      setOriginCamelot(e.target.value);
                    }}
                    {...stylex.props(styles.inlineKeySelect)}
                  >
                    {CAMELOT_OPTIONS.map((k) => (
                      <option key={k} value={k}>
                        {k}
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              <div {...stylex.props(styles.connectorArrow)} aria-hidden="true">
                <svg
                  width="18"
                  height="18"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M5 12h14M13 6l6 6-6 6" />
                </svg>
              </div>

              {/* Puente sugerido */}
              <div {...stylex.props(styles.bridgeCenter)}>
                <h2 {...stylex.props(styles.bridgeHeading)}>Puente sugerido</h2>

                <div {...stylex.props(styles.stepsList)}>
                  {activeSteps.map((step, index) => {
                    const auditionKey = `step-${step.track_id}-${index}`;
                    const isPlaying = activeAuditionId === auditionKey;
                    const localTrack = findMatchingLocalTrack(
                      step.artist,
                      step.title,
                      step.track_id,
                    );
                    const rowIndex = String(index + 1).padStart(2, '0');

                    return (
                      <div
                        key={auditionKey}
                        {...stylex.props(
                          styles.stepRow,
                          index > 0 && styles.stepRowBorder,
                        )}
                      >
                        <span {...stylex.props(styles.stepNum)}>
                          {rowIndex}
                        </span>
                        <span {...stylex.props(styles.stepTitle)}>
                          {step.artist} — {step.title}
                        </span>
                        <span {...stylex.props(styles.stepMeta)}>
                          {Math.round(step.bpm)} BPM · {step.camelot}
                        </span>
                        <span
                          {...stylex.props(styles.cueDots)}
                          title="Puntos: Entrada · Calma · Clímax · Salida"
                        >
                          {CUE_DOT_COLORS.map((color) => (
                            <span
                              key={color}
                              {...stylex.props(styles.cueDot)}
                              style={{ backgroundColor: color }}
                            />
                          ))}
                        </span>
                        <button
                          type="button"
                          onClick={() =>
                            void handleAuditionStep(
                              auditionKey,
                              step.bpm,
                              step.camelot,
                              step.cue_in_seconds,
                              localTrack,
                            )
                          }
                          {...stylex.props(
                            styles.listenBtn,
                            isPlaying && styles.listenBtnActive,
                          )}
                        >
                          <Icon name={isPlaying ? 'pause' : 'play'} size={12} />
                          <span>{isPlaying ? 'Detener' : 'Escuchar'}</span>
                        </button>
                      </div>
                    );
                  })}
                </div>
              </div>

              <div {...stylex.props(styles.connectorArrow)} aria-hidden="true">
                <svg
                  width="18"
                  height="18"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M5 12h14M13 6l6 6-6 6" />
                </svg>
              </div>

              {/* Destino (Deck B) */}
              <div {...stylex.props(styles.endpointBox)}>
                <div {...stylex.props(styles.endpointHeader)}>
                  <span {...stylex.props(styles.endpointLabel)}>Destino</span>
                </div>

                {tracks.length > 0 && (
                  <select
                    value={targetTrackId}
                    onChange={(e) => handleSelectDeckTrack('B', e.target.value)}
                    aria-label="Seleccionar pista destino"
                    {...stylex.props(styles.librarySelect)}
                  >
                    <option value="">Elegir de biblioteca...</option>
                    {tracks.map((t) => (
                      <option key={t.id} value={t.id}>
                        {(t.artists[0] || 'Artista') + ' — ' + t.title}
                      </option>
                    ))}
                  </select>
                )}

                <div {...stylex.props(styles.endpointTrackPill)}>
                  {targetArtist} — {targetTitle}
                </div>

                <div {...stylex.props(styles.endpointMetaRow)}>
                  <input
                    type="number"
                    min={60}
                    max={200}
                    step={1}
                    aria-label="BPM pista destino"
                    value={targetBpm}
                    onChange={(e) => {
                      setCustomSteps(null);
                      setTargetBpm(Number(e.target.value) || 128);
                    }}
                    {...stylex.props(styles.inlineNumber)}
                  />
                  <span>BPM ·</span>
                  <select
                    aria-label="Tono pista destino"
                    value={targetCamelot}
                    onChange={(e) => {
                      setCustomSteps(null);
                      setTargetCamelot(e.target.value);
                    }}
                    {...stylex.props(styles.inlineKeySelect)}
                  >
                    {CAMELOT_OPTIONS.map((k) => (
                      <option key={k} value={k}>
                        {k}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
            </section>

            {/* Collapsible suggestions list */}
            <section {...stylex.props(styles.suggestionsSection)}>
              <button
                type="button"
                onClick={() => setShowMoreSuggestions((prev) => !prev)}
                {...stylex.props(styles.disclosureToggle)}
              >
                <span>Ver más sugerencias ({swarmCatalog.length})</span>
                <Icon
                  name={showMoreSuggestions ? 'chevronUp' : 'chevronDown'}
                  size={14}
                  color="#8e98a8"
                />
              </button>

              {showMoreSuggestions && (
                <div {...stylex.props(styles.suggestionsList)}>
                  {swarmCatalog.slice(0, 4).map((item, idx) => {
                    const isBusy = busyAction === item.cas_fingerprint;
                    const isAdded = addedFingerprints.includes(
                      item.cas_fingerprint,
                    );
                    return (
                      <div
                        key={item.cas_fingerprint}
                        {...stylex.props(
                          styles.suggestionRow,
                          idx > 0 && styles.stepRowBorder,
                        )}
                      >
                        <span {...stylex.props(styles.stepNum)}>
                          {String(idx + 1).padStart(2, '0')}
                        </span>
                        <span {...stylex.props(styles.stepTitle)}>
                          {item.artist} — {item.title}
                        </span>
                        <span {...stylex.props(styles.stepMeta)}>
                          {Math.round(item.bpm)} BPM · {item.camelot}
                        </span>
                        <button
                          type="button"
                          disabled={Boolean(busyAction)}
                          onClick={() =>
                            void handleAcquireAndInsertCandidate(item)
                          }
                          {...stylex.props(
                            styles.addBtn,
                            isAdded && styles.addBtnDone,
                          )}
                        >
                          <Icon
                            name={isAdded ? 'checkCircle' : 'plus'}
                            size={12}
                          />
                          <span>
                            {isBusy
                              ? 'Añadiendo...'
                              : isAdded
                                ? 'Añadida'
                                : 'Añadir'}
                          </span>
                        </button>
                      </div>
                    );
                  })}
                </div>
              )}
            </section>
          </div>
        </div>

        {/* Bottom Spotlight Command Bar + Anchored Asistente Popover */}
        <footer {...stylex.props(styles.commandFooter)}>
          {quietNote && !isAssistantOpen && (
            <div {...stylex.props(styles.quietStatusNote)}>{quietNote}</div>
          )}

          <div {...stylex.props(styles.commandDockRow)} ref={popoverRef}>
            <div {...stylex.props(styles.spotlightBar)}>
              <input
                type="text"
                value={chatPrompt}
                onChange={(e) => setChatPrompt(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    void handleSendAdjustment();
                  }
                }}
                placeholder={
                  isAiLoading ? 'Ajustando ruta...' : 'Ajustar transición...'
                }
                aria-label="Ajustar transición"
                {...stylex.props(styles.spotlightInput)}
              />

              <div {...stylex.props(styles.quickPills)}>
                <button
                  type="button"
                  disabled={isAiLoading}
                  onClick={() => void handleSendAdjustment('Más suave')}
                  {...stylex.props(styles.quickPill)}
                >
                  Más suave
                </button>
                <button
                  type="button"
                  disabled={isAiLoading}
                  onClick={() => void handleSendAdjustment('Más energía')}
                  {...stylex.props(styles.quickPill)}
                >
                  Más energía
                </button>
                <button
                  type="button"
                  disabled={isAiLoading}
                  onClick={() => void handleSendAdjustment('Sin voces')}
                  {...stylex.props(styles.quickPill)}
                >
                  Sin voces
                </button>
              </div>
            </div>

            <button
              type="button"
              aria-label="Configurar asistente"
              aria-expanded={isAssistantOpen}
              title="Asistente"
              onClick={() => setIsAssistantOpen((prev) => !prev)}
              {...stylex.props(
                styles.assistantTriggerBtn,
                isAssistantOpen && styles.assistantTriggerBtnActive,
              )}
            >
              <Icon name="sliders" size={16} />
            </button>

            {isAssistantOpen && (
              <div
                role="dialog"
                aria-label="Asistente"
                {...stylex.props(styles.assistantPopover)}
              >
                <div {...stylex.props(styles.popoverHead)}>
                  <div {...stylex.props(styles.popoverTitleGroup)}>
                    <span {...stylex.props(styles.popoverTitle)}>
                      Asistente
                    </span>
                    <span {...stylex.props(styles.readyDot)} />
                  </div>
                  <span {...stylex.props(styles.popoverReadyText)}>Listo</span>
                </div>

                <div {...stylex.props(styles.popoverRow)}>
                  <span {...stylex.props(styles.popoverLabel)}>Conexión</span>
                  <div {...stylex.props(styles.popoverSegment)}>
                    <button
                      type="button"
                      onClick={() => setConnectionScope('cloud')}
                      {...stylex.props(
                        styles.popSegBtn,
                        connectionScope === 'cloud' && styles.popSegBtnTeal,
                      )}
                    >
                      Nube
                    </button>
                    <button
                      type="button"
                      onClick={() => setConnectionScope('offline')}
                      {...stylex.props(
                        styles.popSegBtn,
                        connectionScope === 'offline' && styles.popSegBtnTeal,
                      )}
                    >
                      Sin internet
                    </button>
                  </div>
                </div>

                <div
                  {...stylex.props(styles.popoverRow, styles.popoverRowDivider)}
                >
                  <span {...stylex.props(styles.popoverLabel)}>Modo</span>
                  <div {...stylex.props(styles.popoverSegment)}>
                    <button
                      type="button"
                      onClick={() => setCommanderMode('executive_boss')}
                      {...stylex.props(
                        styles.popSegBtn,
                        commanderMode === 'executive_boss' &&
                          styles.popSegBtnNeutral,
                      )}
                    >
                      Sugerir
                    </button>
                    <button
                      type="button"
                      onClick={() => setCommanderMode('autonomous_operator')}
                      {...stylex.props(
                        styles.popSegBtn,
                        commanderMode === 'autonomous_operator' &&
                          styles.popSegBtnNeutral,
                      )}
                    >
                      Automático
                    </button>
                  </div>
                </div>

                {showAdvancedUrl && (
                  <div {...stylex.props(styles.advancedBox)}>
                    <input
                      type="text"
                      value={customBaseUrl}
                      onChange={(e) => setCustomBaseUrl(e.target.value)}
                      placeholder="http://127.0.0.1:11434/v1"
                      aria-label="Servidor local"
                      {...stylex.props(styles.advancedInput)}
                    />
                  </div>
                )}

                <div {...stylex.props(styles.popoverFooter)}>
                  <button
                    type="button"
                    onClick={() => setShowAdvancedUrl((v) => !v)}
                    {...stylex.props(styles.advancedLinkBtn)}
                  >
                    {showAdvancedUrl
                      ? 'Ocultar servidor'
                      : 'Conexión avanzada…'}
                  </button>
                </div>
              </div>
            )}
          </div>
        </footer>
      </div>
    </View>
  );
}

const styles = stylex.create({
  pageContainer: {
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    width: '100%',
    backgroundColor: '#0c0e12',
    color: '#edeff3',
    overflow: 'hidden',
  },
  scrollCanvas: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minHeight: 0,
    overflowY: 'auto',
    paddingBlock: '30px',
    paddingInline: '28px',
  },
  mainSheet: {
    maxWidth: '980px',
    marginInline: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '30px',
  },
  headerRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '16px',
    paddingBottom: '18px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
  },
  pageTitle: {
    margin: 0,
    fontSize: '22px',
    fontWeight: 600,
    letterSpacing: '-0.02em',
    color: '#edeff3',
  },
  headerActions: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
  },
  secondaryBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    paddingBlock: '7px',
    paddingInline: '13px',
    borderRadius: '7px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    backgroundColor: '#151a22',
    color: '#edeff3',
    fontSize: '12.5px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  secondaryBtnDone: {
    borderColor: 'rgba(20, 184, 166, 0.4)',
    color: '#2dd4bf',
  },
  primaryBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    paddingBlock: '7px',
    paddingInline: '14px',
    borderRadius: '7px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#14b8a6',
    backgroundColor: '#14b8a6',
    color: '#041311',
    fontSize: '12.5px',
    fontWeight: 600,
    cursor: 'pointer',
  },
  flowGrid: {
    display: 'grid',
    gridTemplateColumns: '210px 24px minmax(0, 1fr) 24px 210px',
    alignItems: 'center',
    columnGap: '10px',
  },
  endpointBox: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '10px',
    paddingBlock: '14px',
    paddingInline: '14px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '10px',
    minWidth: 0,
  },
  endpointHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '6px',
  },
  endpointLabel: {
    fontSize: '12.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  swapIconBtn: {
    borderWidth: 0,
    backgroundColor: 'transparent',
    color: '#8e98a8',
    fontSize: '13px',
    cursor: 'pointer',
    paddingBlock: '2px',
    paddingInline: '4px',
  },
  librarySelect: {
    width: '100%',
    paddingBlock: '5px',
    paddingInline: '7px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#8e98a8',
    fontSize: '11.5px',
  },
  endpointTrackPill: {
    paddingBlock: '6px',
    paddingInline: '10px',
    borderRadius: '6px',
    backgroundColor: '#1a202b',
    color: '#edeff3',
    fontSize: '12.5px',
    fontWeight: 500,
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  endpointMetaRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '5px',
    fontSize: '12px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  inlineNumber: {
    width: '48px',
    paddingBlock: '2px',
    paddingInline: '5px',
    borderRadius: '4px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    fontSize: '12px',
    fontVariantNumeric: 'tabular-nums',
  },
  inlineKeySelect: {
    paddingBlock: '2px',
    paddingInline: '5px',
    borderRadius: '4px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    fontSize: '12px',
    fontVariantNumeric: 'tabular-nums',
  },
  connectorArrow: {
    textAlign: 'center',
    color: '#8e98a8',
    fontSize: '16px',
    userSelect: 'none',
  },
  bridgeCenter: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '10px',
    minWidth: 0,
  },
  bridgeHeading: {
    margin: 0,
    fontSize: '13.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  stepsList: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '10px',
    overflow: 'hidden',
  },
  stepRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '12px',
    paddingBlock: '12px',
    paddingInline: '14px',
    minWidth: 0,
  },
  stepRowBorder: {
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1e2530',
  },
  stepNum: {
    fontSize: '12px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  stepTitle: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minWidth: 0,
    fontSize: '13px',
    fontWeight: 500,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  stepMeta: {
    fontSize: '12px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
    flexShrink: 0,
  },
  cueDots: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '4px',
    flexShrink: 0,
  },
  cueDot: {
    width: '5.5px',
    height: '5.5px',
    borderRadius: '999px',
  },
  listenBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    paddingBlock: '5px',
    paddingInline: '10px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#252d3b',
    backgroundColor: '#19202b',
    color: '#edeff3',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
    flexShrink: 0,
  },
  listenBtnActive: {
    borderColor: '#14b8a6',
    color: '#2dd4bf',
  },
  suggestionsSection: {
    maxWidth: '640px',
    width: '100%',
    marginInline: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '10px',
  },
  disclosureToggle: {
    alignSelf: 'flex-start',
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '8px',
    borderWidth: 0,
    backgroundColor: 'transparent',
    paddingBlock: '4px',
    paddingInline: '2px',
    color: '#edeff3',
    fontSize: '13px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  suggestionsList: {
    backgroundColor: '#10141b',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '10px',
    overflow: 'hidden',
  },
  suggestionRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '12px',
    paddingBlock: '10px',
    paddingInline: '14px',
    minWidth: 0,
  },
  addBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    paddingBlock: '4px',
    paddingInline: '10px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#252d3b',
    backgroundColor: '#171c24',
    color: '#edeff3',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
    flexShrink: 0,
  },
  addBtnDone: {
    borderColor: 'rgba(20, 184, 166, 0.4)',
    color: '#2dd4bf',
  },
  commandFooter: {
    flexShrink: 0,
    paddingTop: '10px',
    paddingBottom: '18px',
    paddingInline: '24px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1c222d',
    backgroundColor: '#0c0e12',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    rowGap: '6px',
  },
  quietStatusNote: {
    fontSize: '11.5px',
    color: '#2dd4bf',
  },
  commandDockRow: {
    position: 'relative',
    width: '100%',
    maxWidth: '640px',
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
  },
  spotlightBar: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    paddingBlock: '6px',
    paddingLeft: '14px',
    paddingRight: '8px',
    borderRadius: '10px',
    backgroundColor: '#131820',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#252d3b',
  },
  spotlightInput: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minWidth: 0,
    borderWidth: 0,
    outline: 'none',
    backgroundColor: 'transparent',
    color: '#edeff3',
    fontSize: '13px',
    paddingBlock: '4px',
  },
  quickPills: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    flexShrink: 0,
  },
  quickPill: {
    paddingBlock: '4px',
    paddingInline: '10px',
    borderRadius: '999px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#262f3d',
    backgroundColor: '#1b222d',
    color: '#edeff3',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  assistantTriggerBtn: {
    width: '36px',
    height: '36px',
    borderRadius: '9px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#252d3b',
    backgroundColor: '#151a22',
    color: '#8e98a8',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
    flexShrink: 0,
  },
  assistantTriggerBtnActive: {
    borderColor: 'rgba(20, 184, 166, 0.45)',
    color: '#edeff3',
    backgroundColor: '#1a212c',
  },
  assistantPopover: {
    position: 'absolute',
    right: 0,
    bottom: '46px',
    width: '296px',
    borderRadius: '12px',
    backgroundColor: '#151a23',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#283140',
    boxShadow: '0 16px 40px rgba(0, 0, 0, 0.55)',
    zIndex: 50,
    overflow: 'hidden',
  },
  popoverHead: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingBlock: '12px',
    paddingInline: '16px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#202836',
  },
  popoverTitleGroup: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '8px',
  },
  popoverTitle: {
    fontSize: '13.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  readyDot: {
    width: '7px',
    height: '7px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
  },
  popoverReadyText: {
    fontSize: '12px',
    color: '#8e98a8',
  },
  popoverRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '10px',
    paddingBlock: '11px',
    paddingInline: '16px',
  },
  popoverRowDivider: {
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#202836',
  },
  popoverLabel: {
    fontSize: '12.5px',
    color: '#edeff3',
    fontWeight: 500,
  },
  popoverSegment: {
    display: 'inline-flex',
    alignItems: 'center',
    padding: '2px',
    borderRadius: '999px',
    backgroundColor: '#0f131a',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#242c3b',
  },
  popSegBtn: {
    paddingBlock: '4px',
    paddingInline: '10px',
    borderRadius: '999px',
    borderWidth: 0,
    backgroundColor: 'transparent',
    color: '#8e98a8',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  popSegBtnTeal: {
    backgroundColor: '#14b8a6',
    color: '#041311',
    fontWeight: 600,
  },
  popSegBtnNeutral: {
    backgroundColor: '#252e3d',
    color: '#edeff3',
    fontWeight: 600,
  },
  advancedBox: {
    paddingTop: '4px',
    paddingBottom: '10px',
    paddingInline: '16px',
  },
  advancedInput: {
    width: '100%',
    paddingBlock: '6px',
    paddingInline: '9px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#252d3b',
    color: '#edeff3',
    fontSize: '11.5px',
  },
  popoverFooter: {
    paddingBlock: '9px',
    paddingInline: '16px',
    backgroundColor: '#11161e',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#202836',
    textAlign: 'center',
  },
  advancedLinkBtn: {
    borderWidth: 0,
    backgroundColor: 'transparent',
    color: '#8e98a8',
    fontSize: '11.5px',
    cursor: 'pointer',
  },
});
