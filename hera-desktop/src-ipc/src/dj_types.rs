use hera_node_rs::export::ExportTrackItem;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

fn default_pinned_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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
    #[ts(optional, as = "Option<_>")]
    pub p2p_boost_free_promo: bool,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub boost_credit_cost_mb: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SwarmCatalogItem {
    pub cas_fingerprint: String,
    pub artist: String,
    pub title: String,
    pub bpm: f32,
    pub camelot: String,
    pub subgenre: String,
    pub hera_seal: String,
    pub opaque_blob_key: String,
    #[ts(type = "number")]
    pub swarm_replicas_count: usize,
    pub seeders: Vec<String>,
    pub p2p_boost_ready: bool,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub p2p_boost_free_promo: bool,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub effective_bandwidth_hz: f32,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub quality_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ResolveHarmonicBridgeResponse {
    pub origin_summary: String,
    pub target_summary: String,
    pub harmonic_distance_steps: u8,
    pub bridge_steps: Vec<HarmonicBridgeStep>,
    pub preview_mix_window_seconds: u8,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub swarm_catalog: Vec<SwarmCatalogItem>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub stun_public_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProbeAiBackendResponse {
    pub reachable: bool,
    pub backend: String,
    pub base_url: String,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DownloadP2PTrackRequest {
    pub track_id: String,
    pub artist: String,
    pub title: String,
    pub bpm: f32,
    pub camelot: String,
    pub hera_seal: String,
    #[ts(optional = nullable)]
    pub peer_endpoint: Option<String>,
    #[ts(optional = nullable)]
    pub turn_relay_endpoint: Option<String>,
    #[ts(optional = nullable)]
    pub chunk_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct DownloadP2PTrackResponse {
    pub track_id: String,
    pub saved_path: String,
    pub quarantine_verified: bool,
    #[ts(type = "number")]
    pub bytes_written: usize,
    pub chunks_verified: u32,
    pub stage_used: String,
    pub sha256_hex: String,
    #[ts(type = "number")]
    pub r2_bytes_used: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SetMiniOverlayModeRequest {
    #[ts(type = "\"full\" | \"card\" | \"pill\"")]
    pub mode: String,
    #[serde(default = "default_pinned_true")]
    #[ts(optional, as = "Option<_>")]
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SetMiniOverlayModeResponse {
    #[ts(type = "\"full\" | \"card\" | \"pill\"")]
    pub mode: String,
    pub pinned: bool,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PushVirtualDjLiveCrateRequest {
    pub tracks: Vec<ExportTrackItem>,
    #[serde(default = "default_pinned_true")]
    #[ts(optional, as = "Option<_>")]
    pub append: bool,
    #[ts(optional = nullable)]
    pub crate_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PushVirtualDjLiveCrateResponse {
    pub crate_name: String,
    #[ts(type = "number")]
    pub total_tracks: usize,
    pub written_files: Vec<String>,
    pub synced_to_virtualdj_home: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct VirtualDjNowPlaying {
    pub artist: String,
    pub title: String,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FolderWorkabilityReport {
    pub total_files_scanned: usize,
    pub workable_tracks_under_10m: usize,
    pub mixes_over_10m: usize,
    pub short_sfx_discarded: usize,
    pub ignored_junk_folders: usize,
    pub vdj_cached_tracks: usize,
    pub rekordbox_cached_tracks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BpmUiBadgeInfo {
    pub bpm: f64,
    pub is_variable: bool,
    #[ts(optional = nullable)]
    pub entry_bpm: Option<f64>,
    #[ts(optional = nullable)]
    pub exit_bpm: Option<f64>,
    pub badge_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ScanPhaseProgress {
    pub phase: u8, // 1 = instant DB import (< 3s), 2 = throttled deep DSP
    pub current: usize,
    pub total: usize,
    pub is_safe_dj_paused: bool,
    pub phase_description: String,
}

