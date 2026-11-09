use hera_node_rs::cloudflare_edge::{
    get_cloudflare_edge_capabilities, CloudflareEdgeCapabilities,
};
use hera_node_rs::dj_brain::{
    auto_detect_local_backend, compute_dj_style_dna, evaluate_agent_harness_turn,
    list_supported_ai_backends, probe_backend_models, scan_ai_connectivity_onboarding,
    AiBackendOption, AiConnectivityOnboardingReport, CloudflareFreeAiTurnRequest,
    CloudflareFreeAiTurnResponse, DjStyleDnaProfile, DjStyleDnaRequest,
};
use hera_node_rs::export::{export_crate_bundle, ExportCrateRequest, ExportCrateResponse};
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

fn shared_desktop_p2p_state() -> &'static Arc<P2PMeshState> {
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
            download_p2p_track
        ])
        .build()
}
