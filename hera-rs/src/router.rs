use crate::cloudflare_edge::{
    get_cloudflare_edge_capabilities, issue_cloudflare_turn_credentials,
    lookup_edge_forensic_cache, query_edge_vectorize_matches, store_edge_forensic_cache,
    sync_d1_dj_dna, D1DnaSyncRequest, EdgeForensicLookupRequest, EdgeForensicStoreRequest,
    TurnCredentialsRequest, VectorizeMatchRequest,
};
use crate::code_mode::{
    execute_code_mode, get_openapi_3_1_spec, get_typescript_sdk_definitions, CodeModeRequest,
};
use crate::dj_brain::{
    auto_detect_local_backend, compute_dj_style_dna, evaluate_agent_harness_turn,
    list_supported_ai_backends, probe_backend_models, CloudflareFreeAiTurnRequest,
    DjStyleDnaRequest,
};
use crate::error::HeraError;
use crate::export::{export_crate_bundle, ExportCrateRequest};
use crate::forensics::{audit_pcm_channels, decode_audio_bytes, ForensicAuditReport};
use crate::mcp::{handle_mcp_request, list_mcp_tools, JsonRpcRequest};
use crate::p2p::{
    build_gdrive_large_file_bypass_url, compute_canonical_audio_sha256, compute_cas_fingerprint,
    compute_hera_seal, compute_opaque_shard_blob_key, get_cloud_referral_catalog,
    BrokerBatchLookupRequest, BrokerBridgeRequest, BrokerSimilarQuery, CloudReferralOption,
    ConnectDjDriveRequest, CustodyAuditSubmission, CustodyChallengeRequest, FederatedSyncRequest,
    P2PMeshState, P2PPeerNode, P2PSearchQuery, P2PVerifiedTrack, ScanLocalDirRequest,
    SwarmHeartbeatRequest, SwarmParallelManifestRequest, SwarmRedirectTicket, SwarmRouteRequest,
    SWARM_ALIGNED_BLOCK_BYTES,
};
use crate::structure::{analyze_structure_and_style, StructuralCueProfile};
use crate::synergy::{
    optimize_crate_with_spine, CrateTrackCandidate, OptimizedCrateResult, SetSpineConfig,
    SpineAnchor,
};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzePcmRequest {
    pub left: Vec<f32>,
    pub right: Option<Vec<f32>>,
    #[serde(default = "default_sr")]
    pub sample_rate: u32,
    #[serde(default = "default_ext")]
    pub claimed_ext: String,
    pub full_duration_seconds: Option<f32>,
    pub bpm_hint: Option<f32>,
    pub track_id: Option<String>,
}

