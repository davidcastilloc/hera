use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Optional per-track acoustic & Genre × Era overrides passed to the curator without
/// altering legacy `CrateTrackCandidate` serialized shapes.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TrackCurationHint {
    #[serde(default)]
    pub entry_bpm: Option<f32>,
    #[serde(default)]
    pub exit_bpm: Option<f32>,
    #[serde(default)]
    pub is_transition_candidate: bool,
    #[serde(default)]
    pub genre_family_code: Option<u8>,
    #[serde(default)]
    pub inferred_era_year: Option<u16>,
    #[serde(default)]
    pub genre_confidence: Option<f32>,
}

/// DJ Learned Curation Rules for crate optimization.
///
/// Encapsulates:
/// 1. Excluded tracks (e.g. "Clavelito ya no" retired tracks or user-blacklisted IDs/titles).
/// 2. Golden Transition Pairs (e.g. "Perdóname mi vida -> Conquístame" with synergy bonuses).
/// 3. Hard duration filtering for normal crates: tracks with duration > `max_track_duration_seconds`
///    (default: 600.0s = 10 minutes) are routed to a logical "Mixes (> 10m)" collection
///    without moving, renaming, or touching files on disk.
/// 4. Genre × Era isolation (`crossover_mode`, `target_genre_family`, `target_era_year`)
///    and optional per-track `entry_bpm` / `exit_bpm` hints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DjCurationRuleSet {
    /// Track IDs explicitly excluded from automatic crate curation (e.g. retired tracks).
    #[serde(default)]
    pub excluded_track_ids: Vec<String>,

    /// Substrings/patterns in track titles or artists to exclude (case-insensitive, e.g. "Clavelito").
    #[serde(default)]
    pub excluded_title_patterns: Vec<String>,

    /// Golden Transition Pairs that have proven dancefloor chemistry.
    #[serde(default)]
    pub golden_pairs: Vec<GoldenTransitionPair>,

    /// Maximum track duration in seconds for inclusion in standard DJ crates.
    /// Tracks exceeding this duration (e.g. > 600.0s / 10m) are routed to `routed_to_mixes_over_10m`.
    /// Zero file moves on disk!
    #[serde(default = "default_max_track_duration_seconds")]
    pub max_track_duration_seconds: f32,

    /// Synergy bonus added to transition matrix for golden pairs (default: +0.38).
    #[serde(default = "default_golden_pair_bonus")]
    pub golden_pair_synergy_bonus: f32,

    /// Whether to allow mixed Audio and Video candidate tracks (default: true).
    #[serde(default = "default_allow_video_tracks")]
    pub allow_video_tracks: bool,

    /// Allow deliberate cross-genre / cross-era sets (e.g. Hora Loca / Open-Format). Default: false.
    #[serde(default)]
    pub crossover_mode: bool,

    /// Optional explicit target Genre family (code `"1".."14"` or name like `"ReggaetonActualPerreo"`).
    #[serde(default)]
    pub target_genre_family: Option<String>,

    /// Optional target release era year (e.g. `1996` or `2025`).
    #[serde(default)]
    pub target_era_year: Option<u16>,

    /// Optional per-track acoustic / Genre × Era metadata keyed by `track_id`.
    #[serde(default)]
    pub track_hints: HashMap<String, TrackCurationHint>,
}

fn default_max_track_duration_seconds() -> f32 {
    600.0 // 10 minutes
}

fn default_golden_pair_bonus() -> f32 {
    0.38
}

fn default_allow_video_tracks() -> bool {
    true
}

impl Default for DjCurationRuleSet {
    fn default() -> Self {
        Self {
            excluded_track_ids: Vec::new(),
            excluded_title_patterns: Vec::new(),
            golden_pairs: Vec::new(),
            max_track_duration_seconds: default_max_track_duration_seconds(),
            golden_pair_synergy_bonus: default_golden_pair_bonus(),
            allow_video_tracks: default_allow_video_tracks(),
            crossover_mode: false,
            target_genre_family: None,
            target_era_year: None,
            track_hints: HashMap::new(),
        }
    }
}

/// A proven DJ Golden Transition Pair (e.g. Track A -> Track B).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoldenTransitionPair {
    pub from_track_id: String,
    pub to_track_id: String,
    /// Optional custom bonus override (defaults to rule set bonus if None).
    #[serde(default)]
    pub custom_bonus: Option<f32>,
    /// Optional human or AI annotation (e.g. "Perdóname mi vida -> Conquístame").
    #[serde(default)]
    pub note: Option<String>,
}

/// Extended result of crate optimization when DJ Curation Rules are applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratedCrateWithRulesResult {
    /// Base optimized crate result from 2-Opt TSP engine.
    pub base_result: crate::synergy::OptimizedCrateResult,

    /// Track IDs of long mixes (> 10m / 600s) cleanly routed to the logical Mixes area.
    /// Strictly zero file moves or disk alteration!
    pub routed_to_mixes_over_10m: Vec<String>,

    /// Track IDs excluded by curatorial rules (blacklists or title patterns like "Clavelito").
    pub excluded_tracks: Vec<String>,

    /// Count of Golden Transition Pairs successfully locked into adjacent slots in the final sequence.
    pub golden_pairs_locked: usize,

    /// Breakdown of video tracks included in the final sequence.
    pub video_tracks_count: usize,

    /// Track IDs logically isolated by Genre × Era rules (zero disk mutation).
    #[serde(default)]
    pub isolated_out_of_genre_tracks: Vec<String>,
}

impl DjCurationRuleSet {
    /// Check whether a candidate track is excluded by ID or title pattern.
    pub fn is_track_excluded(&self, track_id: &str, title: &str, artist: &str) -> bool {
        if self.excluded_track_ids.iter().any(|id| id == track_id) {
            return true;
        }

        let title_lower = title.to_lowercase();
        let artist_lower = artist.to_lowercase();

        for pat in &self.excluded_title_patterns {
            let pat_lower = pat.trim().to_lowercase();
            if !pat_lower.is_empty()
                && (title_lower.contains(&pat_lower) || artist_lower.contains(&pat_lower))
            {
                return true;
            }
        }

        false
    }

    /// Check if Track A -> Track B is an active golden pair and return bonus.
    pub fn get_golden_pair_bonus(&self, from_id: &str, to_id: &str) -> Option<f32> {
        for pair in &self.golden_pairs {
            if pair.from_track_id == from_id && pair.to_track_id == to_id {
                return Some(pair.custom_bonus.unwrap_or(self.golden_pair_synergy_bonus));
            }
        }
        None
    }
}
