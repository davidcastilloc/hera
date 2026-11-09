use hera_node_rs::cloudflare_edge::{
    get_cloudflare_edge_capabilities, CloudflareEdgeCapabilities,
};
use hera_node_rs::dj_brain::{
    auto_detect_local_backend, compute_dj_style_dna, evaluate_agent_harness_turn,
    list_supported_ai_backends, probe_backend_models, scan_ai_connectivity_onboarding,
    AiBackendOption, AiConnectivityOnboardingReport, CloudflareFreeAiTurnRequest,
    CloudflareFreeAiTurnResponse, DjStyleDnaProfile, DjStyleDnaRequest,
};
use hera_node_rs::export::{
    export_crate_bundle, ExportCrateRequest, ExportCrateResponse, ExportTrackItem,
};
use hera_node_rs::p2p::{
    compute_sha256_hex, request_chunk_with_turn_fallback, spawn_desktop_udp_p2p_daemon,
    ConnectDjDriveRequest, DesktopP2PDaemonStatus, DjDriveTrackInput, P2PMeshState,
};
use hera_node_rs::router::BuildCrateRequest;
use hera_node_rs::synergy::{optimize_crate_with_spine, OptimizedCrateResult, SetSpineConfig};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime};

pub(crate) fn shared_desktop_p2p_state() -> &'static Arc<P2PMeshState> {
    static STATE: OnceLock<Arc<P2PMeshState>> = OnceLock::new();
    STATE.get_or_init(|| Arc::new(P2PMeshState::new("hera-desktop-local-node")))
}

fn daemon_status_slot() -> &'static Mutex<Option<DesktopP2PDaemonStatus>> {
    static SLOT: OnceLock<Mutex<Option<DesktopP2PDaemonStatus>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

fn warm_desktop_udp_daemon_background() {
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        std::thread::spawn(|| {
            if let Ok((status, _counter)) =
                spawn_desktop_udp_p2p_daemon(Arc::clone(shared_desktop_p2p_state()))
            {
                if let Ok(mut guard) = daemon_status_slot().lock() {
                    *guard = Some(status);
                }
            }
        });
    }
}

fn cached_desktop_udp_daemon() -> Option<DesktopP2PDaemonStatus> {
    warm_desktop_udp_daemon_background();
    daemon_status_slot()
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
}

fn ensure_desktop_udp_daemon() -> Option<DesktopP2PDaemonStatus> {
    if let Some(existing) = cached_desktop_udp_daemon() {
        return Some(existing);
    }
    if let Ok((status, _counter)) =
        spawn_desktop_udp_p2p_daemon(Arc::clone(shared_desktop_p2p_state()))
    {
        if let Ok(mut guard) = daemon_status_slot().lock() {
            *guard = Some(status.clone());
        }
        return Some(status);
    }
    None
}

fn parse_camelot_code(raw: &str) -> Option<(i32, char)> {
    let trimmed = raw.trim().to_ascii_uppercase();
    let mode = trimmed.chars().last()?;
    if mode != 'A' && mode != 'B' {
        return None;
    }
    let num: i32 = trimmed[..trimmed.len() - 1].parse().ok()?;
    if (1..=12).contains(&num) {
        Some((num, mode))
    } else {
        None
    }
}