fn default_sr() -> u32 {
    44100
}
fn default_ext() -> String {
    "flac".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeFileQuery {
    pub ext: Option<String>,
    pub claimed_ext: Option<String>,
    pub bpm_hint: Option<f32>,
    pub track_id: Option<String>,
    pub max_seconds: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullDspAnalysisResponse {
    pub track_id: String,
    pub forensics: ForensicAuditReport,
    pub structure: StructuralCueProfile,
    pub p2p_eligible: bool,
    pub hera_seal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCrateRequest {
    pub brief: String,
    #[serde(default = "default_arc")]
    pub energy_arc: String,
    pub max_tracks: Option<usize>,
    #[serde(default)]
    pub anchors: Vec<SpineAnchor>,
    #[serde(default)]
    pub custom_energy_curve: Vec<f32>,
    pub candidates: Vec<CrateTrackCandidate>,
}

fn default_arc() -> String {
    "steady_climb".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnounceTrackRequest {
    pub track: P2PVerifiedTrack,
    pub forensics: ForensicAuditReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeManifestResponse {
    pub service: String,
    pub version: String,
    pub runtime_target: String,
    pub node_id: String,
    pub memory_budget_mb: u32,
    pub invariants: Vec<String>,
    pub endpoints: Vec<String>,
    pub cloud_referrals: Vec<CloudReferralOption>,
}

/// Build the single universal `axum::Router` shared between Cloudflare Workers (WASM) and Native VPS (Tokio).
pub fn build_hera_router(state: P2PMeshState) -> Router {
    Router::new()
        .route("/", get(health_handler))
        .route("/health", get(health_handler))
        .route("/openapi.json", get(openapi_handler))
        .route("/mcp", post(mcp_http_handler))
        .route("/mcp/tools", get(mcp_tools_handler))
        .route("/mcp/code-mode", post(code_mode_exec_handler))
        .route("/mcp/code-mode.d.ts", get(code_mode_dts_handler))
        .route("/api/v1/node/manifest", get(manifest_handler))
        .route("/api/v1/broker/stats", get(broker_stats_handler))
        .route("/api/v1/broker/lookup-intel", post(broker_lookup_intel_handler))
        .route("/api/v1/broker/resolve-bridge", post(broker_resolve_bridge_handler))
        .route("/api/v1/broker/similar", post(broker_similar_handler))
        .route("/api/v1/swarm/strength", get(swarm_strength_handler))
        .route("/api/v1/swarm/heartbeat", post(swarm_heartbeat_handler))
        .route("/api/v1/swarm/route-ticket", post(swarm_route_ticket_handler))
        .route("/api/v1/swarm/parallel-manifest", post(swarm_parallel_manifest_handler))
        .route("/api/v1/swarm/crdt-sync", post(swarm_crdt_sync_handler))
        .route("/api/v1/swarm/subscribe", post(swarm_subscribe_handler))
        .route("/api/v1/swarm/shard/:blob_id", get(swarm_shard_delivery_handler))
        .route("/api/v1/swarm/custody-challenge", post(swarm_custody_challenge_handler))
        .route("/api/v1/swarm/custody-probe/:blob_id", get(swarm_custody_probe_handler))
        .route("/api/v1/swarm/custody-audit", post(swarm_custody_audit_handler))
        .route("/api/v1/swarm/quarantine", get(swarm_quarantine_list_handler))
        .route("/api/v1/node/scan-local-dir", post(scan_local_dir_handler))
        .route("/api/v1/dj/connect-drive", post(connect_dj_drive_handler))
        .route("/api/v1/dj/profiles", get(list_dj_profiles_handler))
        .route("/api/v1/dj/style-dna", post(dj_style_dna_handler))
        .route("/api/v1/ai/backends", get(ai_backends_list_handler))
        .route("/api/v1/ai/probe", get(ai_probe_backend_handler))
        .route("/api/v1/ai/chat", post(ai_cloudflare_free_chat_handler))
        .route("/api/v1/audio/analyze-pcm", post(analyze_pcm_handler))
        .route("/api/v1/audio/analyze-file", post(analyze_file_handler))
        .route("/api/v1/crates/build", post(build_crate_handler))
        .route("/api/v1/crates/set-spine", post(build_crate_handler))
        .route("/api/v1/crates/export", post(export_crate_handler))
        .route("/api/v1/edge/cloudflare-capabilities", get(edge_capabilities_handler))
        .route("/api/v1/edge/forensic-cache/lookup", post(edge_forensic_lookup_handler))
        .route("/api/v1/edge/forensic-cache/store", post(edge_forensic_store_handler))
        .route("/api/v1/edge/d1-dna-sync", post(edge_d1_dna_sync_handler))
        .route("/api/v1/edge/vectorize-match", post(edge_vectorize_match_handler))
        .route("/api/v1/p2p/search", get(p2p_search_handler))
        .route("/api/v1/p2p/announce", post(p2p_announce_handler))
        .route("/api/v1/p2p/peers", get(p2p_list_peers_handler))
        .route("/api/v1/p2p/peers/register", post(p2p_register_peer_handler))
        .route("/api/v1/p2p/federate-sync", post(p2p_federate_sync_handler))
        .route("/api/v1/p2p/export-federation", get(p2p_export_federation_handler))
        .route("/api/v1/p2p/stream/:track_id", get(p2p_stream_handler))
        .route("/api/v1/p2p/download/:track_id", get(p2p_download_local_file_handler))
        .route("/api/v1/p2p/turn-credentials", post(p2p_turn_credentials_handler))
        .route("/api/v1/p2p/turn-relay-proof", post(p2p_turn_relay_proof_handler))
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
        .with_state(state)
}

async fn broker_stats_handler(State(state): State<P2PMeshState>) -> impl IntoResponse {
    Json(state.broker_stats())
}

async fn broker_lookup_intel_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<BrokerBatchLookupRequest>,
) -> impl IntoResponse {
    Json(state.broker_lookup_intel_batch(req))
}

async fn broker_resolve_bridge_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<BrokerBridgeRequest>,
) -> impl IntoResponse {
    let candidates = state.broker_resolve_emergency_bridge(req);
    Json(serde_json::json!({
        "node_id": state.node_id,
        "bridge_candidates_count": candidates.len(),
        "candidates": candidates
    }))
}

async fn broker_similar_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<BrokerSimilarQuery>,
) -> impl IntoResponse {
    let matches = state.broker_find_similar_by_popcnt(req);
    Json(serde_json::json!({
        "node_id": state.node_id,
        "engine": "u64_simhash_popcnt_in_ram",
        "matches_count": matches.len(),
        "matches": matches
    }))
}

async fn scan_local_dir_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<ScanLocalDirRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), HeraError> {
    match state.scan_local_directory(req) {
        Ok(res) => Ok((
            StatusCode::CREATED,
            Json(serde_json::to_value(res).unwrap_or_default()),
        )),
        Err(e) => Err(HeraError::BadRequest(e)),
    }
}

async fn p2p_list_peers_handler(State(state): State<P2PMeshState>) -> impl IntoResponse {
    let peers = state.list_peers();
    Json(serde_json::json!({
        "node_id": state.node_id,
        "active_peers_count": peers.len(),
        "peers": peers
    }))
}

async fn p2p_register_peer_handler(
    State(state): State<P2PMeshState>,
    Json(peer): Json<P2PPeerNode>,
) -> impl IntoResponse {
    let peers = state.register_peer_node(peer);
    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "peer_registered",
            "active_peers_count": peers.len(),
            "peers": peers
        })),
    )
}

