use crate::p2p::get_cloud_referral_catalog;
use crate::router::current_runtime_target;
use serde_json::{json, Value};

/// Complete TypeScript ambient SDK definition (`GET /mcp/code-mode.d.ts`) following Cloudflare's Code Mode specification.
pub fn get_typescript_sdk_definitions() -> &'static str {
    r#"// ============================================================================
// HERA CURATOR — CLOUDFLARE CODE MODE TYPESCRIPT SDK (`hera.*`)
// Reference: https://blog.cloudflare.com/code-mode/
// ============================================================================

export interface HotCuePoint {
  num: number;
  name: string;
  seconds: number;
  bar_index: number;
  role: "downbeat" | "phrase_in" | "drop" | "mix_out";
  confidence: number;
}

export interface ForensicAuditReport {
  is_authentic_lossless: boolean;
  is_synthetic_tone: boolean;
  effective_bandwidth_hz: number;
  nyquist_hz: number;
  suspected_source_codec: string;
  spectral_flatness: number;
  peak_to_mean_ratio: number;
  crest_factor_db: number;
  clipped_samples: number;
  clipping_ratio: number;
  sub_bass_phase_correlation: number;
  mono_compatible_sub: boolean;
  quality_score: number;
  forensic_verdict: "VERIFIED_STUDIO_MASTER" | "REJECT_SYNTHETIC_TONE" | "WARN_FAKE_LOSSLESS_TRANSCODE" | "WARN_SUB_BASS_PHASE_CANCELLATION";
  warnings: string[];
}

export interface P2PVerifiedTrack {
  track_id: string;
  title: string;
  artist: string;
  bpm: number;
  camelot_key: string;
  top_subgenre: string;
  duration_seconds: number;
  format: string;
  effective_bandwidth_hz: number;
  sub_bass_phase_correlation: number;
  mono_compatible_sub: boolean;
  quality_score: number;
  forensic_verdict: string;
  hot_cues: HotCuePoint[];
  stream_url?: string;
  node_id: string;
  hera_seal: string;
}

export interface OptimizedCrateResult {
  brief: string;
  energy_arc: "steady_climb" | "rollercoaster" | "peak_hour";
  total_tracks: number;
  total_duration_seconds: number;
  mean_synergy_score: number;
  harmonic_match_pct: number;
  polyrhythmic_bridges: number;
  vocal_clash_warnings: number;
  sequence: P2PVerifiedTrack[];
  m3u8_export: string;
  rekordbox_xml_export: string;
  set_guide_markdown: string;
}

declare namespace hera {
  /** Search verified studio masters in the P2P node catalog. */
  function searchMusic(params?: {
    query?: string;
    camelot?: string;
    bpm_min?: number;
    bpm_max?: number;
    subgenre?: string;
    verified_only?: boolean;
  }): Promise<P2PVerifiedTrack[]>;

  /** Filter a track list by forensic quality, sub-bass phase correlation, BPM, or Camelot compatibility. */
  function filterTracks(params: {
    tracks: P2PVerifiedTrack[];
    min_quality_score?: number;
    require_mono_compatible_sub?: boolean;
    min_bandwidth_hz?: number;
    compatible_with_camelot?: string;
    bpm_min?: number;
    bpm_max?: number;
  }): Promise<P2PVerifiedTrack[]>;

  /** Compute 4-layer transition synergy (Camelot + BPM/3:4 Polyrhythm + Transient/Vocals + 64D Style) between two tracks. */
  function evaluateTransition(params: {
    from_track: P2PVerifiedTrack;
    to_track: P2PVerifiedTrack;
  }): Promise<{
    total_synergy: number;
    camelot_score: number;
    camelot_relation: string;
    bpm_score: number;
    is_polyrhythmic_bridge: boolean;
    vocal_clash_warning: boolean;
    mix_recommendation: string;
  }>;

