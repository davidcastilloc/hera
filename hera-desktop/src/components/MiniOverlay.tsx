import * as stylex from '@stylexjs/stylex';
import { useNavigate } from '@tanstack/react-router';
import {
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import LibraryAPI from '../api/LibraryAPI';
import type { Track } from '../generated/typings';
import useInvalidate from '../hooks/useInvalidate';
import usePlayingTrack from '../hooks/usePlayingTrack';
import {
  downloadP2PTrackToLibrary,
  type HarmonicBridgeStep,
  pushVirtualDjLiveCrate,
  readVirtualDjNowPlaying,
  resolveHarmonicBridge,
} from '../lib/bridge-hera';
import { useMiniModeStore } from '../lib/mini-mode-store';
import player from '../lib/player';
import useLibraryStore from '../lib/store';

const CUE_COLORS = ['#22c55e', '#38bdf8', '#f43f5e', '#f59e0b'] as const;

const cueStyles = stylex.create({
  dots: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '4px',
    flexShrink: 0,
  },
  dot: {
    width: '6px',
    height: '6px',
    borderRadius: '999px',
  },
});

type MiniSuggestionItem = {
  id: string;
  artist: string;
  title: string;
  bpm: number;
  camelot: string;
  cueSeconds: number;
  heraSeal: string;
  localTrack?: Track;
};

// Hoisted static SVG primitives (strokeWidth 1.5, zero Unicode symbol stand-ins)
const GRIP_DOTS_SVG = (
  <svg
    width="10"
    height="14"
    viewBox="0 0 10 14"
    fill="currentColor"
    aria-hidden="true"
  >
    <circle cx="2.5" cy="2.5" r="1.1" />
    <circle cx="7.5" cy="2.5" r="1.1" />
    <circle cx="2.5" cy="7" r="1.1" />
    <circle cx="7.5" cy="7" r="1.1" />
    <circle cx="2.5" cy="11.5" r="1.1" />
    <circle cx="7.5" cy="11.5" r="1.1" />
  </svg>
);

const ARROW_RIGHT_SVG = (
  <svg
    width="14"
    height="14"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M5 12h14M13 6l6 6-6 6" />
  </svg>
);

const PLAY_SVG = (
  <svg
    width="11"
    height="11"
    viewBox="0 0 24 24"
    fill="currentColor"
    aria-hidden="true"
  >
    <path d="M7 5.5v13l11-6.5-11-6.5z" />
  </svg>
);

const PAUSE_SVG = (
  <svg
    width="11"
    height="11"
    viewBox="0 0 24 24"
    fill="currentColor"
    aria-hidden="true"
  >
    <rect x="6" y="5" width="4" height="14" rx="1" />
    <rect x="14" y="5" width="4" height="14" rx="1" />
  </svg>
);

const CHECK_MINI_SVG = (
  <svg
    width="11"
    height="11"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M5 12.5l4.5 4.5L19 7.5" />
  </svg>
);

const PIN_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M15 4.5l4.5 4.5-4 1.5-3 3-1 4-6.5-6.5 4-1 3-3 1.5-4z" />
    <path d="M9 15l-4.5 4.5" />
  </svg>
);

const CHEVRON_DOWN_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M6 9l6 6 6-6" />
  </svg>
);

const CHEVRON_UP_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M18 15l-6-6-6 6" />
  </svg>
);

const EXPAND_WINDOW_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M14 4h6v6M10 20H4v-6M20 4l-7 7M4 20l7-7" />
  </svg>
);

const SPARKLE_MINI_SVG = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9L12 3z" />
  </svg>
);

const SWAP_ARROWS_SVG = (
  <svg
    width="14"
    height="14"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M17 3l4 4-4 4" />
    <path d="M3 7h18" />
    <path d="M7 21l-4-4 4-4" />
    <path d="M21 17H3" />
  </svg>
);

const FOUR_CUE_DOTS = (
  <span
    {...stylex.props(cueStyles.dots)}
    title="Entrada · Calma · Clímax · Salida"
  >
    {CUE_COLORS.map((color) => (
      <span
        key={color}
        {...stylex.props(cueStyles.dot)}
        style={{ backgroundColor: color }}
      />
    ))}
  </span>
);

function parseCamelot(raw: string): { num: number; mode: 'A' | 'B' } {
  const clean = raw.trim().toUpperCase();
  const mode = clean.endsWith('B') ? 'B' : 'A';
  const parsed = Number.parseInt(clean.slice(0, -1), 10);
  const num =
    Number.isFinite(parsed) && parsed >= 1 && parsed <= 12 ? parsed : 8;
  return { num, mode };
}

function camelotDistance(a: string, b: string): number {
  const ca = parseCamelot(a);
  const cb = parseCamelot(b);
  const ring = Math.min(
    Math.abs(ca.num - cb.num),
    12 - Math.abs(ca.num - cb.num),
  );
  return ring + (ca.mode === cb.mode ? 0 : 1);
}

