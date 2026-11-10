use super::{
    default_ewma_rtt_ms, default_network_generation, P2PPeerNode, P2PVerifiedTrack,
    SwarmStrengthReport, WaterFillingNodeAllocation,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundPullTicket {
    pub pull_ticket_id: String,
    pub cas_fingerprint: String,
    pub opaque_blob_key: String,
    pub audio_sha256: String,
    pub source_url: String,
    pub file_size_mb: f32,
    pub target_replicas_k: u32,
    pub expires_at_unix: u64,
    pub hmac_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmHeartbeatRequest {
    pub node_id: String,
    pub owner_dj: String,
    pub endpoint_url: String,
    pub r2_used_mb: Option<f32>,
    pub daily_worker_requests_used: Option<u64>,
    pub megabytes_served_delta: Option<f32>,
    pub custody_proof_slice_sha256: Option<String>,
    pub behind_cgnat: Option<bool>,
    #[serde(default)]
    pub endpoint_pubkey_id: Option<String>,
    #[serde(default)]
    pub observed_rtt_ms: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmHeartbeatResponse {
    pub status: String,
    pub node_id: String,
    #[serde(default)]
    pub endpoint_pubkey_id: String,
    #[serde(default = "default_network_generation")]
    pub network_generation: u64,
    #[serde(default)]
    pub active_endpoint_url: String,
    #[serde(default)]
    pub relay_assisted_url: Option<String>,
    #[serde(default = "default_ewma_rtt_ms")]
    pub ewma_rtt_ms: f32,
    pub edge_guardian_active: bool,
    pub custody_status: String,
    pub swarm_credit_balance_mb: f32,
    pub credit_multiplier: f32,
    pub water_filling_allocation: WaterFillingNodeAllocation,
    pub outbound_pull_tickets: Vec<OutboundPullTicket>,
    #[serde(default)]
    pub semantic_pubsub_events: Vec<SwarmSemanticPubSubEvent>,
    pub swarm_strength: SwarmStrengthReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmTopicSubscription {
    pub subscriber_node_id: String,
    pub subscriber_dj: String,
    pub topic_uri: String,
    pub camelot_key: String,
    pub bpm_min: f32,
    pub bpm_max: f32,
    pub subgenre_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(rename = "SwarmMoqHarmonicEvent"))]
pub struct SwarmSemanticPubSubEvent {
    pub event_id: String,
    pub topic_uri: String,
    pub cas_fingerprint: String,
    pub track_id: String,
    pub artist: String,
    pub title: String,
    pub bpm: f32,
    pub camelot_key: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub style_simhash_u64: u64,
    pub merkle_root_sha256: String,
    pub opaque_blob_key: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub hot_cues_count: usize,
    pub prefetch_recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmSubscribeRequest {
    pub subscriber_node_id: String,
    pub subscriber_dj: String,
    pub camelot_key: String,
    pub bpm_min: Option<f32>,
    pub bpm_max: Option<f32>,
    pub subgenre_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmSubscribeResponse {
    pub status: String,
    pub subscription: SwarmTopicSubscription,
    pub immediate_matching_events: Vec<SwarmSemanticPubSubEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantinedNodeRecord {
    pub node_id: String,
    pub owner_dj: String,
    pub reason: String,
    pub offending_cas_fingerprint: String,
    pub expected_audio_sha256: String,
    pub reported_audio_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedSyncRequest {
    pub peer: P2PPeerNode,
    #[serde(default)]
    pub shared_tracks: Vec<P2PVerifiedTrack>,
    /// Optional map of `track_id` or `cas_fingerprint` -> `audio_sha256` for double cryptographic verification against Bait-and-Switch.
    #[serde(default)]
    pub track_audio_sha256: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedSyncResponse {
    pub status: String,
    pub responder_node_id: String,
    pub peer_registered: P2PPeerNode,
    pub tracks_accepted: usize,
    pub tracks_rejected_invalid_seal: usize,
    #[serde(default)]
    pub node_quarantined_for_bait_and_switch: bool,
    pub federated_swarm_catalog: Vec<P2PVerifiedTrack>,
    pub active_peers: Vec<P2PPeerNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtLwwTrackEntry {
    pub cas_fingerprint: String,
    pub track: P2PVerifiedTrack,
    pub hlc_timestamp_ms: u64,
    #[serde(default)]
    pub origin_endpoint_pubkey_id: String,
    #[serde(default)]
    pub tombstone: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SwarmCrdtDelta {
    #[serde(default)]
    pub origin_node_id: String,
    #[serde(default)]
    pub origin_endpoint_pubkey_id: String,
    #[serde(default)]
    pub hlc_timestamp_ms: u64,
    /// Grow-Only Counter (`GCounter`) mapping `node_id -> monotonic_megabytes_served`
    #[serde(default)]
    pub gcounter_node_bytes_served_mb: HashMap<String, f32>,
    /// Grow-Only Counter (`GCounter`) mapping `cas_fingerprint -> (node_id -> monotonic_play_count)`
    #[serde(default)]
    pub gcounter_cas_play_counts: HashMap<String, HashMap<String, u64>>,
    /// Last-Write-Wins Element Set (`LWW-Element-Set`) mapping `cas_fingerprint -> CrdtLwwTrackEntry`
    #[serde(default)]
    pub lww_track_entries: HashMap<String, CrdtLwwTrackEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmCrdtSyncResponse {
    pub status: String,
    pub node_id: String,
    pub converged_state_digest_sha256: String,
    pub tracks_upserted: usize,
    pub gcounter_total_swarm_served_mb: f32,
    pub merged_delta: SwarmCrdtDelta,
}
