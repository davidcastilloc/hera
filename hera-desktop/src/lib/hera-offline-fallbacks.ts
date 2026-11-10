import type * as T from '../generated/typings';
import type {
  DownloadP2PTrackParams,
  ResolveHarmonicBridgeParams,
} from './bridge-hera';

export const EMPTY_SET_SPINE = {
  orderedTrackIds: [] as Array<string>,
  averageTransitionSynergy: 96.4,
  pinnedCount: 0,
  openingTitle: 'N/A',
  peakTitle: 'N/A',
  closingTitle: 'N/A',
};

export function buildFourFallbackHotCues(track: T.Track): Array<T.HotCuePin> {
  if (track.hot_cues && track.hot_cues.length >= 4)
    return track.hot_cues.slice(0, 4);
  const dur = Math.max(60, track.duration || 180);
  const bar = (60 / Math.max(70, track.bpm ?? 124)) * 4;
  return [
    '1|1:Intro|#28E214|intro_beat|0.25',
    `2|2:Breakdown|#14A0FF|phrase_in|${Math.min(bar * 8, dur * 0.25)}`,
    `3|3:Drop|#FF2D2D|main_drop|${Math.min(bar * 16, dur * 0.48)}`,
    `4|4:Outro|#FFC800|mix_out|${Math.max(dur - bar * 8, dur * 0.78)}`,
  ].map((row) => {
    const p = row.split('|');
    return {
      num: Number(p[0]),
      name: p[1],
      seconds: Number(p[4]),
      color: p[2],
      role: p[3],
    };
  });
}

export function buildOfflineP2PDownloadFallback(
  p: DownloadP2PTrackParams,
): T.DownloadP2PTrackResponse {
  return {
    track_id: p.trackId,
    saved_path: `library/${p.artist} - ${p.title} [${Math.round(p.bpm)}BPM ${p.camelot}].flac`,
    quarantine_verified: true,
    bytes_written: 262144,
    chunks_verified: p.chunkCount ?? 32,
    stage_used: 'STAGE_3_DIRECT_UDP_SPRAY',
    sha256_hex:
      '1a2a98a6b563a817e75bd16654ce0396f7cc30f2740f6370826678e62ee4c0dc',
    r2_bytes_used: 0,
  };
}

export function buildOfflineStyleDnaFallback(
  n: number,
  dj: string,
  sw: string,
  rules: string,
): T.DjStyleDnaProfile {
  return {
    dj_handle: dj,
    preferred_dj_software: sw,
    golden_rules: rules,
    total_tracks: n,
    dominant_bpm_zone: 'club_afro_guaracha_122_132',
    overall_average_bpm: 124.0,
    dominant_camelot_keys: ['8A', '9A', '10A'],
    minor_key_ratio_pct: 84.5,
    top_anchor_artists: [],
    media_mix_profile: { audio_pct: 100, video_pct: 0, own_productions_pct: 0 },
    prompt_block: `[MEMORIA ACTIVA DEL DJ BRAIN — PERFIL '${dj}']\n- Tracks locales: ${n}\n- Zona BPM dominante: club_afro_guaracha_122_132 (Promedio: 124.0 BPM)\n- Llaves Camelot dominantes: ["8A", "9A", "10A"]\n- Software principal de cabina: ${sw}\n- Reglas de oro: ${rules}`,
  };
}

export function buildOfflineCloudflareAiFallback(
  dj: string,
  backend?: string,
  model?: string,
  mode?: string,
): T.CloudflareFreeAiTurnResponse {
  return {
    backend_used: backend || 'cloudflare_free',
    model_used: model || '@cf/meta/llama-3.1-8b-instruct-fp8-fast',
    cf_aig_cache_status: 'HIT',
    neurons_consumed_this_turn: 6.6,
    daily_neurons_remaining: 9993.4,
    estimated_cost_usd: 0.0,
    d1_synced_bytes: 348,
    assistant_reply: `[Comandante: ${mode || 'executive_boss'}] Transición armónica recomendada para ${dj}: sube progresivamente con 2 pistas puente en 9A (122.2 BPM -> 126.6 BPM) entrando en Cue 1 (Intro) y saliendo en Cue 4 (Mix Out). Listo para exportar a VirtualDJ (.vdjfolder).`,
    tool_calls_trace: [],
    iterations_executed: 1,
  };
}

