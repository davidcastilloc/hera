mod bridge_p2p;
mod mini_overlay;
mod p2p_download;
#[cfg(test)]
mod tests;
mod virtualdj_live;

use hera_node_rs::cloudflare_edge::{CloudflareEdgeCapabilities, get_cloudflare_edge_capabilities};
use hera_node_rs::dj_brain::{
    AiBackendOption, AiConnectivityOnboardingReport, CloudflareFreeAiTurnRequest,
    CloudflareFreeAiTurnResponse, DjStyleDnaProfile, DjStyleDnaRequest, auto_detect_local_backend,
    compute_dj_style_dna, evaluate_agent_harness_turn, list_supported_ai_backends,
    probe_backend_models, scan_ai_connectivity_onboarding,
};
use hera_node_rs::export::{
    ExportCrateRequest, ExportCrateResponse, ExportTrackItem, export_crate_bundle,
};
use hera_node_rs::p2p::{
    ConnectDjDriveRequest, DesktopP2PDaemonStatus, DjDriveTrackInput, P2PMeshState,
    compute_sha256_hex, request_chunk_with_turn_fallback, spawn_desktop_udp_p2p_daemon,
};
use hera_node_rs::router::BuildCrateRequest;
use hera_node_rs::synergy::{OptimizedCrateResult, SetSpineConfig, optimize_crate_with_spine};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime};

#[allow(unused_imports)]
pub use bridge_p2p::*;
pub use hera_ipc::dj_types::*;
#[allow(unused_imports)]
pub use mini_overlay::*;
#[allow(unused_imports)]
pub use p2p_download::*;
#[allow(unused_imports)]
pub use virtualdj_live::*;

#[tauri::command]
pub async fn compute_style_dna(request: DjStyleDnaRequest) -> Result<DjStyleDnaProfile, String> {
    tauri::async_runtime::spawn_blocking(move || {
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
    })
    .await
    .map_err(|e| format!("Fallo en tarea compute_style_dna: {e}"))?
}

#[tauri::command]
pub async fn export_crate(request: ExportCrateRequest) -> Result<ExportCrateResponse, String> {
    tauri::async_runtime::spawn_blocking(move || Ok(export_crate_bundle(&request)))
        .await
        .map_err(|e| format!("Fallo en tarea export_crate: {e}"))?
}

#[tauri::command]
pub async fn build_dj_set_spine(
    request: BuildCrateRequest,
) -> Result<OptimizedCrateResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if request.candidates.is_empty() {
            return Err(
                "La lista de pistas candidatas para ordenar no puede estar vacía".to_string(),
            );
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
    })
    .await
    .map_err(|e| format!("Fallo en tarea build_dj_set_spine: {e}"))?
}

#[tauri::command]
pub async fn get_cloudflare_edge_radar() -> Result<CloudflareEdgeCapabilities, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let state = shared_desktop_p2p_state();
        Ok(get_cloudflare_edge_capabilities(state))
    })
    .await
    .map_err(|e| format!("Fallo en tarea get_cloudflare_edge_radar: {e}"))?
}

#[tauri::command]
pub async fn chat_copilot(
    request: CloudflareFreeAiTurnRequest,
) -> Result<CloudflareFreeAiTurnResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = shared_desktop_p2p_state();
        Ok(evaluate_agent_harness_turn(&request, state))
    })
    .await
    .map_err(|e| format!("Fallo en tarea chat_copilot: {e}"))?
}

#[tauri::command]
pub async fn probe_ai_backend(
    base_url: Option<String>,
    api_key: Option<String>,
) -> Result<ProbeAiBackendResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(url) = base_url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
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
    })
    .await
    .map_err(|e| format!("Fallo en tarea probe_ai_backend: {e}"))?
}

#[tauri::command]
pub async fn list_ai_backends() -> Result<Vec<AiBackendOption>, String> {
    Ok(list_supported_ai_backends())
}

#[tauri::command]
pub async fn scan_ai_connectivity(
    commander_mode: Option<String>,
) -> Result<AiConnectivityOnboardingReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(scan_ai_connectivity_onboarding(commander_mode.as_deref()))
    })
    .await
    .map_err(|e| format!("Fallo en tarea scan_ai_connectivity: {e}"))?
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    warm_desktop_udp_daemon_background();
    Builder::<R>::new("hera-dj")
        .invoke_handler(tauri::generate_handler![
            compute_style_dna,
            export_crate,
            build_dj_set_spine,
            get_cloudflare_edge_radar,
            chat_copilot,
            probe_ai_backend,
            list_ai_backends,
            scan_ai_connectivity,
            bridge_p2p::resolve_harmonic_bridge,
            p2p_download::download_p2p_track,
            mini_overlay::set_mini_overlay_mode,
            virtualdj_live::push_virtualdj_live_crate,
            virtualdj_live::read_virtualdj_now_playing
        ])
        .build()
}
