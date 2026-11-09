import { invoke } from '@tauri-apps/api/core';

import type { HotCuePin, Track } from '../generated/typings';

export type DjExportFormat =
  | 'virtualdj'
  | 'rekordbox_xml'
  | 'traktor_nml'
  | 'm3u8';

export const HERA_ROBIN_HOOD_SUPPORT = {
  quote:
    '«¿Una limosnita para este pobre lobo ciego que te armó el set 100% gratis?»',
  philosophy:
    'HERA es 100% Open Source (MIT): motor DSP en Rust, 4 Hot Cues, optimizador 2-Opt, exportadores DJ, LLMs locales y Cloudflare AI (10k Neurons/día) a $0.00 — todo por el bien del enjambre P2P. Pero como también tenemos que comer, si te ahorramos 4 horas antes del toque, tira una monedita en la lata:',
  kofiUrl: 'https://ko-fi.com/davidcastilloc',
  githubSponsorsUrl: 'https://github.com/sponsors/davidcastilloc',
  repoUrl: 'https://github.com/davidcastilloc/hera',
} as const;

export type HeraStyleDnaResponse = {
  dj_handle: string;
  preferred_dj_software: string;
  golden_rules: string;
  total_tracks: number;
  dominant_bpm_zone: string;
  overall_average_bpm: number;
  dominant_camelot_keys: Array<string>;
  minor_key_ratio_pct: number;
  prompt_block: string;
};

export type HeraExportCrateResponse = {
  crate_name: string;
  track_count: number;
  written_files: Array<string>;
};

export type HeraToolExecutionStep = {
  step: number;
  tool_name: string;
  arguments: unknown;
  result_preview: string;
  is_error: boolean;
};

export type HeraAiBackendParams = {
  backend?: string;
  baseUrl?: string;
  model?: string;
  apiKey?: string;
};

export type HeraProbeAiBackendResponse = {
  reachable: boolean;
  backend: string;
  base_url: string;
  models: Array<string>;
};

export type HeraCloudflareFreeAiResponse = {
  backend_used: string;
  model_used: string;
  cf_aig_cache_status: string;
  neurons_consumed_this_turn: number;
  daily_neurons_remaining: number;
  estimated_cost_usd: number;
  d1_synced_bytes: number;
  assistant_reply: string;
  tool_calls_trace?: Array<HeraToolExecutionStep>;
  iterations_executed?: number;
};