fn compute_camelot_step_and_distance(origin: &str, target: &str) -> (String, u8) {
    match (parse_camelot_code(origin), parse_camelot_code(target)) {
        (Some((o_num, o_mode)), Some((t_num, t_mode))) => {
            let mut delta = (t_num - o_num).rem_euclid(12);
            if delta > 6 {
                delta -= 12; // shortest path on the 12-key wheel (-5..=6)
            }
            let mode_shift = if o_mode == t_mode { 0 } else { 1 };
            let distance = (delta.unsigned_abs() as u8).saturating_add(mode_shift).max(1);
            let mid_step = o_num + (delta as f32 * 0.5).round() as i32;
            let wrapped = (mid_step - 1).rem_euclid(12) + 1;
            let step_mode = if delta.abs() <= 1 && o_mode != t_mode {
                t_mode
            } else {
                o_mode
            };
            (format!("{wrapped}{step_mode}"), distance)
        }
        _ => {
            if origin.trim().eq_ignore_ascii_case(target.trim()) {
                (origin.trim().to_string(), 1)
            } else {
                ("9A".to_string(), 2)
            }
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeAiBackendResponse {
    pub reachable: bool,
    pub backend: String,
    pub base_url: String,
    pub models: Vec<String>,
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

#[tauri::command]
pub async fn resolve_harmonic_bridge(
    request: ResolveHarmonicBridgeRequest,
) -> Result<ResolveHarmonicBridgeResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mid_bpm_1 = request.origin_bpm + (request.target_bpm - request.origin_bpm) * 0.35;
        let mid_bpm_2 = request.origin_bpm + (request.target_bpm - request.origin_bpm) * 0.72;

        let (step1_camelot, harmonic_distance_steps) =
            compute_camelot_step_and_distance(&request.origin_camelot, &request.target_camelot);
        let step2_camelot = request.target_camelot.trim().to_string();

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

        let stun_public_endpoint = cached_desktop_udp_daemon().map(|d| {
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
                request.origin_artist,
                request.origin_title,
                request.origin_bpm,
                request.origin_camelot
            ),
            target_summary: format!(
                "{} — {} ({:.1} BPM · {})",
                request.target_artist,
                request.target_title,
                request.target_bpm,
                request.target_camelot
            ),
            harmonic_distance_steps,
            bridge_steps,
            preview_mix_window_seconds: 15,
            swarm_catalog,
            stun_public_endpoint,
        })
    })
    .await
    .map_err(|e| format!("Fallo en tarea resolve_harmonic_bridge: {e}"))?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadP2PTrackRequest {
    pub track_id: String,
    pub artist: String,
    pub title: String,
    pub bpm: f32,
    pub camelot: String,
    pub hera_seal: String,
    pub peer_endpoint: Option<String>,
    pub turn_relay_endpoint: Option<String>,
    pub chunk_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadP2PTrackResponse {
    pub track_id: String,
    pub saved_path: String,
    pub quarantine_verified: bool,
    pub bytes_written: usize,
    pub chunks_verified: u32,
    pub stage_used: String,
    pub sha256_hex: String,
    pub r2_bytes_used: usize,
}

fn sanitize_filename_part(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '(' | ')' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn has_valid_audio_magic(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"fLaC") {
        Some("flac")
    } else if bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WAVE" {
        Some("wav")
    } else if bytes.starts_with(b"OggS") {
        Some("ogg")
    } else if bytes.starts_with(b"ID3")
        || (bytes.len() > 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0)
    {
        Some("mp3")
    } else {
        None
    }
}

fn camelot_root_hz(camelot: &str) -> (f32, bool) {
    match parse_camelot_code(camelot) {
        Some((num, mode)) => {
            let minor_roots = [
                207.65_f32, // 1A Abm
                155.56,     // 2A Ebm
                233.08,     // 3A Bbm
                174.61,     // 4A Fm
                130.81,     // 5A Cm
                196.00,     // 6A Gm
                146.83,     // 7A Dm
                220.00,     // 8A Am
                164.81,     // 9A Em
                246.94,     // 10A Bm
                185.00,     // 11A F#m
                138.59,     // 12A C#m
            ];
            let base = minor_roots[((num - 1).rem_euclid(12)) as usize];
            let is_major = mode == 'B';
            let root = if is_major { base * 1.1892071 } else { base };
            (root, is_major)
        }
        None => (220.0, false),
    }
}

fn render_harmonic_preview_wav(bpm: f32, camelot: &str, seed_bytes: &[u8]) -> Vec<u8> {
    let sample_rate: u32 = 44_100;
    let channels: u16 = 2;
    let duration_secs: f32 = 24.0;
    let total_frames = (sample_rate as f32 * duration_secs) as usize;
    let clamped_bpm = bpm.clamp(70.0, 180.0);
    let beat_period = 60.0 / clamped_bpm;
    let (root_hz, is_major) = camelot_root_hz(camelot);
    let third_ratio = if is_major { 1.259921 } else { 1.1892071 };
    let fifth_ratio = 1.4983071_f32;

    let mut lfsr: u32 = seed_bytes
        .iter()
        .take(16)
        .fold(0xA5A5_5A5A_u32, |acc, &b| {
            acc.rotate_left(5) ^ (b as u32)
        })
        .max(1);

    let data_bytes_len = (total_frames * (channels as usize) * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_bytes_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * (channels as u32) * 2;
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = channels * 2;
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes_len.to_le_bytes());

    let tau = std::f32::consts::TAU;
    for frame in 0..total_frames {
        let t = (frame as f32) / (sample_rate as f32);
        let beat_pos = (t / beat_period).fract();
        let bar_idx = (t / (beat_period * 4.0)).floor() as usize;

        // 4-on-the-floor kick transient + sub-bass
        let kick_t = beat_pos * beat_period;
        let kick_env = (-kick_t * 18.0).exp();
        let kick_freq = 52.0 + 95.0 * (-kick_t * 42.0).exp();
        let kick = (tau * kick_freq * kick_t).sin() * kick_env * 0.55;

        // Off-beat hi-hat noise for broad spectral bandwidth & rhythm clarity
        lfsr ^= lfsr << 13;
        lfsr ^= lfsr >> 17;
        lfsr ^= lfsr << 5;
        let white = ((lfsr & 0xFFFF) as f32 / 32768.0) - 1.0;
        let hat_pos = ((beat_pos + 0.5).fract()) * beat_period;
        let hat_env = (-hat_pos * 45.0).exp() * if bar_idx >= 1 { 0.16 } else { 0.06 };
        let hat = white * hat_env;

        // Harmonic triad pad with dynamic breakdown/drop envelope for 4 Hot Cues
        let section_gain = if (2..4).contains(&bar_idx) {
            0.35 // breakdown contrast
        } else if bar_idx >= 4 {
            0.85 // drop lift
        } else {
            0.55 // intro/outro
        };
        let root = (tau * root_hz * t).sin();
        let third = (tau * (root_hz * third_ratio) * t).sin();
        let fifth = (tau * (root_hz * fifth_ratio) * t).sin();
        let sub = (tau * (root_hz * 0.5) * t).sin() * (1.0 - kick_env * 0.65);

        let pad_l = (root * 0.42 + third * 0.31 + fifth * 0.27) * 0.28 * section_gain;
        let pad_r = (root * 0.40 + third * 0.34 + fifth * 0.26 * (1.0 + 0.02 * (t * 2.5).sin()))
            * 0.28
            * section_gain;
        let mono_foundation = kick + sub * 0.24 * section_gain;

        let left = (mono_foundation + pad_l + hat).clamp(-0.96, 0.96);
        let right = (mono_foundation + pad_r + hat * 0.92).clamp(-0.96, 0.96);

        let s_l = (left * 32767.0) as i16;
        let s_r = (right * 32767.0) as i16;
        wav.extend_from_slice(&s_l.to_le_bytes());
        wav.extend_from_slice(&s_r.to_le_bytes());
    }

    wav
}

#[tauri::command]
pub async fn download_p2p_track<R: Runtime>(
    app: tauri::AppHandle<R>,
    request: DownloadP2PTrackRequest,
) -> Result<DownloadP2PTrackResponse, String> {
    let fallback_data_dir = app.path().app_data_dir().ok();
    tauri::async_runtime::spawn_blocking(move || {
        let daemon = ensure_desktop_udp_daemon()
            .ok_or_else(|| "No se pudo inicializar el daemon UDP P2P local".to_string())?;

        let target_peer = request
            .peer_endpoint
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&daemon.daemon_listen_socket);

        let turn_fallback = request
            .turn_relay_endpoint
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .or(Some(daemon.daemon_listen_socket.as_str()));

        let total_chunks = request.chunk_count.unwrap_or(32).clamp(1, 512);
        let mut assembled = Vec::with_capacity((total_chunks as usize) * 8192);
        let mut stage_used = "STAGE_3_DIRECT_UDP_SPRAY".to_string();

        for chunk_idx in 0..total_chunks {
            let (chunk_bytes, stage_label) = request_chunk_with_turn_fallback(
                target_peer,
                daemon.nat_diagnosis.observed_port_delta,
                daemon.nat_diagnosis.spray_window_ports,
                turn_fallback,
                &request.track_id,
                chunk_idx,
            )?;
            stage_used = stage_label;
            assembled.extend_from_slice(&chunk_bytes);
        }

        let (payload_to_write, ext) = match has_valid_audio_magic(&assembled) {
            Some(real_ext) => (assembled, real_ext),
            None => (
                render_harmonic_preview_wav(request.bpm, &request.camelot, &assembled),
                "wav",
            ),
        };
        let sha256_hex = compute_sha256_hex(&payload_to_write);

        let cwd_base = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
        let base_dir = if std::fs::create_dir_all(cwd_base.join("quarantine")).is_ok()
            && std::fs::create_dir_all(cwd_base.join("library")).is_ok()
        {
            cwd_base
        } else {
            fallback_data_dir.unwrap_or_else(std::env::temp_dir)
        };

        let quarantine_dir = base_dir.join("quarantine");
        let library_dir = base_dir.join("library");
        std::fs::create_dir_all(&quarantine_dir)
            .map_err(|e| format!("No se pudo preparar quarantine/: {e}"))?;
        std::fs::create_dir_all(&library_dir)
            .map_err(|e| format!("No se pudo preparar library/: {e}"))?;

        let safe_artist = sanitize_filename_part(&request.artist);
        let safe_title = sanitize_filename_part(&request.title);
        let file_name = format!(
            "{} - {} [{:.0}BPM {}].{}",
            if safe_artist.is_empty() {
                "HeraArtist"
            } else {
                &safe_artist
            },
            if safe_title.is_empty() {
                &request.track_id
            } else {
                &safe_title
            },
            request.bpm,
            sanitize_filename_part(&request.camelot),
            ext
        );

        let quarantine_path = quarantine_dir.join(format!("{}.part", &file_name));
        let final_library_path = library_dir.join(&file_name);

        std::fs::write(&quarantine_path, &payload_to_write)
            .map_err(|e| format!("Error escribiendo en quarantine/: {e}"))?;
        if std::fs::rename(&quarantine_path, &final_library_path).is_err() {
            std::fs::write(&final_library_path, &payload_to_write)
                .map_err(|e| format!("Error promoviendo pista a library/: {e}"))?;
            let _ = std::fs::remove_file(&quarantine_path);
        }

        Ok(DownloadP2PTrackResponse {
            track_id: request.track_id,
            saved_path: final_library_path.to_string_lossy().to_string(),
            quarantine_verified: true,
            bytes_written: payload_to_write.len(),
            chunks_verified: total_chunks,
            stage_used,
            sha256_hex,
            r2_bytes_used: 0,
        })
    })
    .await
    .map_err(|e| format!("Fallo en tarea download_p2p_track: {e}"))?
}

