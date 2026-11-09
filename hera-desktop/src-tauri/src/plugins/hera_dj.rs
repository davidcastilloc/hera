use hera_node_rs::cloudflare_edge::{
    get_cloudflare_edge_capabilities, CloudflareEdgeCapabilities,
};
use hera_node_rs::dj_brain::{
    auto_detect_local_backend, compute_dj_style_dna, evaluate_agent_harness_turn,
    list_supported_ai_backends, probe_backend_models, AiBackendOption, CloudflareFreeAiTurnRequest,
    CloudflareFreeAiTurnResponse, DjStyleDnaProfile, DjStyleDnaRequest,
};
use hera_node_rs::export::{export_crate_bundle, ExportCrateRequest, ExportCrateResponse};
use hera_node_rs::p2p::{
    spawn_desktop_udp_p2p_daemon, ConnectDjDriveRequest, DesktopP2PDaemonStatus, DjDriveTrackInput,
    P2PMeshState,
};
use hera_node_rs::router::BuildCrateRequest;
use hera_node_rs::synergy::{optimize_crate_with_spine, OptimizedCrateResult, SetSpineConfig};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

fn shared_desktop_p2p_state() -> &'static Arc<P2PMeshState> {
    static STATE: OnceLock<Arc<P2PMeshState>> = OnceLock::new();
    STATE.get_or_init(|| Arc::new(P2PMeshState::new("hera-desktop-local-node")))
}

fn shared_desktop_udp_daemon() -> Option<DesktopP2PDaemonStatus> {
    static DAEMON: OnceLock<Option<DesktopP2PDaemonStatus>> = OnceLock::new();
    DAEMON
        .get_or_init(|| {
            spawn_desktop_udp_p2p_daemon(Arc::clone(shared_desktop_p2p_state()))
                .ok()
                .map(|(status, _counter)| status)
        })
        .clone()
}

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
    #[serde(default)]
    pub p2p_boost_free_promo: bool,
    #[serde(default)]
    pub boost_credit_cost_mb: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmCatalogItem {
    pub cas_fingerprint: String,
    pub artist: String,
    pub title: String,
    pub bpm: f32,
    pub camelot: String,
    pub subgenre: String,
    pub hera_seal: String,
    pub opaque_blob_key: String,
    pub swarm_replicas_count: usize,
    pub seeders: Vec<String>,
    pub p2p_boost_ready: bool,
    #[serde(default)]
    pub p2p_boost_free_promo: bool,
    #[serde(default)]
    pub effective_bandwidth_hz: f32,
    #[serde(default)]
    pub quality_score: f32,
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
    #[serde(default)]
    pub swarm_catalog: Vec<SwarmCatalogItem>,
    #[serde(default)]
    pub stun_public_endpoint: Option<String>,
}

#[tauri::command]
pub fn compute_style_dna(request: DjStyleDnaRequest) -> Result<DjStyleDnaProfile, String> {
    if !request.tracks.is_empty() {
        let state = shared_desktop_p2p_state();
        let p2p_tracks: Vec<DjDriveTrackInput> = request
            .tracks
            .iter()
            .map(|t| DjDriveTrackInput {
                title: t.title.clone(),
                artist: t.artist.clone(),
                bpm: t.bpm,
                camelot_key: t.camelot.clone(),
                top_subgenre: "Club / Local Desktop Seeder".to_string(),
                duration_seconds: t.duration_seconds,
                format: "flac".to_string(),
                gdrive_file_link_or_id: None,
                r2_object_key: None,
                cloud_share_url: None,
                file_size_mb: 34.0,
                effective_bandwidth_hz: 21200.0,
                sub_bass_phase_correlation: 0.94,
                is_synthetic_tone: false,
                share_on_p2p: Some(true),
                audio_sha256: None,
            })
            .collect();
        let handle = request
            .dj_handle
            .clone()
            .unwrap_or_else(|| "DaviDJ".to_string());
        let _ = state.connect_dj_drive(ConnectDjDriveRequest {
            dj_handle: handle.clone(),
            display_name: Some(format!("{} (HERA Desktop Node)", handle)),
            gdrive_folder_link: None,
            gdrive_embedded_html: None,
            r2_bucket_url: None,
            subscription_tier: Some("local_unlimited".to_string()),
            p2p_sharing_enabled: Some(true),
            tracks: p2p_tracks,
        });
    }
    Ok(compute_dj_style_dna(&request))
}

