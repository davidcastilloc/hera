use super::*;

pub(crate) fn shared_desktop_p2p_state() -> &'static Arc<P2PMeshState> {
    static STATE: OnceLock<Arc<P2PMeshState>> = OnceLock::new();
    STATE.get_or_init(|| Arc::new(P2PMeshState::new("hera-desktop-local-node")))
}

fn daemon_status_slot() -> &'static Mutex<Option<DesktopP2PDaemonStatus>> {
    static SLOT: OnceLock<Mutex<Option<DesktopP2PDaemonStatus>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

pub(super) fn warm_desktop_udp_daemon_background() {
    static STARTED: AtomicBool = AtomicBool::new(false);
    if STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        std::thread::spawn(|| {
            if let Ok((status, _counter)) =
                spawn_desktop_udp_p2p_daemon(Arc::clone(shared_desktop_p2p_state()))
                && let Ok(mut guard) = daemon_status_slot().lock()
            {
                *guard = Some(status);
            }
        });
    }
}

pub(super) fn cached_desktop_udp_daemon() -> Option<DesktopP2PDaemonStatus> {
    warm_desktop_udp_daemon_background();
    daemon_status_slot()
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
}

pub(super) fn ensure_desktop_udp_daemon() -> Option<DesktopP2PDaemonStatus> {
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

pub(super) fn parse_camelot_code(raw: &str) -> Option<(i32, char)> {
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
            let distance = (delta.unsigned_abs() as u8)
                .saturating_add(mode_shift)
                .max(1);
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