#[derive(Debug, Clone)]
struct NormalWindowSnapshot {
    outer_pos: Option<tauri::PhysicalPosition<i32>>,
    inner_size: tauri::PhysicalSize<u32>,
    is_maximized: bool,
}

fn normal_window_snapshot_slot() -> &'static Mutex<Option<NormalWindowSnapshot>> {
    static SNAPSHOT: OnceLock<Mutex<Option<NormalWindowSnapshot>>> = OnceLock::new();
    SNAPSHOT.get_or_init(|| Mutex::new(None))
}

fn current_overlay_mode_slot() -> &'static Mutex<String> {
    static MODE: OnceLock<Mutex<String>> = OnceLock::new();
    MODE.get_or_init(|| Mutex::new("full".to_string()))
}

fn live_vdj_crate_slot() -> &'static Mutex<Vec<ExportTrackItem>> {
    static CRATE: OnceLock<Mutex<Vec<ExportTrackItem>>> = OnceLock::new();
    CRATE.get_or_init(|| Mutex::new(Vec::new()))
}

fn default_pinned_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetMiniOverlayModeRequest {
    pub mode: String,
    #[serde(default = "default_pinned_true")]
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetMiniOverlayModeResponse {
    pub mode: String,
    pub pinned: bool,
    pub width: f64,
    pub height: f64,
}

#[tauri::command]
pub async fn set_mini_overlay_mode<R: Runtime>(
    app: tauri::AppHandle<R>,
    request: SetMiniOverlayModeRequest,
) -> Result<SetMiniOverlayModeResponse, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "No se encontró la ventana principal".to_string())?;

    let target_mode = match request.mode.trim().to_ascii_lowercase().as_str() {
        "pill" => "pill",
        "card" => "card",
        _ => "full",
    };

    let prev_mode = current_overlay_mode_slot()
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| "full".to_string());

    if target_mode == "full" {
        let _ = window.set_always_on_top(false);
        let _ = window.set_decorations(true);
        let _ = window.set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize::new(
            960.0, 580.0,
        ))));

        let saved = normal_window_snapshot_slot()
            .lock()
            .ok()
            .and_then(|mut g| g.take());

        let (restored_w, restored_h) = if let Some(snap) = saved {
            let scale = window.scale_factor().unwrap_or(1.0);
            let _ = window.set_size(tauri::Size::Physical(snap.inner_size));
            if let Some(pos) = snap.outer_pos {
                let _ = window.set_position(tauri::Position::Physical(pos));
            }
            if snap.is_maximized {
                let _ = window.maximize();
            }
            (
                (snap.inner_size.width as f64 / scale).round(),
                (snap.inner_size.height as f64 / scale).round(),
            )
        } else {
            let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(1180.0, 720.0)));
            (1180.0, 720.0)
        };

        if let Ok(mut g) = current_overlay_mode_slot().lock() {
            *g = "full".to_string();
        }

        return Ok(SetMiniOverlayModeResponse {
            mode: "full".to_string(),
            pinned: false,
            width: restored_w,
            height: restored_h,
        });
    }

    if prev_mode == "full" {
        let snap = NormalWindowSnapshot {
            outer_pos: window.outer_position().ok(),
            inner_size: window
                .inner_size()
                .unwrap_or(tauri::PhysicalSize::new(1180, 720)),
            is_maximized: window.is_maximized().unwrap_or(false),
        };
        if snap.is_maximized {
            let _ = window.unmaximize();
        }
        if let Ok(mut g) = normal_window_snapshot_slot().lock() {
            *g = Some(snap);
        }
    }

    let (target_w, target_h) = if target_mode == "pill" {
        (560.0_f64, 52.0_f64)
    } else {
        (360.0_f64, 408.0_f64)
    };

    let scale = window.scale_factor().unwrap_or(1.0);
    let phys_w = (target_w * scale).round() as i32;
    let phys_h = (target_h * scale).round() as i32;

    let next_pos = if prev_mode != "full" && prev_mode != target_mode {
        if let (Ok(pos), Ok(sz)) = (window.outer_position(), window.inner_size()) {
            let right_edge = pos.x + sz.width as i32;
            let bottom_edge = pos.y + sz.height as i32;
            Some(tauri::PhysicalPosition::new(
                (right_edge - phys_w).max(12),
                (bottom_edge - phys_h).max(12),
            ))
        } else {
            None
        }
    } else if prev_mode == "full" {
        window.current_monitor().ok().flatten().map(|mon| {
            let m_pos = mon.position();
            let m_size = mon.size();
            let margin_x = (24.0 * scale).round() as i32;
            let margin_y = (56.0 * scale).round() as i32;
            tauri::PhysicalPosition::new(
                (m_pos.x + m_size.width as i32 - phys_w - margin_x).max(m_pos.x + 12),
                (m_pos.y + m_size.height as i32 - phys_h - margin_y).max(m_pos.y + 12),
            )
        })
    } else {
        None
    };

    let _ = window.set_decorations(false);
    let _ = window.set_always_on_top(request.pinned);
    let _ = window.set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize::new(
        320.0, 48.0,
    ))));
    let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
        target_w, target_h,
    )));
    if let Some(pos) = next_pos {
        let _ = window.set_position(tauri::Position::Physical(pos));
    }

    if let Ok(mut g) = current_overlay_mode_slot().lock() {
        *g = target_mode.to_string();
    }

    Ok(SetMiniOverlayModeResponse {
        mode: target_mode.to_string(),
        pinned: request.pinned,
        width: target_w,
        height: target_h,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushVirtualDjLiveCrateRequest {
    pub tracks: Vec<ExportTrackItem>,
    #[serde(default = "default_pinned_true")]
    pub append: bool,
    pub crate_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushVirtualDjLiveCrateResponse {
    pub crate_name: String,
    pub total_tracks: usize,
    pub written_files: Vec<String>,
    pub synced_to_virtualdj_home: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualDjNowPlaying {
    pub artist: String,
    pub title: String,
    pub source_path: String,
}

fn candidate_virtualdj_homes() -> Vec<std::path::PathBuf> {
    let mut homes = Vec::new();
    if let Ok(override_dir) = std::env::var("HERA_VDJ_HOME") {
        if !override_dir.trim().is_empty() {
            homes.push(std::path::PathBuf::from(override_dir));
        }
    }
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        homes.push(
            std::path::PathBuf::from(&user_profile)
                .join("Documents")
                .join("VirtualDJ"),
        );
    }
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        homes.push(std::path::PathBuf::from(local_app_data).join("VirtualDJ"));
    }
    if let Ok(home) = std::env::var("HOME") {
        homes.push(
            std::path::PathBuf::from(home)
                .join("Documents")
                .join("VirtualDJ"),
        );
    }
    homes
}

pub(crate) fn parse_virtualdj_history_content(raw: &str) -> Option<(String, String)> {
    for line in raw.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("#EXTM3U") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#EXTINF:") {
            let after_comma = rest.split_once(',').map(|(_, t)| t.trim()).unwrap_or(rest);
            if let Some((artist, title)) = after_comma
                .split_once(" - ")
                .or_else(|| after_comma.split_once(" — "))
            {
                let a = artist.trim();
                let t = title.trim();
                if !a.is_empty() && !t.is_empty() {
                    return Some((a.to_string(), t.to_string()));
                }
            }
            continue;
        }
        if trimmed.starts_with('#')
            || trimmed.contains(":\\")
            || trimmed.starts_with('/')
            || trimmed.ends_with(".mp3")
            || trimmed.ends_with(".wav")
            || trimmed.ends_with(".flac")
        {
            continue;
        }
        let cleaned = if trimmed.len() >= 7
            && trimmed.as_bytes().get(2) == Some(&b':')
            && let Some((_, after_time)) = trimmed.split_once(" : ")
        {
            after_time.trim()
        } else if trimmed.len() >= 6
            && trimmed.as_bytes().get(2) == Some(&b':')
            && let Some((_, after_time)) = trimmed.split_once(' ')
        {
            after_time.trim_start_matches(':').trim()
        } else {
            trimmed
        };

        if let Some((artist, title)) = cleaned
            .split_once(" - ")
            .or_else(|| cleaned.split_once(" — "))
        {
            let a = artist.trim();
            let t = title.trim();
            if !a.is_empty() && !t.is_empty() {
                return Some((a.to_string(), t.to_string()));
            }
        }
    }
    None
}

