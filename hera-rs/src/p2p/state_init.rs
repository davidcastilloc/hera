use super::*;
use crate::structure::HotCuePoint;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Default)]
pub struct P2PMeshState {
    pub node_id: String,
    pub catalog: Arc<RwLock<Vec<P2PVerifiedTrack>>>,
    pub dj_profiles: Arc<RwLock<Vec<DjProfile>>>,
    pub peers: Arc<RwLock<Vec<P2PPeerNode>>>,
    /// Maps `track_id -> absolute_local_file_path` when running as a Local Laptop / VPS P2P Node
    pub local_files: Arc<RwLock<HashMap<String, String>>>,
    /// Canonical `cas_fingerprint -> audio_sha256` double seal to defeat Bait-and-Switch attacks
    pub cas_audio_sha256: Arc<RwLock<HashMap<String, String>>>,
    /// Thermal popularity map (`cas_fingerprint -> heat_score`) driving elastic K-replication
    pub cas_heat_scores: Arc<RwLock<HashMap<String, f32>>>,
    /// Nodes blacklisted for Bait-and-Switch or forged custody proofs
    pub quarantined_nodes: Arc<RwLock<HashMap<String, QuarantinedNodeRecord>>>,
    /// Immutable Proof-of-Delivery receipts minted when serving 60s signed tickets
    pub delivery_receipts: Arc<RwLock<Vec<ProofOfDeliveryReceipt>>>,
    /// Conflict-Free Replicated Data Type (`Delta-CRDT`: `GCounter` + `LWW-Element-Set`) state
    pub crdt_state: Arc<RwLock<SwarmCrdtDelta>>,
    /// Layer 5 MoQ-style Harmonic Topic Subscriptions (`hera://swarm/<camelot>/<bpm_band>`)
    pub topic_subscriptions: Arc<RwLock<Vec<SwarmTopicSubscription>>>,
    /// Runtime Swarm secrets for tickets & catalog blinding
    pub swarm_secrets: Arc<RwLock<SwarmCryptoSecrets>>,
    /// Optional Bearer / x-hera-node-token API token guarding administrative routes
    pub node_api_token: Arc<RwLock<Option<String>>>,
    /// Testable clock override for real ticket TTL expiration without HTTP query params
    pub clock_override_unix: Arc<RwLock<Option<u64>>>,
}

