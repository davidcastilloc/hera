use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ScanLevel {
    Indexed = 0,
    Fast = 1,
    Full = 2,
}

impl ScanLevel {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Fast,
            2 => Self::Full,
            _ => Self::Indexed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackScanState {
    pub track_id: String,
    pub file_path: String,
    pub partial_hash_64kb: String,
    pub file_size_bytes: u64,
    pub mtime_unix_ms: i64,
    pub scan_level: ScanLevel,
    pub bpm: Option<f64>,
    pub camelot_key: Option<String>,
    pub is_transition_candidate: bool,
    pub is_fake_320kbps: bool,
    pub effective_bandwidth_hz: Option<f32>,
    pub updated_at_unix_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackLegacyCue {
    pub track_id: String,
    pub slot_index: u8,
    pub label: String,
    pub position_sec: f64,
    pub source_software: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDoctorIssue {
    pub id: Option<i64>,
    pub track_id: String,
    pub file_path: String,
    pub issue_kind: String,
    pub severity: String,
    pub details: String,
    pub detected_at_unix_ms: i64,
}

pub const SCANNER_V3_WAL_PRAGMAS: &str = "\
PRAGMA journal_mode = WAL;\n\
PRAGMA synchronous = NORMAL;\n\
PRAGMA busy_timeout = 5000;\n\
PRAGMA temp_store = MEMORY;\n";

pub const SCANNER_V3_MIGRATION_SQL: &str = "\
CREATE TABLE IF NOT EXISTS tracks_scan_state (
    track_id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL UNIQUE,
    partial_hash_64kb TEXT NOT NULL,
    file_size_bytes INTEGER NOT NULL,
    mtime_unix_ms INTEGER NOT NULL,
    scan_level INTEGER NOT NULL DEFAULT 0,
    bpm REAL,
    camelot_key TEXT,
    is_transition_candidate INTEGER NOT NULL DEFAULT 0,
    is_fake_320kbps INTEGER NOT NULL DEFAULT 0,
    effective_bandwidth_hz REAL,
    waveform_peaks_blob BLOB,
    updated_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tracks_scan_hash_size
    ON tracks_scan_state (partial_hash_64kb, file_size_bytes);

CREATE INDEX IF NOT EXISTS idx_tracks_scan_level
    ON tracks_scan_state (scan_level, updated_at_unix_ms);

CREATE TABLE IF NOT EXISTS track_legacy_cues (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    slot_index INTEGER NOT NULL,
    label TEXT NOT NULL,
    position_sec REAL NOT NULL,
    source_software TEXT NOT NULL,
    UNIQUE(track_id, slot_index, source_software)
);

CREATE INDEX IF NOT EXISTS idx_legacy_cues_track
    ON track_legacy_cues (track_id);

CREATE TABLE IF NOT EXISTS library_doctor_issues (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    issue_kind TEXT NOT NULL,
    severity TEXT NOT NULL,
    details TEXT NOT NULL,
    detected_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_doctor_issues_track
    ON library_doctor_issues (track_id, issue_kind);
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_v3_schema_and_scan_level_invariants() {
        assert!(ScanLevel::Indexed < ScanLevel::Fast);
        assert!(ScanLevel::Fast < ScanLevel::Full);
        assert_eq!(ScanLevel::from_u8(0), ScanLevel::Indexed);
        assert_eq!(ScanLevel::from_u8(1), ScanLevel::Fast);
        assert_eq!(ScanLevel::from_u8(2), ScanLevel::Full);
        assert!(SCANNER_V3_WAL_PRAGMAS.contains("journal_mode = WAL"));
        assert!(SCANNER_V3_MIGRATION_SQL.contains("tracks_scan_state"));
        assert!(SCANNER_V3_MIGRATION_SQL.contains("idx_tracks_scan_hash_size"));
        assert!(SCANNER_V3_MIGRATION_SQL.contains("track_legacy_cues"));
        assert!(SCANNER_V3_MIGRATION_SQL.contains("library_doctor_issues"));
    }
}
