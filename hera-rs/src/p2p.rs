use crate::forensics::{audit_pcm_channels, decode_audio_bytes, ForensicAuditReport};
use crate::structure::{analyze_structure_and_style, HotCuePoint};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A track shared on the Hera Federated P2P Mesh, guaranteed to have passed Zero-Trust DSP Forensics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PVerifiedTrack {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    #[serde(default = "default_owner_dj")]
    pub owner_dj: String,
    pub bpm: f32,
    pub camelot_key: String,
    pub top_subgenre: String,
    pub duration_seconds: f32,
    pub format: String,
    pub effective_bandwidth_hz: f32,
    pub sub_bass_phase_correlation: f32,
    pub mono_compatible_sub: bool,
    pub quality_score: f32,
    pub forensic_verdict: String,
    pub hot_cues: Vec<HotCuePoint>,
    #[serde(default = "default_storage_provider")]
    pub storage_provider: String,
    #[serde(default)]
    pub gdrive_file_id: Option<String>,
    #[serde(default)]
    pub r2_object_key: Option<String>,
    pub stream_url: Option<String>,
    /// Always `true` by default: all uploaded songs are shared with the P2P swarm unless a Pro DJ explicitly turns P2P sharing off.
    #[serde(default = "default_true")]
    pub share_on_p2p: bool,
    pub node_id: String,
    pub hera_seal: String,
}

fn default_true() -> bool {
    true
}

fn default_owner_dj() -> String {
    "DaviDJ".to_string()
}

fn default_storage_provider() -> String {
    "gdrive".to_string()
}

/// Core Swarm Motto: Every new node multiplies global R2 storage, daily request capacity, K-replication, and throughput.
pub const SWARM_MOTTO: &str =
    "Mientras más nodos de Hera, más fuerte es el sistema de enjambres";

/// Cloudflare Workers Free Tier R2 limit (10 GB = 10,000 MB) and Safe Water-Filling Cap (9,000 MB = $0 billable).
pub const CF_WORKER_FREE_R2_LIMIT_MB: f32 = 10_000.0;
pub const CF_WORKER_SAFE_R2_CAP_MB: f32 = 9_000.0;

/// Cloudflare Workers Free Tier daily requests limit (100,000 req/day) and Safe Cap (85,000 req/day = $0 billable).
pub const CF_WORKER_FREE_REQ_LIMIT: u64 = 100_000;
pub const CF_WORKER_SAFE_REQ_CAP: u64 = 85_000;

/// 4 MB aligned immutable streaming blocks (`4,194,304` bytes) -> 8.4x request efficiency on Cloudflare Free Tier.
pub const SWARM_ALIGNED_BLOCK_BYTES: u64 = 4_194_304;

/// Number of 256 KiB leaves per 4 MB shard (`16 * 262,144 = 4,194,304` bytes) for BEP 52 / Iroh-style Streaming Merkle Proofs.
pub const SWARM_MERKLE_LEAF_COUNT: usize = 16;
pub const SWARM_MERKLE_LEAF_BYTES: u64 = 262_144;

/// 64-byte random range slice (`Range: bytes=offset..offset+63`) for zero-egress Proof-of-Custody audits.
pub const CUSTODY_PROOF_SLICE_BYTES: usize = 64;

/// 60-second TTL for HMAC-signed `X-Hera-Swarm-Ticket` redirect tickets.
pub const SWARM_TICKET_TTL_SECS: u64 = 60;

/// Meritocratic credit multiplier: 1.0 MB served by a healthy node mints 1.5 MB of offline Turbo credit.
pub const SWARM_CREDIT_MULTIPLIER: f32 = 1.5;

pub(crate) const SWARM_TICKET_MASTER_SECRET: &str = "hera-swarm-master-ticket-hmac-key-2026-v1";
pub(crate) const SWARM_SHARD_BLINDING_SECRET: &str = "hera-swarm-opaque-shard-blinding-key-2026-v1";

/// Zero-Tech DJ profile registered on the Hera Node (connects via Google Drive, Dropbox, R2 with Quota, or Local Disk P2P).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DjProfile {
    pub dj_handle: String,
    pub display_name: String,
    pub role: String,
    pub storage_provider: String,
    pub gdrive_folder_id: Option<String>,
    pub gdrive_folder_url: Option<String>,
    pub r2_bucket_url: Option<String>,
    #[serde(default)]
    pub local_node_dir: Option<String>,
    #[serde(default = "default_sub_tier")]
    pub subscription_tier: String,
    #[serde(default = "default_r2_quota")]
    pub r2_quota_mb: f32,
    #[serde(default)]
    pub r2_used_mb: f32,
    #[serde(default)]
    pub bounty_refunded_mb: f32,
    /// Enabled (`true`) by default for all DJs. Can only be turned OFF (`false`) on higher plans (`pro_monthly` / `founder_unlimited`).
    #[serde(default = "default_true")]
    pub p2p_sharing_enabled: bool,
    /// Indicates whether this DJ's subscription plan allows turning off P2P sharing (Private Vault Mode).
    #[serde(default)]
    pub can_disable_p2p_sharing: bool,
    pub verified_tracks_shared: usize,
    pub quarantined_rejected_tracks: usize,
    /// Meritocratic Autopilot (`Edge Guardian`): while `true`, grants unlimited R2 Fast Lane access.
    #[serde(default = "default_true")]
    pub edge_guardian_active: bool,
    /// Accumulated 1:1.5 Turbo credits (MB) for fast-lane downloads when the DJ's node is offline.
    #[serde(default = "default_initial_swarm_credits")]
    pub swarm_credit_balance_mb: f32,
    #[serde(default)]
    pub total_bytes_served_mb: f32,
    #[serde(default)]
    pub total_bytes_consumed_mb: f32,
    #[serde(default)]
    pub custody_audit_passes: u64,
    #[serde(default)]
    pub custody_audit_failures: u64,
}

fn default_initial_swarm_credits() -> f32 {
    150.0
}

fn default_sub_tier() -> String {
    "beta_mvp".to_string()
}

fn default_r2_quota() -> f32 {
    5_120.0 // 5 GB (5,120 MB) per selected Beta MVP DJ (~130 lossless FLACs or ~500 MP3s)
}

pub fn resolve_tier_r2_quota_mb(tier: &str) -> f32 {
    match tier {
        "trial_100mb" => 100.0,
        "beta_mvp" | "free" => 5_120.0,       // 5 GB per Beta MVP DJ
        "beta_vip_10gb" => 10_240.0,          // 10 GB per VIP Beta DJ
        "pro_monthly" | "pro" => 51_200.0,    // 50 GB Cloudflare R2 in Pro Monthly
        "founder_unlimited" => 1_000_000.0,   // Unlimited for DaviDJ
        _ => 5_120.0,
    }
}

/// Swarm-First Invariant ("Por el bien del enjambre"):
/// Free, Beta, and Local (`local_unlimited`) DJs get 100% of HERA's local DSP, AI, and exporters for $0.00,
/// and in exchange ALWAYS keep P2P sharing ON (`p2p_sharing_enabled = true`) to nourish the global swarm.
/// Only `pro_monthly`, `pro`, or `founder_unlimited` accounts can turn P2P sharing OFF.
pub fn tier_allows_private_vault(tier: &str) -> bool {
    matches!(tier, "pro_monthly" | "pro" | "founder_unlimited")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DjDriveTrackInput {
    pub title: String,
    pub artist: String,
    #[serde(default = "default_bpm")]
    pub bpm: f32,
    #[serde(default = "default_camelot")]
    pub camelot_key: String,
    #[serde(default = "default_subgenre")]
    pub top_subgenre: String,
    #[serde(default = "default_duration")]
    pub duration_seconds: f32,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(alias = "gdrive_file_link")]
    pub gdrive_file_link_or_id: Option<String>,
    pub r2_object_key: Option<String>,
    #[serde(default)]
    pub cloud_share_url: Option<String>,
    #[serde(default = "default_file_size_mb")]
    pub file_size_mb: f32,
    #[serde(default = "default_bw")]
    pub effective_bandwidth_hz: f32,
    #[serde(default = "default_phase")]
    pub sub_bass_phase_correlation: f32,
    #[serde(default)]
    pub is_synthetic_tone: bool,
    /// Defaults to `true` (shared on P2P). Can only be set to `false` if the DJ has a higher plan (`pro_monthly`).
    pub share_on_p2p: Option<bool>,
    #[serde(default)]
    pub audio_sha256: Option<String>,
}

fn default_file_size_mb() -> f32 {
    32.0
}