function nextHarmonicCamelot(raw: string, stepOffset = 1): string {
  const { num, mode } = parseCamelot(raw);
  const wrapped = ((num - 1 + stepOffset + 12) % 12) + 1;
  return `${wrapped}${mode}`;
}

function camelotRootHz(camelot: string): { root: number; major: boolean } {
  const clean = camelot.trim().toUpperCase();
  const major = clean.endsWith('B');
  const mapMinor: Record<string, number> = {
    '1A': 207.65,
    '2A': 155.56,
    '3A': 233.08,
    '4A': 174.61,
    '5A': 130.81,
    '6A': 196.0,
    '7A': 146.83,
    '8A': 220.0,
    '9A': 164.81,
    '10A': 246.94,
    '11A': 185.0,
    '12A': 138.59,
  };
  const key = `${clean.slice(0, -1)}A`;
  const base = mapMinor[key] ?? 220.0;
  return { root: major ? base * 1.1892 : base, major };
}

function buildSyntheticTrack(
  item: MiniSuggestionItem,
  savedPath: string,
): Track {
  return {
    id: item.id,
    path: savedPath,
    title: item.title,
    album: 'HERA En Vivo',
    artists: [item.artist],
    album_artist: item.artist,
    genres: ['Club'],
    year: 2026,
    duration: 210,
    track_no: null,
    track_of: null,
    disk_no: null,
    disk_of: null,
    bpm: item.bpm,
    camelot: item.camelot,
    hot_cues: [],
    is_compilation: false,
  };
}

type SuggestionRowProps = {
  index: number;
  item: MiniSuggestionItem;
  isSent: boolean;
  isBusy: boolean;
  isPlaying: boolean;
  onPushVdj: (item: MiniSuggestionItem) => void;
  onToggleAudition: (item: MiniSuggestionItem) => void;
};

function MiniSuggestionRow({
  index,
  item,
  isSent,
  isBusy,
  isPlaying,
  onPushVdj,
  onToggleAudition,
}: SuggestionRowProps) {
  return (
    <div {...stylex.props(styles.suggestionCard)}>
      <span {...stylex.props(styles.rowIndex)}>
        {String(index + 1).padStart(2, '0')}
      </span>

      <div {...stylex.props(styles.rowInfo)}>
        <div {...stylex.props(styles.rowTitle)}>
          {item.artist} — {item.title}
        </div>
        <div {...stylex.props(styles.rowSub)}>
          <span {...stylex.props(styles.rowMetaTabular)}>
            {Math.round(item.bpm)} · {item.camelot}
          </span>
          <button
            type="button"
            disabled={isBusy}
            onClick={() => onPushVdj(item)}
            {...stylex.props(
              styles.vdjInlineBtn,
              isSent && styles.vdjInlineBtnSent,
            )}
          >
            {isSent ? CHECK_MINI_SVG : null}
            <span>{isBusy ? '...' : isSent ? 'En VDJ' : '+ VDJ'}</span>
          </button>
        </div>
      </div>

      {FOUR_CUE_DOTS}

      <button
        type="button"
        aria-label={isPlaying ? 'Detener escucha' : 'Escuchar sugerencia'}
        onClick={() => onToggleAudition(item)}
        {...stylex.props(
          styles.squarePlayBtn,
          isPlaying && styles.squarePlayBtnActive,
        )}
      >
        {isPlaying ? PAUSE_SVG : PLAY_SVG}
      </button>
    </div>
  );
}