impl P2PMeshState {
    pub fn new(node_id: impl Into<String>) -> Self {
        let nid = node_id.into();
        let seed_catalog = vec![
            P2PVerifiedTrack {
                track_id: "hera-seed-001".to_string(),
                title: "Ritual Del Sol (DaviDJ Original Club Master)".to_string(),
                artist: "DaviDJ".to_string(),
                owner_dj: "DaviDJ".to_string(),
                bpm: 122.0,
                camelot_key: "8A".to_string(),
                top_subgenre: "Afro House / Deep Tech".to_string(),
                duration_seconds: 342.0,
                format: "flac".to_string(),
                effective_bandwidth_hz: 21450.0,
                sub_bass_phase_correlation: 0.96,
                mono_compatible_sub: true,
                quality_score: 98.5,
                forensic_verdict: "VERIFIED_STUDIO_MASTER".to_string(),
                hot_cues: vec![
                    HotCuePoint { num: 1, name: "1.1.1 Downbeat".to_string(), seconds: 0.25, bar_index: 0, role: "downbeat".to_string(), confidence: 0.98 },
                    HotCuePoint { num: 2, name: "Phrase In (8b)".to_string(), seconds: 15.98, bar_index: 8, role: "phrase_in".to_string(), confidence: 0.93 },
                    HotCuePoint { num: 3, name: "Main Drop (16b)".to_string(), seconds: 31.72, bar_index: 16, role: "drop".to_string(), confidence: 0.96 },
                    HotCuePoint { num: 4, name: "Mix Out (144b)".to_string(), seconds: 283.52, bar_index: 144, role: "mix_out".to_string(), confidence: 0.94 },
                ],
                storage_provider: "gdrive".to_string(),
                gdrive_file_id: Some("1DaviDJ_RitualDelSol_MasterFLAC".to_string()),
                r2_object_key: Some("masters/davidj/ritual_del_sol.flac".to_string()),
                stream_url: Some(build_gdrive_direct_download_url(
                    "1DaviDJ_RitualDelSol_MasterFLAC",
                )),
                share_on_p2p: true,
                node_id: nid.clone(),
                hera_seal: compute_hera_seal(
                    "hera-seed-001",
                    122.0,
                    "8A",
                    98.5,
                    "VERIFIED_STUDIO_MASTER",
                ),
            },
            P2PVerifiedTrack {
                track_id: "hera-seed-002".to_string(),
                title: "Guaracha Turbo Club (DaviDJ Extended Mix)".to_string(),
                artist: "DaviDJ".to_string(),
                owner_dj: "DaviDJ".to_string(),
                bpm: 128.0,
                camelot_key: "8A".to_string(),
                top_subgenre: "Tech House / Guaracha".to_string(),
                duration_seconds: 296.0,
                format: "flac".to_string(),
                effective_bandwidth_hz: 21800.0,
                sub_bass_phase_correlation: 0.94,
                mono_compatible_sub: true,
                quality_score: 97.0,
                forensic_verdict: "VERIFIED_STUDIO_MASTER".to_string(),
                hot_cues: vec![
                    HotCuePoint { num: 1, name: "1.1.1 Downbeat".to_string(), seconds: 0.12, bar_index: 0, role: "downbeat".to_string(), confidence: 0.97 },
                    HotCuePoint { num: 3, name: "Main Drop (16b)".to_string(), seconds: 30.12, bar_index: 16, role: "drop".to_string(), confidence: 0.95 },
                ],
                storage_provider: "gdrive".to_string(),
                gdrive_file_id: Some("1DaviDJ_GuarachaTurbo_MasterFLAC".to_string()),
                r2_object_key: Some("masters/davidj/guaracha_turbo.flac".to_string()),
                stream_url: Some(build_gdrive_direct_download_url(
                    "1DaviDJ_GuarachaTurbo_MasterFLAC",
                )),
                share_on_p2p: true,
                node_id: nid.clone(),
                hera_seal: compute_hera_seal(
                    "hera-seed-002",
                    128.0,
                    "8A",
                    97.0,
                    "VERIFIED_STUDIO_MASTER",
                ),
            },
            P2PVerifiedTrack {
                track_id: "hera-seed-003".to_string(),
                title: "Teteo Polirritmico 96 (DaviDJ 3:4 Bridge Master)".to_string(),
                artist: "DaviDJ".to_string(),
                owner_dj: "DaviDJ".to_string(),
                bpm: 96.0,
                camelot_key: "8A".to_string(),
                top_subgenre: "Dembow / Urbano".to_string(),
                duration_seconds: 218.0,
                format: "flac".to_string(),
                effective_bandwidth_hz: 21100.0,
                sub_bass_phase_correlation: 0.95,
                mono_compatible_sub: true,
                quality_score: 96.5,
                forensic_verdict: "VERIFIED_STUDIO_MASTER".to_string(),
                hot_cues: vec![
                    HotCuePoint { num: 1, name: "1.1.1 Downbeat".to_string(), seconds: 0.10, bar_index: 0, role: "downbeat".to_string(), confidence: 0.98 },
                    HotCuePoint { num: 3, name: "Main Drop (16b)".to_string(), seconds: 40.10, bar_index: 16, role: "drop".to_string(), confidence: 0.95 },
                ],
                storage_provider: "gdrive".to_string(),
                gdrive_file_id: Some("1DaviDJ_Teteo96_MasterFLAC".to_string()),
                r2_object_key: Some("masters/davidj/teteo_96.flac".to_string()),
                stream_url: Some(build_gdrive_direct_download_url(
                    "1DaviDJ_Teteo96_MasterFLAC",
                )),
                share_on_p2p: true,
                node_id: nid.clone(),
                hera_seal: compute_hera_seal(
                    "hera-seed-003",
                    96.0,
                    "8A",
                    96.5,
                    "VERIFIED_STUDIO_MASTER",
                ),
            },
        ];

        let mut initial_sha = HashMap::new();
        let mut initial_heat = HashMap::new();
        let mut initial_lww = HashMap::new();
        let founder_pubkey = derive_node_endpoint_pubkey_id("DaviDJ", &nid);
        for t in &seed_catalog {
            let cas = compute_cas_fingerprint(&t.artist, &t.title, t.bpm);
            let sha = compute_canonical_audio_sha256(&cas, &t.hera_seal, 38.0);
            initial_sha.insert(cas.clone(), sha);
            initial_heat.insert(cas.clone(), 2.5);
            initial_lww.insert(
                cas.clone(),
                CrdtLwwTrackEntry {
                    cas_fingerprint: cas,
                    track: t.clone(),
                    hlc_timestamp_ms: 1_762_600_000_000,
                    origin_endpoint_pubkey_id: founder_pubkey.clone(),
                    tombstone: false,
                },
            );
        }
        let mut initial_gcounter_served = HashMap::new();
        initial_gcounter_served.insert(nid.clone(), 333.3);
        let initial_crdt = SwarmCrdtDelta {
            origin_node_id: nid.clone(),
            origin_endpoint_pubkey_id: founder_pubkey.clone(),
            hlc_timestamp_ms: 1_762_600_000_000,
            gcounter_node_bytes_served_mb: initial_gcounter_served,
            gcounter_cas_play_counts: HashMap::new(),
            lww_track_entries: initial_lww,
        };

        let founder_profile = vec![DjProfile {
            dj_handle: "DaviDJ".to_string(),
            display_name: "David José (DaviDJ — Founder & Master Curator)".to_string(),
            role: "founder_node_operator".to_string(),
            storage_provider: "gdrive+r2".to_string(),
            gdrive_folder_id: Some("1DaviDJ_Official_Repertoire_Folder".to_string()),
            gdrive_folder_url: Some(
                "https://drive.google.com/drive/folders/1DaviDJ_Official_Repertoire_Folder"
                    .to_string(),
            ),
            r2_bucket_url: Some("https://cdn.hera-davidj.workers.dev".to_string()),
            local_node_dir: None,
            subscription_tier: "founder_unlimited".to_string(),
            r2_quota_mb: 1_000_000.0,
            r2_used_mb: 112.5,
            bounty_refunded_mb: 0.0,
            p2p_sharing_enabled: true,
            can_disable_p2p_sharing: true,
            verified_tracks_shared: 3,
            quarantined_rejected_tracks: 0,
            edge_guardian_active: true,
            swarm_credit_balance_mb: 500.0,
            total_bytes_served_mb: 333.3,
            total_bytes_consumed_mb: 0.0,
            custody_audit_passes: 10,
            custody_audit_failures: 0,
        }];

        let founder_peer = vec![P2PPeerNode {
            node_id: nid.clone(),
            dj_handle: "DaviDJ".to_string(),
            endpoint_url: "https://hera-node.pc911itservices.workers.dev".to_string(),
            runtime_target: "cloudflare-workers-wasm32".to_string(),
            storage_mode: "r2_edge_guardian".to_string(),
            verified_tracks_count: 3,
            bandwidth_mbps: 1000,
            r2_free_tier_limit_mb: CF_WORKER_FREE_R2_LIMIT_MB,
            r2_safe_cap_mb: CF_WORKER_SAFE_R2_CAP_MB,
            r2_used_mb: 114.0,
            daily_worker_request_limit: CF_WORKER_FREE_REQ_LIMIT,
            daily_worker_request_safe_cap: CF_WORKER_SAFE_REQ_CAP,
            daily_worker_requests_used: 120,
            edge_guardian_active: true,
            custody_status: "ONLINE_HEALTHY".to_string(),
            last_custody_proof_hash: "seed-custody-proof-verified".to_string(),
            custody_failures: 0,
            credits_earned_mb: 500.0,
            behind_cgnat: false,
            quarantined: false,
            endpoint_pubkey_id: founder_pubkey,
            network_generation: 1,
            relay_assisted_url: None,
            ewma_rtt_ms: 18.0,
        }];

        Self {
            node_id: nid,
            catalog: Arc::new(RwLock::new(seed_catalog)),
            dj_profiles: Arc::new(RwLock::new(founder_profile)),
            peers: Arc::new(RwLock::new(founder_peer)),
            local_files: Arc::new(RwLock::new(HashMap::new())),
            cas_audio_sha256: Arc::new(RwLock::new(initial_sha)),
            cas_heat_scores: Arc::new(RwLock::new(initial_heat)),
            quarantined_nodes: Arc::new(RwLock::new(HashMap::new())),
            delivery_receipts: Arc::new(RwLock::new(Vec::new())),
            crdt_state: Arc::new(RwLock::new(initial_crdt)),
            topic_subscriptions: Arc::new(RwLock::new(Vec::new())),
            swarm_secrets: Arc::new(RwLock::new(SwarmCryptoSecrets::default())),
            node_api_token: Arc::new(RwLock::new(None)),
            clock_override_unix: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_swarm_secrets(&self, ticket_secret: String, blinding_secret: String) {
        if let Ok(mut guard) = self.swarm_secrets.write() {
            guard.ticket_hmac_secret = ticket_secret;
            guard.shard_blinding_secret = blinding_secret;
        }
    }

    pub fn get_swarm_secrets(&self) -> SwarmCryptoSecrets {
        self.swarm_secrets.read().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn set_node_api_token(&self, token: Option<String>) {
        if let Ok(mut guard) = self.node_api_token.write() {
            *guard = token;
        }
    }

    pub fn verify_node_api_token(&self, provided: Option<&str>) -> Result<(), String> {
        let expected = self
            .node_api_token
            .read()
            .map(|g| g.clone())
            .unwrap_or(None);
        if let Some(exp) = expected {
            let p = provided.unwrap_or("").trim();
            let token_clean = p.strip_prefix("Bearer ").or_else(|| p.strip_prefix("bearer ")).unwrap_or(p).trim();
            if token_clean.is_empty() || !crate::error::constant_time_eq(token_clean.as_bytes(), exp.as_bytes()) {
                return Err("Missing or invalid node API token (401 Unauthorized)".to_string());
            }
        }
        Ok(())
    }

    pub fn set_clock_override_unix(&self, unix_secs: Option<u64>) {
        if let Ok(mut guard) = self.clock_override_unix.write() {
            *guard = unix_secs;
        }
    }

    pub fn current_unix_secs(&self) -> u64 {
        if let Ok(guard) = self.clock_override_unix.read() {
            if let Some(override_secs) = *guard {
                return override_secs;
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            (worker::Date::now().as_millis() / 1000) as u64
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(1_762_600_000)
        }
    }
}