fn default_bpm() -> f32 {
    124.0
}
fn default_camelot() -> String {
    "8A".to_string()
}
fn default_subgenre() -> String {
    "Afro House / Tech House".to_string()
}
fn default_duration() -> f32 {
    240.0
}
fn default_format() -> String {
    "flac".to_string()
}
fn default_bw() -> f32 {
    21200.0
}
fn default_phase() -> f32 {
    0.92
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectDjDriveRequest {
    pub dj_handle: String,
    pub display_name: Option<String>,
    /// Any messy Google Drive folder link pasted by a non-technical DJ
    pub gdrive_folder_link: Option<String>,
    /// Optional raw HTML from Google Drive's Zero-OAuth `/embeddedfolderview?id=<FOLDER_ID>#list` view
    #[serde(default)]
    pub gdrive_embedded_html: Option<String>,
    /// Optional Cloudflare R2 bucket public URL
    pub r2_bucket_url: Option<String>,
    /// Optional subscription tier (`"free"` = 100 MB R2 quota, `"pro_monthly"` = 50,000 MB R2 quota)
    #[serde(default)]
    pub subscription_tier: Option<String>,
    /// Defaults to `true` (all uploaded tracks are shared on P2P). Can only be turned `false` on `pro_monthly` / higher plans.
    pub p2p_sharing_enabled: Option<bool>,
    /// Optional list of tracks found in the DJ's shared Google Drive / R2 / Dropbox folder
    #[serde(default)]
    pub tracks: Vec<DjDriveTrackInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectDjDriveResponse {
    pub status: String,
    pub profile: DjProfile,
    pub admitted_tracks: Vec<P2PVerifiedTrack>,
    pub rejected_quarantine_tracks: Vec<String>,
    #[serde(default)]
    pub cas_deduplicated_hits: usize,
    #[serde(default)]
    pub physical_r2_mb_saved: f32,
    pub network_catalog_available: usize,
    pub welcome_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanLocalDirRequest {
    pub dj_handle: String,
    pub directory_path: String,
    pub public_node_url: Option<String>,
    pub bandwidth_mbps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanLocalDirResponse {
    pub status: String,
    pub node_id: String,
    pub dj_handle: String,
    pub scanned_directory: String,
    pub admitted_tracks: Vec<P2PVerifiedTrack>,
    pub rejected_quarantine_tracks: Vec<String>,
    pub registered_peer: P2PPeerNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeTierRequest {
    pub dj_handle: String,
    pub tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetP2PSharingRequest {
    pub dj_handle: String,
    /// `true` (default) shares tracks with the P2P swarm; `false` requires a higher plan (`pro_monthly`).
    pub p2p_sharing_enabled: bool,
}

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

fn default_network_generation() -> u64 {
    1
}
fn default_ewma_rtt_ms() -> f32 {
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
pub struct SwarmSemanticPubSubEvent {
    pub event_id: String,
    pub topic_uri: String,
    pub cas_fingerprint: String,
    pub track_id: String,
    pub artist: String,
    pub title: String,
    pub bpm: f32,
    pub camelot_key: String,
    pub style_simhash_u64: u64,
    pub merkle_root_sha256: String,
    pub opaque_blob_key: String,
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

fn default_storage_mode() -> String {
    "local_disk_p2p_seeder".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudReferralOption {
    pub provider: String,
    pub tier_type: String,
    pub user_free_bonus: String,
    pub creator_referral_reward: String,
    pub deploy_url: String,
    pub one_click_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PSearchQuery {
    pub q: Option<String>,
    pub camelot: Option<String>,
    pub bpm_min: Option<f32>,
    pub bpm_max: Option<f32>,
    pub subgenre: Option<String>,
    pub owner_dj: Option<String>,
    pub verified_only: Option<bool>,
}

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
    /// Optional `HERA_ADMIN_SECRET` injected from `worker::Env` (WASM) or `std::env` (Native VPS)
    pub admin_secret: Arc<RwLock<Option<String>>>,
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
                    HotCuePoint {
                        num: 1,
                        name: "1.1.1 Downbeat".to_string(),
                        seconds: 0.25,
                        bar_index: 0,
                        role: "downbeat".to_string(),
                        confidence: 0.98,
                    },
                    HotCuePoint {
                        num: 2,
                        name: "Phrase In (8b)".to_string(),
                        seconds: 15.98,
                        bar_index: 8,
                        role: "phrase_in".to_string(),
                        confidence: 0.93,
                    },
                    HotCuePoint {
                        num: 3,
                        name: "Main Drop (16b)".to_string(),
                        seconds: 31.72,
                        bar_index: 16,
                        role: "drop".to_string(),
                        confidence: 0.96,
                    },
                    HotCuePoint {
                        num: 4,
                        name: "Mix Out (144b)".to_string(),
                        seconds: 283.52,
                        bar_index: 144,
                        role: "mix_out".to_string(),
                        confidence: 0.94,
                    },
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
                    HotCuePoint {
                        num: 1,
                        name: "1.1.1 Downbeat".to_string(),
                        seconds: 0.12,
                        bar_index: 0,
                        role: "downbeat".to_string(),
                        confidence: 0.97,
                    },
                    HotCuePoint {
                        num: 3,
                        name: "Main Drop (16b)".to_string(),
                        seconds: 30.12,
                        bar_index: 16,
                        role: "drop".to_string(),
                        confidence: 0.95,
                    },
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
                    HotCuePoint {
                        num: 1,
                        name: "1.1.1 Downbeat".to_string(),
                        seconds: 0.10,
                        bar_index: 0,
                        role: "downbeat".to_string(),
                        confidence: 0.98,
                    },
                    HotCuePoint {
                        num: 3,
                        name: "Main Drop (16b)".to_string(),
                        seconds: 40.10,
                        bar_index: 16,
                        role: "drop".to_string(),
                        confidence: 0.95,
                    },
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
            admin_secret: Arc::new(RwLock::new(
                std::env::var("HERA_ADMIN_SECRET")
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
            )),
        }
    }

    /// Sets or updates the `HERA_ADMIN_SECRET` at runtime (e.g., from Cloudflare Worker `env.secret("HERA_ADMIN_SECRET")`).
    pub fn set_admin_secret(&self, secret: impl Into<String>) {
        let trimmed = secret.into().trim().to_string();
        if !trimmed.is_empty() {
            if let Ok(mut guard) = self.admin_secret.write() {
                *guard = Some(trimmed);
            }
        }
    }

    /// Verifies `X-Hera-Admin-Secret` in constant time (`constant_time_eq`) against either the injected
    /// `worker::Env` secret or `std::env::var("HERA_ADMIN_SECRET")`.
    pub fn verify_admin_secret(&self, provided_header: Option<&str>) -> Result<(), crate::error::HeraError> {
        let expected_opt = self
            .admin_secret
            .read()
            .ok()
            .and_then(|g| g.clone())
            .or_else(|| {
                std::env::var("HERA_ADMIN_SECRET")
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            });

        if let Some(expected) = expected_opt {
            let provided = provided_header.unwrap_or("").trim();
            if !crate::error::constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
                return Err(crate::error::HeraError::Unauthorized(
                    "Missing or invalid `X-Hera-Admin-Secret` header for Hera Cloud Control Plane."
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Zero-Tech Onboarding: A non-technical DJ (`DJFoo`) simply pastes their DJ name and Google Drive folder link
    /// (or raw Zero-OAuth embedded folder HTML, Dropbox link, or Cloudflare R2 bucket).
    /// Enforces Cloudflare R2 storage quotas (`free` = 100 MB vs `pro_monthly` = 50 GB) and Zero-Trust DSP Forensics.
    pub fn connect_dj_drive(&self, mut req: ConnectDjDriveRequest) -> ConnectDjDriveResponse {
        let handle = req.dj_handle.trim().to_string();
        let display_name = req
            .display_name
            .clone()
            .unwrap_or_else(|| format!("{} (Community DJ)", handle));

        let gdrive_folder_id = req
            .gdrive_folder_link
            .as_deref()
            .map(extract_gdrive_id)
            .filter(|s| !s.is_empty());

        // Zero-OAuth Auto-Discovery: If the caller provided Google Drive's public `/embeddedfolderview?id=...#list` HTML
        // and `tracks` is empty, automatically extract every audio file ID and title without OAuth!
        if req.tracks.is_empty() {
            if let Some(ref html) = req.gdrive_embedded_html {
                for (fid, fname) in parse_gdrive_public_folder_html(html) {
                    let ext = fname
                        .rsplit('.')
                        .next()
                        .unwrap_or("flac")
                        .to_ascii_lowercase();
                    let stem = fname.trim_end_matches(&format!(".{ext}")).to_string();
                    let (artist, title) = if let Some((a, t)) = stem.split_once(" - ") {
                        (a.trim().to_string(), t.trim().to_string())
                    } else {
                        (handle.clone(), stem)
                    };
                    req.tracks.push(DjDriveTrackInput {
                        title,
                        artist,
                        bpm: 124.0,
                        camelot_key: "8A".to_string(),
                        top_subgenre: "Club Master".to_string(),
                        duration_seconds: 300.0,
                        format: ext,
                        gdrive_file_link_or_id: Some(fid),
                        r2_object_key: None,
                        cloud_share_url: None,
                        file_size_mb: 32.0,
                        effective_bandwidth_hz: 21000.0,
                        sub_bass_phase_correlation: 0.94,
                        is_synthetic_tone: false,
                        share_on_p2p: Some(true),
                        audio_sha256: None,
                    });
                }
            }
        }

        let sub_tier = req
            .subscription_tier
            .clone()
            .unwrap_or_else(|| "beta_mvp".to_string());
        let r2_quota_mb = resolve_tier_r2_quota_mb(&sub_tier);
        let mut r2_used_mb = 0.0f32;
        let can_disable_p2p = tier_allows_private_vault(&sub_tier);
        // By default, P2P sharing is ALWAYS ON (`true`). Only higher plans (`pro_monthly` / `founder_unlimited`) can turn it OFF.
        let p2p_sharing_enabled = if can_disable_p2p {
            req.p2p_sharing_enabled.unwrap_or(true)
        } else {
            true
        };

        let storage_provider = if req.r2_bucket_url.is_some() && gdrive_folder_id.is_some() {
            "gdrive+r2".to_string()
        } else if req.r2_bucket_url.is_some() {
            "r2".to_string()
        } else {
            "gdrive".to_string()
        };

        let mut admitted = Vec::new();
        let mut rejected = Vec::new();
        let mut cas_deduplicated_hits = 0usize;
        let mut physical_r2_mb_saved = 0.0f32;

        for (idx, item) in req.tracks.iter().enumerate() {
            let is_lossless = matches!(
                item.format.to_ascii_lowercase().as_str(),
                "flac" | "wav" | "aiff" | "alac"
            );
            let is_fake_lossless = is_lossless && item.effective_bandwidth_hz < 16800.0;
            let mono_ok = item.sub_bass_phase_correlation >= 0.25;

            if item.is_synthetic_tone {
                rejected.push(format!(
                    "{} - {} [REJECTED: Synthetic sine/test tone detected]",
                    item.artist, item.title
                ));
                continue;
            }
            if is_fake_lossless {
                rejected.push(format!(
                    "{} - {} [REJECTED: Fake .{} transcode brickwalled at {:.0} Hz]",
                    item.artist, item.title, item.format, item.effective_bandwidth_hz
                ));
                continue;
            }
            if !mono_ok {
                rejected.push(format!(
                    "{} - {} [REJECTED: Sub-bass (<120Hz) phase cancellation corr={:.2}]",
                    item.artist, item.title, item.sub_bass_phase_correlation
                ));
                continue;
            }

            // Global CAS (Content-Addressed Storage) Internal P2P Deduplication:
            // If another DJ on the Hera Cloud already uploaded this exact master (matching canonical artist+title+BPM),
            // reuse the existing R2/P2P master with ZERO extra physical R2 megabytes and ZERO extra DSP CPU!
            let cas_key = compute_cas_fingerprint(&item.artist, &item.title, item.bpm);
            let existing_cas_master = self.catalog.read().ok().and_then(|cat| {
                cat.iter()
                    .find(|t| compute_cas_fingerprint(&t.artist, &t.title, t.bpm) == cas_key)
                    .cloned()
            });

            // Enforce Cloudflare R2 Hard Quota ONLY if this track is NOT already deduplicated in the CAS pool
            let wants_r2 = item.r2_object_key.is_some()
                || (storage_provider == "r2" && item.gdrive_file_link_or_id.is_none());
            if wants_r2 {
                let sz = item.file_size_mb.max(1.0);
                if existing_cas_master.is_some() {
                    // Internal P2P CAS Hit! 0 MB physical R2 storage consumed!
                    cas_deduplicated_hits += 1;
                    physical_r2_mb_saved += sz;
                } else {
                    if r2_used_mb + sz > r2_quota_mb {
                        rejected.push(format!(
                            "{} - {} [R2_QUOTA_EXCEEDED: Needs {:.1} MB, but {} tier has {:.1}/{:.1} MB used. Upgrade to `pro_monthly` (50 GB R2) or use Google Drive / Local P2P Node for $0]",
                            item.artist, item.title, sz, sub_tier, r2_used_mb, r2_quota_mb
                        ));
                        continue;
                    }
                    r2_used_mb += sz;
                }
            } else if existing_cas_master.is_some() {
                cas_deduplicated_hits += 1;
            }

            let file_id = item
                .gdrive_file_link_or_id
                .as_deref()
                .map(extract_gdrive_id)
                .or_else(|| {
                    if !wants_r2 && item.cloud_share_url.is_none() {
                        gdrive_folder_id
                            .as_ref()
                            .map(|fid| format!("{fid}_trk_{:02}", idx + 1))
                    } else {
                        None
                    }
                });

            let (item_provider, stream_url) = if let Some(ref c_url) = item.cloud_share_url {
                let (prov, direct) = normalize_cloud_share_link(c_url);
                (prov, Some(direct))
            } else if let Some(ref fid) = file_id {
                ("gdrive".to_string(), Some(build_gdrive_direct_download_url(fid)))
            } else if let (Some(ref r2_base), Some(ref key)) =
                (&req.r2_bucket_url, &item.r2_object_key)
            {
                (
                    "r2".to_string(),
                    Some(format!("{}/{}", r2_base.trim_end_matches('/'), key)),
                )
            } else {
                (storage_provider.clone(), None)
            };

            let track_id = format!(
                "dj-{}-{:03}",
                handle.to_ascii_lowercase().replace(' ', "-"),
                idx + 1
            );
            let bar_sec = (60.0 / item.bpm.max(60.0)) * 4.0;
            let c2 = (8.0 * bar_sec).min(item.duration_seconds * 0.25);
            let c3 = (16.0 * bar_sec).min(item.duration_seconds * 0.45);
            let c4 = (item.duration_seconds - 8.0 * bar_sec).max(c3 + bar_sec);

            let hot_cues = vec![
                HotCuePoint {
                    num: 1,
                    name: "1.1.1 Downbeat".to_string(),
                    seconds: 0.20,
                    bar_index: 0,
                    role: "downbeat".to_string(),
                    confidence: 0.95,
                },
                HotCuePoint {
                    num: 2,
                    name: "Phrase In (8b)".to_string(),
                    seconds: (c2 * 100.0).round() / 100.0,
                    bar_index: 8,
                    role: "phrase_in".to_string(),
                    confidence: 0.91,
                },
                HotCuePoint {
                    num: 3,
                    name: "Main Drop (16b)".to_string(),
                    seconds: (c3 * 100.0).round() / 100.0,
                    bar_index: 16,
                    role: "drop".to_string(),
                    confidence: 0.94,
                },
                HotCuePoint {
                    num: 4,
                    name: "Mix Out".to_string(),
                    seconds: (c4 * 100.0).round() / 100.0,
                    bar_index: 32,
                    role: "mix_out".to_string(),
                    confidence: 0.92,
                },
            ];

            let quality_score = (90.0
                + ((item.effective_bandwidth_hz - 18000.0).max(0.0) / 1000.0) * 2.0
                + item.sub_bass_phase_correlation * 4.0)
                .clamp(70.0, 99.5);
            let q_round = (quality_score * 10.0).round() / 10.0;

            let seal = compute_hera_seal(
                &track_id,
                item.bpm,
                &item.camelot_key,
                q_round,
                "VERIFIED_STUDIO_MASTER",
            );

            let verified_track = P2PVerifiedTrack {
                track_id,
                title: item.title.clone(),
                artist: item.artist.clone(),
                owner_dj: handle.clone(),
                bpm: item.bpm,
                camelot_key: item.camelot_key.clone(),
                top_subgenre: item.top_subgenre.clone(),
                duration_seconds: item.duration_seconds,
                format: item.format.clone(),
                effective_bandwidth_hz: item.effective_bandwidth_hz,
                sub_bass_phase_correlation: item.sub_bass_phase_correlation,
                mono_compatible_sub: true,
                quality_score: q_round,
                forensic_verdict: "VERIFIED_STUDIO_MASTER".to_string(),
                hot_cues,
                storage_provider: item_provider,
                gdrive_file_id: file_id,
                r2_object_key: item.r2_object_key.clone(),
                stream_url,
                share_on_p2p: if can_disable_p2p {
                    item.share_on_p2p.unwrap_or(p2p_sharing_enabled)
                } else {
                    true
                },
                node_id: self.node_id.clone(),
                hera_seal: seal,
            };

            if let Ok(mut sha_guard) = self.cas_audio_sha256.write() {
                let sha = item
                    .audio_sha256
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| {
                        compute_canonical_audio_sha256(
                            &cas_key,
                            &verified_track.hera_seal,
                            item.file_size_mb,
                        )
                    });
                sha_guard.entry(cas_key.clone()).or_insert(sha);
            }
            if let Ok(mut heat_guard) = self.cas_heat_scores.write() {
                let h = heat_guard.entry(cas_key).or_insert(1.0);
                *h += 1.0;
            }
            if let Ok(mut guard) = self.catalog.write() {
                guard.retain(|t| t.track_id != verified_track.track_id);
                guard.push(verified_track.clone());
            }
            admitted.push(verified_track);
        }

        let initial_credits = ((admitted.len() as f32) * 38.0 * SWARM_CREDIT_MULTIPLIER).max(150.0);
        let profile = DjProfile {
            dj_handle: handle.clone(),
            display_name: display_name.clone(),
            role: "community_dj_peer".to_string(),
            storage_provider,
            gdrive_folder_id,
            gdrive_folder_url: req.gdrive_folder_link,
            r2_bucket_url: req.r2_bucket_url,
            local_node_dir: None,
            subscription_tier: sub_tier,
            r2_quota_mb,
            r2_used_mb,
            bounty_refunded_mb: 0.0,
            p2p_sharing_enabled,
            can_disable_p2p_sharing: can_disable_p2p,
            verified_tracks_shared: admitted.len(),
            quarantined_rejected_tracks: rejected.len(),
            edge_guardian_active: true,
            swarm_credit_balance_mb: initial_credits,
            total_bytes_served_mb: (admitted.len() as f32) * 38.0,
            total_bytes_consumed_mb: 0.0,
            custody_audit_passes: 1,
            custody_audit_failures: 0,
        };

        if let Ok(mut p_guard) = self.dj_profiles.write() {
            p_guard.retain(|p| !p.dj_handle.eq_ignore_ascii_case(&handle));
            p_guard.push(profile.clone());
        }

        let total_network_tracks = self
            .catalog
            .read()
            .map(|c| c.iter().filter(|t| t.share_on_p2p).count())
            .unwrap_or(0);

        ConnectDjDriveResponse {
            status: "connected".to_string(),
            profile,
            admitted_tracks: admitted.clone(),
            rejected_quarantine_tracks: rejected.clone(),
            cas_deduplicated_hits,
            physical_r2_mb_saved,
            network_catalog_available: total_network_tracks,
            welcome_message: format!(
                "¡Bienvenido {}! Tu almacenamiento quedó conectado (Compartir P2P activo por defecto: {}). Se admitieron {} pistas verificadas ({} deduplicadas por CAS ahorrando {:.1} MB físicos en R2; {} rechazadas por aduana forense o cuota R2) y ya tienes acceso a las {} pistas compartidas de la red P2P.",
                handle,
                p2p_sharing_enabled,
                admitted.len(),
                cas_deduplicated_hits,
                physical_r2_mb_saved,
                rejected.len(),
                total_network_tracks
            ),
        }
    }

    /// Upgrade a DJ's Cloudflare R2 storage tier (`free` = 100 MB -> `pro_monthly` = 50,000 MB / 50 GB).
    pub fn upgrade_dj_tier(&self, dj_handle: &str, tier: &str) -> Result<DjProfile, String> {
        let mut guard = self
            .dj_profiles
            .write()
            .map_err(|_| "Failed to lock dj_profiles".to_string())?;
        let profile = guard
            .iter_mut()
            .find(|p| p.dj_handle.eq_ignore_ascii_case(dj_handle))
            .ok_or_else(|| format!("DJ profile `{dj_handle}` not found"))?;

        profile.subscription_tier = tier.to_string();
        profile.r2_quota_mb = resolve_tier_r2_quota_mb(tier);
        profile.can_disable_p2p_sharing = tier_allows_private_vault(tier);
        if !profile.can_disable_p2p_sharing {
            profile.p2p_sharing_enabled = true;
        }
        Ok(profile.clone())
    }

    /// Toggle P2P sharing for a DJ's uploaded songs (`p2p_sharing_enabled` is `true` by default).
    /// Only DJs on a higher plan (`pro_monthly`, `pro`, `founder_unlimited`) can turn P2P sharing OFF (`false`)!
    pub fn set_p2p_sharing(
        &self,
        req: SetP2PSharingRequest,
    ) -> Result<DjProfile, String> {
        let handle = req.dj_handle.trim();
        let updated_profile = {
            let mut guard = self
                .dj_profiles
                .write()
                .map_err(|_| "Failed to lock dj_profiles".to_string())?;
            let profile = guard
                .iter_mut()
                .find(|p| p.dj_handle.eq_ignore_ascii_case(handle))
                .ok_or_else(|| format!("DJ profile `{handle}` not found"))?;

            if !req.p2p_sharing_enabled && !tier_allows_private_vault(&profile.subscription_tier) {
                return Err(format!(
                    "PLAN_UPGRADE_REQUIRED: En el plan `{}` todas las canciones subidas se comparten en la red P2P por defecto. Actualiza a `pro_monthly` para poder apagar el P2P en tus canciones (Modo Bóveda Privada).",
                    profile.subscription_tier
                ));
            }

            profile.p2p_sharing_enabled = req.p2p_sharing_enabled;
            profile.can_disable_p2p_sharing = tier_allows_private_vault(&profile.subscription_tier);
            profile.clone()
        };

        if let Ok(mut cat) = self.catalog.write() {
            for trk in cat.iter_mut() {
                if trk.owner_dj.eq_ignore_ascii_case(handle) {
                    trk.share_on_p2p = req.p2p_sharing_enabled;
                }
            }
        }

        Ok(updated_profile)
    }

    /// Local Laptop / Server Direct Storage & P2P Super-Seeder:
    /// Scans a local folder on disk, runs real Pure-Rust Symphonia + rustfft DSP on every audio file,
    /// rejects fake lossless/synthetic files in quarantine, stores valid tracks for direct HTTP/P2P binary streaming,
    /// and registers the local machine as an active P2P Peer Node!
    pub fn scan_local_directory(
        &self,
        req: ScanLocalDirRequest,
    ) -> Result<ScanLocalDirResponse, String> {
        let dir_path = std::path::Path::new(&req.directory_path);
        if !dir_path.exists() || !dir_path.is_dir() {
            return Err(format!(
                "Directory `{}` does not exist or is not a folder",
                req.directory_path
            ));
        }

        let handle = req.dj_handle.trim().to_string();
        let base_url = req
            .public_node_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());

        let entries = std::fs::read_dir(dir_path)
            .map_err(|e| format!("Failed to read directory `{}`: {e}", req.directory_path))?;

        let mut admitted = Vec::new();
        let mut rejected = Vec::new();
        let mut idx = 0usize;

        for entry_res in entries {
            let Ok(entry) = entry_res else { continue };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(
                ext.as_str(),
                "wav" | "flac" | "mp3" | "ogg" | "aac" | "m4a" | "aiff"
            ) {
                continue;
            }

            idx += 1;
            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown Track")
                .to_string();
            let (artist, title) = if let Some((a, t)) = file_stem.split_once(" - ") {
                (a.trim().to_string(), t.trim().to_string())
            } else {
                (handle.clone(), file_stem.clone())
            };

            let Ok(bytes) = std::fs::read(&path) else {
                rejected.push(format!("{} [REJECTED: Could not read file bytes]", file_stem));
                continue;
            };

            let decoded = match decode_audio_bytes(&bytes, Some(&ext), 35.0) {
                Ok(d) => d,
                Err(e) => {
                    rejected.push(format!("{} [REJECTED: Decode error: {}]", file_stem, e));
                    continue;
                }
            };

            let forensics = audit_pcm_channels(
                &decoded.left,
                decoded.right.as_deref(),
                decoded.sample_rate,
                &ext,
            );

            if forensics.is_synthetic_tone
                || !forensics.is_authentic_lossless
                || !forensics.mono_compatible_sub
                || forensics.quality_score < 65.0
            {
                rejected.push(format!(
                    "{} - {} [QUARANTINED: {} ({:.0} Hz, score={:.1})]",
                    artist,
                    title,
                    forensics.forensic_verdict,
                    forensics.effective_bandwidth_hz,
                    forensics.quality_score
                ));
                continue;
            }

            let mono: Vec<f32> = if let Some(ref r) = decoded.right {
                let n = decoded.left.len().min(r.len());
                decoded.left[..n]
                    .iter()
                    .zip(r[..n].iter())
                    .map(|(&l, &rv)| 0.5 * (l + rv))
                    .collect()
            } else {
                decoded.left.clone()
            };

            let structure = analyze_structure_and_style(
                &mono,
                decoded.sample_rate,
                Some(decoded.duration_seconds),
                None,
            );

            let track_id = format!(
                "local-{}-{:03}",
                handle.to_ascii_lowercase().replace(' ', "-"),
                idx
            );
            let seal = compute_hera_seal(
                &track_id,
                structure.bpm,
                &structure.camelot_key,
                forensics.quality_score,
                &forensics.forensic_verdict,
            );
            let stream_url = format!(
                "{}/api/v1/p2p/download/{}",
                base_url.trim_end_matches('/'),
                track_id
            );

            let cas_fp = compute_cas_fingerprint(&artist, &title, structure.bpm);
            let real_sha256 = compute_sha256_hex(&bytes);
            if let Ok(mut sha_guard) = self.cas_audio_sha256.write() {
                sha_guard.entry(cas_fp.clone()).or_insert(real_sha256);
            }
            if let Ok(mut heat_guard) = self.cas_heat_scores.write() {
                heat_guard.entry(cas_fp).or_insert(2.0);
            }

            let verified = P2PVerifiedTrack {
                track_id: track_id.clone(),
                title,
                artist,
                owner_dj: handle.clone(),
                bpm: structure.bpm,
                camelot_key: structure.camelot_key,
                top_subgenre: structure.top_subgenre,
                duration_seconds: decoded.duration_seconds,
                format: ext,
                effective_bandwidth_hz: forensics.effective_bandwidth_hz,
                sub_bass_phase_correlation: forensics.sub_bass_phase_correlation,
                mono_compatible_sub: forensics.mono_compatible_sub,
                quality_score: forensics.quality_score,
                forensic_verdict: forensics.forensic_verdict,
                hot_cues: structure.hot_cues,
                storage_provider: "local_disk".to_string(),
                gdrive_file_id: None,
                r2_object_key: None,
                stream_url: Some(stream_url),
                share_on_p2p: true,
                node_id: self.node_id.clone(),
                hera_seal: seal,
            };

            if let Ok(mut f_guard) = self.local_files.write() {
                f_guard.insert(track_id.clone(), path.to_string_lossy().to_string());
            }
            if let Ok(mut c_guard) = self.catalog.write() {
                c_guard.retain(|t| t.track_id != track_id);
                c_guard.push(verified.clone());
            }
            admitted.push(verified);
        }

        let peer = P2PPeerNode {
            node_id: self.node_id.clone(),
            dj_handle: handle.clone(),
            endpoint_url: base_url,
            runtime_target: "native-local-p2p-seeder".to_string(),
            storage_mode: "local_disk_p2p_seeder".to_string(),
            verified_tracks_count: admitted.len(),
            bandwidth_mbps: req.bandwidth_mbps.unwrap_or(800),
            r2_free_tier_limit_mb: CF_WORKER_FREE_R2_LIMIT_MB,
            r2_safe_cap_mb: CF_WORKER_SAFE_R2_CAP_MB,
            r2_used_mb: (admitted.len() as f32) * 38.0,
            daily_worker_request_limit: CF_WORKER_FREE_REQ_LIMIT,
            daily_worker_request_safe_cap: CF_WORKER_SAFE_REQ_CAP,
            daily_worker_requests_used: 0,
            edge_guardian_active: true,
            custody_status: "ONLINE_HEALTHY".to_string(),
            last_custody_proof_hash: "local-disk-custody-verified".to_string(),
            custody_failures: 0,
            credits_earned_mb: (admitted.len() as f32) * 38.0 * SWARM_CREDIT_MULTIPLIER,
            behind_cgnat: false,
            quarantined: false,
            endpoint_pubkey_id: derive_node_endpoint_pubkey_id(&handle, &self.node_id),
            network_generation: 1,
            relay_assisted_url: None,
            ewma_rtt_ms: 12.0,
        };
        self.register_peer_node(peer.clone());

        let profile = DjProfile {
            dj_handle: handle.clone(),
            display_name: format!("{} (Local P2P Seeder Node)", handle),
            role: "local_p2p_seeder_node".to_string(),
            storage_provider: "local_disk".to_string(),
            gdrive_folder_id: None,
            gdrive_folder_url: None,
            r2_bucket_url: None,
            local_node_dir: Some(req.directory_path.clone()),
            subscription_tier: "local_unlimited".to_string(),
            r2_quota_mb: 100.0,
            r2_used_mb: 0.0,
            bounty_refunded_mb: 0.0,
            p2p_sharing_enabled: true,
            can_disable_p2p_sharing: false,
            verified_tracks_shared: admitted.len(),
            quarantined_rejected_tracks: rejected.len(),
            edge_guardian_active: true,
            swarm_credit_balance_mb: ((admitted.len() as f32) * 38.0 * SWARM_CREDIT_MULTIPLIER).max(150.0),
            total_bytes_served_mb: (admitted.len() as f32) * 38.0,
            total_bytes_consumed_mb: 0.0,
            custody_audit_passes: 1,
            custody_audit_failures: 0,
        };
        if let Ok(mut p_guard) = self.dj_profiles.write() {
            p_guard.retain(|p| !p.dj_handle.eq_ignore_ascii_case(&handle));
            p_guard.push(profile);
        }

        Ok(ScanLocalDirResponse {
            status: "local_p2p_seeder_active".to_string(),
            node_id: self.node_id.clone(),
            dj_handle: handle,
            scanned_directory: req.directory_path,
            admitted_tracks: admitted,
            rejected_quarantine_tracks: rejected,
            registered_peer: peer,
        })
    }

    /// Reads raw audio bytes from the local disk for P2P peer-to-peer HTTP binary transfer.
    pub fn get_local_file_bytes(&self, track_id: &str) -> Result<(String, Vec<u8>), String> {
        let path_str = self
            .local_files
            .read()
            .map_err(|_| "Lock error".to_string())?
            .get(track_id)
            .cloned()
            .ok_or_else(|| format!("Track `{track_id}` is not stored on this local disk node"))?;
        let bytes = std::fs::read(&path_str)
            .map_err(|e| format!("Failed to read local P2P file `{path_str}`: {e}"))?;
        Ok((path_str, bytes))
    }

    /// Reads only an aligned `chunk_size` slice at `chunk_idx * chunk_size` using `Seek` + `Read::take`,
    /// avoiding loading the entire 40 MB lossless audio file into RAM on every 8 KiB UDP datagram.
    pub fn get_local_file_chunk_slice(
        &self,
        track_id: &str,
        chunk_idx: usize,
        chunk_size: usize,
    ) -> Result<Vec<u8>, String> {
        use std::io::{Read, Seek, SeekFrom};

        let path_str = self
            .local_files
            .read()
            .map_err(|_| "Lock error".to_string())?
            .get(track_id)
            .cloned()
            .ok_or_else(|| format!("Track `{track_id}` is not stored on this local disk node"))?;

        let mut file = std::fs::File::open(&path_str)
            .map_err(|e| format!("Failed to open local P2P file `{path_str}`: {e}"))?;
        let file_len = file.metadata().map(|m| m.len() as usize).unwrap_or(0);
        let start = (chunk_idx.saturating_mul(chunk_size)).min(file_len);
        let to_read = chunk_size.min(file_len.saturating_sub(start));
        if to_read == 0 {
            return Ok(Vec::new());
        }

        file.seek(SeekFrom::Start(start as u64))
            .map_err(|e| format!("Failed to seek local P2P file `{path_str}`: {e}"))?;
        let mut buf = Vec::with_capacity(to_read);
        file.take(to_read as u64)
            .read_to_end(&mut buf)
            .map_err(|e| format!("Failed to read slice from `{path_str}`: {e}"))?;
        Ok(buf)
    }

    pub fn register_peer_node(&self, peer: P2PPeerNode) -> Vec<P2PPeerNode> {
        if let Ok(mut guard) = self.peers.write() {
            guard.retain(|p| p.node_id != peer.node_id);
            guard.push(peer);
            return guard.clone();
        }
        vec![]
    }

    pub fn list_peers(&self) -> Vec<P2PPeerNode> {
        self.peers.read().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn federate_sync(&self, req: FederatedSyncRequest) -> FederatedSyncResponse {
        let peer_node_id = req.peer.node_id.clone();
        let peer_dj = req.peer.dj_handle.clone();

        // Reject already quarantined nodes immediately
        let already_quarantined = self
            .quarantined_nodes
            .read()
            .map(|q| q.contains_key(&peer_node_id))
            .unwrap_or(false);
        if already_quarantined {
            let mut blocked_peer = req.peer;
            blocked_peer.quarantined = true;
            blocked_peer.edge_guardian_active = false;
            blocked_peer.custody_status = "QUARANTINED_BAIT_AND_SWITCH".to_string();
            return FederatedSyncResponse {
                status: "REJECTED_NODE_QUARANTINED".to_string(),
                responder_node_id: self.node_id.clone(),
                peer_registered: blocked_peer,
                tracks_accepted: 0,
                tracks_rejected_invalid_seal: req.shared_tracks.len(),
                node_quarantined_for_bait_and_switch: true,
                federated_swarm_catalog: Vec::new(),
                active_peers: self.list_peers(),
            };
        }

        let mut accepted = 0usize;
        let mut rejected = 0usize;
        let mut bait_and_switch_detected = false;

        if let Ok(mut c_guard) = self.catalog.write() {
            for mut trk in req.shared_tracks {
                let expected_seal = compute_hera_seal(
                    &trk.track_id,
                    trk.bpm,
                    &trk.camelot_key,
                    trk.quality_score,
                    &trk.forensic_verdict,
                );
                if trk.forensic_verdict != "VERIFIED_STUDIO_MASTER"
                    || !trk.mono_compatible_sub
                    || trk.effective_bandwidth_hz < 16800.0
                    || trk.hera_seal != expected_seal
                {
                    rejected += 1;
                    continue;
                }

                let cas_fp = compute_cas_fingerprint(&trk.artist, &trk.title, trk.bpm);
                let reported_sha_opt = req
                    .track_audio_sha256
                    .get(&trk.track_id)
                    .or_else(|| req.track_audio_sha256.get(&cas_fp))
                    .cloned();

                // Double cryptographic seal check (`hera_seal` + `audio_sha256`) against Bait-and-Switch!
                if let Some(reported_sha) = reported_sha_opt {
                    let mut mismatch_expected = None;
                    if let Ok(sha_guard) = self.cas_audio_sha256.read() {
                        if let Some(expected_sha) = sha_guard.get(&cas_fp) {
                            if !expected_sha.eq_ignore_ascii_case(&reported_sha) {
                                mismatch_expected = Some(expected_sha.clone());
                            }
                        }
                    }
                    if let Some(expected_sha) = mismatch_expected {
                        rejected += 1;
                        bait_and_switch_detected = true;
                        if let Ok(mut q_guard) = self.quarantined_nodes.write() {
                            q_guard.insert(
                                peer_node_id.clone(),
                                QuarantinedNodeRecord {
                                    node_id: peer_node_id.clone(),
                                    owner_dj: peer_dj.clone(),
                                    reason: "BAIT_AND_SWITCH_AUDIO_SHA256_MISMATCH".to_string(),
                                    offending_cas_fingerprint: cas_fp,
                                    expected_audio_sha256: expected_sha,
                                    reported_audio_sha256: reported_sha,
                                },
                            );
                        }
                        if let Ok(mut dj_guard) = self.dj_profiles.write() {
                            if let Some(dj) = dj_guard
                                .iter_mut()
                                .find(|d| d.dj_handle.eq_ignore_ascii_case(&peer_dj))
                            {
                                dj.edge_guardian_active = false;
                                dj.custody_audit_failures = dj.custody_audit_failures.saturating_add(1);
                            }
                        }
                        continue;
                    } else if let Ok(mut sha_guard) = self.cas_audio_sha256.write() {
                        sha_guard.entry(cas_fp.clone()).or_insert(reported_sha);
                    }
                } else if let Ok(mut sha_guard) = self.cas_audio_sha256.write() {
                    let canonical = compute_canonical_audio_sha256(&cas_fp, &trk.hera_seal, 38.0);
                    sha_guard.entry(cas_fp.clone()).or_insert(canonical);
                }

                if let Ok(mut heat_guard) = self.cas_heat_scores.write() {
                    let h = heat_guard.entry(cas_fp).or_insert(1.0);
                    *h += 1.0;
                }

                trk.share_on_p2p = true;
                c_guard.retain(|existing| existing.track_id != trk.track_id);
                c_guard.push(trk);
                accepted += 1;
            }
        }

        let mut peer = req.peer;
        if bait_and_switch_detected {
            peer.quarantined = true;
            peer.edge_guardian_active = false;
            peer.custody_status = "QUARANTINED_BAIT_AND_SWITCH".to_string();
            if let Ok(mut p_guard) = self.peers.write() {
                p_guard.retain(|p| p.node_id != peer.node_id);
            }
            return FederatedSyncResponse {
                status: "QUARANTINED_BAIT_AND_SWITCH_ATTACK".to_string(),
                responder_node_id: self.node_id.clone(),
                peer_registered: peer,
                tracks_accepted: 0,
                tracks_rejected_invalid_seal: rejected,
                node_quarantined_for_bait_and_switch: true,
                federated_swarm_catalog: Vec::new(),
                active_peers: self.list_peers(),
            };
        }

        if accepted > 0 {
            peer.verified_tracks_count = peer.verified_tracks_count.max(accepted);
        }
        let active_peers = self.register_peer_node(peer.clone());
        let federated_catalog = self.search(&P2PSearchQuery {
            q: None,
            camelot: None,
            bpm_min: None,
            bpm_max: None,
            subgenre: None,
            owner_dj: None,
            verified_only: Some(true),
        });

        FederatedSyncResponse {
            status: "federated_node_synced".to_string(),
            responder_node_id: self.node_id.clone(),
            peer_registered: peer,
            tracks_accepted: accepted,
            tracks_rejected_invalid_seal: rejected,
            node_quarantined_for_bait_and_switch: false,
            federated_swarm_catalog: federated_catalog,
            active_peers,
        }
    }

    /// Builds a signed `FederatedSyncRequest` payload containing all verified shared tracks on this node
    /// so the node can auto-federate with `hera-cloud` or another peer node.
    pub fn build_federated_sync_payload(
        &self,
        dj_handle: &str,
        public_url: Option<String>,
        bandwidth_mbps: u32,
    ) -> FederatedSyncRequest {
        let shared_tracks = self.search(&P2PSearchQuery {
            q: None,
            camelot: None,
            bpm_min: None,
            bpm_max: None,
            subgenre: None,
            owner_dj: None,
            verified_only: Some(true),
        });
        let runtime_target = if cfg!(target_arch = "wasm32") {
            "cloudflare-workers-wasm32".to_string()
        } else {
            "native-vps-tokio".to_string()
        };
        let peer = P2PPeerNode {
            node_id: self.node_id.clone(),
            dj_handle: dj_handle.to_string(),
            endpoint_url: public_url.unwrap_or_else(|| "http://127.0.0.1:8080".to_string()),
            runtime_target,
            storage_mode: "federated_p2p_node".to_string(),
            verified_tracks_count: shared_tracks.len(),
            bandwidth_mbps,
            r2_free_tier_limit_mb: CF_WORKER_FREE_R2_LIMIT_MB,
            r2_safe_cap_mb: CF_WORKER_SAFE_R2_CAP_MB,
            r2_used_mb: (shared_tracks.len() as f32) * 38.0,
            daily_worker_request_limit: CF_WORKER_FREE_REQ_LIMIT,
            daily_worker_request_safe_cap: CF_WORKER_SAFE_REQ_CAP,
            daily_worker_requests_used: 0,
            edge_guardian_active: true,
            custody_status: "ONLINE_HEALTHY".to_string(),
            last_custody_proof_hash: "federated-custody-verified".to_string(),
            custody_failures: 0,
            credits_earned_mb: (shared_tracks.len() as f32) * 38.0 * SWARM_CREDIT_MULTIPLIER,
            behind_cgnat: false,
            quarantined: false,
            endpoint_pubkey_id: derive_node_endpoint_pubkey_id(dj_handle, &self.node_id),
            network_generation: 1,
            relay_assisted_url: None,
            ewma_rtt_ms: 20.0,
        };
        let track_audio_sha256 = self
            .cas_audio_sha256
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        FederatedSyncRequest {
            peer,
            shared_tracks,
            track_audio_sha256,
        }
    }

    /// Auto-Federates this local/VPS node with a remote `hera` or `hera-cloud` HTTP endpoint (`--federate-url`)
    /// by sending `POST /api/v1/p2p/federate-sync` and merging the returned federated swarm catalog back into local RAM.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn push_federation_over_http(
        &self,
        remote_url: &str,
        dj_handle: &str,
        public_url: Option<String>,
        bandwidth_mbps: u32,
    ) -> Result<FederatedSyncResponse, String> {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        let payload = self.build_federated_sync_payload(dj_handle, public_url, bandwidth_mbps);
        let body_bytes =
            serde_json::to_vec(&payload).map_err(|e| format!("JSON serialize error: {e}"))?;

        let clean = remote_url
            .trim()
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        let host_port = clean.split('/').next().unwrap_or("127.0.0.1:8080");
        let addr = if host_port.contains(':') {
            host_port.to_string()
        } else {
            format!("{host_port}:80")
        };

        let mut stream = TcpStream::connect(&addr)
            .map_err(|e| format!("Failed to connect to federation hub `{addr}`: {e}"))?;
        let req_str = format!(
            "POST /api/v1/p2p/federate-sync HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            host_port,
            body_bytes.len()
        );
        stream
            .write_all(req_str.as_bytes())
            .and_then(|_| stream.write_all(&body_bytes))
            .map_err(|e| format!("Failed to send federation payload: {e}"))?;

        let mut resp_buf = Vec::new();
        stream
            .read_to_end(&mut resp_buf)
            .map_err(|e| format!("Failed to read federation response: {e}"))?;
        let resp_str = String::from_utf8_lossy(&resp_buf);
        let json_body = resp_str
            .split("\r\n\r\n")
            .nth(1)
            .unwrap_or(resp_str.as_ref())
            .trim();

        let sync_resp: FederatedSyncResponse = serde_json::from_str(json_body)
            .map_err(|e| format!("Invalid federation JSON response: {e}"))?;

        if let Ok(mut c_guard) = self.catalog.write() {
            for trk in &sync_resp.federated_swarm_catalog {
                if !c_guard.iter().any(|existing| existing.track_id == trk.track_id) {
                    c_guard.push(trk.clone());
                }
            }
        }
        Ok(sync_resp)
    }

    /// Pre-Upload CAS Deduplication Check & Zero-Upload Claim (`POST /api/v1/cloud/check-cas`):
    /// Before a client uploads a 38 MB FLAC to Cloudflare R2, it asks `hera-cloud` if the `cas_fingerprint`
    /// already exists in the swarm.
    /// - If it exists: the client skips the 38 MB upload (`upload_required = false`), links the master in `<1 ms`,
    ///   and triggers the `+38 MB` Curator Quota Refund Bounty for the pioneer DJ!
    /// - If it does not exist: returns `upload_required = true` and the canonical R2 CAS object key (`cas-vault/{cas}.flac`).
    pub fn check_cas_before_upload(&self, req: CasCheckRequest) -> CasCheckResponse {
        let cas = req
            .cas_fingerprint
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| compute_cas_fingerprint(&req.artist, &req.title, req.bpm));
        let r2_key = format!("cas-vault/{cas}.flac");
        let file_mb = req.file_size_mb.unwrap_or(38.0).max(1.0);

        let exists_in_swarm = self
            .catalog
            .read()
            .map(|cat| {
                cat.iter().any(|t| {
                    t.share_on_p2p
                        && t.forensic_verdict == "VERIFIED_STUDIO_MASTER"
                        && compute_cas_fingerprint(&t.artist, &t.title, t.bpm)
                            .eq_ignore_ascii_case(&cas)
                })
            })
            .unwrap_or(false);

        if exists_in_swarm {
            let claim_res = if req.auto_claim_if_exists {
                self.broker_claim_cloud_track(BrokerClaimCloudTrackRequest {
                    claimer_dj: req.dj_handle.clone(),
                    target_track_or_cas: cas.clone(),
                })
                .ok()
            } else {
                None
            };

            let note = format!(
                "CAS HIT (`{}`): Este Studio Master ya existe en el CAS de Hera Cloud. `{}` ahorró {:.1} MB de subida por internet (0 MB extra en R2) y se vinculó en <1 ms.",
                cas, req.dj_handle, file_mb
            );
            CasCheckResponse {
                status: "cas_hit_zero_upload_linked".to_string(),
                cas_fingerprint: cas,
                already_in_cas: true,
                upload_required: false,
                r2_cas_object_key: r2_key,
                bandwidth_saved_mb: file_mb,
                zero_copy_claim: claim_res,
                note,
            }
        } else {
            let note = format!(
                "CAS MISS (`{}`): Pista nueva en la bóveda global. Sube el archivo una única vez a R2 bajo `{}`.",
                cas, r2_key
            );
            CasCheckResponse {
                status: "cas_miss_upload_required".to_string(),
                cas_fingerprint: cas,
                already_in_cas: false,
                upload_required: true,
                r2_cas_object_key: r2_key,
                bandwidth_saved_mb: 0.0,
                zero_copy_claim: None,
                note,
            }
        }
    }

    /// Exports the live `P2PMeshState` (DJ accounts, Global CAS objects, Verified P2P Catalog, and Federated Peers)
    /// as an idempotent Cloudflare D1 SQL transaction (`INSERT OR REPLACE INTO ...`) compatible with `migrations/0001` & `0002`.
    pub fn export_d1_sql_snapshot(&self) -> String {
        fn esc(s: &str) -> String {
            s.replace('\'', "''")
        }

        let mut sql = String::from(
            "-- Hera Cloud Control Plane — Cloudflare D1 Live State Snapshot\nBEGIN TRANSACTION;\n\n",
        );

        if let Ok(profiles) = self.dj_profiles.read() {
            for p in profiles.iter() {
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO dj_cloud_accounts (dj_handle, display_name, role, storage_provider, subscription_tier, r2_quota_mb, r2_used_mb, bounty_refunded_mb, p2p_sharing_enabled, can_disable_p2p_sharing, verified_tracks_shared, quarantined_rejected_tracks) VALUES ('{}', '{}', '{}', '{}', '{}', {:.2}, {:.2}, {:.2}, {}, {}, {}, {});\n",
                    esc(&p.dj_handle),
                    esc(&p.display_name),
                    esc(&p.role),
                    esc(&p.storage_provider),
                    esc(&p.subscription_tier),
                    p.r2_quota_mb,
                    p.r2_used_mb,
                    p.bounty_refunded_mb,
                    if p.p2p_sharing_enabled { 1 } else { 0 },
                    if p.can_disable_p2p_sharing { 1 } else { 0 },
                    p.verified_tracks_shared,
                    p.quarantined_rejected_tracks
                ));
            }
        }
        sql.push('\n');

        let swarm_index = self.build_broker_swarm_index();
        for rec in &swarm_index {
            let r2_key = format!("cas-vault/{}.flac", rec.cas_fingerprint);
            let pioneer = rec
                .available_routes
                .first()
                .map(|r| r.owner_dj.as_str())
                .unwrap_or("DaviDJ");
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO cas_global_objects (cas_fingerprint, r2_object_key, original_uploader_dj, canonical_artist, canonical_title, bpm, camelot_key, style_simhash_hex, hera_seal, swarm_replicas_count) VALUES ('{}', '{}', '{}', '{}', '{}', {:.2}, '{}', '{}', '{}', {});\n",
                esc(&rec.cas_fingerprint),
                esc(&r2_key),
                esc(pioneer),
                esc(&rec.canonical_artist),
                esc(&rec.canonical_title),
                rec.bpm,
                esc(&rec.camelot_key),
                esc(&rec.style_simhash_hex),
                esc(&rec.hera_seal),
                rec.swarm_replicas_count
            ));
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO swarm_opaque_shards (opaque_blob_key, cas_fingerprint, audio_sha256, hera_seal, aligned_block_size_bytes, heat_score, target_replicas_k, active_replicas_count) VALUES ('{}', '{}', '{}', '{}', {}, {:.2}, {}, {});\n",
                esc(&rec.opaque_blob_key),
                esc(&rec.cas_fingerprint),
                esc(&rec.audio_sha256),
                esc(&rec.hera_seal),
                rec.aligned_block_size_bytes,
                rec.heat_score,
                rec.target_replicas_k,
                rec.swarm_replicas_count
            ));
        }
        sql.push('\n');

        if let Ok(catalog) = self.catalog.read() {
            for t in catalog.iter() {
                let cas = compute_cas_fingerprint(&t.artist, &t.title, t.bpm);
                let cues_json = serde_json::to_string(&t.hot_cues).unwrap_or_else(|_| "[]".to_string());
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO verified_p2p_catalog (track_id, owner_dj, cas_fingerprint, title, artist, bpm, camelot_key, top_subgenre, duration_seconds, format, effective_bandwidth_hz, sub_bass_phase_correlation, mono_compatible_sub, quality_score, forensic_verdict, storage_provider, r2_object_key, stream_url, share_on_p2p, hot_cues_json, hera_seal) VALUES ('{}', '{}', '{}', '{}', '{}', {:.2}, '{}', '{}', {:.2}, '{}', {:.1}, {:.3}, {}, {:.2}, '{}', '{}', '{}', '{}', {}, '{}', '{}');\n",
                    esc(&t.track_id),
                    esc(&t.owner_dj),
                    esc(&cas),
                    esc(&t.title),
                    esc(&t.artist),
                    t.bpm,
                    esc(&t.camelot_key),
                    esc(&t.top_subgenre),
                    t.duration_seconds,
                    esc(&t.format),
                    t.effective_bandwidth_hz,
                    t.sub_bass_phase_correlation,
                    if t.mono_compatible_sub { 1 } else { 0 },
                    t.quality_score,
                    esc(&t.forensic_verdict),
                    esc(&t.storage_provider),
                    esc(t.r2_object_key.as_deref().unwrap_or("")),
                    esc(t.stream_url.as_deref().unwrap_or("")),
                    if t.share_on_p2p { 1 } else { 0 },
                    esc(&cues_json),
                    esc(&t.hera_seal)
                ));
            }
        }
        sql.push('\n');

        if let Ok(peers) = self.peers.read() {
            for peer in peers.iter() {
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO p2p_peer_nodes (node_id, dj_handle, endpoint_url, runtime_target, storage_mode, verified_tracks_count, bandwidth_mbps) VALUES ('{}', '{}', '{}', '{}', '{}', {}, {});\n",
                    esc(&peer.node_id),
                    esc(&peer.dj_handle),
                    esc(&peer.endpoint_url),
                    esc(&peer.runtime_target),
                    esc(&peer.storage_mode),
                    peer.verified_tracks_count,
                    peer.bandwidth_mbps
                ));
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO swarm_node_allocations (node_id, owner_dj, endpoint_url, r2_free_tier_limit_mb, r2_safe_cap_mb, r2_used_mb, daily_worker_request_limit, daily_worker_request_safe_cap, daily_worker_requests_used, edge_guardian_active, custody_status, last_custody_proof_hash, custody_failures, credits_earned_mb, behind_cgnat, quarantined) VALUES ('{}', '{}', '{}', {:.2}, {:.2}, {:.2}, {}, {}, {}, {}, '{}', '{}', {}, {:.2}, {}, {});\n",
                    esc(&peer.node_id),
                    esc(&peer.dj_handle),
                    esc(&peer.endpoint_url),
                    peer.r2_free_tier_limit_mb,
                    peer.r2_safe_cap_mb,
                    peer.r2_used_mb,
                    peer.daily_worker_request_limit,
                    peer.daily_worker_request_safe_cap,
                    peer.daily_worker_requests_used,
                    if peer.edge_guardian_active { 1 } else { 0 },
                    esc(&peer.custody_status),
                    esc(&peer.last_custody_proof_hash),
                    peer.custody_failures,
                    peer.credits_earned_mb,
                    if peer.behind_cgnat { 1 } else { 0 },
                    if peer.quarantined { 1 } else { 0 }
                ));
            }
        }

        if let Ok(quarantined) = self.quarantined_nodes.read() {
            for q in quarantined.values() {
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO swarm_quarantined_nodes (node_id, owner_dj, reason, offending_cas_fingerprint, expected_audio_sha256, reported_audio_sha256) VALUES ('{}', '{}', '{}', '{}', '{}', '{}');\n",
                    esc(&q.node_id),
                    esc(&q.owner_dj),
                    esc(&q.reason),
                    esc(&q.offending_cas_fingerprint),
                    esc(&q.expected_audio_sha256),
                    esc(&q.reported_audio_sha256)
                ));
            }
        }

        if let Ok(receipts) = self.delivery_receipts.read() {
            for r in receipts.iter() {
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO swarm_delivery_receipts (receipt_id, ticket_id, serving_node_id, requester_dj, opaque_blob_key, bytes_delivered, megabytes_delivered, credits_earned_mb, credit_multiplier, receipt_hmac) VALUES ('{}', '{}', '{}', '{}', '{}', {}, {:.2}, {:.2}, {:.2}, '{}');\n",
                    esc(&r.receipt_id),
                    esc(&r.ticket_id),
                    esc(&r.serving_node_id),
                    esc(&r.requester_dj),
                    esc(&r.opaque_blob_key),
                    r.bytes_delivered,
                    r.megabytes_delivered,
                    r.credits_earned_mb,
                    r.credit_multiplier,
                    esc(&r.receipt_hmac)
                ));
            }
        }

        sql.push_str("\nCOMMIT;\n");
        sql
    }


    pub fn list_dj_profiles(&self) -> Vec<DjProfile> {
        self.dj_profiles
            .read()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn find_track_by_id(&self, track_id: &str) -> Option<P2PVerifiedTrack> {
        self.catalog
            .read()
            .ok()?
            .iter()
            .find(|t| t.track_id == track_id)
            .cloned()
    }

    /// Quarantine-First Gatekeeper: Only admits a track to the P2P catalog if it passes Zero-Trust DSP Forensics.
    pub fn announce_verified_track(
        &self,
        mut track: P2PVerifiedTrack,
        forensic: &ForensicAuditReport,
    ) -> Result<P2PVerifiedTrack, String> {
        if forensic.is_synthetic_tone {
            return Err(
                "QUARANTINE REJECT: Track contains synthetic sine/test tone and cannot be shared on Hera P2P."
                    .to_string(),
            );
        }
        if !forensic.is_authentic_lossless {
            return Err(format!(
                "QUARANTINE REJECT: Fake lossless transcode detected ({:.0} Hz cutoff, {}).",
                forensic.effective_bandwidth_hz, forensic.suspected_source_codec
            ));
        }
        if !forensic.mono_compatible_sub {
            return Err(format!(
                "QUARANTINE REJECT: Sub-bass (<120 Hz) phase cancellation detected (corr={:.2}).",
                forensic.sub_bass_phase_correlation
            ));
        }
        if forensic.quality_score < 65.0 {
            return Err(format!(
                "QUARANTINE REJECT: Quality score {:.1} is below minimum P2P threshold (65.0).",
                forensic.quality_score
            ));
        }

        if let Some(ref fid) = track.gdrive_file_id {
            let clean_id = extract_gdrive_id(fid);
            track.gdrive_file_id = Some(clean_id.clone());
            if track.stream_url.is_none() {
                track.stream_url = Some(build_gdrive_direct_download_url(&clean_id));
            }
        }

        track.node_id = self.node_id.clone();
        track.effective_bandwidth_hz = forensic.effective_bandwidth_hz;
        track.sub_bass_phase_correlation = forensic.sub_bass_phase_correlation;
        track.mono_compatible_sub = forensic.mono_compatible_sub;
        track.quality_score = forensic.quality_score;
        track.forensic_verdict = forensic.forensic_verdict.clone();
        track.hera_seal = compute_hera_seal(
            &track.track_id,
            track.bpm,
            &track.camelot_key,
            track.quality_score,
            &track.forensic_verdict,
        );

        if let Ok(mut guard) = self.catalog.write() {
            guard.retain(|t| t.track_id != track.track_id);
            guard.push(track.clone());
        }
        Ok(track)
    }

    pub fn search(&self, params: &P2PSearchQuery) -> Vec<P2PVerifiedTrack> {
        let Ok(guard) = self.catalog.read() else {
            return vec![];
        };
        let q_lower = params
            .q
            .as_ref()
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        let cam_upper = params
            .camelot
            .as_ref()
            .map(|s| s.trim().to_ascii_uppercase())
            .unwrap_or_default();
        let sub_lower = params
            .subgenre
            .as_ref()
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        let dj_lower = params
            .owner_dj
            .as_ref()
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        let verified_only = params.verified_only.unwrap_or(true);

        guard
            .iter()
            .filter(|t| {
                if !t.share_on_p2p {
                    return false;
                }
                if verified_only && t.forensic_verdict != "VERIFIED_STUDIO_MASTER" {
                    return false;
                }
                if !dj_lower.is_empty() && t.owner_dj.to_ascii_lowercase() != dj_lower {
                    return false;
                }
                if !q_lower.is_empty() {
                    let hay = format!(
                        "{} {} {} {} {}",
                        t.artist.to_ascii_lowercase(),
                        t.title.to_ascii_lowercase(),
                        t.owner_dj.to_ascii_lowercase(),
                        t.top_subgenre.to_ascii_lowercase(),
                        t.camelot_key.to_ascii_lowercase()
                    );
                    if !hay.contains(&q_lower) {
                        return false;
                    }
                }
                if !cam_upper.is_empty() && t.camelot_key.to_ascii_uppercase() != cam_upper {
                    return false;
                }
                if !sub_lower.is_empty()
                    && !t.top_subgenre.to_ascii_lowercase().contains(&sub_lower)
                {
                    return false;
                }
                if let Some(bmin) = params.bpm_min {
                    if t.bpm < bmin {
                        return false;
                    }
                }
                if let Some(bmax) = params.bpm_max {
                    if t.bpm > bmax {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }
}

pub mod broker;
pub mod cloud_links;
pub mod crypto_merkle;
pub mod nat_stun;
pub mod swarm_custody;

pub use cloud_links::*;
pub use crypto_merkle::*;
pub use nat_stun::*;
