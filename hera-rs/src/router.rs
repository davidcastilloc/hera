use crate::forensics::{audit_pcm_channels, decode_audio_bytes, ForensicAuditReport};
use crate::mcp::{handle_mcp_request, list_mcp_tools, JsonRpcRequest};
use crate::p2p::{
    compute_hera_seal, get_cloud_referral_catalog, CloudReferralOption, P2PMeshState,
    P2PSearchQuery, P2PVerifiedTrack,
};
use crate::structure::{analyze_structure_and_style, StructuralCueProfile};
use crate::synergy::{optimize_crate_2opt, CrateTrackCandidate, OptimizedCrateResult};
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Query, State},
    http::StatusCode,
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
        .route("/mcp", post(mcp_http_handler))
        .route("/mcp/tools", get(mcp_tools_handler))
        .route("/api/v1/node/manifest", get(manifest_handler))
        .route("/api/v1/audio/analyze-pcm", post(analyze_pcm_handler))
        .route("/api/v1/audio/analyze-file", post(analyze_file_handler))
        .route("/api/v1/crates/build", post(build_crate_handler))
        .route("/api/v1/p2p/search", get(p2p_search_handler))
        .route("/api/v1/p2p/announce", post(p2p_announce_handler))
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024))
        .with_state(state)
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
) -> Result<Json<FullDspAnalysisResponse>, (StatusCode, Json<serde_json::Value>)> {
    if req.left.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "PCM `left` channel buffer cannot be empty" })),
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
) -> Result<Json<FullDspAnalysisResponse>, (StatusCode, Json<serde_json::Value>)> {
    if body.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Empty audio file body" })),
        ));
    }

    let ext = q.ext.as_deref().unwrap_or("wav");
    let claimed_ext = q.claimed_ext.as_deref().unwrap_or(ext);
    let max_sec = q.max_seconds.unwrap_or(45.0).clamp(5.0, 90.0);

    let decoded = decode_audio_bytes(&body, Some(ext), max_sec).map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": e })),
        )
    })?;

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
) -> Result<Json<OptimizedCrateResult>, (StatusCode, Json<serde_json::Value>)> {
    if req.candidates.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Crate `candidates` list cannot be empty" })),
        ));
    }
    let result = optimize_crate_2opt(&req.brief, &req.energy_arc, req.candidates, req.max_tracks);
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