async fn p2p_federate_sync_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<FederatedSyncRequest>,
) -> impl IntoResponse {
    let resp = state.federate_sync(req);
    (StatusCode::OK, Json(resp))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFederationQuery {
    pub dj_handle: Option<String>,
    pub public_url: Option<String>,
    pub bandwidth_mbps: Option<u32>,
}

async fn p2p_export_federation_handler(
    State(state): State<P2PMeshState>,
    Query(q): Query<ExportFederationQuery>,
) -> impl IntoResponse {
    let dj = q.dj_handle.unwrap_or_else(|| "DaviDJ".to_string());
    let bw = q.bandwidth_mbps.unwrap_or(800);
    let payload = state.build_federated_sync_payload(&dj, q.public_url, bw);
    Json(payload)
}

async fn p2p_download_local_file_handler(
    State(state): State<P2PMeshState>,
    Path(track_id): Path<String>,
) -> Result<impl IntoResponse, HeraError> {
    match state.get_local_file_bytes(&track_id) {
        Ok((path_str, bytes)) => {
            let content_type = if path_str.ends_with(".flac") {
                "audio/flac"
            } else if path_str.ends_with(".mp3") {
                "audio/mpeg"
            } else {
                "audio/wav"
            };
            Ok(([(http::header::CONTENT_TYPE, content_type)], bytes))
        }
        Err(e) => Err(HeraError::NotFound(e)),
    }
}

async fn connect_dj_drive_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<ConnectDjDriveRequest>,
) -> impl IntoResponse {
    let resp = state.connect_dj_drive(req);
    (StatusCode::CREATED, Json(resp))
}