  /** Sequence a complete DJ crate globally using the 2-Opt Traveling Salesperson (TSP) optimizer. */
  function buildDjCrate(params: {
    brief: string;
    energy_arc?: "steady_climb" | "rollercoaster" | "peak_hour";
    max_tracks?: number;
    candidates?: P2PVerifiedTrack[];
  }): Promise<OptimizedCrateResult>;

  /** Audit the entire node repertoire and return Zero-Trust forensic statistics. */
  function auditRepertoire(): Promise<{
    node_id: string;
    total_tracks: number;
    verified_studio_masters: number;
    average_quality_score: number;
    tracks: P2PVerifiedTrack[];
  }>;

  /** Compute Federated R2 Edge Swarm strength & Constrained Water-Filling allocations (9,000 MB R2 & 85,000 req/day safety caps). */
  function getSwarmStrength(): Promise<any>;

  /** Register an Edge Guardian node heartbeat and receive Water-Filling allocations & 1:1.5 Turbo Credits. */
  function swarmHeartbeat(params: {
    node_id: string;
    owner_dj: string;
    endpoint_url: string;
    r2_used_mb?: number;
    daily_worker_requests_used?: number;
    megabytes_served_delta?: number;
  }): Promise<any>;

  /** Route a track download via Power-of-Two-Choices (P2C) and issue a single-use 60s signed X-Hera-Swarm-Ticket. */
  function swarmRouteTicket(params: {
    requester_dj: string;
    track_id_or_cas: string;
    block_index?: number;
  }): Promise<any>;

  /** Issue or verify a 64-byte Proof-of-Custody range audit against an Edge Guardian node. */
  function swarmCustodyAudit(params: {
    node_id: string;
    cas_fingerprint_or_blob?: string;
    opaque_blob_key?: string;
    byte_offset?: number;
    http_status?: number;
    returned_slice_sha256?: string;
  }): Promise<any>;

  /** Issue an HTTP/3 QUIC Multi-Node Parallel Striping Manifest across distinct low-latency Edge Guardian nodes. */
  function swarmParallelManifest(params: {
    requester_dj: string;
    track_id_or_cas: string;
    file_size_mb?: number;
  }): Promise<any>;

  /** Synchronize an offline-first Delta-CRDT state (GCounter + LWW-Element-Set) with guaranteed convergence. */
  function swarmCrdtSync(params: {
    origin_node_id?: string;
    origin_endpoint_pubkey_id?: string;
    hlc_timestamp_ms?: number;
    gcounter_node_bytes_served_mb?: Record<string, number>;
    gcounter_cas_play_counts?: Record<string, Record<string, number>>;
    lww_track_entries?: Record<string, any>;
  }): Promise<any>;

  /** Subscribe an Edge Guardian or DJ node to an MoQ-style harmonic topic (e.g. hera://swarm/8A/120-128) for proactive track notifications. */
  function swarmSubscribe(params: {
    subscriber_node_id: string;
    subscriber_dj: string;
    camelot_key: string;
    bpm_min?: number;
    bpm_max?: number;
    subgenre_filter?: string;
  }): Promise<any>;
}
"#
}