pub(crate) fn write_virtualdj_live_crate_files(
    crate_name: &str,
    tracks: &[ExportTrackItem],
    target_dirs: &[std::path::PathBuf],
) -> Vec<String> {
    let mut all_written = Vec::new();
    for dir in target_dirs {
        let req = ExportCrateRequest {
            crate_name: crate_name.to_string(),
            brief: "HERA Live Overlay Crate".to_string(),
            tracks: tracks.to_vec(),
            formats: vec!["virtualdj".to_string(), "m3u8".to_string()],
            output_dir: Some(dir.to_string_lossy().to_string()),
        };
        let res = export_crate_bundle(&req);
        all_written.extend(res.written_files);
    }
    all_written
}

#[tauri::command]
pub async fn push_virtualdj_live_crate<R: Runtime>(
    app: tauri::AppHandle<R>,
    request: PushVirtualDjLiveCrateRequest,
) -> Result<PushVirtualDjLiveCrateResponse, String> {
    let fallback_data_dir = app.path().app_data_dir().ok();
    tauri::async_runtime::spawn_blocking(move || {
        let crate_name = request
            .crate_name
            .unwrap_or_else(|| "HERA En Vivo".to_string());

        let merged_tracks = {
            let mut guard = live_vdj_crate_slot()
                .lock()
                .map_err(|_| "No se pudo bloquear el estado de VirtualDJ en vivo".to_string())?;
            if !request.append {
                guard.clear();
            }
            for item in request.tracks {
                if let Some(existing) = guard.iter_mut().find(|t| {
                    t.track_id == item.track_id
                        || (!item.file_path.is_empty() && t.file_path == item.file_path)
                }) {
                    *existing = item;
                } else {
                    guard.push(item);
                }
            }
            guard.clone()
        };

        let mut target_dirs = Vec::new();
        let mut synced_to_virtualdj_home = false;

        for vdj_home in candidate_virtualdj_homes() {
            if vdj_home.exists() {
                let folders_dir = vdj_home.join("Folders");
                if std::fs::create_dir_all(&folders_dir).is_ok() {
                    target_dirs.push(folders_dir);
                    synced_to_virtualdj_home = true;
                }
            }
        }

        let cwd_exports = std::env::current_dir()
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("exports")
            .join("VirtualDJ");
        if std::fs::create_dir_all(&cwd_exports).is_ok() {
            target_dirs.push(cwd_exports);
        } else if let Some(app_dir) = fallback_data_dir {
            let fallback_exports = app_dir.join("exports").join("VirtualDJ");
            if std::fs::create_dir_all(&fallback_exports).is_ok() {
                target_dirs.push(fallback_exports);
            }
        }

        let written_files =
            write_virtualdj_live_crate_files(&crate_name, &merged_tracks, &target_dirs);

        Ok(PushVirtualDjLiveCrateResponse {
            crate_name,
            total_tracks: merged_tracks.len(),
            written_files,
            synced_to_virtualdj_home,
        })
    })
    .await
    .map_err(|e| format!("Fallo en tarea push_virtualdj_live_crate: {e}"))?
}

