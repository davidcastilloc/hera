use super::{
    default_owner_dj, default_true, CF_WORKER_FREE_R2_LIMIT_MB, CF_WORKER_FREE_REQ_LIMIT,
    CF_WORKER_SAFE_R2_CAP_MB, CF_WORKER_SAFE_REQ_CAP,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PPeerNode {
    pub node_id: String,
    #[serde(default = "default_owner_dj")]
    pub dj_handle: String,
    pub endpoint_url: String,
    pub runtime_target: String,
    #[serde(default = "default_storage_mode")]
    pub storage_mode: String,
    pub verified_tracks_count: usize,
    pub bandwidth_mbps: u32,
    #[serde(default = "default_free_r2_limit_mb")]
    pub r2_free_tier_limit_mb: f32,
    #[serde(default = "default_safe_r2_cap_mb")]
    pub r2_safe_cap_mb: f32,
    #[serde(default = "default_peer_r2_used_mb")]
    pub r2_used_mb: f32,
    #[serde(default = "default_free_req_limit")]
    pub daily_worker_request_limit: u64,
    #[serde(default = "default_safe_req_cap")]
    pub daily_worker_request_safe_cap: u64,
    #[serde(default)]
    pub daily_worker_requests_used: u64,
    #[serde(default = "default_true")]
    pub edge_guardian_active: bool,
    #[serde(default = "default_custody_status")]
    pub custody_status: String,
    #[serde(default)]
    pub last_custody_proof_hash: String,
    #[serde(default)]
    pub custody_failures: u32,
    #[serde(default)]
    pub credits_earned_mb: f32,
    #[serde(default)]
    pub behind_cgnat: bool,
    #[serde(default)]
    pub quarantined: bool,
    #[serde(default)]
    pub endpoint_pubkey_id: String,
    #[serde(default = "default_network_generation")]
    pub network_generation: u64,
    #[serde(default)]
    pub relay_assisted_url: Option<String>,
    #[serde(default = "default_ewma_rtt_ms")]
    pub ewma_rtt_ms: f32,
}

fn default_storage_mode() -> String {
    "local_disk_p2p_seeder".to_string()
}

pub(crate) fn default_network_generation() -> u64 {
    1
}
pub(crate) fn default_ewma_rtt_ms() -> f32 {
    24.0
}

fn default_free_r2_limit_mb() -> f32 {
    CF_WORKER_FREE_R2_LIMIT_MB
}
fn default_safe_r2_cap_mb() -> f32 {
    CF_WORKER_SAFE_R2_CAP_MB
}
fn default_peer_r2_used_mb() -> f32 {
    114.0
}
fn default_free_req_limit() -> u64 {
    CF_WORKER_FREE_REQ_LIMIT
}
fn default_safe_req_cap() -> u64 {
    CF_WORKER_SAFE_REQ_CAP
}
fn default_custody_status() -> String {
    "ONLINE_HEALTHY".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterFillingNodeAllocation {
    pub node_id: String,
    pub owner_dj: String,
    pub allocated_r2_mb: f32,
    pub safe_r2_cap_mb: f32,
    pub free_headroom_mb: f32,
    pub assigned_shards_count: usize,
    pub daily_requests_used: u64,
    pub daily_requests_safe_cap: u64,
    pub daily_requests_headroom: u64,
    pub load_factor_pct: f32,
    pub edge_guardian_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmStrengthReport {
    pub motto: String,
    pub active_healthy_nodes: usize,
    pub quarantined_nodes: usize,
    pub total_free_r2_capacity_gb: f32,
    pub safe_usable_r2_capacity_gb: f32,
    pub total_daily_worker_requests_capacity: u64,
    pub safe_daily_worker_requests_capacity: u64,
    pub aligned_4mb_block_throughput_multiplier: f32,
    pub effective_daily_lossless_streams_capacity: u64,
    pub base_replication_factor_k: u32,
    pub max_thermal_replication_k: u32,
    pub swarm_strength_multiplier: f32,
    pub water_filling_allocations: Vec<WaterFillingNodeAllocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmRedirectTicket {
    pub ticket_id: String,
    pub requester_dj: String,
    pub target_node_id: String,
    pub opaque_blob_key: String,
    pub audio_sha256: String,
    #[serde(default)]
    pub merkle_root_sha256: String,
    pub aligned_offset_bytes: u64,
    pub aligned_length_bytes: u64,
    pub issued_at_unix: u64,
    pub expires_at_unix: u64,
    pub ttl_seconds: u64,
    pub cache_control_header: String,
    pub hmac_signature: String,
}

/// Binary SHA-256 Merkle Tree over the 16 x 256 KiB sub-slices of a 4 MB aligned shard (Layer 3 Post-P2P / BEP 52 / Iroh).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMerkleTree {
    pub opaque_blob_key: String,
    pub aligned_offset_bytes: u64,
    pub leaf_count: usize,
    pub leaf_size_bytes: u64,
    pub leaf_hashes_sha256: Vec<String>,
    pub merkle_root_sha256: String,
}

/// Compact 4-sibling Merkle Inclusion Branch (`log2(16) = 4` hashes) allowing a client to verify
/// any 256 KiB sub-slice at the first kilobyte without waiting for the full 4 MB shard or 38 MB master.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleInclusionProof {
    pub opaque_blob_key: String,
    pub aligned_offset_bytes: u64,
    pub leaf_index: usize,
    pub leaf_size_bytes: u64,
    pub leaf_hash_sha256: String,
    pub sibling_hashes_sha256: Vec<String>,
    pub merkle_root_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmRouteRequest {
    pub requester_dj: String,
    pub track_id_or_cas: String,
    pub block_index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmRouteResponse {
    pub status: String,
    pub p2c_selected_node_id: String,
    pub p2c_selected_node_endpoint: String,
    #[serde(default)]
    pub selected_node_ewma_rtt_ms: f32,
    #[serde(default)]
    pub p2c_composite_cost: f32,
    pub signed_worker_shard_url: String,
    pub gdrive_fallback_url: String,
    pub access_mode: String,
    pub elastic_replicas_k: u32,
    pub ticket: SwarmRedirectTicket,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmParallelManifestRequest {
    pub requester_dj: String,
    pub track_id_or_cas: String,
    pub file_size_mb: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmParallelBlockAssignment {
    pub block_index: u32,
    pub aligned_offset_bytes: u64,
    pub aligned_length_bytes: u64,
    pub assigned_node_id: String,
    pub assigned_endpoint_pubkey_id: String,
    pub assigned_node_endpoint: String,
    pub assigned_node_ewma_rtt_ms: f32,
    pub p2c_composite_cost: f32,
    pub signed_worker_shard_url: String,
    pub merkle_root_sha256: String,
    pub ticket: SwarmRedirectTicket,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmParallelManifestResponse {
    pub status: String,
    pub transport_protocol: String,
    pub cas_fingerprint: String,
    pub track_id: String,
    pub requester_dj: String,
    pub access_mode: String,
    pub total_size_mb: f32,
    pub block_size_bytes: u64,
    pub total_blocks: usize,
    pub distinct_edge_guardians_striped: usize,
    pub aggregate_bandwidth_multiplier: f32,
    pub blocks: Vec<SwarmParallelBlockAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfDeliveryReceipt {
    pub receipt_id: String,
    pub ticket_id: String,
    pub serving_node_id: String,
    pub requester_dj: String,
    pub opaque_blob_key: String,
    pub bytes_delivered: u64,
    pub megabytes_delivered: f32,
    pub credits_earned_mb: f32,
    pub credit_multiplier: f32,
    pub receipt_hmac: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyChallengeRequest {
    pub node_id: String,
    pub cas_fingerprint_or_blob: String,
    pub seed_nonce: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyChallenge {
    pub challenge_id: String,
    pub node_id: String,
    pub opaque_blob_key: String,
    pub range_header: String,
    pub byte_offset: u64,
    pub slice_length: usize,
    pub expected_slice_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyAuditSubmission {
    pub challenge_id: Option<String>,
    pub node_id: String,
    pub opaque_blob_key: String,
    pub byte_offset: u64,
    pub http_status: u16,
    pub returned_slice_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyAuditResult {
    pub node_id: String,
    pub passed: bool,
    pub edge_guardian_active: bool,
    pub custody_status: String,
    pub node_quarantined: bool,
    pub swarm_credit_balance_mb: f32,
    pub message: String,
}