/// Complete OpenAPI 3.1.0 Specification (`GET /openapi.json`) for Google AI Studio, Gemini CLI, OpenWebUI & Custom Actions.
pub fn get_openapi_3_1_spec(node_id: &str) -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Hera Universal Audio DSP, MCP Code Mode & P2P Node API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": format!(
                "Pure-Rust Audio DSP, 4-Layer 2-Opt TSP Crate Curator, Cloudflare Code Mode & Quarantine-First P2P Node (Node: {}, Runtime: {})",
                node_id,
                current_runtime_target()
            )
        },
        "paths": {
            "/health": {
                "get": {
                    "operationId": "getNodeHealth",
                    "summary": "Check Hera node status, runtime target, and P2P catalog count",
                    "responses": { "200": { "description": "Node is healthy" } }
                }
            },
            "/mcp": {
                "post": {
                    "operationId": "mcpJsonRpc",
                    "summary": "Model Context Protocol (MCP) JSON-RPC 2.0 endpoint",
                    "responses": { "200": { "description": "JSON-RPC 2.0 response" } }
                }
            },
            "/mcp/code-mode": {
                "post": {
                    "operationId": "executeHeraCodeMode",
                    "summary": "Cloudflare Code Mode execution endpoint: run multi-step TypeScript `hera.*` scripts or pipelines in a single sub-millisecond turn",
                    "responses": { "200": { "description": "Code Mode execution output and log" } }
                }
            },
            "/mcp/code-mode.d.ts": {
                "get": {
                    "operationId": "getCodeModeTypeScriptDefinitions",
                    "summary": "Download TypeScript `.d.ts` ambient SDK declarations for `hera.*` Code Mode",
                    "responses": { "200": { "description": "TypeScript declarations" } }
                }
            },
            "/api/v1/audio/analyze-file": {
                "post": {
                    "operationId": "analyzeAudioFile",
                    "summary": "Upload WAV/FLAC/MP3/OGG bytes for pure-Rust Symphonia + rustfft spectral forensics & 4 Acoustic Hot Cues",
                    "responses": { "200": { "description": "Full forensic & structural Hot-Cue telemetry" } }
                }
            },
            "/api/v1/crates/build": {
                "post": {
                    "operationId": "buildDjCrate2Opt",
                    "summary": "Sequence a harmonic DJ crate using the 4-Layer Synergy Engine + Global 2-Opt TSP optimizer",
                    "responses": { "200": { "description": "Optimized sequence with M3U8, Rekordbox XML, and Set Guide" } }
                }
            },
            "/api/v1/p2p/search": {
                "get": {
                    "operationId": "searchP2PVerifiedCatalog",
                    "summary": "Search acoustically verified studio masters in the P2P node catalog",
                    "responses": { "200": { "description": "Matching verified tracks" } }
                }
            },
            "/api/v1/p2p/announce": {
                "post": {
                    "operationId": "announceP2PVerifiedTrack",
                    "summary": "Quarantine-First P2P gatekeeper: admit a track only if it passes Zero-Trust spectral & phase forensics",
                    "responses": { "201": { "description": "Track admitted with cryptographic hera_seal" } }
                }
            },
            "/api/v1/node/manifest": {
                "get": {
                    "operationId": "getNodeManifestAndReferrals",
                    "summary": "Get node invariants and 1-Click Cloud Referral deployment catalog",
                    "responses": { "200": { "description": "Manifest and referral catalog" } }
                }
            },
            "/api/v1/swarm/strength": {
                "get": {
                    "operationId": "getSwarmStrength",
                    "summary": "Compute Federated R2 Edge Swarm strength, Constrained Water-Filling allocations, and elastic replication K",
                    "responses": { "200": { "description": "Swarm strength and Water-Filling report" } }
                }
            },
            "/api/v1/swarm/heartbeat": {
                "post": {
                    "operationId": "postSwarmHeartbeat",
                    "summary": "Register an Edge Guardian node heartbeat, credit 1:1.5 Turbo Credits, and receive Water-Filling allocations",
                    "responses": { "200": { "description": "Heartbeat response and outbound pull tickets" } }
                }
            },
            "/api/v1/swarm/route-ticket": {
                "post": {
                    "operationId": "postSwarmRouteTicket",
                    "summary": "Route a shard download via Power-of-Two-Choices (P2C) and issue a single-use 60s signed X-Hera-Swarm-Ticket",
                    "responses": { "200": { "description": "Signed 60s redirect ticket and P2C selected node" } }
                }
            },
            "/api/v1/swarm/custody-audit": {
                "post": {
                    "operationId": "postSwarmCustodyAudit",
                    "summary": "Verify a 64-byte Proof-of-Custody range audit response and enforce Edge Guardian meritocracy",
                    "responses": { "200": { "description": "Custody audit result" } }
                }
            }
        },
        "x-cloud-referrals": get_cloud_referral_catalog()
    })
}
