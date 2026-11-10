use super::{default_true, P2PVerifiedTrack, SwarmStrengthReport, SWARM_ALIGNED_BLOCK_BYTES};
use crate::structure::HotCuePoint;
use serde::{Deserialize, Serialize};

/// A live download/streaming route registered inside the Internal P2P Data Broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerSourceRoute {
    pub owner_dj: String,
    pub track_id: String,
    pub storage_provider: String,
    pub direct_stream_url: Option<String>,
    pub cost_tier_for_platform: String,
    pub priority_rank: u8,
}

/// Canonical deduplicated Master Intelligence entry in the Internal P2P Data Broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerIntelRecord {
    pub cas_fingerprint: String,
    pub canonical_title: String,
    pub canonical_artist: String,
    pub bpm: f32,
    pub camelot_key: String,
    pub top_subgenre: String,
    pub quality_score: f32,
    pub effective_bandwidth_hz: f32,
    pub style_simhash_u64: u64,
    pub style_simhash_hex: String,
    pub hot_cues: Vec<HotCuePoint>,
    pub hera_seal: String,
    #[serde(default)]
    pub audio_sha256: String,
    #[serde(default)]
    pub opaque_blob_key: String,
    #[serde(default)]
    pub merkle_root_sha256: String,
    #[serde(default = "default_heat_score")]
    pub heat_score: f32,
    #[serde(default = "default_target_k")]
    pub target_replicas_k: u32,
    #[serde(default = "default_aligned_block_bytes")]
    pub aligned_block_size_bytes: u64,
    pub swarm_replicas_count: usize,
    pub available_routes: Vec<BrokerSourceRoute>,
}

fn default_heat_score() -> f32 {
    1.0
}

fn default_target_k() -> u32 {
    1
}

fn default_aligned_block_bytes() -> u64 {
    SWARM_ALIGNED_BLOCK_BYTES
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerClaimCloudTrackRequest {
    pub claimer_dj: String,
    /// Either a `track_id` (e.g. `"hera-seed-001"`) or a `cas_fingerprint` (e.g. `"cas-davidj-ritualdelsol-122bpm"`)
    pub target_track_or_cas: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerClaimCloudTrackResponse {
    pub status: String,
    pub claimer_dj: String,
    pub pioneer_curator_dj: String,
    pub linked_track: P2PVerifiedTrack,
    pub physical_r2_mb_consumed: f32,
    pub transfer_latency_ms: f32,
    pub pioneer_bounty_refunded_mb: f32,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerSimilarQuery {
    pub reference_track_id: Option<String>,
    pub bpm: Option<f32>,
    pub camelot_key: Option<String>,
    pub subgenre: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerSimilarMatch {
    pub hamming_distance_bits: u32,
    pub popcnt_similarity_pct: f32,
    pub combined_sonic_match_score: f32,
    pub master_intel: BrokerIntelRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerTierOptimizationResult {
    pub status: String,
    pub cold_r2_replicas_hibernated: usize,
    pub r2_megabytes_freed: f32,
    pub zero_data_loss_verified_backups: usize,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerBatchLookupItem {
    pub artist: String,
    pub title: String,
    pub bpm: Option<f32>,
    pub local_bandwidth_hz: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerBatchLookupRequest {
    pub requesting_dj: String,
    pub tracks: Vec<BrokerBatchLookupItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerIntelMatch {
    pub query_artist: String,
    pub query_title: String,
    pub needs_auto_heal: bool,
    pub auto_heal_reason: Option<String>,
    pub cpu_ms_saved: f32,
    pub verified_master_intel: BrokerIntelRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerBatchLookupResponse {
    pub requesting_dj: String,
    pub instant_intel_hits: Vec<BrokerIntelMatch>,
    pub unknown_tracks: Vec<String>,
    pub auto_healed_count: usize,
    pub total_cpu_ms_saved: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerBridgeRequest {
    pub from_bpm: f32,
    pub from_camelot: String,
    pub to_bpm: f32,
    pub to_camelot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerBridgeCandidate {
    pub bridge_synergy_score: f32,
    pub step_1_note: String,
    pub step_2_note: String,
    pub uses_polyrhythmic_bridge: bool,
    pub master_intel: BrokerIntelRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerStatsResponse {
    pub node_id: String,
    pub total_swarm_replicas: usize,
    pub unique_cas_masters: usize,
    pub deduplication_ratio_pct: f32,
    pub active_djs_in_broker: usize,
    pub active_local_p2p_seeders: usize,
    pub swarm_strength: SwarmStrengthReport,
    pub swarm_index: Vec<BrokerIntelRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasCheckRequest {
    pub dj_handle: String,
    pub artist: String,
    pub title: String,
    pub bpm: f32,
    pub cas_fingerprint: Option<String>,
    pub file_size_mb: Option<f32>,
    #[serde(default = "default_true")]
    pub auto_claim_if_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasCheckResponse {
    pub status: String,
    pub cas_fingerprint: String,
    pub already_in_cas: bool,
    pub upload_required: bool,
    pub r2_cas_object_key: String,
    pub bandwidth_saved_mb: f32,
    pub zero_copy_claim: Option<BrokerClaimCloudTrackResponse>,
    pub note: String,
}
