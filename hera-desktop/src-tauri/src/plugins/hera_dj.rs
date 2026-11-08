use hera_node_rs::dj_brain::{
    auto_detect_local_backend, compute_dj_style_dna, evaluate_agent_harness_turn,
    list_supported_ai_backends, probe_backend_models, AiBackendOption, CloudflareFreeAiTurnRequest,
    CloudflareFreeAiTurnResponse, DjStyleDnaProfile, DjStyleDnaRequest,
};
use hera_node_rs::export::{export_crate_bundle, ExportCrateRequest, ExportCrateResponse};
use hera_node_rs::p2p::P2PMeshState;
use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarmonicBridgeStep {
    pub step_number: u8,
    pub track_id: String,
    pub artist: String,
    pub title: String,
    pub bpm: f32,
    pub camelot: String,
    pub owner_dj: String,
    pub hera_seal: String,
    pub cue_in_seconds: f32,
    pub cue_out_seconds: f32,
    pub synergy_score: f32,
    pub transition_tip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveHarmonicBridgeRequest {
    pub origin_title: String,
    pub origin_artist: String,
    pub origin_bpm: f32,
    pub origin_camelot: String,
    pub target_title: String,
    pub target_artist: String,
    pub target_bpm: f32,
    pub target_camelot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveHarmonicBridgeResponse {
    pub origin_summary: String,
    pub target_summary: String,
    pub harmonic_distance_steps: u8,
    pub bridge_steps: Vec<HarmonicBridgeStep>,
    pub preview_mix_window_seconds: u8,
}

#[tauri::command]
pub fn compute_style_dna(request: DjStyleDnaRequest) -> Result<DjStyleDnaProfile, String> {
    Ok(compute_dj_style_dna(&request))
}

#[tauri::command]
pub fn export_crate(request: ExportCrateRequest) -> Result<ExportCrateResponse, String> {
    Ok(export_crate_bundle(&request))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeAiBackendResponse {
    pub reachable: bool,
    pub backend: String,
    pub base_url: String,
    pub models: Vec<String>,
}

#[tauri::command]
pub fn chat_copilot(
    request: CloudflareFreeAiTurnRequest,
) -> Result<CloudflareFreeAiTurnResponse, String> {
    let state = P2PMeshState::new("hera-desktop-local-node");
    Ok(evaluate_agent_harness_turn(&request, &state))
}

#[tauri::command]
pub fn probe_ai_backend(
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<ProbeAiBackendResponse, String> {
    if let Some(url) = base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let models = probe_backend_models(url, api_key.as_deref());
        let reachable = models.is_ok();
        return Ok(ProbeAiBackendResponse {
            reachable,
            backend: if url.contains("11434") {
                "ollama".to_string()
            } else {
                "lmstudio".to_string()
            },
            base_url: url.to_string(),
            models: models.unwrap_or_default(),
        });
    }
    if let Some((backend, url, _first)) = auto_detect_local_backend() {
        let models = probe_backend_models(&url, api_key.as_deref()).unwrap_or_default();
        return Ok(ProbeAiBackendResponse {
            reachable: true,
            backend,
            base_url: url,
            models,
        });
    }
    Ok(ProbeAiBackendResponse {
        reachable: false,
        backend: "lmstudio".to_string(),
        base_url: "http://localhost:1234/v1".to_string(),
        models: Vec::new(),
    })
}

#[tauri::command]
pub fn list_ai_backends() -> Result<Vec<AiBackendOption>, String> {
    Ok(list_supported_ai_backends())
}

#[tauri::command]
pub fn resolve_harmonic_bridge(
    request: ResolveHarmonicBridgeRequest,
) -> Result<ResolveHarmonicBridgeResponse, String> {
    let mid_bpm_1 = request.origin_bpm + (request.target_bpm - request.origin_bpm) * 0.35;
    let mid_bpm_2 = request.origin_bpm + (request.target_bpm - request.origin_bpm) * 0.72;

    let step1_camelot = if request.origin_camelot == request.target_camelot {
        request.origin_camelot.clone()
    } else {
        "9A".to_string()
    };
    let step2_camelot = request.target_camelot.clone();

    let bridge_steps = vec![
        HarmonicBridgeStep {
            step_number: 1,
            track_id: "p2p-bridge-01".to_string(),
            artist: "Keinemusik & Rampa".to_string(),
            title: "Cesonia (Afro-Tech P2P Bridge Edit)".to_string(),
            bpm: (mid_bpm_1 * 10.0).round() / 10.0,
            camelot: step1_camelot.clone(),
            owner_dj: "DaviDJ_EdgeGuardian".to_string(),
            hera_seal: "hera_seal_9f81c2e4b0a1".to_string(),
            cue_in_seconds: 15.2,
            cue_out_seconds: 168.4,
            synergy_score: 0.96,
            transition_tip: format!(
                "Cruza Cue 4 (Mix Out) de '{}' con Cue 1 (Intro) en {} a {:.1} BPM",
                request.origin_title, step1_camelot, mid_bpm_1
            ),
        },
        HarmonicBridgeStep {
            step_number: 2,
            track_id: "p2p-bridge-02".to_string(),
            artist: "Hugelkultur & Hugel".to_string(),
            title: "Morenita (Latin Club Harmonic Connector)".to_string(),
            bpm: (mid_bpm_2 * 10.0).round() / 10.0,
            camelot: step2_camelot.clone(),
            owner_dj: "DJFoo_MedellinNode".to_string(),
            hera_seal: "hera_seal_4d72a8f103e9".to_string(),
            cue_in_seconds: 14.8,
            cue_out_seconds: 174.0,
            synergy_score: 0.95,
            transition_tip: format!(
                "Entrega en llave {} ({:.1} BPM) directo al Cue 1 (Intro) de '{}'",
                step2_camelot, mid_bpm_2, request.target_title
            ),
        },
    ];

    Ok(ResolveHarmonicBridgeResponse {
        origin_summary: format!(
            "{} — {} ({:.1} BPM · {})",
            request.origin_artist, request.origin_title, request.origin_bpm, request.origin_camelot
        ),
        target_summary: format!(
            "{} — {} ({:.1} BPM · {})",
            request.target_artist, request.target_title, request.target_bpm, request.target_camelot
        ),
        harmonic_distance_steps: 2,
        bridge_steps,
        preview_mix_window_seconds: 15,
    })
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("hera-dj")
        .invoke_handler(tauri::generate_handler![
            compute_style_dna,
            export_crate,
            chat_copilot,
            probe_ai_backend,
            list_ai_backends,
            resolve_harmonic_bridge
        ])
        .build()
}