async fn list_dj_profiles_handler(State(state): State<P2PMeshState>) -> impl IntoResponse {
    let profiles = state.list_dj_profiles();
    Json(serde_json::json!({
        "node_id": state.node_id,
        "connected_djs_count": profiles.len(),
        "profiles": profiles
    }))
}

async fn p2p_stream_handler(
    State(state): State<P2PMeshState>,
    Path(track_id): Path<String>,
) -> Result<Json<serde_json::Value>, HeraError> {
    if let Some(track) = state.find_track_by_id(&track_id) {
        let cas = compute_cas_fingerprint(&track.artist, &track.title, track.bpm);
        let audio_sha256 = state
            .cas_audio_sha256
            .read()
            .ok()
            .and_then(|g| g.get(&cas).cloned())
            .unwrap_or_else(|| compute_canonical_audio_sha256(&cas, &track.hera_seal, 38.0));
        let opaque_blob_key = compute_opaque_shard_blob_key(&cas, &audio_sha256);
        let gdrive_bypass_url = track
            .gdrive_file_id
            .as_deref()
            .map(|fid| build_gdrive_large_file_bypass_url(fid, 38.0));

        Ok(Json(serde_json::json!({
            "track_id": track.track_id,
            "title": track.title,
            "artist": track.artist,
            "owner_dj": track.owner_dj,
            "storage_provider": track.storage_provider,
            "gdrive_file_id": track.gdrive_file_id,
            "r2_object_key": track.r2_object_key,
            "direct_download_url": track.stream_url,
            "gdrive_large_file_bypass_url": gdrive_bypass_url,
            "aligned_block_size_bytes": SWARM_ALIGNED_BLOCK_BYTES,
            "opaque_blob_key": opaque_blob_key,
            "audio_sha256": audio_sha256,
            "hera_seal": track.hera_seal,
            "hot_cues": track.hot_cues
        })))
    } else {
        Err(HeraError::NotFound(format!(
            "Track `{track_id}` not found in P2P catalog"
        )))
    }
}

async fn swarm_strength_handler(State(state): State<P2PMeshState>) -> impl IntoResponse {
    Json(state.compute_swarm_strength())
}

async fn swarm_heartbeat_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<SwarmHeartbeatRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), HeraError> {
    match state.process_swarm_heartbeat(req) {
        Ok(res) => Ok((
            StatusCode::OK,
            Json(serde_json::to_value(res).unwrap_or_default()),
        )),
        Err(e) => Err(HeraError::Forbidden(e)),
    }
}

async fn swarm_route_ticket_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<SwarmRouteRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), HeraError> {
    match state.issue_route_ticket(req) {
        Ok(res) => Ok((
            StatusCode::OK,
            Json(serde_json::to_value(res).unwrap_or_default()),
        )),
        Err(e) => Err(HeraError::NotFound(e)),
    }
}

async fn swarm_parallel_manifest_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<SwarmParallelManifestRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), HeraError> {
    match state.issue_parallel_swarm_manifest(req) {
        Ok(res) => Ok((
            StatusCode::OK,
            Json(serde_json::to_value(res).unwrap_or_default()),
        )),
        Err(e) => Err(HeraError::NotFound(e)),
    }
}

async fn swarm_crdt_sync_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<crate::p2p::SwarmCrdtDelta>,
) -> impl IntoResponse {
    let res = state.sync_crdt_delta(req);
    (StatusCode::OK, Json(res))
}

