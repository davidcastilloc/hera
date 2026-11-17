//! Typed IPC DTOs for HERA Desktop's OMKF Filesystem Catalog & Agentic File Manager (`ts-rs`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct OmkfFsHotCueDto {
    pub slot: u8,
    pub label: String,
    pub seconds: f64,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct OmkfFsTrackDto {
    pub id: String,
    pub crate_id: String,
    pub path: String,
    pub file_name: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_sec: u32,
    #[ts(optional = nullable)]
    pub bpm: Option<f64>,
    #[ts(optional = nullable)]
    pub camelot: Option<String>,
    #[ts(optional = nullable)]
    pub energy: Option<f64>,
    pub hot_cues: Vec<OmkfFsHotCueDto>,
    pub scan_level: i8,
    pub is_video: bool,
    pub is_fake_320kbps: bool,
    pub has_audio_skips: bool,
    pub is_variable_bpm: bool,
    pub has_intro_vocals: bool,
    pub has_outro_vocals: bool,
    #[ts(optional = nullable)]
    pub replay_gain_db: Option<f64>,
    #[ts(optional = nullable)]
    pub waveform_peaks_b64: Option<String>,
    #[ts(type = "number")]
    pub copy_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct FolderCrateSummary {
    pub crate_id: String,
    pub folder_path: String,
    pub display_name: String,
    pub tree_seal_64: String,
    #[ts(type = "number")]
    pub track_count: usize,
    #[ts(type = "number")]
    pub video_count: usize,
    #[ts(type = "number")]
    pub full_count: usize,
    #[ts(optional = nullable)]
    pub bpm_min: Option<f64>,
    #[ts(optional = nullable)]
    pub bpm_max: Option<f64>,
    #[ts(optional = nullable)]
    pub dominant_camelot: Option<String>,
    #[ts(optional = nullable)]
    pub dominant_genre: Option<String>,
    #[ts(optional = nullable)]
    pub era_span: Option<String>,
    #[ts(optional = nullable)]
    pub lufs_avg: Option<f64>,
    pub has_custom_rules: bool,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub updated_at_unix_ms: Option<i64>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub parent_crate_id: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub rel_path: String,
    #[serde(default)]
    #[ts(optional, as = "Option<u32>")]
    pub depth: usize,
    #[serde(default)]
    #[ts(optional, as = "Option<u32>")]
    pub subfolder_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct FolderCrateDetail {
    pub summary: FolderCrateSummary,
    pub rules_markdown: String,
    pub ledger_entries: Vec<String>,
    pub track_ids: Vec<String>,
    pub hydrated_from_omkf: bool,
    pub disk_online: bool,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub subfolders: Vec<FolderCrateSummary>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub tracks: Vec<OmkfFsTrackDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PlannedFileMove {
    pub track_id: String,
    pub source_path: String,
    pub target_path: String,
    pub reason: String,
    pub bucket_label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct OrganizeFolderRequest {
    pub crate_id: String,
    pub strategy: String,
    pub dry_run: bool,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub rollback_tx_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct OrganizeFolderResponse {
    pub crate_id: String,
    pub dry_run: bool,
    pub tx_id: String,
    pub moves: Vec<PlannedFileMove>,
    #[ts(type = "number")]
    pub applied_count: usize,
    pub rolled_back: bool,
}