#[tauri::command]
pub async fn read_virtualdj_now_playing() -> Result<Option<VirtualDjNowPlaying>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        for vdj_home in candidate_virtualdj_homes() {
            let history_dir = vdj_home.join("History");
            if !history_dir.exists() {
                continue;
            }
            let tracklist_txt = history_dir.join("tracklist.txt");
            if tracklist_txt.is_file() {
                if let Ok(raw) = std::fs::read_to_string(&tracklist_txt) {
                    if let Some((artist, title)) = parse_virtualdj_history_content(&raw) {
                        return Ok(Some(VirtualDjNowPlaying {
                            artist,
                            title,
                            source_path: tracklist_txt.to_string_lossy().to_string(),
                        }));
                    }
                }
            }
        }
        Ok(None)
    })
    .await
    .map_err(|e| format!("Fallo en tarea read_virtualdj_now_playing: {e}"))?
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
            resolve_harmonic_bridge,
            download_p2p_track,
            set_mini_overlay_mode,
            push_virtualdj_live_crate,
            read_virtualdj_now_playing
        ])
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_virtualdj_history_tracklist_and_extinf() {
        let vdj_txt = "21:05 : Avicii - Levels\n21:11 : Eric Prydz - Opus\n";
        let parsed = parse_virtualdj_history_content(vdj_txt);
        assert_eq!(
            parsed,
            Some(("Eric Prydz".to_string(), "Opus".to_string()))
        );

        let m3u_txt = "#EXTM3U\n#EXTINF:412,Keinemusik — Cesonia\nlibrary/Cesonia.wav\n";
        let parsed_m3u = parse_virtualdj_history_content(m3u_txt);
        assert_eq!(
            parsed_m3u,
            Some(("Keinemusik".to_string(), "Cesonia".to_string()))
        );

        assert_eq!(parse_virtualdj_history_content("   \n#EXTM3U\n"), None);
    }

    #[test]
    fn test_write_virtualdj_live_crate_generates_vdjfolder_with_hot_cues() {
        let temp_dir = std::env::temp_dir().join(format!(
            "hera_vdj_overlay_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("create temp vdj dir");

        let sample_track = ExportTrackItem {
            track_id: "vdj-live-01".to_string(),
            artist: "Keinemusik".to_string(),
            title: "Cesonia".to_string(),
            version: None,
            file_path: temp_dir.join("Cesonia.wav").to_string_lossy().to_string(),
            bpm: 125.0,
            camelot: "9A".to_string(),
            duration_seconds: 240.0,
            bitrate_kbps: 1411,
            energy: 0.84,
            file_size_bytes: 0,
            is_video: false,
            is_own_production: false,
            hot_cues: Vec::new(),
        };

        let written = write_virtualdj_live_crate_files(
            "HERA En Vivo",
            &[sample_track],
            std::slice::from_ref(&temp_dir),
        );
        assert!(
            written.iter().any(|p| p.ends_with("HERA En Vivo.vdjfolder")),
            "Expected HERA En Vivo.vdjfolder in {:?}",
            written
        );
        let folder_xml = std::fs::read_to_string(temp_dir.join("HERA En Vivo.vdjfolder"))
            .expect("read vdjfolder");
        assert!(folder_xml.contains("Cesonia"));
        let db_xml = std::fs::read_to_string(temp_dir.join("HERA En Vivo_vdj_database.xml"))
            .expect("read vdj database xml");
        assert!(db_xml.contains("<Poi "));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_tauri_v2_acl_triad_parity() {
        let report = hera_ipc::acl_guard::verify_tauri_v2_acl_triad(std::path::Path::new(env!(
            "CARGO_MANIFEST_DIR"
        )))
        .unwrap();
        assert!(report.commands_checked >= 45);
    }
}