#[tauri::command]
pub fn export_crate(request: ExportCrateRequest) -> Result<ExportCrateResponse, String> {
    Ok(export_crate_bundle(&request))
}

#[tauri::command]
pub fn build_dj_set_spine(request: BuildCrateRequest) -> Result<OptimizedCrateResult, String> {
    if request.candidates.is_empty() {
        return Err("La lista de pistas candidatas para ordenar no puede estar vacía".to_string());
    }
    let spine = SetSpineConfig {
        anchors: request.anchors,
        custom_energy_curve: request.custom_energy_curve,
    };
    Ok(optimize_crate_with_spine(
        &request.brief,
        &request.energy_arc,
        request.candidates,
        request.max_tracks,
        &spine,
    ))
}

#[tauri::command]
pub fn get_cloudflare_edge_radar() -> Result<CloudflareEdgeCapabilities, String> {
    let state = shared_desktop_p2p_state();
    Ok(get_cloudflare_edge_capabilities(state))
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
    let state = shared_desktop_p2p_state();
    Ok(evaluate_agent_harness_turn(&request, state))
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
            p2p_boost_free_promo: true,
            boost_credit_cost_mb: 0.0,
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
            p2p_boost_free_promo: false,
            boost_credit_cost_mb: 38.0,
        },
    ];

    let state = shared_desktop_p2p_state();
    let mut swarm_catalog: Vec<SwarmCatalogItem> = state
        .build_broker_swarm_index()
        .into_iter()
        .enumerate()
        .map(|(idx, rec)| {
            let p2p_boost_ready = rec.available_routes.iter().any(|r| {
                r.storage_provider == "r2" || r.storage_provider == "r2_cas_zero_copy_link"
            }) || idx == 0;
            let seeders = rec
                .available_routes
                .into_iter()
                .map(|r| format!("{} ({})", r.owner_dj, r.storage_provider))
                .collect();
            SwarmCatalogItem {
                cas_fingerprint: rec.cas_fingerprint,
                artist: rec.canonical_artist,
                title: rec.canonical_title,
                bpm: rec.bpm,
                camelot: rec.camelot_key,
                subgenre: rec.top_subgenre,
                hera_seal: rec.hera_seal,
                opaque_blob_key: rec.opaque_blob_key,
                swarm_replicas_count: rec.swarm_replicas_count,
                seeders,
                p2p_boost_ready,
                p2p_boost_free_promo: idx == 0,
                effective_bandwidth_hz: rec.effective_bandwidth_hz,
                quality_score: rec.quality_score,
            }
        })
        .collect();

    for step in &bridge_steps {
        swarm_catalog.push(SwarmCatalogItem {
            cas_fingerprint: format!("cas-{}-{:.0}bpm", step.track_id, step.bpm),
            artist: step.artist.clone(),
            title: step.title.clone(),
            bpm: step.bpm,
            camelot: step.camelot.clone(),
            subgenre: if step.p2p_boost_free_promo {
                "Afro-Tech / Promo Pool".to_string()
            } else {
                "Latin Tech House".to_string()
            },
            hera_seal: step.hera_seal.clone(),
            opaque_blob_key: format!("shards/boost/{}.shard", step.track_id),
            swarm_replicas_count: 3,
            seeders: vec![
                format!("{} (r2_boost)", step.owner_dj),
                "DaviDJ (local_disk)".to_string(),
            ],
            p2p_boost_ready: true,
            p2p_boost_free_promo: step.p2p_boost_free_promo,
            effective_bandwidth_hz: 21600.0,
            quality_score: 98.4,
        });
    }

    let stun_public_endpoint = shared_desktop_udp_daemon().map(|d| {
        format!(
            "{} · {} (LAN {})",
            d.nat_diagnosis.primary_stun_endpoint,
            d.nat_diagnosis.nat_topology,
            d.nat_diagnosis.lan_direct_endpoint
        )
    });

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
        swarm_catalog,
        stun_public_endpoint,
    })
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("hera-dj")
        .invoke_handler(tauri::generate_handler![
            compute_style_dna,
            export_crate,
            build_dj_set_spine,
            get_cloudflare_edge_radar,
            chat_copilot,
            probe_ai_backend,
            list_ai_backends,
            resolve_harmonic_bridge
        ])
        .build()
}
