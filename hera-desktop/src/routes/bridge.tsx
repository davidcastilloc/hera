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
  getCloudflareEdgeRadar,
  probeLocalAiBackend,
  resolveHarmonicBridge,
  scanAiConnectivityOnboarding,
  type AiConnectivityOnboardingReport,
  type CloudflareEdgeCapabilities,
  type CommanderAuthorityMode,
  type HarmonicBridgeStep,
  type HeraCloudflareFreeAiResponse,
  type ResolveHarmonicBridgeResponse,
  type SwarmCatalogItem,
} from '../lib/bridge-hera';
import {
  getHumanMatchLabel,
  useHeraViewStore,
} from '../lib/hera-human-language';
import player from '../lib/player';
import useLibraryStore from '../lib/store';
import toastManager from '../lib/toast-manager';

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

const PHRASE_RAIL_MARKERS = [
  {
    num: 1,
    simpleLabel: '1 · Entrada',
    proLabel: '1 · Intro',
    time: '00:16',
    pct: '5%',
    color: '#22c55e',
  },
  {
    num: 2,
    simpleLabel: '2 · Calma',
    proLabel: '2 · Breakdown',
    time: '01:48',
    pct: '33%',
    color: '#38bdf8',
  },
  {
    num: 3,
    simpleLabel: '3 · Explosión',
    proLabel: '3 · Drop',
    time: '02:34',
    pct: '48%',
    color: '#ef4444',
  },
  {
    num: 4,
    simpleLabel: '4 · Salida',
    proLabel: '4 · Outro',
    time: '05:12',
    pct: '92%',
    color: '#f59e0b',
  },
] as const;

function camelotRootHz(camelot: string): { root: number; major: boolean } {
  const clean = camelot.trim().toUpperCase();
  const mode = clean.endsWith('B') ? 'B' : 'A';
  const num = Number.parseInt(clean.slice(0, -1), 10);
  const minorRoots = [
    207.65, 155.56, 233.08, 174.61, 130.81, 196.0, 146.83, 220.0, 164.81,
    246.94, 185.0, 138.59,
  ];
  const idx = Number.isFinite(num) && num >= 1 && num <= 12 ? num - 1 : 7;
  const base = minorRoots[idx];
  return {
    root: mode === 'B' ? base * 1.1892071 : base,
    major: mode === 'B',
  };
}

function buildSyntheticTrackDescriptor(
  id: string,
  artist: string,
  title: string,
  bpm: number,
  camelot: string,
  savedPath?: string,
): Track {
  const duration = 210;
  return {
    id,
    path: savedPath || `library/${artist} - ${title}.wav`,
    title,
    album: 'Puente Armónico HERA',
    album_artist: artist,
    artists: [artist],
    genres: ['Club'],
    year: 2026,
    duration,
    track_no: null,
    track_of: null,
    disk_no: null,
    disk_of: null,
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
        color: '#3b82f6',
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
        color: '#eab308',
        role: 'outro',
      },
    ],
  };
}