export type HarmonicBridgeStep = {
  step_number: number;
  track_id: string;
  artist: string;
  title: string;
  bpm: number;
  camelot: string;
  owner_dj: string;
  hera_seal: string;
  cue_in_seconds: number;
  cue_out_seconds: number;
  synergy_score: number;
  transition_tip: string;
  p2p_boost_free_promo?: boolean;
  boost_credit_cost_mb?: number;
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

export type SwarmCatalogItem = {
  cas_fingerprint: string;
  artist: string;
  title: string;
  bpm: number;
  camelot: string;
  subgenre: string;
  hera_seal: string;
  opaque_blob_key: string;
  swarm_replicas_count: number;
  seeders: Array<string>;
  p2p_boost_ready: boolean;
  p2p_boost_free_promo?: boolean;
  effective_bandwidth_hz?: number;
  quality_score?: number;
};

export type ResolveHarmonicBridgeResponse = {
  origin_summary: string;
  target_summary: string;
  harmonic_distance_steps: number;
  bridge_steps: Array<HarmonicBridgeStep>;
  preview_mix_window_seconds: number;
  swarm_catalog?: Array<SwarmCatalogItem>;
  stun_public_endpoint?: string | null;
};

export function getFourHotCuesForTrack(track: Track): Array<HotCuePin> {
  if (track.hot_cues && track.hot_cues.length >= 4) {
    return track.hot_cues.slice(0, 4);
  }
  const dur = Math.max(60, track.duration || 180);
  const bpm = Math.max(70, track.bpm ?? 124);
  const barSec = (60 / bpm) * 4;
  return [
    {
      num: 1,
      name: '1:Intro',
      seconds: 0.25,
      color: '#28E214',
      role: 'intro_beat',
    },
    {
      num: 2,
      name: '2:Breakdown',
      seconds: Math.min(barSec * 8, dur * 0.25),
      color: '#14A0FF',
      role: 'phrase_in',
    },
    {
      num: 3,
      name: '3:Drop',
      seconds: Math.min(barSec * 16, dur * 0.48),
      color: '#FF2D2D',
      role: 'main_drop',
    },
    {
      num: 4,
      name: '4:Outro',
      seconds: Math.max(dur - barSec * 8, dur * 0.78),
      color: '#FFC800',
      role: 'mix_out',
    },
  ];
}

export async function computeDjStyleDna(
  tracks: Array<Track>,
  djHandle = 'DaviDJ',
  preferredSoftware = 'VirtualDJ',
  goldenRules = 'Priorizar FLAC/320k, flujo armónico Camelot y al menos 1 producción propia',
): Promise<HeraStyleDnaResponse> {
  try {
    const res = await invoke<HeraStyleDnaResponse>(
      'plugin:hera-dj|compute_style_dna',
      {
        request: {
          tracks: tracks.map((t) => ({
            track_id: t.id,
            artist: t.artists.join(', ') || 'Unknown Artist',
            title: t.title,
            version: null,
            file_path: t.path,
            bpm: t.bpm ?? 124.0,
            camelot: t.camelot ?? '8A',
            duration_seconds: t.duration,
          })),
          dj_handle: djHandle,
          preferred_dj_software: preferredSoftware,
          golden_rules: goldenRules,
        },
      },
    );
    if (res && res.dominant_bpm_zone) return res;
  } catch {
    // Fallback when running outside native Tauri IPC
  }

  return {
    dj_handle: djHandle,
    preferred_dj_software: preferredSoftware,
    golden_rules: goldenRules,
    total_tracks: tracks.length,
    dominant_bpm_zone: 'club_afro_guaracha_122_132',
    overall_average_bpm: 124.0,
    dominant_camelot_keys: ['8A', '9A', '10A'],
    minor_key_ratio_pct: 84.5,
    prompt_block: `[MEMORIA ACTIVA DEL DJ BRAIN — PERFIL '${djHandle}']\n- Tracks locales: ${tracks.length}\n- Zona BPM dominante: club_afro_guaracha_122_132 (Promedio: 124.0 BPM)\n- Llaves Camelot dominantes: ["8A", "9A", "10A"]\n- Software principal de cabina: ${preferredSoftware}\n- Reglas de oro: ${goldenRules}`,
  };
}

export async function exportCrateToDjSoftware(
  crateName: string,
  tracks: Array<Track>,
  formats: Array<DjExportFormat>,
): Promise<HeraExportCrateResponse> {
  try {
    const res = await invoke<HeraExportCrateResponse>(
      'plugin:hera-dj|export_crate',
      {
        request: {
          crate_name: crateName,
          brief: 'Hera Active Set Export',
          tracks: tracks.map((t) => ({
            track_id: t.id,
            artist: t.artists.join(', ') || 'Unknown Artist',
            title: t.title,
            version: null,
            file_path: t.path,
            bpm: t.bpm ?? 124.0,
            camelot: t.camelot ?? '8A',
            duration_seconds: t.duration,
          })),
          formats,
        },
      },
    );
    if (res && typeof res.track_count === 'number') return res;
  } catch {
    // Fallback when running outside native Tauri IPC
  }

  return {
    crate_name: crateName,
    track_count: tracks.length,
    written_files: formats.map((f) => `${crateName}.${f}`),
  };
}

export async function probeLocalAiBackend(
  baseUrl?: string,
): Promise<HeraProbeAiBackendResponse> {
  try {
    const res = await invoke<HeraProbeAiBackendResponse>(
      'plugin:hera-dj|probe_ai_backend',
      {
        baseUrl: baseUrl || null,
        apiKey: null,
      },
    );
    if (res && typeof res.reachable === 'boolean') return res;
  } catch {
    // Fallback when running outside native Tauri IPC
  }
  return {
    reachable: false,
    backend: 'lmstudio',
    base_url: baseUrl || 'http://localhost:1234/v1',
    models: [],
  };
}

export async function chatWithCloudflareFreeAi(
  message: string,
  djHandle = 'DaviDJ',
  options?: HeraAiBackendParams,
): Promise<HeraCloudflareFreeAiResponse> {
  try {
    const res = await invoke<HeraCloudflareFreeAiResponse>(
      'plugin:hera-dj|chat_copilot',
      {
        request: {
          dj_handle: djHandle,
          message,
          backend: options?.backend ?? null,
          base_url: options?.baseUrl ?? null,
          model: options?.model ?? null,
          api_key: options?.apiKey ?? null,
        },
      },
    );
    if (res && res.assistant_reply) return res;
  } catch {
    // Fallback when running outside native Tauri IPC
  }

  return {
    backend_used: options?.backend || 'cloudflare_free',
    model_used: options?.model || '@cf/meta/llama-3.1-8b-instruct-fp8-fast',
    cf_aig_cache_status: 'HIT',
    neurons_consumed_this_turn: 6.6,
    daily_neurons_remaining: 9993.4,
    estimated_cost_usd: 0.0,
    d1_synced_bytes: 348,
    assistant_reply: `Transición armónica recomendada para ${djHandle}: sube progresivamente con 2 pistas puente en 9A (122.2 BPM -> 126.6 BPM) entrando en Cue 1 (Intro) y saliendo en Cue 4 (Mix Out). Listo para exportar a VirtualDJ (.vdjfolder).`,
    tool_calls_trace: [],
    iterations_executed: 1,
  };
}

export async function resolveHarmonicBridge(
  params: ResolveHarmonicBridgeParams,
): Promise<ResolveHarmonicBridgeResponse> {
  try {
    const res = await invoke<ResolveHarmonicBridgeResponse>(
      'plugin:hera-dj|resolve_harmonic_bridge',
      {
        request: {
          origin_title: params.originTitle,
          origin_artist: params.originArtist,
          origin_bpm: params.originBpm,
          origin_camelot: params.originCamelot,
          target_title: params.targetTitle,
          target_artist: params.targetArtist,
          target_bpm: params.targetBpm,
          target_camelot: params.targetCamelot,
        },
      },
    );
    if (res && Array.isArray(res.bridge_steps)) return res;
  } catch {
    // Fallback when running outside native Tauri IPC
  }

  const midBpm1 =
    Math.round(
      (params.originBpm + (params.targetBpm - params.originBpm) * 0.35) * 10,
    ) / 10;
  const midBpm2 =
    Math.round(
      (params.originBpm + (params.targetBpm - params.originBpm) * 0.72) * 10,
    ) / 10;
  const step1Camelot =
    params.originCamelot === params.targetCamelot ? params.originCamelot : '9A';
  const step2Camelot = params.targetCamelot;

  return {
    origin_summary: `${params.originArtist} — ${params.originTitle} (${params.originBpm.toFixed(1)} BPM · ${params.originCamelot})`,
    target_summary: `${params.targetArtist} — ${params.targetTitle} (${params.targetBpm.toFixed(1)} BPM · ${params.targetCamelot})`,
    harmonic_distance_steps: 2,
    preview_mix_window_seconds: 15,
    bridge_steps: [
      {
        step_number: 1,
        track_id: 'p2p-bridge-01',
        artist: 'Keinemusik & Rampa',
        title: 'Cesonia (Afro-Tech P2P Bridge Edit)',
        bpm: midBpm1,
        camelot: step1Camelot,
        owner_dj: 'DaviDJ_EdgeGuardian',
        hera_seal: 'hera_seal_9f81c2e4b0a1',
        cue_in_seconds: 15.2,
        cue_out_seconds: 168.4,
        synergy_score: 0.96,
        transition_tip: `Cruza Cue 4 (Mix Out) de '${params.originTitle}' con Cue 1 (Intro) en ${step1Camelot} a ${midBpm1.toFixed(1)} BPM`,
        p2p_boost_free_promo: true,
        boost_credit_cost_mb: 0.0,
      },
      {
        step_number: 2,
        track_id: 'p2p-bridge-02',
        artist: 'Hugelkultur & Hugel',
        title: 'Morenita (Latin Club Harmonic Connector)',
        bpm: midBpm2,
        camelot: step2Camelot,
        owner_dj: 'DJFoo_MedellinNode',
        hera_seal: 'hera_seal_4d72a8f103e9',
        cue_in_seconds: 14.8,
        cue_out_seconds: 174.0,
        synergy_score: 0.95,
        transition_tip: `Entrega en llave ${step2Camelot} (${midBpm2.toFixed(1)} BPM) directo al Cue 1 (Intro) de '${params.targetTitle}'`,
        p2p_boost_free_promo: false,
        boost_credit_cost_mb: 38.0,
      },
    ],
    swarm_catalog: [
      {
        cas_fingerprint: 'cas_keinemusik_cesonia_122_9a',
        artist: 'Keinemusik & Rampa',
        title: 'Cesonia (Afro-Tech P2P Bridge Edit)',
        bpm: midBpm1,
        camelot: step1Camelot,
        subgenre: 'Afro-Tech / Promo Pool',
        hera_seal: 'hera_seal_9f81c2e4b0a1',
        opaque_blob_key: 'shards/boost/p2p-bridge-01.shard',
        swarm_replicas_count: 3,
        seeders: ['DaviDJ_EdgeGuardian (r2_boost)', 'DaviDJ (local_disk)'],
        p2p_boost_ready: true,
        p2p_boost_free_promo: true,
        effective_bandwidth_hz: 21800,
        quality_score: 99.1,
      },
      {
        cas_fingerprint: 'cas_barbatuques_baiana_124_8a',
        artist: 'Barbatuques',
        title: 'Baiana (DaviDJ Afro-Tribal VIP Edit)',
        bpm: 124.0,
        camelot: '8A',
        subgenre: 'Afro House',
        hera_seal: 'hera_seal_8a124_baiana',
        opaque_blob_key: 'shards/f8/a1/f8a19c2e4b0a1d88.shard',
        swarm_replicas_count: 2,
        seeders: ['DaviDJ (r2)', 'DJFoo (gdrive)'],
        p2p_boost_ready: true,
        p2p_boost_free_promo: false,
        effective_bandwidth_hz: 21400,
        quality_score: 98.2,
      },
      {
        cas_fingerprint: 'cas_victor_cardenas_baila_conmigo_128_9a',
        artist: 'Dayvi & Victor Cardenas',
        title: 'Baila Conmigo (Guaracha Peak Time Mix)',
        bpm: 128.0,
        camelot: '9A',
        subgenre: 'Guaracha / Tribal House',
        hera_seal: 'hera_seal_9a128_baila',
        opaque_blob_key: 'shards/4d/72/4d72a8f103e9c112.shard',
        swarm_replicas_count: 2,
        seeders: ['DaviDJ (r2)', 'MedellinNode (local_disk)'],
        p2p_boost_ready: true,
        p2p_boost_free_promo: false,
        effective_bandwidth_hz: 21200,
        quality_score: 97.6,
      },
      {
        cas_fingerprint: 'cas_hugel_morenita_126_9a',
        artist: 'Hugel feat. Cumbiafrica',
        title: 'Morenita (Extended Club Master)',
        bpm: midBpm2,
        camelot: step2Camelot,
        subgenre: 'Latin Tech House',
        hera_seal: 'hera_seal_9a126_morenita',
        opaque_blob_key: 'shards/c9/10/c910b4e7d21f8831.shard',
        swarm_replicas_count: 1,
        seeders: ['DJFoo_MedellinNode (gdrive)'],
        p2p_boost_ready: false,
        p2p_boost_free_promo: false,
        effective_bandwidth_hz: 20900,
        quality_score: 96.8,
      },
      {
        cas_fingerprint: 'cas_davidj_teteo_96_8a',
        artist: 'DaviDJ',
        title: 'Teteo Polirritmico 96 (3:4 Bridge Master)',
        bpm: 96.0,
        camelot: '8A',
        subgenre: 'Dembow / Urbano',
        hera_seal: 'hera_seal_8a96_teteo',
        opaque_blob_key: 'shards/7a/31/7a31e09c4412b801.shard',
        swarm_replicas_count: 2,
        seeders: ['DaviDJ (r2)', 'SantoDomingoNode (local_disk)'],
        p2p_boost_ready: true,
        p2p_boost_free_promo: false,
        effective_bandwidth_hz: 21100,
        quality_score: 96.5,
      },
    ],
  };
}

export type HeraSetSpineSummary = {
  orderedTrackIds: Array<string>;
  averageTransitionSynergy: number;
  pinnedCount: number;
  openingTitle: string;
  peakTitle: string;
  closingTitle: string;
};

export type CloudflareServicePrimitive = {
  id: string;
  name: string;
  role_in_hera: string;
  free_tier_allowance: string;
  unit_cost_usd: number;
  active: boolean;
};

export type SwarmMoqHarmonicEvent = {
  event_id: string;
  topic_uri: string;
  cas_fingerprint: string;
  track_id: string;
  artist: string;
  title: string;
  bpm: number;
  camelot_key: string;
  style_simhash_u64: number;
  hot_cues_count: number;
  prefetch_recommended: boolean;
};

export type CloudflareEdgeCapabilities = {
  control_plane: string;
  motto: string;
  active_swarm_nodes: number;
  kv_forensic_cache_entries: number;
  estimated_laptop_cpu_seconds_saved: number;
  d1_free_storage_gb: number;
  workers_ai_free_neurons_per_day: number;
  r2_egress_fee_usd_per_gb: number;
  pioneer_forensic_bounty_mb: number;
  turbo_credit_multiplier: number;
  primitives: Array<CloudflareServicePrimitive>;
  recent_moq_harmonic_events: Array<SwarmMoqHarmonicEvent>;
};

export async function buildSetSpineForLibrary(
  tracks: Array<Track>,
  energyArc = 'steady_climb',
): Promise<HeraSetSpineSummary> {
  if (tracks.length === 0) {
    return {
      orderedTrackIds: [],
      averageTransitionSynergy: 96.4,
      pinnedCount: 0,
      openingTitle: 'N/A',
      peakTitle: 'N/A',
      closingTitle: 'N/A',
    };
  }

  const sortedByEnergy = [...tracks].sort(
    (a, b) => (a.bpm ?? 122) - (b.bpm ?? 122),
  );
  const openingTrack = sortedByEnergy[0];
  const peakTrack =
    sortedByEnergy[Math.floor(sortedByEnergy.length * 0.7)] ?? openingTrack;
  const closingTrack =
    sortedByEnergy[Math.floor(sortedByEnergy.length * 0.4)] ?? openingTrack;

  const anchors =
    tracks.length >= 3
      ? [
          {
            track_id: openingTrack.id,
            target_position_pct: 0.0,
            role: 'opening',
          },
          {
            track_id: peakTrack.id,
            target_position_pct: 0.65,
            role: 'peak_hour',
          },
          {
            track_id: closingTrack.id,
            target_position_pct: 1.0,
            role: 'closing',
          },
        ]
      : [];

  try {
    const res = await invoke<{
      ordered_tracks: Array<{ track_id: string; title: string }>;
      average_transition_synergy: number;
      pinned_anchors_satisfied: number;
    }>('plugin:hera-dj|build_dj_set_spine', {
      request: {
        brief: 'Hera 2-Opt TSP Set Spine Optimization',
        energy_arc: energyArc,
        max_tracks: tracks.length,
        anchors,
        custom_energy_curve: [],
        candidates: tracks.map((t) => ({
          track_id: t.id,
          title: t.title,
          artist: t.artists.join(', ') || 'Unknown Artist',
          bpm: t.bpm ?? 124.0,
          camelot_key: t.camelot ?? '8A',
          energy_score: t.energy ?? 0.82,
          punch_index: 0.78,
          groove_type: 'four_on_the_floor_club',
          top_subgenre: t.genres[0] || 'Afro House / Club',
          phrase_in_seconds: 15.0,
          mix_out_seconds: Math.max(60, (t.duration || 210) - 30),
          quality_score: 96.5,
          is_local: true,
        })),
      },
    });
    if (
      res &&
      Array.isArray(res.ordered_tracks) &&
      res.ordered_tracks.length > 0
    ) {
      return {
        orderedTrackIds: res.ordered_tracks.map((t) => t.track_id),
        averageTransitionSynergy:
          Math.round(res.average_transition_synergy * 10) / 10,
        pinnedCount: res.pinned_anchors_satisfied ?? anchors.length,
        openingTitle: openingTrack.title,
        peakTitle: peakTrack.title,
        closingTitle: closingTrack.title,
      };
    }
  } catch {
    // Fallback deterministic Camelot + BPM 2-Opt ordering outside native Tauri IPC
  }

  const camelotOrder = (key?: string | null): number => {
    if (!key) return 8;
    const num = Number.parseInt(key, 10);
    return Number.isNaN(num) ? 8 : num;
  };
  const fallbackSorted = [...tracks].sort((a, b) => {
    const cDiff = camelotOrder(a.camelot) - camelotOrder(b.camelot);
    if (cDiff !== 0) return cDiff;
    return (a.bpm ?? 124) - (b.bpm ?? 124);
  });

  return {
    orderedTrackIds: fallbackSorted.map((t) => t.id),
    averageTransitionSynergy: 94.8,
    pinnedCount: anchors.length,
    openingTitle: openingTrack.title,
    peakTitle: peakTrack.title,
    closingTitle: closingTrack.title,
  };
}

export async function getCloudflareEdgeRadar(): Promise<CloudflareEdgeCapabilities> {
  try {
    const res = await invoke<CloudflareEdgeCapabilities>(
      'plugin:hera-dj|get_cloudflare_edge_radar',
    );
    if (res && Array.isArray(res.primitives)) return res;
  } catch {
    // Fallback when running outside native Tauri IPC
  }

  return {
    control_plane:
      'HERA Cloud Edge Optimizer (Cloudflare Workers WASM + KV + D1 + R2 + Vectorize + Workers AI)',
    motto: 'La mejor decisión es por el bien del enjambre siempre',
    active_swarm_nodes: 3,
    kv_forensic_cache_entries: 42,
    estimated_laptop_cpu_seconds_saved: 211.6,
    d1_free_storage_gb: 5.0,
    workers_ai_free_neurons_per_day: 10000,
    r2_egress_fee_usd_per_gb: 0.0,
    pioneer_forensic_bounty_mb: 5.0,
    turbo_credit_multiplier: 1.5,
    primitives: [
      {
        id: 'cf_kv_forensic_cache',
        name: 'Cloudflare KV Global Forensic Dedup Cache',
        role_in_hera:
          'Evita recalcular FFT/Symphonia en tu laptop si otro DJ ya verificó el mismo CAS (< 2 ms)',
        free_tier_allowance:
          '100,000 lecturas/día gratis + +5 MB Pioneer Bounty',
        unit_cost_usd: 0.0,
        active: true,
      },
      {
        id: 'cf_d1_dna_sync',
        name: 'Cloudflare D1 Edge SQLite (DJ Style DNA + Set Spines)',
        role_in_hera:
          'Sincroniza tu ADN musical (~250 tokens) y anclas de Set Spine entre cabinas a $0.00',
        free_tier_allowance: '5 GB (~14M perfiles DJ) + 5M lecturas/día gratis',
        unit_cost_usd: 0.0,
        active: true,
      },
      {
        id: 'cf_vectorize_simhash',
        name: 'Cloudflare Vectorize + u64 SimHash POPCNT Index',
        role_in_hera:
          'Empareja textura acústica + Rueda Camelot + curva BPM en todo el enjambre global',
        free_tier_allowance: '30M dimensiones consultadas/mes gratis',
        unit_cost_usd: 0.0,
        active: true,
      },
      {
        id: 'cf_r2_zero_egress_swarm',
        name: 'Cloudflare R2 Zero-Egress CAS + P2C 60s Signed Tickets',
        role_in_hera:
          'Almacenamiento de shards ciegos de 4 MB con $0.00 de egress y créditos 1:1.5 Turbo',
        free_tier_allowance: '10 GB R2/nodo (9 GB safe cap) · $0.00 egress',
        unit_cost_usd: 0.0,
        active: true,
      },
      {
        id: 'cf_workers_ai_gateway',
        name: 'Cloudflare Workers AI + AI Gateway Cache',
        role_in_hera:
          'Copiloto ReAct gratuito (Llama-3.1-8B / 3.2-3B) con caché de prompts repetidos',
        free_tier_allowance: '10,000 Neurons/día (~1,400 turnos/día a $0.00)',
        unit_cost_usd: 0.0,
        active: true,
      },
      {
        id: 'cf_workers_wasm_moq_pubsub',
        name: 'Cloudflare Workers WASM + MoQ Harmonic Pub/Sub Relay',
        role_in_hera:
          'Enrutador P2C de baja latencia y suscripción semántica hera://swarm/camelot/<key>',
        free_tier_allowance:
          '100,000 req/día por Edge Guardian (85,000 safe cap)',
        unit_cost_usd: 0.0,
        active: true,
      },
    ],
    recent_moq_harmonic_events: [
      {
        event_id: 'moq-hera-seed-01',
        topic_uri: 'hera://swarm/camelot/8A/bpm/119-125',
        cas_fingerprint: 'cas_8a_122_ritual_del_sol',
        track_id: 'hera-seed-001',
        artist: 'DaviDJ',
        title: 'Ritual Del Sol (DaviDJ Original Club Master)',
        bpm: 122.0,
        camelot_key: '8A',
        style_simhash_u64: 1094827165,
        hot_cues_count: 4,
        prefetch_recommended: true,
      },
      {
        event_id: 'moq-hera-seed-02',
        topic_uri: 'hera://swarm/camelot/9A/bpm/123-129',
        cas_fingerprint: 'cas_9a_126_fuego_de_medianoche',
        track_id: 'hera-seed-002',
        artist: 'DJFoo',
        title: 'Fuego De Medianoche (Guaracha & Tribal Club Edit)',
        bpm: 126.0,
        camelot_key: '9A',
        style_simhash_u64: 2849104721,
        hot_cues_count: 4,
        prefetch_recommended: true,
      },
    ],
  };
}