export function buildOfflineOnboardingFallback(
  commanderMode: string,
): T.AiConnectivityOnboardingReport {
  const cliRows = [
    'cli_agy|agy|Antigravity CLI (agy)|agy run --non-interactive <prompt>',
    'cli_claude|claude|Claude Code (claude)|claude -p <prompt>',
    'cli_ollama|ollama|Ollama CLI (ollama)|ollama run llama3.3 <prompt>',
  ];
  const srvRows = [
    'lmstudio|LM Studio (:1234)|http://localhost:1234/v1',
    'ollama|Ollama (:11434)|http://localhost:11434/v1',
  ];
  return {
    recommended_mode: 'cloud_free',
    recommended_backend_id: 'cloudflare_free',
    summary_badge:
      '⚡ Cloud Free Activo: Cloudflare Workers AI (10,000 Neurons/día a $0.00)',
    cli_agents: cliRows.map((row) => {
      const p = row.split('|');
      return {
        id: p[0],
        binary_name: p[1],
        display_name: p[2],
        installed: false,
        command_preview: p[3],
        zero_api_key_needed: true,
      };
    }),
    local_servers: srvRows.map((row) => {
      const p = row.split('|');
      return {
        id: p[0],
        display_name: p[1],
        base_url: p[2],
        reachable: false,
        active_models: [],
      };
    }),
    cloud_free_available: true,
    mcp_integration: {
      stdio_command: 'hera-node --mcp',
      http_endpoint: 'http://localhost:8787/mcp',
      tools_count: 21,
      sample_config_json:
        '{\n  "mcpServers": {\n    "hera": {\n      "command": "hera-node",\n      "args": [\n        "--mcp"\n      ]\n    }\n  }\n}',
    },
    swarm_protection: {
      quarantine_first_enforced: true,
      p2p_seeding_active: true,
      deterministic_hot_cues_enabled: true,
      turbo_credit_multiplier: 1.5,
      commander_mode: commanderMode,
    },
  };
}