function ViewHarmonicBridgeP2P() {
  const viewMode = useHeraViewStore((state) => state.viewMode);
  const isOnboardingOpen = useHeraViewStore((state) => state.isOnboardingOpen);
  const closeOnboarding = useHeraViewStore((state) => state.closeOnboarding);
  const openOnboarding = useHeraViewStore((state) => state.openOnboarding);
  const tracks = useLibraryStore((state) => state.tracks);
  const invalidate = useInvalidate();
  const navigate = useNavigate();

  const [originTrackId, setOriginTrackId] = useState<string>('');
  const [originTitle, setOriginTitle] = useState('Levels (Original Mix)');
  const [originArtist, setOriginArtist] = useState('Avicii');
  const [originBpm, setOriginBpm] = useState(124.0);
  const [originCamelot, setOriginCamelot] = useState('8A');

  const [targetTrackId, setTargetTrackId] = useState<string>('');
  const [targetTitle, setTargetTitle] = useState("Don't You Worry Child");
  const [targetArtist, setTargetArtist] = useState('Swedish House Mafia');
  const [targetBpm, setTargetBpm] = useState(128.0);
  const [targetCamelot, setTargetCamelot] = useState('10A');

  const [bridgeResult, setBridgeResult] =
    useState<ResolveHarmonicBridgeResponse | null>(null);
  const [customSteps, setCustomSteps] = useState<HarmonicBridgeStep[] | null>(
    null,
  );
  const [edgeRadar, setEdgeRadar] = useState<CloudflareEdgeCapabilities | null>(
    null,
  );
  const [previewStatus, setPreviewStatus] = useState<string>('');
  const [activeAuditionId, setActiveAuditionId] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);

  const [chatPrompt, setChatPrompt] = useState('');
  const [activeInstruction, setActiveInstruction] = useState(
    "Quiero pasar de «Levels» a «Don't You Worry Child» subiendo el ritmo de forma progresiva y sin superponer vocales.",
  );
  const [selectedBackend, setSelectedBackend] =
    useState<string>('cloudflare_free');
  const [customBaseUrl, setCustomBaseUrl] = useState<string>(
    'http://localhost:1234/v1',
  );
  const [probeBanner, setProbeBanner] = useState<string>('');
  const [commanderMode, setCommanderMode] =
    useState<CommanderAuthorityMode>('executive_boss');
  const [onboardingReport, setOnboardingReport] =
    useState<AiConnectivityOnboardingReport | null>(null);
  const [isScanningOnboarding, setIsScanningOnboarding] =
    useState<boolean>(false);
  const [aiResponse, setAiResponse] =
    useState<HeraCloudflareFreeAiResponse | null>(null);
  const [isAiLoading, setIsAiLoading] = useState<boolean>(false);
  const [showProOptions, setShowProOptions] = useState<boolean>(false);

  const audioCtxRef = useRef<AudioContext | null>(null);
  const previewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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

  const handleSelectDeckTrack = (deck: 'A' | 'B', trackId: string) => {
    const found = tracks.find((t) => t.id === trackId);
    if (!found) return;
    const artist = found.artists[0] || found.album_artist || 'Artista Local';
    const bpm = Math.round((found.bpm ?? 124) * 10) / 10;
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

  const handleRunOnboardingScan = useCallback(async () => {
    setIsScanningOnboarding(true);
    try {
      const rep = await scanAiConnectivityOnboarding(commanderMode);
      setOnboardingReport(rep);
      if (rep.recommended_backend_id) {
        setSelectedBackend(rep.recommended_backend_id);
      }
      setProbeBanner(rep.summary_badge);
    } catch {
      // Offline fallback
    } finally {
      setIsScanningOnboarding(false);
    }
  }, [commanderMode]);

  useEffect(() => {
    void getCloudflareEdgeRadar()
      .then(setEdgeRadar)
      .catch(() => {});
    void handleRunOnboardingScan();
  }, [handleRunOnboardingScan]);

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

  const activeSteps: HarmonicBridgeStep[] = useMemo(() => {
    if (customSteps && customSteps.length > 0) return customSteps;
    if (bridgeResult?.bridge_steps && bridgeResult.bridge_steps.length > 0) {
      return bridgeResult.bridge_steps;
    }
    return [
      {
        step_number: 1,
        track_id: 'p2p-bridge-01',
        artist: 'Keinemusik & Rampa',
        title: 'Cesonia (Afro-Tech P2P Bridge Edit)',
        bpm: Math.round((originBpm + (targetBpm - originBpm) * 0.35) * 10) / 10,
        camelot: '9A',
        owner_dj: 'DaviDJ_EdgeGuardian',
        hera_seal: 'hera_seal_9f81c2e4b0a1',
        cue_in_seconds: 15.2,
        cue_out_seconds: 168.4,
        synergy_score: 0.96,
        transition_tip: `Cruza Cue 4 de «${originTitle}» con Cue 1 en 9A`,
        p2p_boost_free_promo: true,
        boost_credit_cost_mb: 0,
      },
      {
        step_number: 2,
        track_id: 'p2p-bridge-02',
        artist: 'Hugelkultur & Hugel',
        title: 'Morenita (Latin Club Harmonic Connector)',
        bpm: Math.round((originBpm + (targetBpm - originBpm) * 0.72) * 10) / 10,
        camelot: targetCamelot,
        owner_dj: 'DJFoo_MedellinNode',
        hera_seal: 'hera_seal_4d72a8f103e9',
        cue_in_seconds: 14.8,
        cue_out_seconds: 174.0,
        synergy_score: 0.95,
        transition_tip: `Entrega en llave ${targetCamelot} directo al Cue 1 de «${targetTitle}»`,
        p2p_boost_free_promo: false,
        boost_credit_cost_mb: 38,
      },
    ];
  }, [
    bridgeResult,
    customSteps,
    originBpm,
    originTitle,
    targetBpm,
    targetCamelot,
    targetTitle,
  ]);

  const swarmCatalog: SwarmCatalogItem[] = useMemo(() => {
    if (bridgeResult?.swarm_catalog && bridgeResult.swarm_catalog.length > 0) {
      return bridgeResult.swarm_catalog;
    }
    return activeSteps.map((s, idx) => ({
      cas_fingerprint: `cas-${s.track_id}`,
      artist: s.artist,
      title: s.title,
      bpm: s.bpm,
      camelot: s.camelot,
      subgenre: idx === 0 ? 'Afro-Tech / Promo Pool' : 'Latin Tech House',
      hera_seal: s.hera_seal,
      opaque_blob_key: `shards/boost/${s.track_id}.shard`,
      swarm_replicas_count: 3,
      seeders: [`${s.owner_dj} (r2_boost)`],
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
    label: string,
    bpm: number,
    camelot: string,
    cueSeconds: number,
    localCandidate?: Track,
  ) => {
    if (activeAuditionId === auditionId) {
      stopWebAudioPreview();
      player.pause();
      setPreviewStatus('Preescucha detenida.');
      return;
    }

    stopWebAudioPreview();

    if (localCandidate) {
      await player.start([localCandidate], localCandidate.id, {
        type: 'library',
      });
      player.setCurrentTime(Math.max(0, cueSeconds));
      setActiveAuditionId(auditionId);
      setPreviewStatus(
        `Reproduciendo pista real «${localCandidate.title}» desde Cue (${cueSeconds.toFixed(1)}s).`,
      );
      return;
    }

    try {
      const AudioContextCtor =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext })
          .webkitAudioContext;
      const ctx = new AudioContextCtor();
      audioCtxRef.current = ctx;
      const durationSecs = 6.0;
      const now = ctx.currentTime + 0.04;
      const beatSec = 60 / Math.max(70, Math.min(180, bpm));
      const { root, major } = camelotRootHz(camelot);
      const thirdRatio = major ? 1.259921 : 1.1892071;

      const masterGain = ctx.createGain();
      masterGain.gain.setValueAtTime(0.22, now);
      masterGain.gain.exponentialRampToValueAtTime(0.001, now + durationSecs);
      masterGain.connect(ctx.destination);

      // Harmonic chord pad in target Camelot key
      [root, root * thirdRatio, root * 1.4983071].forEach((freq) => {
        const osc = ctx.createOscillator();
        const padGain = ctx.createGain();
        osc.type = 'triangle';
        osc.frequency.setValueAtTime(freq, now);
        padGain.gain.setValueAtTime(0.001, now);
        padGain.gain.linearRampToValueAtTime(0.26, now + 0.4);
        padGain.gain.linearRampToValueAtTime(0.18, now + durationSecs - 0.5);
        padGain.gain.exponentialRampToValueAtTime(0.001, now + durationSecs);
        osc.connect(padGain);
        padGain.connect(masterGain);
        osc.start(now);
        osc.stop(now + durationSecs);
      });

      // Beat-matched 4-on-the-floor kick pulse
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
      setPreviewStatus(
        `Preescuchando pulso armónico de «${label}» (${bpm.toFixed(1)} BPM · ${camelot})...`,
      );
      previewTimerRef.current = setTimeout(() => {
        stopWebAudioPreview();
        setPreviewStatus(`Preescucha completada (${label}).`);
      }, durationSecs * 1000);
    } catch {
      setPreviewStatus(`Preescucha lista para «${label}» (${camelot}).`);
    }
  };

  const handleAcquireAndInsertCandidate = async (item: SwarmCatalogItem) => {
    if (busyAction) return;
    setBusyAction(item.cas_fingerprint);
    setPreviewStatus(
      `Descargando e incorporando «${item.artist} — ${item.title}»...`,
    );
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
        owner_dj: item.seeders[0] || 'SwarmPeer',
        hera_seal: item.hera_seal,
        cue_in_seconds: 15.0,
        cue_out_seconds: 170.0,
        synergy_score: 0.97,
        transition_tip: `Incorporada desde el enjambre (${item.camelot} · ${item.bpm.toFixed(1)} BPM)`,
        p2p_boost_free_promo: item.p2p_boost_free_promo,
        boost_credit_cost_mb: 0,
      };
      setCustomSteps([updatedStep, ...activeSteps.slice(1)]);
      setPreviewStatus(
        `«${item.artist} — ${item.title}» importada a tu biblioteca e incorporada al Paso 1.`,
      );
    } catch {
      setPreviewStatus(`No se pudo incorporar «${item.title}».`);
    } finally {
      setBusyAction(null);
    }
  };

  const handleApproveAndCreatePlaylist = async () => {
    if (busyAction) return;
    setBusyAction('approve_set');
    setPreviewStatus(
      'Preparando pistas del puente y creando lista en tu biblioteca...',
    );
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

      const playlistName = `Puente ${originCamelot}→${targetCamelot} (${Math.round(originBpm)}-${Math.round(targetBpm)} BPM)`;
      const playlist = await PlaylistsAPI.create(
        playlistName,
        orderedIds,
        true,
      );
      await invalidate();

      const msg = `Set «${playlistName}» creado con ${orderedIds.length} pista(s) listas para reproducir.`;
      setPreviewStatus(msg);
      toastManager.add({ title: msg, type: 'success', timeout: 5000 });

      if (playlist) {
        void navigate({
          to: '/playlists/$playlistID',
          params: { playlistID: playlist.id },
        });
      }
    } catch {
      setPreviewStatus('No se pudo guardar el puente en Listas.');
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

      const res = await exportCrateToDjSoftware(
        `Hera_Bridge_${originCamelot}_${targetCamelot}`,
        [deckATrack, ...stepTracks, deckBTrack],
        ['virtualdj', 'm3u8'],
      );
      const outPath = res.written_files[0] ?? 'exports/Hera_Bridge.vdjfolder';
      const msg = `Exportado a VirtualDJ (${res.track_count} pistas + 4 Hot Cues): ${outPath}`;
      setPreviewStatus(msg);
      toastManager.add({ title: msg, type: 'success', timeout: 5500 });
    } catch {
      setPreviewStatus('No se pudo exportar el crate a VirtualDJ.');
    } finally {
      setBusyAction(null);
    }
  };

  const handleSendCopilotCommand = async (customText?: string) => {
    const promptToSend = (customText ?? chatPrompt).trim();
    if (!promptToSend || isAiLoading) return;

    setActiveInstruction(promptToSend);
    if (!customText) {
      setChatPrompt('');
    }
    setIsAiLoading(true);
    try {
      const useCustomEndpoint =
        selectedBackend === 'custom' ||
        selectedBackend === 'lmstudio' ||
        selectedBackend === 'ollama';

      const reply = await chatWithCloudflareFreeAi(promptToSend, 'DaviDJ', {
        backend: selectedBackend,
        baseUrl: useCustomEndpoint ? customBaseUrl : undefined,
        commanderMode,
      });
      setAiResponse(reply);
      setPreviewStatus(
        viewMode === 'simple'
          ? 'Propuesta actualizada por el Copiloto · Revisa los pasos abajo'
          : `Respuesta generada (${reply.model_used})`,
      );
    } catch {
      setPreviewStatus(
        'El motor seleccionado no respondió; usando motor local determinista.',
      );
    } finally {
      setIsAiLoading(false);
    }
  };

  const matchInfo = getHumanMatchLabel(
    originCamelot,
    targetCamelot,
    originBpm,
    targetBpm,
  );

  const avgStepDelta =
    activeSteps.length > 0
      ? ((targetBpm - originBpm) / (activeSteps.length + 1)).toFixed(1)
      : (targetBpm - originBpm).toFixed(1);

  return (
    <View aria-label="Puente Armónico y Copiloto de Cabina">
      <div {...stylex.props(styles.pageContainer)}>
        <header {...stylex.props(styles.topBar)}>
          <div {...stylex.props(styles.brandGroup)}>
            <Icon name="bridge" size={18} color="#2dd4bf" />
            <h1 {...stylex.props(styles.pageTitle)}>
              {viewMode === 'simple'
                ? 'Puente Musical y Copiloto de Cabina'
                : 'Puente Armónico 2-Opt y Enjambre P2P'}
            </h1>
          </div>

          <div {...stylex.props(styles.topActions)}>
            <button
              type="button"
              onClick={openOnboarding}
              {...stylex.props(styles.statusPill, styles.activePill)}
              title="Configurar motor de IA y autoridad de cabina"
            >
              <span {...stylex.props(styles.emeraldDot)} />
              <span>
                {viewMode === 'simple'
                  ? `Copiloto: ${selectedBackend === 'cloudflare_free' ? 'Cloud Free' : selectedBackend.replace('cli_', '')}`
                  : `Motor: ${selectedBackend} · ${commanderMode}`}
              </span>
            </button>

            <button
              type="button"
              onClick={openOnboarding}
              {...stylex.props(styles.statusPill)}
            >
              <Icon name="cpu" size={14} />
              <span>{viewMode === 'simple' ? 'Motor IA' : '5 Modos IA'}</span>
            </button>

            <div {...stylex.props(styles.swarmPill)}>
              <Icon name="checkCircle" size={14} color="#2dd4bf" />
              <span>
                {viewMode === 'simple'
                  ? 'Enjambre conectado'
                  : `${edgeRadar?.active_swarm_nodes ?? 3} nodos · ${activeSteps.length} pasos`}
              </span>
            </div>
          </div>
        </header>

        <div {...stylex.props(styles.splitWorkspace)}>
          <section {...stylex.props(styles.leftCol)}>
            <div {...stylex.props(styles.sectionHead)}>
              <h2 {...stylex.props(styles.sectionTitle)}>
                {viewMode === 'simple'
                  ? 'Secuencia sugerida entre ambas pistas'
                  : 'Ruta óptima Deck A a Deck B (2-Opt TSP)'}
              </h2>
              <span {...stylex.props(styles.matchTag)}>
                {viewMode === 'simple'
                  ? `${matchInfo.compatibilityLabel} · ${matchInfo.tempoAdvice}`
                  : `Delta ${targetBpm >= originBpm ? '+' : ''}${(targetBpm - originBpm).toFixed(1)} BPM · ${originCamelot} → ${targetCamelot}`}
              </span>
            </div>

            <div {...stylex.props(styles.deckRack)}>
              <div {...stylex.props(styles.decksRow)}>
                <div {...stylex.props(styles.deckCard)}>
                  <div {...stylex.props(styles.deckTopLine)}>
                    <span {...stylex.props(styles.deckRole)}>
                      {viewMode === 'simple'
                        ? 'Deck A · En reproducción'
                        : 'Deck A · Origen'}
                    </span>
                    <span {...stylex.props(styles.deckMetricBadge)}>
                      {originCamelot} · {originBpm.toFixed(1)} BPM
                    </span>
                  </div>

                  {tracks.length > 0 && (
                    <select
                      value={originTrackId}
                      onChange={(e) =>
                        handleSelectDeckTrack('A', e.target.value)
                      }
                      aria-label="Seleccionar pista de tu biblioteca para Deck A"
                      {...stylex.props(styles.deckSelect)}
                    >
                      <option value="">Elegir de tu biblioteca...</option>
                      {tracks.map((t) => (
                        <option key={t.id} value={t.id}>
                          {t.artists[0] || 'Artista'} — {t.title}
                        </option>
                      ))}
                    </select>
                  )}

                  <div {...stylex.props(styles.deckSongTitle)}>
                    {originArtist} — {originTitle}
                  </div>

                  <div {...stylex.props(styles.deckControlsInline)}>
                    <label {...stylex.props(styles.inlineFieldLabel)}>
                      <span>BPM</span>
                      <input
                        type="number"
                        min={70}
                        max={190}
                        step={0.5}
                        value={originBpm}
                        onChange={(e) => {
                          const v = Number.parseFloat(e.target.value);
                          if (Number.isFinite(v)) {
                            setCustomSteps(null);
                            setOriginBpm(v);
                          }
                        }}
                        {...stylex.props(styles.compactNumInput)}
                      />
                    </label>
                    <label {...stylex.props(styles.inlineFieldLabel)}>
                      <span>Llave</span>
                      <select
                        value={originCamelot}
                        onChange={(e) => {
                          setCustomSteps(null);
                          setOriginCamelot(e.target.value);
                        }}
                        {...stylex.props(styles.compactKeySelect)}
                      >
                        {CAMELOT_OPTIONS.map((k) => (
                          <option key={k} value={k}>
                            {k}
                          </option>
                        ))}
                      </select>
                    </label>
                  </div>
                </div>

                <button
                  type="button"
                  onClick={handleSwapDecks}
                  title="Invertir Deck A y Deck B"
                  {...stylex.props(styles.swapBtn)}
                >
                  ⇄
                </button>

                <div {...stylex.props(styles.deckCard)}>
                  <div {...stylex.props(styles.deckTopLine)}>
                    <span {...stylex.props(styles.deckRole)}>
                      {viewMode === 'simple'
                        ? 'Deck B · Pista destino'
                        : 'Deck B · Destino'}
                    </span>
                    <span {...stylex.props(styles.deckMetricBadge)}>
                      {targetCamelot} · {targetBpm.toFixed(1)} BPM
                    </span>
                  </div>

                  {tracks.length > 0 && (
                    <select
                      value={targetTrackId}
                      onChange={(e) =>
                        handleSelectDeckTrack('B', e.target.value)
                      }
                      aria-label="Seleccionar pista de tu biblioteca para Deck B"
                      {...stylex.props(styles.deckSelect)}
                    >
                      <option value="">Elegir de tu biblioteca...</option>
                      {tracks.map((t) => (
                        <option key={t.id} value={t.id}>
                          {t.artists[0] || 'Artista'} — {t.title}
                        </option>
                      ))}
                    </select>
                  )}

                  <div {...stylex.props(styles.deckSongTitle)}>
                    {targetArtist} — {targetTitle}
                  </div>

                  <div {...stylex.props(styles.deckControlsInline)}>
                    <label {...stylex.props(styles.inlineFieldLabel)}>
                      <span>BPM</span>
                      <input
                        type="number"
                        min={70}
                        max={190}
                        step={0.5}
                        value={targetBpm}
                        onChange={(e) => {
                          const v = Number.parseFloat(e.target.value);
                          if (Number.isFinite(v)) {
                            setCustomSteps(null);
                            setTargetBpm(v);
                          }
                        }}
                        {...stylex.props(styles.compactNumInput)}
                      />
                    </label>
                    <label {...stylex.props(styles.inlineFieldLabel)}>
                      <span>Llave</span>
                      <select
                        value={targetCamelot}
                        onChange={(e) => {
                          setCustomSteps(null);
                          setTargetCamelot(e.target.value);
                        }}
                        {...stylex.props(styles.compactKeySelect)}
                      >
                        {CAMELOT_OPTIONS.map((k) => (
                          <option key={k} value={k}>
                            {k}
                          </option>
                        ))}
                      </select>
                    </label>
                  </div>
                </div>
              </div>

              <div {...stylex.props(styles.stepsContainer)}>
                {activeSteps.map((step, index) => {
                  const auditionKey = `step-${step.track_id}-${index}`;
                  const isPlayingThis = activeAuditionId === auditionKey;
                  const localTrack = findMatchingLocalTrack(
                    step.artist,
                    step.title,
                    step.track_id,
                  );
                  return (
                    <div key={auditionKey} {...stylex.props(styles.stepRow)}>
                      <span {...stylex.props(styles.stepIdx)}>
                        Paso {index + 1}
                      </span>
                      <div {...stylex.props(styles.stepName)}>
                        {step.artist} — {step.title}{' '}
                        <span {...stylex.props(styles.dimText)}>
                          · {step.camelot}
                        </span>
                      </div>
                      <span {...stylex.props(styles.matchTag)}>
                        Afinidad {Math.round(step.synergy_score * 100)}%
                      </span>
                      <span {...stylex.props(styles.bpmTag)}>
                        {step.bpm.toFixed(1)} BPM
                      </span>
                      <button
                        type="button"
                        onClick={() =>
                          void handleAuditionStep(
                            auditionKey,
                            `${step.artist} — ${step.title}`,
                            step.bpm,
                            step.camelot,
                            step.cue_in_seconds,
                            localTrack,
                          )
                        }
                        {...stylex.props(
                          styles.previewBtn,
                          isPlayingThis && styles.previewBtnActive,
                        )}
                      >
                        <Icon
                          name={isPlayingThis ? 'pause' : 'play'}
                          size={12}
                        />
                        <span>
                          {isPlayingThis
                            ? 'Detener'
                            : viewMode === 'simple'
                              ? 'Oír cruce'
                              : 'Cruce 4→1'}
                        </span>
                      </button>
                    </div>
                  );
                })}
              </div>

              <div {...stylex.props(styles.phraseRailPanel)}>
                <div {...stylex.props(styles.phraseRailHeader)}>
                  <span {...stylex.props(styles.phraseRailTitle)}>
                    {viewMode === 'simple'
                      ? 'Mapa proporcional de 4 Hot Cues de frase'
                      : 'Riel acústico de 4 Hot Cues (detección espectral)'}
                  </span>
                  <span {...stylex.props(styles.dimText)}>
                    Ventana de cruce:{' '}
                    {bridgeResult?.preview_mix_window_seconds ?? 15}s
                  </span>
                </div>

                <div {...stylex.props(styles.phraseTimelineTrack)}>
                  {PHRASE_RAIL_MARKERS.map((marker) => (
                    <span
                      key={marker.num}
                      {...stylex.props(styles.phraseTimelineTick)}
                      style={{
                        left: marker.pct,
                        backgroundColor: marker.color,
                      }}
                      title={`${marker.simpleLabel} (${marker.time})`}
                    />
                  ))}
                </div>

                <div {...stylex.props(styles.cuePinsList)}>
                  {PHRASE_RAIL_MARKERS.map((marker) => (
                    <span key={marker.num} {...stylex.props(styles.cuePin)}>
                      <span
                        {...stylex.props(styles.cueDot)}
                        style={{ backgroundColor: marker.color }}
                      />
                      <span>
                        {viewMode === 'simple'
                          ? marker.simpleLabel
                          : marker.proLabel}
                      </span>
                      <span {...stylex.props(styles.cueTime)}>
                        {marker.time}
                      </span>
                    </span>
                  ))}
                </div>
              </div>
            </div>

            <div {...stylex.props(styles.sectionHead)}>
              <h2 {...stylex.props(styles.sectionTitle)}>
                {viewMode === 'simple'
                  ? 'Candidatos compatibles en la red de cabinas'
                  : 'Índice federado del enjambre P2P'}
              </h2>
              <span {...stylex.props(styles.dimText)}>
                {viewMode === 'simple'
                  ? 'Calidad de estudio verificada'
                  : 'Auditoría espectral FFT > 20 kHz'}
              </span>
            </div>

            <div {...stylex.props(styles.tableWrap)}>
              <table {...stylex.props(styles.table)}>
                <thead>
                  <tr>
                    <th {...stylex.props(styles.th)}>Pista / Artista</th>
                    <th {...stylex.props(styles.th)}>Compatibilidad</th>
                    <th {...stylex.props(styles.th)}>BPM</th>
                    <th {...stylex.props(styles.th)}>Calidad</th>
                    <th {...stylex.props(styles.th)}>Acción</th>
                  </tr>
                </thead>
                <tbody>
                  {swarmCatalog.slice(0, 6).map((item) => {
                    const isBusy = busyAction === item.cas_fingerprint;
                    return (
                      <tr key={item.cas_fingerprint}>
                        <td {...stylex.props(styles.td)}>
                          <strong>
                            {item.artist} — {item.title}
                          </strong>{' '}
                          <span {...stylex.props(styles.dimText)}>
                            · {item.subgenre}
                          </span>
                        </td>
                        <td {...stylex.props(styles.td)}>
                          <span {...stylex.props(styles.matchTag)}>
                            {item.camelot} · {item.swarm_replicas_count} nodos
                          </span>
                        </td>
                        <td {...stylex.props(styles.td, styles.tabularCell)}>
                          {item.bpm.toFixed(1)}
                        </td>
                        <td {...stylex.props(styles.td, styles.verifiedCell)}>
                          Estudio (
                          {Math.round(
                            (item.effective_bandwidth_hz ?? 21200) / 1000,
                          )}{' '}
                          kHz)
                        </td>
                        <td {...stylex.props(styles.td)}>
                          <button
                            type="button"
                            disabled={Boolean(busyAction)}
                            onClick={() =>
                              void handleAcquireAndInsertCandidate(item)
                            }
                            {...stylex.props(styles.tableBtn)}
                          >
                            <Icon name="plus" size={12} />
                            <span>
                              {isBusy ? 'Importando...' : 'Incorporar'}
                            </span>
                          </button>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </section>

          <section {...stylex.props(styles.rightCol)}>
            <div {...stylex.props(styles.sectionHead)}>
              <h2 {...stylex.props(styles.sectionTitle)}>
                {viewMode === 'simple'
                  ? 'Copiloto de Cabina'
                  : 'Consola ReAct de Curaduría'}
              </h2>
              <span {...stylex.props(styles.freeMeta)}>
                {viewMode === 'simple'
                  ? 'Aprobación manual por cambio'
                  : `${selectedBackend} · $0.00`}
              </span>
            </div>

            <div {...stylex.props(styles.copilotBody)}>
              <div {...stylex.props(styles.bossMsg)}>
                <div {...stylex.props(styles.bossHeader)}>
                  <strong>Tu instrucción</strong>
                  <span>Activa</span>
                </div>
                {activeInstruction}
              </div>

              <div {...stylex.props(styles.receiptBox)}>
                <div {...stylex.props(styles.receiptHead)}>
                  <div {...stylex.props(styles.receiptTitle)}>
                    <span {...stylex.props(styles.emeraldDot)} />
                    <span>
                      {viewMode === 'simple'
                        ? `Puente de ${activeSteps.length} pistas listo para cabina`
                        : `Secuencia 2-Opt TSP verificada (${bridgeResult?.harmonic_distance_steps ?? 2} pasos Camelot)`}
                    </span>
                  </div>
                  <span {...stylex.props(styles.dimText)}>
                    {viewMode === 'simple'
                      ? 'Audio verificado'
                      : 'Sinergia 96%'}
                  </span>
                </div>

                <div {...stylex.props(styles.receiptBody)}>
                  <p {...stylex.props(styles.receiptParagraph)}>
                    {aiResponse
                      ? aiResponse.assistant_reply
                      : viewMode === 'simple'
                        ? `Para enlazar «${originTitle}» (${originBpm.toFixed(1)} BPM) con «${targetTitle}» (${targetBpm.toFixed(1)} BPM) sin saltos bruscos, preparé esta ruta armónica:`
                        : `Curva calculada con pendiente media de ${Number(avgStepDelta) >= 0 ? '+' : ''}${avgStepDelta} BPM por paso (${originCamelot} → ${activeSteps[0]?.camelot ?? originCamelot} → ${targetCamelot}):`}
                  </p>

                  <div {...stylex.props(styles.metricsRow)}>
                    <div {...stylex.props(styles.metricCell)}>
                      <div {...stylex.props(styles.metricVal)}>
                        {Number(avgStepDelta) >= 0 ? '+' : ''}
                        {avgStepDelta} BPM
                      </div>
                      <div {...stylex.props(styles.metricName)}>
                        Pendiente por paso
                      </div>
                    </div>
                    <div {...stylex.props(styles.metricCell)}>
                      <div {...stylex.props(styles.metricVal)}>Cue 4 → 1</div>
                      <div {...stylex.props(styles.metricName)}>
                        Cero choque vocal
                      </div>
                    </div>
                    <div {...stylex.props(styles.metricCell)}>
                      <div {...stylex.props(styles.metricVal)}>Lossless</div>
                      <div {...stylex.props(styles.metricName)}>
                        Quarantine-First
                      </div>
                    </div>
                  </div>

                  <div {...stylex.props(styles.humanInstructions)}>
                    {activeSteps.map((step, idx) => (
                      <div key={step.track_id}>
                        <strong>
                          {idx + 1}. {step.artist} — {step.title}:
                        </strong>{' '}
                        {step.transition_tip}
                      </div>
                    ))}
                    <div>
                      <strong>
                        {activeSteps.length + 1}. {targetArtist} — {targetTitle}
                        :
                      </strong>{' '}
                      cierre del puente en {targetBpm.toFixed(1)} BPM (
                      {targetCamelot}).
                    </div>
                  </div>
                </div>
              </div>

              {previewStatus && (
                <div {...stylex.props(styles.statusAlert)}>
                  <Icon name="checkCircle" size={14} color="#2dd4bf" />
                  <span>{previewStatus}</span>
                </div>
              )}
            </div>

            <div {...stylex.props(styles.receiptActions)}>
              <button
                type="button"
                disabled={Boolean(busyAction)}
                onClick={() => void handleApproveAndCreatePlaylist()}
                {...stylex.props(styles.primaryBtn)}
              >
                <Icon name="checkCircle" size={14} />
                <span>
                  {busyAction === 'approve_set'
                    ? 'Guardando Set...'
                    : viewMode === 'simple'
                      ? 'Agregar al Set'
                      : 'Aprobar y Crear Lista'}
                </span>
              </button>

              <button
                type="button"
                disabled={Boolean(busyAction)}
                onClick={() => void handleExportBridgeVirtualDj()}
                {...stylex.props(styles.secondaryBtn)}
              >
                <Icon name="export" size={14} />
                <span>
                  {busyAction === 'export_vdj'
                    ? 'Exportando...'
                    : 'Exportar a VirtualDJ'}
                </span>
              </button>

              <button
                type="button"
                onClick={() => {
                  const first = activeSteps[0];
                  if (!first) return;
                  const local = findMatchingLocalTrack(
                    first.artist,
                    first.title,
                    first.track_id,
                  );
                  void handleAuditionStep(
                    'bridge-main-preview',
                    `${first.artist} — ${first.title}`,
                    first.bpm,
                    first.camelot,
                    first.cue_in_seconds,
                    local,
                  );
                }}
                {...stylex.props(styles.secondaryBtn)}
              >
                <Icon
                  name={
                    activeAuditionId === 'bridge-main-preview'
                      ? 'pause'
                      : 'play'
                  }
                  size={12}
                />
                <span>
                  {activeAuditionId === 'bridge-main-preview'
                    ? 'Detener audio'
                    : 'Escuchar puente'}
                </span>
              </button>
            </div>

            <div {...stylex.props(styles.inputDock)}>
              <div {...stylex.props(styles.chipsList)}>
                <button
                  type="button"
                  disabled={isAiLoading}
                  onClick={() =>
                    void handleSendCopilotCommand(
                      'Propón una alternativa con más energía para hora pico',
                    )
                  }
                  {...stylex.props(styles.chip)}
                >
                  Subir energía
                </button>
                <button
                  type="button"
                  disabled={isAiLoading}
                  onClick={() =>
                    void handleSendCopilotCommand(
                      'Reduce la pendiente de BPM para una transición más larga',
                    )
                  }
                  {...stylex.props(styles.chip)}
                >
                  Transición más suave
                </button>
                <button
                  type="button"
                  disabled={isAiLoading}
                  onClick={() =>
                    void handleSendCopilotCommand(
                      'Prioriza pistas 100% instrumentales en la entrada',
                    )
                  }
                  {...stylex.props(styles.chip)}
                >
                  Solo entradas sin voz
                </button>
              </div>

              <div {...stylex.props(styles.inputRow)}>
                <input
                  type="text"
                  value={chatPrompt}
                  onChange={(e) => setChatPrompt(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') void handleSendCopilotCommand();
                  }}
                  placeholder="Indica qué ajuste quieres en el puente o en tu set..."
                  {...stylex.props(styles.chatInput)}
                />
                <button
                  type="button"
                  onClick={() => void handleSendCopilotCommand()}
                  disabled={isAiLoading || !chatPrompt.trim()}
                  {...stylex.props(styles.primaryBtn)}
                >
                  {isAiLoading ? 'Calculando...' : 'Enviar'}
                </button>
              </div>
            </div>
          </section>
        </div>

        {isOnboardingOpen && (
          <div
            {...stylex.props(styles.drawerOverlay)}
            role="dialog"
            aria-modal="true"
            aria-label="Configuración de Motor IA y Autoridad de Cabina"
          >
            <button
              type="button"
              aria-label="Cerrar panel de configuración"
              onClick={closeOnboarding}
              {...stylex.props(styles.drawerBackdropBtn)}
            />
            <aside {...stylex.props(styles.drawer)}>
              <div {...stylex.props(styles.drawerHeader)}>
                <div>
                  <h2 {...stylex.props(styles.drawerTitle)}>
                    Configuración de Motor IA y Autoridad de Cabina
                  </h2>
                  <p {...stylex.props(styles.drawerSub)}>
                    Selecciona cómo interviene el copiloto y qué motor local o
                    en la nube procesa tus consultas.
                  </p>
                </div>
                <button
                  type="button"
                  onClick={closeOnboarding}
                  {...stylex.props(styles.closeBtn)}
                >
                  Cerrar
                </button>
              </div>

              <div {...stylex.props(styles.drawerBody)}>
                <div {...stylex.props(styles.drawerSection)}>
                  <div {...stylex.props(styles.subHeadRow)}>
                    <strong>1. Nivel de autonomía en cabina</strong>
                    <span {...stylex.props(styles.freeMeta)}>
                      {commanderMode === 'executive_boss'
                        ? 'Aprobación manual'
                        : commanderMode === 'autonomous_operator'
                          ? 'Piloto automático'
                          : 'Solo lectura'}
                    </span>
                  </div>

                  <div {...stylex.props(styles.authGrid)}>
                    <button
                      type="button"
                      {...stylex.props(
                        styles.authCard,
                        commanderMode === 'executive_boss' &&
                          styles.authCardActive,
                      )}
                      onClick={() => setCommanderMode('executive_boss')}
                    >
                      <div {...stylex.props(styles.authTitle)}>
                        Control manual del DJ
                      </div>
                      <div {...stylex.props(styles.authDesc)}>
                        El copiloto sugiere puentes y ordenamientos; tú apruebas
                        con un clic antes de aplicar cambios.
                      </div>
                    </button>

                    <button
                      type="button"
                      {...stylex.props(
                        styles.authCard,
                        commanderMode === 'autonomous_operator' &&
                          styles.authCardActive,
                      )}
                      onClick={() => setCommanderMode('autonomous_operator')}
                    >
                      <div {...stylex.props(styles.authTitle)}>
                        Curaduría automática
                      </div>
                      <div {...stylex.props(styles.authDesc)}>
                        Ordena crates y prepara exportaciones automáticamente
                        respetando tus invariantes acústicos.
                      </div>
                    </button>

                    <button
                      type="button"
                      {...stylex.props(
                        styles.authCard,
                        commanderMode === 'advisor_only' &&
                          styles.authCardActive,
                      )}
                      onClick={() => setCommanderMode('advisor_only')}
                    >
                      <div {...stylex.props(styles.authTitle)}>
                        Modo consulta
                      </div>
                      <div {...stylex.props(styles.authDesc)}>
                        Muestra sugerencias de compatibilidad sin modificar
                        listas ni archivos.
                      </div>
                    </button>
                  </div>
                </div>

                <div {...stylex.props(styles.drawerSection)}>
                  <div {...stylex.props(styles.subHeadRow)}>
                    <strong>2. Motor de inferencia ($0.00 incluidos)</strong>
                    <span {...stylex.props(styles.freeMeta)}>
                      {isScanningOnboarding
                        ? 'Detectando motores...'
                        : probeBanner ||
                          (onboardingReport
                            ? `${onboardingReport.cli_agents.filter((c) => c.installed).length} CLI detectados`
                            : 'Listo')}
                    </span>
                  </div>

                  <div {...stylex.props(styles.modesContainer)}>
                    <div
                      {...stylex.props(
                        styles.modeRow,
                        selectedBackend === 'cloudflare_free' &&
                          styles.modeRowActive,
                      )}
                    >
                      <div {...stylex.props(styles.modeInfo)}>
                        <div {...stylex.props(styles.modeTitle)}>
                          Cloudflare Workers AI Free + DJ Brain Instantáneo
                        </div>
                        <div {...stylex.props(styles.modeDesc)}>
                          Respuesta inmediata ($0.00, 10k Neurons/día) sin
                          consumir RAM ni bloquear tu cabina.
                        </div>
                      </div>
                      <button
                        type="button"
                        onClick={() => setSelectedBackend('cloudflare_free')}
                        {...stylex.props(
                          selectedBackend === 'cloudflare_free'
                            ? styles.primaryBtn
                            : styles.secondaryBtn,
                        )}
                      >
                        {selectedBackend === 'cloudflare_free'
                          ? 'Activo'
                          : 'Seleccionar'}
                      </button>
                    </div>

                    <div
                      {...stylex.props(
                        styles.modeRow,
                        selectedBackend === 'ollama' && styles.modeRowActive,
                      )}
                    >
                      <div {...stylex.props(styles.modeInfo)}>
                        <div {...stylex.props(styles.modeTitle)}>
                          Cabina Offline (Ollama / LM Studio local)
                        </div>
                        <div {...stylex.props(styles.modeDesc)}>
                          100% sin conexión a internet para sets en vivo en
                          clubes o eventos remotos.
                        </div>
                      </div>
                      <button
                        type="button"
                        onClick={() => setSelectedBackend('ollama')}
                        {...stylex.props(
                          selectedBackend === 'ollama'
                            ? styles.primaryBtn
                            : styles.secondaryBtn,
                        )}
                      >
                        {selectedBackend === 'ollama'
                          ? 'Activo'
                          : 'Seleccionar'}
                      </button>
                    </div>

                    <div
                      {...stylex.props(
                        styles.modeRow,
                        selectedBackend.startsWith('cli_') &&
                          styles.modeRowActive,
                      )}
                    >
                      <div {...stylex.props(styles.modeInfo)}>
                        <div {...stylex.props(styles.modeTitle)}>
                          Agentes CLI locales en segundo plano (agy / claude)
                        </div>
                        <div {...stylex.props(styles.modeDesc)}>
                          Ejecuta consultas asíncronas con timeout de seguridad
                          usando tus herramientas CLI instaladas.
                        </div>
                      </div>
                      <button
                        type="button"
                        onClick={() => setSelectedBackend('cli_agy')}
                        {...stylex.props(
                          selectedBackend === 'cli_agy'
                            ? styles.primaryBtn
                            : styles.secondaryBtn,
                        )}
                      >
                        {selectedBackend === 'cli_agy'
                          ? 'Activo'
                          : 'Seleccionar'}
                      </button>
                    </div>

                    <div {...stylex.props(styles.proExpanderRow)}>
                      <span {...stylex.props(styles.dimText)}>
                        Servidor OpenAI-compatible personalizado o protocolo MCP
                      </span>
                      <button
                        type="button"
                        onClick={() => setShowProOptions(!showProOptions)}
                        {...stylex.props(styles.secondaryBtn)}
                      >
                        {showProOptions ? 'Ocultar' : 'Configurar'}
                      </button>
                    </div>

                    {showProOptions && (
                      <div {...stylex.props(styles.proBox)}>
                        <div {...stylex.props(styles.modeDesc)}>
                          Servidor MCP nativo disponible mediante{' '}
                          <code>hera-node --mcp</code>.
                        </div>
                        <div {...stylex.props(styles.inputRow)}>
                          <input
                            type="text"
                            value={customBaseUrl}
                            onChange={(e) => setCustomBaseUrl(e.target.value)}
                            placeholder="http://localhost:1234/v1"
                            {...stylex.props(styles.chatInput)}
                          />
                          <button
                            type="button"
                            onClick={() => {
                              void probeLocalAiBackend(customBaseUrl).then(
                                (probe) => {
                                  if (probe.reachable) {
                                    setSelectedBackend('custom');
                                  }
                                  setPreviewStatus(
                                    probe.reachable
                                      ? `Servidor local activo (${probe.base_url} · ${probe.models.length} modelo(s))`
                                      : `Sin servidor activo en ${customBaseUrl}`,
                                  );
                                },
                              );
                            }}
                            {...stylex.props(styles.secondaryBtn)}
                          >
                            Verificar
                          </button>
                        </div>
                      </div>
                    )}
                  </div>
                </div>

                <div {...stylex.props(styles.swarmLockBar)}>
                  <div {...stylex.props(styles.modeInfo)}>
                    <div {...stylex.props(styles.modeTitle)}>
                      Invariante colaborativo del enjambre HERA
                    </div>
                    <div {...stylex.props(styles.modeDesc)}>
                      El motor DSP, los 4 Hot Cues y los exportadores DJ son
                      100% libres manteniendo activa la red P2P comunitaria.
                    </div>
                  </div>
                  <span {...stylex.props(styles.swarmActiveTag)}>
                    Siempre activo
                  </span>
                </div>
              </div>
            </aside>
          </div>
        )}
      </div>
    </View>
  );
}

const styles = stylex.create({
  pageContainer: {
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    backgroundColor: '#0c0e12',
    color: '#edeff3',
    overflow: 'hidden',
  },
  topBar: {
    height: '46px',
    flexShrink: 0,
    backgroundColor: '#12161d',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    paddingBlock: 0,
    paddingInline: '18px',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
  },
  brandGroup: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    minWidth: 0,
  },
  pageTitle: {
    margin: 0,
    fontSize: '13.5px',
    fontWeight: 600,
    color: '#edeff3',
    whiteSpace: 'nowrap',
  },
  topActions: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    flexShrink: 0,
  },
  statusPill: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    paddingBlock: '5px',
    paddingInline: '10px',
    borderRadius: '6px',
    fontSize: '12px',
    fontWeight: 500,
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    cursor: 'pointer',
  },
  activePill: {
    borderColor: 'rgba(20, 184, 166, 0.32)',
    backgroundColor: 'rgba(20, 184, 166, 0.1)',
  },
  swarmPill: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    paddingBlock: '5px',
    paddingInline: '10px',
    borderRadius: '6px',
    fontSize: '12px',
    fontWeight: 500,
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#8e98a8',
  },
  emeraldDot: {
    width: '7px',
    height: '7px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
    flexShrink: 0,
  },
  splitWorkspace: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minHeight: 0,
    display: 'grid',
    gridTemplateColumns: '1.25fr 0.95fr',
    overflow: 'hidden',
  },
  leftCol: {
    padding: '18px',
    overflowY: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '14px',
    borderRightWidth: '1px',
    borderRightStyle: 'solid',
    borderRightColor: '#232a36',
    minWidth: 0,
  },
  rightCol: {
    padding: '18px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
    backgroundColor: '#10141b',
    minWidth: 0,
    minHeight: 0,
    overflow: 'hidden',
  },
  sectionHead: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '10px',
    flexWrap: 'wrap',
    rowGap: '4px',
  },
  sectionTitle: {
    margin: 0,
    fontSize: '13.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  matchTag: {
    display: 'inline-flex',
    alignItems: 'center',
    paddingBlock: '2px',
    paddingInline: '8px',
    borderRadius: '4px',
    fontSize: '11.5px',
    fontWeight: 600,
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.28)',
    color: '#2dd4bf',
    fontVariantNumeric: 'tabular-nums',
  },
  deckRack: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    padding: '14px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
  },
  decksRow: {
    display: 'grid',
    gridTemplateColumns: '1fr auto 1fr',
    alignItems: 'center',
    columnGap: '10px',
  },
  deckCard: {
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    padding: '12px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '7px',
    minWidth: 0,
  },
  deckTopLine: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '8px',
  },
  deckRole: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#8e98a8',
  },
  deckMetricBadge: {
    fontSize: '11px',
    fontWeight: 600,
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    paddingBlock: '2px',
    paddingInline: '7px',
    borderRadius: '4px',
    fontVariantNumeric: 'tabular-nums',
  },
  deckSelect: {
    width: '100%',
    paddingBlock: '4px',
    paddingInline: '8px',
    fontSize: '11.5px',
    backgroundColor: '#171c24',
    color: '#edeff3',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '5px',
  },
  deckSongTitle: {
    fontSize: '13.5px',
    fontWeight: 700,
    color: '#edeff3',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
  deckControlsInline: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '10px',
    flexWrap: 'wrap',
    rowGap: '6px',
  },
  inlineFieldLabel: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    fontSize: '11px',
    color: '#8e98a8',
  },
  compactNumInput: {
    width: '62px',
    paddingBlock: '3px',
    paddingInline: '6px',
    fontSize: '11.5px',
    fontVariantNumeric: 'tabular-nums',
    backgroundColor: '#171c24',
    color: '#edeff3',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '4px',
  },
  compactKeySelect: {
    paddingBlock: '3px',
    paddingInline: '6px',
    fontSize: '11.5px',
    fontVariantNumeric: 'tabular-nums',
    backgroundColor: '#171c24',
    color: '#edeff3',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '4px',
  },
  swapBtn: {
    width: '30px',
    height: '30px',
    borderRadius: '6px',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    fontSize: '14px',
    fontWeight: 700,
    cursor: 'pointer',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  stepsContainer: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
  },
  stepRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '10px',
    paddingBlock: '8px',
    paddingInline: '10px',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
  },
  stepIdx: {
    fontSize: '11px',
    fontWeight: 600,
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    paddingBlock: '2px',
    paddingInline: '7px',
    borderRadius: '4px',
    flexShrink: 0,
  },
  stepName: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minWidth: 0,
    fontSize: '12.5px',
    fontWeight: 600,
    color: '#edeff3',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  },
  dimText: {
    color: '#8e98a8',
    fontSize: '11.5px',
    fontWeight: 400,
  },
  bpmTag: {
    fontSize: '12px',
    fontWeight: 600,
    color: '#edeff3',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  previewBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    paddingBlock: '4px',
    paddingInline: '9px',
    borderRadius: '5px',
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
    flexShrink: 0,
  },
  previewBtnActive: {
    borderColor: '#14b8a6',
    color: '#2dd4bf',
    backgroundColor: 'rgba(20, 184, 166, 0.14)',
  },
  phraseRailPanel: {
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    paddingBlock: '10px',
    paddingInline: '12px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
  },
  phraseRailHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  phraseRailTitle: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  phraseTimelineTrack: {
    position: 'relative',
    height: '6px',
    width: '100%',
    backgroundColor: '#171c24',
    borderRadius: '999px',
  },
  phraseTimelineTick: {
    position: 'absolute',
    top: '-3px',
    width: '6px',
    height: '12px',
    borderRadius: '2px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#0c0e12',
    transform: 'translateX(-3px)',
  },
  cuePinsList: {
    display: 'flex',
    flexWrap: 'wrap',
    alignItems: 'center',
    justifyContent: 'space-between',
    rowGap: '6px',
    columnGap: '10px',
  },
  cuePin: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '6px',
    fontSize: '11.5px',
    color: '#edeff3',
  },
  cueDot: {
    width: '7px',
    height: '7px',
    borderRadius: '999px',
    flexShrink: 0,
  },
  cueTime: {
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  tableWrap: {
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    overflow: 'hidden',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    fontSize: '12.5px',
  },
  th: {
    textAlign: 'left',
    paddingBlock: '9px',
    paddingInline: '12px',
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#8e98a8',
    backgroundColor: '#171c24',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
  },
  td: {
    paddingBlock: '9px',
    paddingInline: '12px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: 'rgba(35, 42, 54, 0.6)',
    color: '#edeff3',
  },
  tabularCell: {
    fontVariantNumeric: 'tabular-nums',
    fontWeight: 600,
  },
  verifiedCell: {
    color: '#2dd4bf',
    fontSize: '12px',
    fontWeight: 500,
  },
  tableBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    paddingBlock: '4px',
    paddingInline: '9px',
    borderRadius: '5px',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    fontSize: '11.5px',
    fontWeight: 600,
    cursor: 'pointer',
  },
  freeMeta: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#2dd4bf',
  },
  copilotBody: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minHeight: 0,
    overflowY: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
  },
  bossMsg: {
    flexShrink: 0,
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    padding: '12px',
    fontSize: '12.5px',
    lineHeight: 1.5,
    color: '#edeff3',
  },
  bossHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '4px',
    fontSize: '11.5px',
    color: '#8e98a8',
  },
  receiptBox: {
    flexShrink: 0,
    backgroundColor: '#12161d',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '8px',
    padding: '14px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
  },
  receiptHead: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '8px',
  },
  receiptTitle: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    fontSize: '13px',
    fontWeight: 700,
    color: '#edeff3',
  },
  receiptBody: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '12px',
  },
  receiptParagraph: {
    margin: 0,
    fontSize: '12.5px',
    lineHeight: 1.5,
    color: '#8e98a8',
  },
  metricsRow: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    columnGap: '8px',
  },
  metricCell: {
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    padding: '9px',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
  },
  metricVal: {
    fontSize: '13px',
    fontWeight: 700,
    color: '#2dd4bf',
    fontVariantNumeric: 'tabular-nums',
  },
  metricName: {
    fontSize: '11px',
    color: '#8e98a8',
  },
  humanInstructions: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
    fontSize: '12px',
    lineHeight: 1.45,
    color: '#edeff3',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '6px',
    padding: '10px',
    fontVariantNumeric: 'tabular-nums',
  },
  receiptActions: {
    flexShrink: 0,
    display: 'flex',
    flexWrap: 'wrap',
    rowGap: '8px',
    columnGap: '8px',
    paddingTop: '10px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#232a36',
  },
  primaryBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    columnGap: '6px',
    paddingBlock: '7px',
    paddingInline: '13px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#14b8a6',
    backgroundColor: '#14b8a6',
    color: '#041311',
    fontSize: '12px',
    fontWeight: 700,
    cursor: 'pointer',
    flexShrink: 0,
  },
  secondaryBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    columnGap: '6px',
    paddingBlock: '7px',
    paddingInline: '11px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    backgroundColor: '#171c24',
    color: '#edeff3',
    fontSize: '12px',
    fontWeight: 600,
    cursor: 'pointer',
    flexShrink: 0,
  },
  statusAlert: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
    paddingBlock: '8px',
    paddingInline: '12px',
    borderRadius: '6px',
    backgroundColor: 'rgba(20, 184, 166, 0.1)',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.28)',
    color: '#2dd4bf',
    fontSize: '12px',
    fontWeight: 500,
  },
  inputDock: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
    paddingTop: '10px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#232a36',
    flexShrink: 0,
  },
  chipsList: {
    display: 'flex',
    flexWrap: 'wrap',
    rowGap: '6px',
    columnGap: '6px',
  },
  chip: {
    paddingBlock: '4px',
    paddingInline: '9px',
    borderRadius: '999px',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#8e98a8',
    fontSize: '11.5px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  inputRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
  },
  chatInput: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    minWidth: 0,
    paddingBlock: '8px',
    paddingInline: '11px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    fontSize: '12.5px',
  },
  drawerOverlay: {
    position: 'fixed',
    top: 0,
    right: 0,
    bottom: 0,
    left: 0,
    backgroundColor: 'rgba(6, 8, 12, 0.72)',
    display: 'flex',
    justifyContent: 'flex-end',
    zIndex: 1000,
  },
  drawerBackdropBtn: {
    flexGrow: 1,
    flexShrink: 1,
    flexBasis: '0%',
    borderWidth: 0,
    padding: 0,
    margin: 0,
    backgroundColor: 'transparent',
    cursor: 'default',
  },
  drawer: {
    width: '520px',
    maxWidth: '92vw',
    height: '100%',
    backgroundColor: '#12161d',
    borderLeftWidth: '1px',
    borderLeftStyle: 'solid',
    borderLeftColor: '#232a36',
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  },
  drawerHeader: {
    paddingBlock: '16px',
    paddingInline: '20px',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    display: 'flex',
    alignItems: 'flex-start',
    justifyContent: 'space-between',
    columnGap: '12px',
  },
  drawerTitle: {
    margin: 0,
    fontSize: '15px',
    fontWeight: 700,
    color: '#edeff3',
  },
  drawerSub: {
    marginTop: '4px',
    marginRight: 0,
    marginBottom: 0,
    marginLeft: 0,
    fontSize: '12px',
    color: '#8e98a8',
    lineHeight: 1.4,
  },
  closeBtn: {
    paddingBlock: '5px',
    paddingInline: '10px',
    borderRadius: '6px',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    fontSize: '12px',
    cursor: 'pointer',
  },
  drawerBody: {
    padding: '20px',
    overflowY: 'auto',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '18px',
  },
  drawerSection: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '10px',
  },
  subHeadRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    fontSize: '13px',
    color: '#edeff3',
  },
  authGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(3, 1fr)',
    columnGap: '8px',
  },
  authCard: {
    textAlign: 'left',
    padding: '11px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    color: '#edeff3',
    cursor: 'pointer',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '5px',
  },
  authCardActive: {
    backgroundColor: 'rgba(20, 184, 166, 0.12)',
    borderColor: 'rgba(20, 184, 166, 0.4)',
  },
  authTitle: {
    fontSize: '12px',
    fontWeight: 700,
    color: '#edeff3',
  },
  authDesc: {
    fontSize: '11.5px',
    lineHeight: 1.4,
    color: '#8e98a8',
  },
  modesContainer: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
  },
  modeRow: {
    padding: '12px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
  },
  modeRowActive: {
    borderColor: 'rgba(20, 184, 166, 0.36)',
  },
  modeInfo: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '3px',
    minWidth: 0,
  },
  modeTitle: {
    fontSize: '12.5px',
    fontWeight: 600,
    color: '#edeff3',
  },
  modeDesc: {
    fontSize: '11.5px',
    lineHeight: 1.4,
    color: '#8e98a8',
  },
  proExpanderRow: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '10px',
    paddingTop: '4px',
  },
  proBox: {
    padding: '12px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '8px',
  },
  swarmLockBar: {
    padding: '12px',
    borderRadius: '6px',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: 'rgba(20, 184, 166, 0.28)',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '12px',
  },
  swarmActiveTag: {
    fontSize: '11.5px',
    fontWeight: 600,
    color: '#2dd4bf',
    flexShrink: 0,
  },
});