export default function MiniOverlay() {
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

  // Puente tab endpoints
  const [bridgeOriginId, setBridgeOriginId] = useState<string>('');
  const [bridgeTargetId, setBridgeTargetId] = useState<string>('');
  const [bridgeSwapped, setBridgeSwapped] = useState(false);
  const [bridgeSteps, setBridgeSteps] = useState<Array<HarmonicBridgeStep>>([]);

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

  // Lightweight VirtualDJ History/tracklist.txt poll while Mini Overlay is open
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

  // Derive current track cleanly during render (zero effect-driven state cascade)
  const currentTrackInfo = useMemo(() => {
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
        artist:
          playingTrack.artists[0] || playingTrack.album_artist || 'Artista',
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
  }, [playingTrack, selectedCurrentId, tracks, vdjLiveTrack]);

  // Derive top 3 harmonic suggestions (local library + swarm bridge fallback)
  const suggestions: Array<MiniSuggestionItem> = useMemo(() => {
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
      if (
        !result.some((r) => r.title.toLowerCase() === fb.title.toLowerCase())
      ) {
        result.push(fb);
      }
    }

    return result.length > 0 ? result : fallbackPool.slice(0, 3);
  }, [currentTrackInfo, energyDeltaBpm, filterText, tracks]);

  // Derive Puente endpoints
  const bridgeEndpoints = useMemo(() => {
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
  }, [bridgeOriginId, bridgeSwapped, bridgeTargetId, tracks]);

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

      try {
        const AudioContextCtor =
          window.AudioContext ||
          (window as unknown as { webkitAudioContext: typeof AudioContext })
            .webkitAudioContext;
        const ctx = new AudioContextCtor();
        audioCtxRef.current = ctx;
        const durationSecs = 4.5;
        const now = ctx.currentTime + 0.03;
        const beatSec = 60 / Math.max(70, Math.min(180, item.bpm));
        const { root, major } = camelotRootHz(item.camelot);
        const thirdRatio = major ? 1.259921 : 1.1892071;

        const master = ctx.createGain();
        master.gain.setValueAtTime(0.2, now);
        master.gain.exponentialRampToValueAtTime(0.001, now + durationSecs);
        master.connect(ctx.destination);

        for (const freq of [root, root * thirdRatio, root * 1.4983071]) {
          const osc = ctx.createOscillator();
          const g = ctx.createGain();
          osc.type = 'triangle';
          osc.frequency.setValueAtTime(freq, now);
          g.gain.setValueAtTime(0.001, now);
          g.gain.linearRampToValueAtTime(0.22, now + 0.3);
          g.gain.exponentialRampToValueAtTime(0.001, now + durationSecs);
          osc.connect(g);
          g.connect(master);
          osc.start(now);
          osc.stop(now + durationSecs);
        }

        const beats = Math.floor(durationSecs / beatSec);
        for (let b = 0; b < beats; b += 1) {
          const bt = now + b * beatSec;
          const kick = ctx.createOscillator();
          const kg = ctx.createGain();
          kick.type = 'sine';
          kick.frequency.setValueAtTime(132, bt);
          kick.frequency.exponentialRampToValueAtTime(48, bt + 0.1);
          kg.gain.setValueAtTime(0.7, bt);
          kg.gain.exponentialRampToValueAtTime(0.001, bt + 0.2);
          kick.connect(kg);
          kg.connect(master);
          kick.start(bt);
          kick.stop(bt + 0.22);
        }

        setAuditionId(item.id);
        previewTimerRef.current = setTimeout(() => {
          stopPreview();
        }, durationSecs * 1000);
      } catch {
        setAuditionId(null);
      }
    },
    [auditionId, stopPreview],
  );

  const handlePushSingleTrackToVdj = useCallback(
    async (item: MiniSuggestionItem) => {
      if (busyId) return;
      setBusyId(item.id);
      try {
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
          trackToPush = buildSyntheticTrack(item, dl.saved_path);
        }

        await pushVirtualDjLiveCrate([trackToPush], true, 'HERA En Vivo');
        setSentIds((prev) => {
          const next = new Set(prev);
          next.add(item.id);
          return next;
        });
      } finally {
        setBusyId(null);
      }
    },
    [busyId, invalidate],
  );

  const handlePushBridgeToVdj = useCallback(async () => {
    if (busyId) return;
    setBusyId('bridge-push');
    try {
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
          return buildSyntheticTrack(
            {
              id: step.track_id,
              artist: step.artist,
              title: step.title,
              bpm: step.bpm,
              camelot: step.camelot,
              cueSeconds: step.cue_in_seconds,
              heraSeal: step.hera_seal,
            },
            dl.saved_path,
          );
        }),
      );

      await pushVirtualDjLiveCrate(resolvedTracks, false, 'HERA En Vivo');
      setBridgeSent(true);
      if (bridgeBadgeTimerRef.current)
        clearTimeout(bridgeBadgeTimerRef.current);
      bridgeBadgeTimerRef.current = setTimeout(() => {
        setBridgeSent(false);
      }, 2500);
    } finally {
      setBusyId(null);
    }
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

  const topSuggestion = suggestions[0];

  // 1. Collapsed HUD Pill State (560 × 52 px)
  if (layoutMode === 'pill') {
    const isTopSent = topSuggestion ? sentIds.has(topSuggestion.id) : false;
    const isTopPlaying = topSuggestion
      ? auditionId === topSuggestion.id
      : false;

    return (
      <section
        aria-label="Barra Mini HERA"
        {...stylex.props(styles.pillRoot)}
        data-tauri-drag-region
      >
        <div {...stylex.props(styles.pillBrand)} data-tauri-drag-region>
          <span {...stylex.props(styles.gripHandle)} data-tauri-drag-region>
            {GRIP_DOTS_SVG}
          </span>
          <span {...stylex.props(styles.emeraldDot)} data-tauri-drag-region />
          <span {...stylex.props(styles.brandText)} data-tauri-drag-region>
            HERA
          </span>
        </div>

        <span {...stylex.props(styles.pillDivider)} data-tauri-drag-region />

        <div {...stylex.props(styles.pillNowPlaying)} data-tauri-drag-region>
          <span {...stylex.props(styles.pillMuted)} data-tauri-drag-region>
            Sonando
          </span>
          <span {...stylex.props(styles.pillTrackName)} data-tauri-drag-region>
            {currentTrackInfo.title}
          </span>
          <span {...stylex.props(styles.pillTabular)} data-tauri-drag-region>
            {currentTrackInfo.bpm}
          </span>
        </div>

        <span {...stylex.props(styles.pillArrow)} data-tauri-drag-region>
          {ARROW_RIGHT_SVG}
        </span>

        {topSuggestion ? (
          <div {...stylex.props(styles.pillNextBlock)} data-tauri-drag-region>
            <div {...stylex.props(styles.pillNextTop)} data-tauri-drag-region>
              <span
                {...stylex.props(styles.pillNextTitle)}
                data-tauri-drag-region
              >
                {topSuggestion.title}
              </span>
              <span {...stylex.props(styles.keyBadge)} data-tauri-drag-region>
                {topSuggestion.camelot}
              </span>
            </div>
            <div {...stylex.props(styles.pillNextSub)} data-tauri-drag-region>
              <span {...stylex.props(styles.pillTabular)}>
                {Math.round(topSuggestion.bpm)} BPM
              </span>
              {FOUR_CUE_DOTS}
            </div>
          </div>
        ) : null}

        <div {...stylex.props(styles.pillActions)}>
          {topSuggestion ? (
            <>
              <button
                type="button"
                aria-label={
                  isTopPlaying ? 'Detener escucha' : 'Escuchar sugerencia'
                }
                onClick={() => void handleToggleAudition(topSuggestion)}
                {...stylex.props(
                  styles.squarePlayBtn,
                  isTopPlaying && styles.squarePlayBtnActive,
                )}
              >
                {isTopPlaying ? PAUSE_SVG : PLAY_SVG}
              </button>

              <button
                type="button"
                disabled={Boolean(busyId)}
                onClick={() => void handlePushSingleTrackToVdj(topSuggestion)}
                {...stylex.props(
                  styles.pillVdjBtn,
                  isTopSent && styles.pillVdjBtnSent,
                )}
              >
                {isTopSent ? CHECK_MINI_SVG : null}
                <span>{isTopSent ? 'En VDJ' : '+ VDJ'}</span>
              </button>
            </>
          ) : null}

          <button
            type="button"
            aria-label="Desplegar tarjeta"
            title="Desplegar tarjeta"
            onClick={() => void expandToCard()}
            {...stylex.props(styles.headerIconBtn)}
          >
            {CHEVRON_UP_SVG}
          </button>

          <button
            type="button"
            aria-label="Volver a ventana completa"
            title="Volver a HERA completo"
            onClick={() => void restoreFullWindow()}
            {...stylex.props(styles.headerIconBtn)}
          >
            {EXPAND_WINDOW_SVG}
          </button>
        </div>
      </section>
    );
  }

  // 2. Expanded Booth Card State (360 × 408 px)
  return (
    <section aria-label="Modo Mini HERA" {...stylex.props(styles.cardRoot)}>
      {/* Draggable Mini Header */}
      <header {...stylex.props(styles.cardHeader)} data-tauri-drag-region>
        <div {...stylex.props(styles.cardBrandGroup)} data-tauri-drag-region>
          <span {...stylex.props(styles.brandText)} data-tauri-drag-region>
            HERA
          </span>
          <span
            {...stylex.props(styles.emeraldDotSmall)}
            data-tauri-drag-region
          />
        </div>

        <div
          role="tablist"
          aria-label="Modo de sugerencia"
          {...stylex.props(styles.modeSwitch)}
        >
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'siguientes'}
            onClick={() => setActiveTab('siguientes')}
            {...stylex.props(
              styles.modeSwitchBtn,
              activeTab === 'siguientes' && styles.modeSwitchBtnActive,
            )}
          >
            Siguientes
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'puente'}
            onClick={() => setActiveTab('puente')}
            {...stylex.props(
              styles.modeSwitchBtn,
              activeTab === 'puente' && styles.modeSwitchBtnActive,
            )}
          >
            Puente
          </button>
        </div>

        <div {...stylex.props(styles.windowControls)}>
          <button
            type="button"
            aria-label="Abrir en Agente"
            title="Abrir en Agente"
            onClick={() => void handleOpenAgentFull()}
            {...stylex.props(styles.headerIconBtn)}
          >
            {SPARKLE_MINI_SVG}
          </button>

          <button
            type="button"
            aria-label="Fijar encima"
            aria-pressed={pinned}
            title={pinned ? 'Siempre visible activo' : 'Fijar siempre visible'}
            onClick={() => void togglePinned()}
            {...stylex.props(
              styles.headerIconBtn,
              pinned && styles.headerIconBtnPinned,
            )}
          >
            {PIN_SVG}
          </button>

          <button
            type="button"
            aria-label="Plegar a barra"
            title="Plegar a barra compacta"
            onClick={() => void collapseToPill()}
            {...stylex.props(styles.headerIconBtn)}
          >
            {CHEVRON_DOWN_SVG}
          </button>

          <button
            type="button"
            aria-label="Volver a ventana completa"
            title="Volver a HERA completo"
            onClick={() => void restoreFullWindow()}
            {...stylex.props(styles.headerIconBtn)}
          >
            {EXPAND_WINDOW_SVG}
          </button>
        </div>
      </header>

      {activeTab === 'siguientes' ? (
        <div {...stylex.props(styles.cardBody)}>
          {/* ACTUAL Strip */}
          <div {...stylex.props(styles.actualStrip)}>
            <span {...stylex.props(styles.actualTag)}>ACTUAL</span>
            <span {...stylex.props(styles.actualDot)}>·</span>
            <span {...stylex.props(styles.actualTitle)}>
              {currentTrackInfo.artist} — {currentTrackInfo.title}
            </span>
            <span {...stylex.props(styles.actualMeta)}>
              {currentTrackInfo.bpm} · {currentTrackInfo.camelot}
            </span>

            {tracks.length > 0 ? (
              <select
                aria-label="Cambiar pista actual"
                value={selectedCurrentId}
                onChange={(e) => setSelectedCurrentId(e.target.value)}
                {...stylex.props(styles.invisibleSelectOverlay)}
              >
                <option value="">Automático (VirtualDJ / Reproductor)</option>
                {tracks.map((t) => (
                  <option key={t.id} value={t.id}>
                    {(t.artists[0] || 'Artista') + ' — ' + t.title}
                  </option>
                ))}
              </select>
            ) : null}
          </div>

          {/* 3 Harmonic Next Suggestions */}
          <div {...stylex.props(styles.suggestionsStack)}>
            {suggestions.map((item, idx) => (
              <MiniSuggestionRow
                key={item.id}
                index={idx}
                item={item}
                isSent={sentIds.has(item.id)}
                isBusy={busyId === item.id}
                isPlaying={auditionId === item.id}
                onPushVdj={(target) => void handlePushSingleTrackToVdj(target)}
                onToggleAudition={(target) => void handleToggleAudition(target)}
              />
            ))}
          </div>

          {/* Bottom Micro Command Bar */}
          <footer {...stylex.props(styles.cardFooter)}>
            <input
              type="text"
              value={filterText}
              onChange={(e) => {
                const next = e.target.value;
                startTransition(() => {
                  setFilterText(next);
                });
              }}
              placeholder="Ajustar..."
              aria-label="Ajustar sugerencias"
              {...stylex.props(styles.microInput)}
            />

            <div {...stylex.props(styles.presetGroup)}>
              <button
                type="button"
                onClick={() => handleApplyPreset('suave')}
                {...stylex.props(
                  styles.presetPill,
                  activePreset === 'suave' && styles.presetPillActive,
                )}
              >
                Suave
              </button>
              <button
                type="button"
                onClick={() => handleApplyPreset('energia')}
                {...stylex.props(
                  styles.presetPill,
                  activePreset === 'energia' && styles.presetPillActive,
                )}
              >
                + Energía
              </button>
              <button
                type="button"
                onClick={() => handleApplyPreset('sin_voz')}
                {...stylex.props(
                  styles.presetPill,
                  activePreset === 'sin_voz' && styles.presetPillActive,
                )}
              >
                Sin voz
              </button>
            </div>
          </footer>
        </div>
      ) : (
        <div {...stylex.props(styles.cardBody)}>
          {/* Compact Actual -> Destino Row */}
          <div {...stylex.props(styles.bridgeEndpointsRow)}>
            <div {...stylex.props(styles.endpointMiniBox)}>
              <span {...stylex.props(styles.endpointMiniLabel)}>Actual</span>
              <span {...stylex.props(styles.endpointMiniTitle)}>
                {bridgeEndpoints.origin.artist} — {bridgeEndpoints.origin.title}
              </span>
              <span {...stylex.props(styles.endpointMiniMeta)}>
                {bridgeEndpoints.origin.bpm} · {bridgeEndpoints.origin.camelot}
              </span>

              {tracks.length > 0 ? (
                <select
                  aria-label="Seleccionar origen"
                  value={bridgeOriginId}
                  onChange={(e) => setBridgeOriginId(e.target.value)}
                  {...stylex.props(styles.invisibleSelectOverlay)}
                >
                  {tracks.map((t) => (
                    <option key={t.id} value={t.id}>
                      {(t.artists[0] || 'Artista') + ' — ' + t.title}
                    </option>
                  ))}
                </select>
              ) : null}
            </div>

            <span {...stylex.props(styles.bridgeArrowEmerald)}>
              {ARROW_RIGHT_SVG}
            </span>

            <div {...stylex.props(styles.endpointMiniBox)}>
              <span {...stylex.props(styles.endpointMiniLabel)}>Destino</span>
              <span {...stylex.props(styles.endpointMiniTitle)}>
                {bridgeEndpoints.target.artist} — {bridgeEndpoints.target.title}
              </span>
              <span {...stylex.props(styles.endpointMiniMeta)}>
                {bridgeEndpoints.target.bpm} · {bridgeEndpoints.target.camelot}
              </span>

              {tracks.length > 0 ? (
                <select
                  aria-label="Seleccionar destino"
                  value={bridgeTargetId}
                  onChange={(e) => setBridgeTargetId(e.target.value)}
                  {...stylex.props(styles.invisibleSelectOverlay)}
                >
                  {tracks.map((t) => (
                    <option key={t.id} value={t.id}>
                      {(t.artists[0] || 'Artista') + ' — ' + t.title}
                    </option>
                  ))}
                </select>
              ) : null}
            </div>
          </div>

          {/* 2 Harmonic Bridge Steps */}
          <div {...stylex.props(styles.suggestionsStack)}>
            {bridgeSteps.map((step, idx) => {
              const stepItem: MiniSuggestionItem = {
                id: step.track_id,
                artist: step.artist,
                title: step.title,
                bpm: step.bpm,
                camelot: step.camelot,
                cueSeconds: step.cue_in_seconds,
                heraSeal: step.hera_seal,
              };
              const isPlaying = auditionId === stepItem.id;
              return (
                <div
                  key={step.track_id}
                  {...stylex.props(styles.suggestionCard)}
                >
                  <span {...stylex.props(styles.rowIndex)}>
                    {String(idx + 1).padStart(2, '0')}
                  </span>

                  <div {...stylex.props(styles.rowInfo)}>
                    <div {...stylex.props(styles.rowTitle)}>
                      {step.artist} — {step.title}
                    </div>
                    <div {...stylex.props(styles.rowMetaTabular)}>
                      {Math.round(step.bpm)} · {step.camelot}
                    </div>
                  </div>

                  {FOUR_CUE_DOTS}

                  <button
                    type="button"
                    aria-label={
                      isPlaying ? 'Detener escucha' : 'Escuchar paso del puente'
                    }
                    onClick={() => void handleToggleAudition(stepItem)}
                    {...stylex.props(
                      styles.squarePlayBtn,
                      isPlaying && styles.squarePlayBtnActive,
                    )}
                  >
                    {isPlaying ? PAUSE_SVG : PLAY_SVG}
                  </button>
                </div>
              );
            })}
          </div>

          {/* Primary Bridge Action Footer */}
          <footer {...stylex.props(styles.bridgeActionFooter)}>
            <button
              type="button"
              disabled={busyId === 'bridge-push'}
              onClick={() => void handlePushBridgeToVdj()}
              {...stylex.props(styles.primaryBridgeBtn)}
            >
              {bridgeSent ? CHECK_MINI_SVG : null}
              <span>
                {busyId === 'bridge-push'
                  ? 'Enviando...'
                  : bridgeSent
                    ? 'Puente en VirtualDJ'
                    : 'Mandar puente a VirtualDJ'}
              </span>
            </button>

            <button
              type="button"
              aria-label="Intercambiar pistas"
              title="Intercambiar actual y destino"
              onClick={() => setBridgeSwapped((v) => !v)}
              {...stylex.props(styles.swapSquareBtn)}
            >
              {SWAP_ARROWS_SVG}
            </button>
          </footer>
        </div>
      )}
    </section>
  );
}