async fn swarm_subscribe_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<crate::p2p::SwarmSubscribeRequest>,
) -> impl IntoResponse {
    let res = state.subscribe_semantic_topic(req);
    (StatusCode::CREATED, Json(res))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardDeliveryQuery {
    pub now_unix: Option<u64>,
    pub leaf_index: Option<usize>,
}

async fn swarm_shard_delivery_handler(
    State(state): State<P2PMeshState>,
    Path(blob_id): Path<String>,
    Query(q): Query<ShardDeliveryQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, HeraError> {
    let ticket_header = headers
        .get("x-hera-swarm-ticket")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    match state.redeem_swarm_shard_delivery(&blob_id, ticket_header, q.now_unix) {
        Ok((bytes, receipt)) => {
            let receipt_json = serde_json::to_string(&receipt).unwrap_or_default();
            let parsed_ticket: Option<SwarmRedirectTicket> =
                serde_json::from_str(ticket_header.trim()).ok();
            let leaf_idx = q.leaf_index.unwrap_or(0).min(15);
            let (merkle_root, merkle_proof_json) = if let Some(ref t) = parsed_ticket {
                if let Ok(proof) = crate::p2p::generate_merkle_inclusion_proof(
                    &t.opaque_blob_key,
                    &t.audio_sha256,
                    t.aligned_offset_bytes,
                    leaf_idx,
                ) {
                    (
                        proof.merkle_root_sha256.clone(),
                        serde_json::to_string(&proof).unwrap_or_default(),
                    )
                } else {
                    (t.merkle_root_sha256.clone(), String::new())
                }
            } else {
                (String::new(), String::new())
            };
            Ok((
                StatusCode::OK,
                [
                    (http::header::CONTENT_TYPE, "application/octet-stream".to_string()),
                    (
                        http::header::CACHE_CONTROL,
                        "public, max-age=31536000, immutable".to_string(),
                    ),
                    (
                        http::header::HeaderName::from_static("x-hera-block-align-bytes"),
                        SWARM_ALIGNED_BLOCK_BYTES.to_string(),
                    ),
                    (
                        http::header::HeaderName::from_static("x-hera-proof-of-delivery-receipt"),
                        receipt_json,
                    ),
                    (
                        http::header::HeaderName::from_static("x-hera-merkle-root"),
                        merkle_root,
                    ),
                    (
                        http::header::HeaderName::from_static("x-hera-merkle-proof"),
                        merkle_proof_json,
                    ),
                    (
                        http::header::HeaderName::from_static("x-hera-p2p-boost-cipher"),
                        "zk-ctr-sha256-v1".to_string(),
                    ),
                ],
                bytes,
            ))
        }
        Err(e) => Err(HeraError::Forbidden(e)),
    }
}

async fn swarm_custody_challenge_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<CustodyChallengeRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), HeraError> {
    match state.generate_custody_challenge(req) {
        Ok(res) => Ok((
            StatusCode::OK,
            Json(serde_json::to_value(res).unwrap_or_default()),
        )),
        Err(e) => Err(HeraError::NotFound(e)),
    }
}

async fn swarm_custody_probe_handler(
    State(state): State<P2PMeshState>,
    Path(blob_id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, HeraError> {
    let range_hdr = headers.get(http::header::RANGE).and_then(|v| v.to_str().ok());
    match state.probe_custody_64_bytes(&blob_id, range_hdr) {
        Ok((offset, slice, sha256)) => Ok((
            StatusCode::PARTIAL_CONTENT,
            [
                (http::header::CONTENT_TYPE, "application/octet-stream".to_string()),
                (
                    http::header::CONTENT_RANGE,
                    format!("bytes {}..{}/{}", offset, offset + 63, SWARM_ALIGNED_BLOCK_BYTES),
                ),
                (
                    http::header::HeaderName::from_static("x-hera-custody-slice-sha256"),
                    sha256,
                ),
            ],
            slice,
        )),
        Err(e) => Err(HeraError::NotFound(e)),
    }
}

async fn swarm_custody_audit_handler(
    State(state): State<P2PMeshState>,
    Json(sub): Json<CustodyAuditSubmission>,
) -> impl IntoResponse {
    let res = state.verify_custody_audit(sub);
    let code = if res.passed {
        StatusCode::OK
    } else {
        StatusCode::FORBIDDEN
    };
    (code, Json(res))
}

async fn swarm_quarantine_list_handler(State(state): State<P2PMeshState>) -> impl IntoResponse {
    let list = state.list_quarantined_nodes();
    Json(serde_json::json!({
        "node_id": state.node_id,
        "quarantined_nodes_count": list.len(),
        "quarantined_nodes": list
    }))
}

async fn openapi_handler(State(state): State<P2PMeshState>) -> impl IntoResponse {
    Json(get_openapi_3_1_spec(&state.node_id))
}

async fn code_mode_dts_handler() -> impl IntoResponse {
    (
        [(http::header::CONTENT_TYPE, "application/typescript; charset=utf-8")],
        get_typescript_sdk_definitions(),
    )
}

async fn code_mode_exec_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<CodeModeRequest>,
) -> Result<Json<serde_json::Value>, HeraError> {
    match execute_code_mode(req, &state) {
        Ok(res) => Ok(Json(serde_json::to_value(res).unwrap_or_default())),
        Err(e) => Err(HeraError::BadRequest(e)),
    }
}

async fn mcp_tools_handler() -> impl IntoResponse {
    Json(list_mcp_tools())
}

async fn mcp_http_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    match handle_mcp_request(req, &state) {
        Some(resp) => (StatusCode::OK, Json(serde_json::to_value(resp).unwrap_or_default())),
        None => (StatusCode::ACCEPTED, Json(serde_json::json!({ "status": "notification_ack" }))),
    }
}

pub fn current_runtime_target() -> &'static str {
    if cfg!(target_arch = "wasm32") {
        "cloudflare-workers-wasm32"
    } else {
        "native-vps-tokio"
    }
}