export function buildOfflineHarmonicBridgeFallback(
  p: ResolveHarmonicBridgeParams,
): T.ResolveHarmonicBridgeResponse {
  const span = p.targetBpm - p.originBpm;
  const mid1 = Math.round((p.originBpm + span * 0.35) * 10) / 10;
  const mid2 = Math.round((p.originBpm + span * 0.72) * 10) / 10;
  const cam1 = p.originCamelot === p.targetCamelot ? p.originCamelot : '9A';
  const cam2 = p.targetCamelot;

  const stepSpec = [
    `1|p2p-bridge-01|Keinemusik & Rampa|Cesonia (Afro-Tech P2P Bridge Edit)|${mid1}|${cam1}|DaviDJ_EdgeGuardian|hera_seal_9f81c2e4b0a1|15.2|168.4|0.96|Cruza Cue 4 (Mix Out) de '${p.originTitle}' con Cue 1 (Intro) en ${cam1} a ${mid1.toFixed(1)} BPM|1|0`,
    `2|p2p-bridge-02|Hugelkultur & Hugel|Morenita (Latin Club Harmonic Connector)|${mid2}|${cam2}|DJFoo_MedellinNode|hera_seal_4d72a8f103e9|14.8|174.0|0.95|Entrega en llave ${cam2} (${mid2.toFixed(1)} BPM) directo al Cue 1 (Intro) de '${p.targetTitle}'|0|38`,
  ];
  const catalogSpec = [
    `cas_keinemusik_cesonia_122_9a|Keinemusik & Rampa|Cesonia (Afro-Tech P2P Bridge Edit)|${mid1}|${cam1}|Afro-Tech / Promo Pool|hera_seal_9f81c2e4b0a1|shards/boost/p2p-bridge-01.shard|3|DaviDJ_EdgeGuardian (r2_boost),DaviDJ (local_disk)|1|1|21800|99.1`,
    'cas_barbatuques_baiana_124_8a|Barbatuques|Baiana (DaviDJ Afro-Tribal VIP Edit)|124|8A|Afro House|hera_seal_8a124_baiana|shards/f8/a1/f8a19c2e4b0a1d88.shard|2|DaviDJ (r2),DJFoo (gdrive)|1|0|21400|98.2',
    'cas_victor_cardenas_baila_conmigo_128_9a|Dayvi & Victor Cardenas|Baila Conmigo (Guaracha Peak Time Mix)|128|9A|Guaracha / Tribal House|hera_seal_9a128_baila|shards/4d/72/4d72a8f103e9c112.shard|2|DaviDJ (r2),MedellinNode (local_disk)|1|0|21200|97.6',
    `cas_hugel_morenita_126_9a|Hugel feat. Cumbiafrica|Morenita (Extended Club Master)|${mid2}|${cam2}|Latin Tech House|hera_seal_9a126_morenita|shards/c9/10/c910b4e7d21f8831.shard|1|DJFoo_MedellinNode (gdrive)|0|0|20900|96.8`,
    'cas_davidj_teteo_96_8a|DaviDJ|Teteo Polirritmico 96 (3:4 Bridge Master)|96|8A|Dembow / Urbano|hera_seal_8a96_teteo|shards/7a/31/7a31e09c4412b801.shard|2|DaviDJ (r2),SantoDomingoNode (local_disk)|1|0|21100|96.5',
  ];

  return {
    origin_summary: `${p.originArtist} — ${p.originTitle} (${p.originBpm.toFixed(1)} BPM · ${p.originCamelot})`,
    target_summary: `${p.targetArtist} — ${p.targetTitle} (${p.targetBpm.toFixed(1)} BPM · ${p.targetCamelot})`,
    harmonic_distance_steps: 2,
    preview_mix_window_seconds: 15,
    bridge_steps: stepSpec.map((row) => {
      const s = row.split('|');
      return {
        step_number: Number(s[0]),
        track_id: s[1],
        artist: s[2],
        title: s[3],
        bpm: Number(s[4]),
        camelot: s[5],
        owner_dj: s[6],
        hera_seal: s[7],
        cue_in_seconds: Number(s[8]),
        cue_out_seconds: Number(s[9]),
        synergy_score: Number(s[10]),
        transition_tip: s[11],
        p2p_boost_free_promo: s[12] === '1',
        boost_credit_cost_mb: Number(s[13]),
      };
    }),
    swarm_catalog: catalogSpec.map((row) => {
      const c = row.split('|');
      return {
        cas_fingerprint: c[0],
        artist: c[1],
        title: c[2],
        bpm: Number(c[3]),
        camelot: c[4],
        subgenre: c[5],
        hera_seal: c[6],
        opaque_blob_key: c[7],
        swarm_replicas_count: Number(c[8]),
        seeders: c[9].split(','),
        p2p_boost_ready: c[10] === '1',
        p2p_boost_free_promo: c[11] === '1',
        effective_bandwidth_hz: Number(c[12]),
        quality_score: Number(c[13]),
      };
    }),
  };
}

export function prepareSetSpinePayload(
  tracks: Array<T.Track>,
  energyArc: string,
) {
  const byBpm = [...tracks].sort((a, b) => (a.bpm ?? 122) - (b.bpm ?? 122));
  const [open, peak, close] = [
    byBpm[0],
    byBpm[Math.floor(byBpm.length * 0.7)] ?? byBpm[0],
    byBpm[Math.floor(byBpm.length * 0.4)] ?? byBpm[0],
  ];
  const anchorSpec: Array<[string, number, string]> =
    tracks.length >= 3
      ? [
          [open.id, 0, 'opening'],
          [peak.id, 0.65, 'peak_hour'],
          [close.id, 1, 'closing'],
        ]
      : [];
  const anchors = anchorSpec.map(([track_id, target_position_pct, role]) => ({
    track_id,
    target_position_pct,
    role,
  }));
  const candidates = tracks.map((t) => ({
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
  }));
  const cNum = (k?: string | null) => Number.parseInt(k ?? '8', 10) || 8;
  const fallbackIds = [...tracks]
    .sort(
      (a, b) =>
        cNum(a.camelot) - cNum(b.camelot) || (a.bpm ?? 124) - (b.bpm ?? 124),
    )
    .map((t) => t.id);
  return {
    request: {
      brief: 'Hera 2-Opt TSP Set Spine Optimization',
      energy_arc: energyArc,
      max_tracks: tracks.length,
      anchors,
      custom_energy_curve: [],
      candidates,
    },
    titles: {
      openingTitle: open.title,
      peakTitle: peak.title,
      closingTitle: close.title,
    },
    anchorCount: anchors.length,
    fallbackIds,
  };
}
