use super::P2PPeerNode;
use crate::structure::HotCuePoint;
use serde::{Deserialize, Serialize};

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

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_owner_dj() -> String {
    "DaviDJ".to_string()
}

fn default_storage_provider() -> String {
    "gdrive".to_string()
}

/// Core Swarm Motto: Every new node multiplies global R2 storage, daily request capacity, K-replication, and throughput.
pub const SWARM_MOTTO: &str = "Mientras más nodos de Hera, más fuerte es el sistema de enjambres";

/// Cloudflare Workers Free Tier R2 limit (10 GB = 10,000 MB) and Safe Water-Filling Cap (9,000 MB = $0 billable).
pub const CF_WORKER_FREE_R2_LIMIT_MB: f32 = 10_000.0;
pub const CF_WORKER_SAFE_R2_CAP_MB: f32 = 9_000.0;

/// Cloudflare Workers Free Tier daily requests limit (100,000 req/day) and Safe Cap (85,000 req/day = $0 billable).
pub const CF_WORKER_FREE_REQ_LIMIT: u64 = 100_000;
pub const CF_WORKER_SAFE_REQ_CAP: u64 = 85_000;

/// Maximum number of ProofOfDeliveryReceipt instances retained in memory before FIFO eviction.
pub const MAX_DELIVERY_RECEIPTS: usize = 10_000;

/// Standard binary mebibyte (1 MiB = 1,048,576 bytes) for integer byte math.
pub const BYTES_PER_MIB: u64 = 1_048_576;

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

/// Cryptographic secrets for Swarm tickets and catalog blinding.
#[derive(Debug, Clone)]
pub struct SwarmCryptoSecrets {
    pub ticket_hmac_secret: String,
    pub shard_blinding_secret: String,
}

impl Default for SwarmCryptoSecrets {
    fn default() -> Self {
        Self {
            ticket_hmac_secret: SWARM_TICKET_MASTER_SECRET.to_string(),
            shard_blinding_secret: SWARM_SHARD_BLINDING_SECRET.to_string(),
        }
    }
}

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
        "beta_mvp" | "free" => 5_120.0,     // 5 GB per Beta MVP DJ
        "beta_vip_10gb" => 10_240.0,        // 10 GB per VIP Beta DJ
        "pro_monthly" | "pro" => 51_200.0,  // 50 GB Cloudflare R2 in Pro Monthly
        "founder_unlimited" => 1_000_000.0, // Unlimited for DaviDJ
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