async fn health_handler(State(state): State<P2PMeshState>) -> impl IntoResponse {
    let count = state.catalog.read().map(|c| c.len()).unwrap_or(0);
    Json(serde_json::json!({
        "status": "ok",
        "engine": "hera-node-rs",
        "version": env!("CARGO_PKG_VERSION"),
        "runtime_target": current_runtime_target(),
        "node_id": state.node_id,
        "verified_p2p_tracks": count,
        "dsp_capabilities": [
            "symphonia_pure_rust_decoder",
            "rustfft_spectral_forensics",
            "fake_lossless_brickwall_detector",
            "sub_bass_120hz_phase_correlation",
            "acoustic_4_hot_cues_detector",
            "holistic_4_layer_synergy_2opt_tsp"
        ]
    }))
}

async fn manifest_handler(State(state): State<P2PMeshState>) -> impl IntoResponse {
    Json(NodeManifestResponse {
        service: "Hera Universal Audio DSP & P2P Node".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        runtime_target: current_runtime_target().to_string(),
        node_id: state.node_id,
        memory_budget_mb: if cfg!(target_arch = "wasm32") { 128 } else { 512 },
        invariants: vec![
            "Quarantine-First: Zero unverified files admitted to P2P catalog".to_string(),
            "Zero-Trust Spectral Forensics: Rejects synthetic sine tones & fake MP3->FLAC transcodes".to_string(),
            "Club Subwoofer Phase Safety: Enforces <120 Hz stereo phase correlation >= 0.25".to_string(),
            "Holistic 2-Opt TSP Sequencing: Optimizes global Camelot, BPM, Polyrhythm (3:4), Vocals & 64D Style".to_string(),
        ],
        endpoints: vec![
            "GET  /health".to_string(),
            "GET  /api/v1/node/manifest".to_string(),
            "POST /api/v1/audio/analyze-pcm".to_string(),
            "POST /api/v1/audio/analyze-file".to_string(),
            "POST /api/v1/crates/build".to_string(),
            "GET  /api/v1/p2p/search".to_string(),
            "POST /api/v1/p2p/announce".to_string(),
        ],
        cloud_referrals: get_cloud_referral_catalog(),
    })
}

