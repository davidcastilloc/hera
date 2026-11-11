use super::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::AtomicUsize;
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
    /// Bounded Proof-of-Delivery receipts history (capped at `max_delivery_receipts`, default 10,000)
    pub delivery_receipts: Arc<RwLock<VecDeque<ProofOfDeliveryReceipt>>>,
    /// Configurable maximum number of delivery receipts kept in FIFO history
    pub max_delivery_receipts: Arc<AtomicUsize>,
    /// $O(1)$ set of redeemed ticket IDs for permanent replay-attack blocking
    pub redeemed_ticket_ids: Arc<RwLock<HashSet<String>>>,
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
        let seed_catalog = seed_data::default_seed_catalog(&nid);
        let founder_pubkey = derive_node_endpoint_pubkey_id("DaviDJ", &nid);
        let mut initial_sha = HashMap::new();
        let mut initial_heat = HashMap::new();
        let mut initial_lww = HashMap::new();
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

        let founder_profile = seed_data::default_founder_profile();
        let founder_peer = seed_data::default_founder_peer(&nid, founder_pubkey);

        Self {
            node_id: nid,
            catalog: Arc::new(RwLock::new(seed_catalog)),
            dj_profiles: Arc::new(RwLock::new(founder_profile)),
            peers: Arc::new(RwLock::new(founder_peer)),
            local_files: Arc::new(RwLock::new(HashMap::new())),
            cas_audio_sha256: Arc::new(RwLock::new(initial_sha)),
            cas_heat_scores: Arc::new(RwLock::new(initial_heat)),
            quarantined_nodes: Arc::new(RwLock::new(HashMap::new())),
            delivery_receipts: Arc::new(RwLock::new(VecDeque::new())),
            max_delivery_receipts: Arc::new(AtomicUsize::new(MAX_DELIVERY_RECEIPTS)),
            redeemed_ticket_ids: Arc::new(RwLock::new(HashSet::new())),
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
        self.swarm_secrets
            .read()
            .map(|g| g.clone())
            .unwrap_or_default()
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
            let token_clean = p
                .strip_prefix("Bearer ")
                .or_else(|| p.strip_prefix("bearer "))
                .unwrap_or(p)
                .trim();
            if token_clean.is_empty()
                || !crate::error::constant_time_eq(token_clean.as_bytes(), exp.as_bytes())
            {
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