const styles = stylex.create({
  // Collapsed HUD Pill (560 × 52 px)
  pillRoot: {
    boxSizing: 'border-box',
    width: '100%',
    height: '100%',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#26303d',
    borderRadius: '10px',
    paddingInline: '12px',
    display: 'flex',
    alignItems: 'center',
    columnGap: '10px',
    color: '#edeff3',
    overflow: 'hidden',
    userSelect: 'none',
  },
  pillBrand: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '7px',
    flexShrink: 0,
  },
  gripHandle: {
    display: 'inline-flex',
    alignItems: 'center',
    color: '#5c6675',
    cursor: 'grab',
  },
  emeraldDot: {
    width: '7px',
    height: '7px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
    flexShrink: 0,
  },
  emeraldDotSmall: {
    width: '6px',
    height: '6px',
    borderRadius: '999px',
    backgroundColor: '#14b8a6',
    flexShrink: 0,
  },
  brandText: {
    fontSize: '13px',
    fontWeight: 700,
    letterSpacing: '0.03em',
    color: '#edeff3',
  },
  pillDivider: {
    width: '1px',
    height: '20px',
    backgroundColor: '#232a36',
    flexShrink: 0,
  },
  pillNowPlaying: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    minWidth: 0,
    maxWidth: '148px',
    flexShrink: 1,
  },
  pillMuted: {
    fontSize: '11px',
    color: '#8e98a8',
    flexShrink: 0,
  },
  pillTrackName: {
    fontSize: '12px',
    fontWeight: 500,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    minWidth: 0,
  },
  pillTabular: {
    fontSize: '11px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  pillArrow: {
    display: 'inline-flex',
    alignItems: 'center',
    color: '#5c6675',
    flexShrink: 0,
  },
  pillNextBlock: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
    minWidth: 0,
    flexGrow: 1,
  },
  pillNextTop: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    minWidth: 0,
  },
  pillNextTitle: {
    fontSize: '12px',
    fontWeight: 600,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    minWidth: 0,
  },
  pillNextSub: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
  },
  keyBadge: {
    display: 'inline-flex',
    alignItems: 'center',
    paddingBlock: '1px',
    paddingInline: '5px',
    borderRadius: '4px',
    fontSize: '10px',
    fontWeight: 600,
    backgroundColor: 'rgba(20, 184, 166, 0.16)',
    color: '#2dd4bf',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  pillActions: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    flexShrink: 0,
  },
  pillVdjBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '4px',
    height: '28px',
    paddingInline: '10px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#14b8a6',
    backgroundColor: '#14b8a6',
    color: '#062320',
    fontSize: '11px',
    fontWeight: 600,
    cursor: 'pointer',
    whiteSpace: 'nowrap',
  },
  pillVdjBtnSent: {
    backgroundColor: 'rgba(20, 184, 166, 0.14)',
    color: '#2dd4bf',
  },

  // Expanded Booth Card (360 × 408 px)
  cardRoot: {
    boxSizing: 'border-box',
    width: '100%',
    height: '100%',
    backgroundColor: '#0c0e12',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#26303d',
    borderRadius: '10px',
    display: 'flex',
    flexDirection: 'column',
    color: '#edeff3',
    overflow: 'hidden',
    userSelect: 'none',
  },
  cardHeader: {
    height: '42px',
    flexShrink: 0,
    paddingInline: '12px',
    backgroundColor: '#10141b',
    borderBottomWidth: '1px',
    borderBottomStyle: 'solid',
    borderBottomColor: '#232a36',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    columnGap: '8px',
  },
  cardBrandGroup: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '5px',
    cursor: 'grab',
    flexShrink: 0,
  },
  modeSwitch: {
    display: 'inline-flex',
    alignItems: 'center',
    backgroundColor: '#171c24',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    borderRadius: '7px',
    padding: '2px',
    columnGap: '2px',
  },
  modeSwitchBtn: {
    borderWidth: 0,
    borderRadius: '5px',
    paddingBlock: '3px',
    paddingInline: '9px',
    fontSize: '11px',
    fontWeight: 500,
    backgroundColor: 'transparent',
    color: '#8e98a8',
    cursor: 'pointer',
  },
  modeSwitchBtnActive: {
    backgroundColor: 'rgba(20, 184, 166, 0.16)',
    color: '#2dd4bf',
  },
  windowControls: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '3px',
    flexShrink: 0,
  },
  headerIconBtn: {
    width: '24px',
    height: '24px',
    borderRadius: '5px',
    borderWidth: 0,
    backgroundColor: {
      default: 'transparent',
      ':hover': '#171c24',
    },
    color: {
      default: '#8e98a8',
      ':hover': '#edeff3',
    },
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
  },
  headerIconBtnPinned: {
    color: '#2dd4bf',
  },
  cardBody: {
    flexGrow: 1,
    minHeight: 0,
    padding: '10px',
    display: 'flex',
    flexDirection: 'column',
    justifyContent: 'space-between',
    rowGap: '8px',
  },
  actualStrip: {
    position: 'relative',
    height: '34px',
    flexShrink: 0,
    paddingInline: '10px',
    borderRadius: '7px',
    backgroundColor: '#141922',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    minWidth: 0,
  },
  actualTag: {
    fontSize: '10px',
    fontWeight: 600,
    letterSpacing: '0.04em',
    color: '#8e98a8',
    flexShrink: 0,
  },
  actualDot: {
    color: '#5c6675',
    flexShrink: 0,
  },
  actualTitle: {
    fontSize: '12px',
    fontWeight: 500,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    minWidth: 0,
    flexGrow: 1,
  },
  actualMeta: {
    fontSize: '11px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
  },
  invisibleSelectOverlay: {
    position: 'absolute',
    inset: 0,
    width: '100%',
    height: '100%',
    opacity: 0,
    cursor: 'pointer',
  },
  suggestionsStack: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '6px',
    flexGrow: 1,
    justifyContent: 'center',
    minHeight: 0,
  },
  suggestionCard: {
    paddingBlock: '8px',
    paddingInline: '10px',
    borderRadius: '8px',
    backgroundColor: '#131820',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#202733',
    display: 'flex',
    alignItems: 'center',
    columnGap: '9px',
    minWidth: 0,
  },
  rowIndex: {
    fontSize: '11px',
    color: '#6d7787',
    fontVariantNumeric: 'tabular-nums',
    flexShrink: 0,
    width: '16px',
  },
  rowInfo: {
    display: 'flex',
    flexDirection: 'column',
    rowGap: '4px',
    minWidth: 0,
    flexGrow: 1,
  },
  rowTitle: {
    fontSize: '12.5px',
    fontWeight: 500,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    minWidth: 0,
  },
  rowSub: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '8px',
  },
  rowMetaTabular: {
    fontSize: '11px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  vdjInlineBtn: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '4px',
    paddingBlock: '1px',
    paddingInline: '7px',
    borderRadius: '4px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: {
      default: '#2a3342',
      ':hover': '#14b8a6',
    },
    backgroundColor: '#1a202b',
    color: '#edeff3',
    fontSize: '10.5px',
    fontWeight: 500,
    cursor: 'pointer',
  },
  vdjInlineBtnSent: {
    borderColor: 'rgba(20, 184, 166, 0.45)',
    backgroundColor: 'rgba(20, 184, 166, 0.14)',
    color: '#2dd4bf',
  },
  squarePlayBtn: {
    width: '28px',
    height: '28px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: {
      default: '#26303d',
      ':hover': '#14b8a6',
    },
    backgroundColor: '#171c24',
    color: '#edeff3',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: 0,
    cursor: 'pointer',
  },
  squarePlayBtnActive: {
    borderColor: '#14b8a6',
    backgroundColor: 'rgba(20, 184, 166, 0.16)',
    color: '#2dd4bf',
  },
  cardFooter: {
    paddingTop: '6px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1f2632',
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    flexShrink: 0,
  },
  microInput: {
    flexGrow: 1,
    minWidth: '82px',
    height: '28px',
    paddingInline: '9px',
    borderRadius: '6px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: {
      default: '#26303d',
      ':focus': '#14b8a6',
    },
    backgroundColor: '#12161d',
    color: '#edeff3',
    fontSize: '11.5px',
    outline: 'none',
  },
  presetGroup: {
    display: 'inline-flex',
    alignItems: 'center',
    columnGap: '4px',
    flexShrink: 0,
  },
  presetPill: {
    height: '26px',
    paddingInline: '8px',
    borderRadius: '999px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#26303d',
    backgroundColor: '#141922',
    color: '#cbd2dc',
    fontSize: '10.5px',
    fontWeight: 500,
    cursor: 'pointer',
    whiteSpace: 'nowrap',
  },
  presetPillActive: {
    borderColor: 'rgba(20, 184, 166, 0.5)',
    backgroundColor: 'rgba(20, 184, 166, 0.15)',
    color: '#2dd4bf',
  },

  // Puente Quick Tab Styles
  bridgeEndpointsRow: {
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    flexShrink: 0,
  },
  endpointMiniBox: {
    position: 'relative',
    flexGrow: 1,
    flexBasis: '0%',
    minWidth: 0,
    paddingBlock: '7px',
    paddingInline: '9px',
    borderRadius: '7px',
    backgroundColor: '#141922',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#232a36',
    display: 'flex',
    flexDirection: 'column',
    rowGap: '2px',
  },
  endpointMiniLabel: {
    fontSize: '10px',
    color: '#8e98a8',
  },
  endpointMiniTitle: {
    fontSize: '11.5px',
    fontWeight: 500,
    color: '#edeff3',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  },
  endpointMiniMeta: {
    fontSize: '10.5px',
    color: '#8e98a8',
    fontVariantNumeric: 'tabular-nums',
  },
  bridgeArrowEmerald: {
    color: '#2dd4bf',
    display: 'inline-flex',
    alignItems: 'center',
    flexShrink: 0,
  },
  bridgeActionFooter: {
    paddingTop: '6px',
    borderTopWidth: '1px',
    borderTopStyle: 'solid',
    borderTopColor: '#1f2632',
    display: 'flex',
    alignItems: 'center',
    columnGap: '6px',
    flexShrink: 0,
  },
  primaryBridgeBtn: {
    flexGrow: 1,
    height: '32px',
    borderRadius: '7px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: '#14b8a6',
    backgroundColor: '#14b8a6',
    color: '#062320',
    fontSize: '12px',
    fontWeight: 600,
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    columnGap: '6px',
    cursor: 'pointer',
  },
  swapSquareBtn: {
    width: '32px',
    height: '32px',
    borderRadius: '7px',
    borderWidth: '1px',
    borderStyle: 'solid',
    borderColor: {
      default: '#26303d',
      ':hover': '#14b8a6',
    },
    backgroundColor: '#141922',
    color: {
      default: '#cbd2dc',
      ':hover': '#edeff3',
    },
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: 0,
    cursor: 'pointer',
  },
});