async fn analyze_pcm_handler(
    Json(req): Json<AnalyzePcmRequest>,
) -> Result<Json<FullDspAnalysisResponse>, HeraError> {
    if req.left.is_empty() {
        return Err(HeraError::BadRequest(
            "PCM `left` channel buffer cannot be empty".to_string(),
        ));
    }

    let forensics = audit_pcm_channels(
        &req.left,
        req.right.as_deref(),
        req.sample_rate,
        &req.claimed_ext,
    );

    let mono: Vec<f32> = if let Some(ref r) = req.right {
        let n = req.left.len().min(r.len());
        req.left[..n]
            .iter()
            .zip(r[..n].iter())
            .map(|(&l, &rv)| 0.5 * (l + rv))
            .collect()
    } else {
        req.left.clone()
    };

    let structure = analyze_structure_and_style(
        &mono,
        req.sample_rate,
        req.full_duration_seconds,
        req.bpm_hint,
    );

    let track_id = req.track_id.unwrap_or_else(|| "pcm-stream".to_string());
    let p2p_eligible = !forensics.is_synthetic_tone
        && forensics.is_authentic_lossless
        && forensics.mono_compatible_sub
        && forensics.quality_score >= 65.0;

    let hera_seal = compute_hera_seal(
        &track_id,
        structure.bpm,
        &structure.camelot_key,
        forensics.quality_score,
        &forensics.forensic_verdict,
    );

    Ok(Json(FullDspAnalysisResponse {
        track_id,
        forensics,
        structure,
        p2p_eligible,
        hera_seal,
    }))
}

async fn analyze_file_handler(
    Query(q): Query<AnalyzeFileQuery>,
    body: Bytes,
) -> Result<Json<FullDspAnalysisResponse>, HeraError> {
    if body.is_empty() {
        return Err(HeraError::BadRequest("Empty audio file body".to_string()));
    }

    let ext = q.ext.as_deref().unwrap_or("wav");
    let claimed_ext = q.claimed_ext.as_deref().unwrap_or(ext);
    let max_sec = q.max_seconds.unwrap_or(45.0).clamp(5.0, 90.0);

    let decoded = decode_audio_bytes(&body, Some(ext), max_sec)
        .map_err(HeraError::UnprocessableEntity)?;

    let forensics = audit_pcm_channels(
        &decoded.left,
        decoded.right.as_deref(),
        decoded.sample_rate,
        claimed_ext,
    );

    let structure = analyze_structure_and_style(
        &decoded.mono,
        decoded.sample_rate,
        Some(decoded.duration_seconds),
        q.bpm_hint,
    );

    let track_id = q.track_id.unwrap_or_else(|| "uploaded-master".to_string());
    let p2p_eligible = !forensics.is_synthetic_tone
        && forensics.is_authentic_lossless
        && forensics.mono_compatible_sub
        && forensics.quality_score >= 65.0;

    let hera_seal = compute_hera_seal(
        &track_id,
        structure.bpm,
        &structure.camelot_key,
        forensics.quality_score,
        &forensics.forensic_verdict,
    );

    Ok(Json(FullDspAnalysisResponse {
        track_id,
        forensics,
        structure,
        p2p_eligible,
        hera_seal,
    }))
}

async fn build_crate_handler(
    Json(req): Json<BuildCrateRequest>,
) -> Result<Json<OptimizedCrateResult>, HeraError> {
    if req.candidates.is_empty() {
        return Err(HeraError::BadRequest(
            "Crate `candidates` list cannot be empty".to_string(),
        ));
    }
    let spine = SetSpineConfig {
        anchors: req.anchors,
        custom_energy_curve: req.custom_energy_curve,
    };
    let result = optimize_crate_with_spine(
        &req.brief,
        &req.energy_arc,
        req.candidates,
        req.max_tracks,
        &spine,
    );
    Ok(Json(result))
}

async fn p2p_search_handler(
    State(state): State<P2PMeshState>,
    Query(query): Query<P2PSearchQuery>,
) -> impl IntoResponse {
    let matches = state.search(&query);
    Json(serde_json::json!({
        "node_id": state.node_id,
        "count": matches.len(),
        "tracks": matches
    }))
}

async fn p2p_announce_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<AnnounceTrackRequest>,
) -> Result<(StatusCode, Json<P2PVerifiedTrack>), (StatusCode, Json<serde_json::Value>)> {
    match state.announce_verified_track(req.track, &req.forensics) {
        Ok(verified) => Ok((StatusCode::CREATED, Json(verified))),
        Err(reason) => Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "status": "quarantine_rejected",
                "reason": reason
            })),
        )),
    }
}

async fn export_crate_handler(Json(req): Json<ExportCrateRequest>) -> impl IntoResponse {
    let resp = export_crate_bundle(&req);
    (StatusCode::OK, Json(resp))
}

async fn dj_style_dna_handler(Json(req): Json<DjStyleDnaRequest>) -> impl IntoResponse {
    let profile = compute_dj_style_dna(&req);
    (StatusCode::OK, Json(profile))
}

async fn ai_backends_list_handler() -> impl IntoResponse {
    let backends = list_supported_ai_backends();
    Json(serde_json::json!({
        "default_backend": "cloudflare_free",
        "free_daily_neurons": 10_000,
        "estimated_free_turns_per_day": 1_500,
        "backends": backends
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProbeQuery {
    pub base_url: Option<String>,
    pub backend: Option<String>,
    pub api_key: Option<String>,
}

async fn ai_probe_backend_handler(Query(q): Query<AiProbeQuery>) -> impl IntoResponse {
    if let Some(url) = q
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let models = probe_backend_models(url, q.api_key.as_deref());
        let reachable = models.is_ok();
        return Json(serde_json::json!({
            "reachable": reachable,
            "backend": q.backend.unwrap_or_else(|| "custom".to_string()),
            "base_url": url,
            "models": models.unwrap_or_default()
        }));
    }

    if let Some((backend, url, _first)) = auto_detect_local_backend() {
        let models = probe_backend_models(&url, None).unwrap_or_default();
        return Json(serde_json::json!({
            "reachable": true,
            "backend": backend,
            "base_url": url,
            "models": models
        }));
    }

    Json(serde_json::json!({
        "reachable": false,
        "backend": "lmstudio",
        "base_url": "http://localhost:1234/v1",
        "models": Vec::<String>::new()
    }))
}

async fn ai_cloudflare_free_chat_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<CloudflareFreeAiTurnRequest>,
) -> impl IntoResponse {
    let resp = evaluate_agent_harness_turn(&req, &state);
    (StatusCode::OK, Json(resp))
}

async fn edge_capabilities_handler(State(state): State<P2PMeshState>) -> impl IntoResponse {
    Json(get_cloudflare_edge_capabilities(&state))
}

async fn edge_forensic_lookup_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<EdgeForensicLookupRequest>,
) -> impl IntoResponse {
    Json(lookup_edge_forensic_cache(&state, &req))
}

async fn edge_forensic_store_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<EdgeForensicStoreRequest>,
) -> impl IntoResponse {
    (StatusCode::CREATED, Json(store_edge_forensic_cache(&state, req)))
}

async fn edge_d1_dna_sync_handler(Json(req): Json<D1DnaSyncRequest>) -> impl IntoResponse {
    Json(sync_d1_dj_dna(req))
}

async fn edge_vectorize_match_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<VectorizeMatchRequest>,
) -> impl IntoResponse {
    Json(query_edge_vectorize_matches(&state, &req))
}

async fn p2p_turn_credentials_handler(
    State(state): State<P2PMeshState>,
    Json(req): Json<TurnCredentialsRequest>,
) -> impl IntoResponse {
    Json(issue_cloudflare_turn_credentials(&state, &req))
}

async fn p2p_turn_relay_proof_handler() -> Result<Json<crate::p2p::TurnRelayTransferReport>, HeraError> {
    let mut audio_bytes = Vec::with_capacity(262_144);
    for i in 0..262_144usize {
        audio_bytes.push(((i.wrapping_mul(73) ^ (i >> 3)) & 0xFF) as u8);
    }
    crate::p2p::run_live_turn_channel_relay_p2p_proof(&audio_bytes)
        .map(Json)
        .map_err(HeraError::Internal)
}

