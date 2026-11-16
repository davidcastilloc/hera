use sqlx::{QueryBuilder, Sqlite, SqliteConnection};

use super::error::AnyResult;
use super::track::Track;

#[derive(Debug, Clone)]
pub struct AgentTrackFilter {
    pub query: Option<String>,
    pub bpm_min: Option<f64>,
    pub bpm_max: Option<f64>,
    pub camelots: Vec<String>,
    pub genre_family: Option<String>,
    pub era_min: Option<i64>,
    pub era_max: Option<i64>,
    pub include_mixes_over_10m: bool,
    pub limit: usize,
}

impl Default for AgentTrackFilter {
    fn default() -> Self {
        Self {
            query: None,
            bpm_min: None,
            bpm_max: None,
            camelots: Vec::new(),
            genre_family: None,
            era_min: None,
            era_max: None,
            include_mixes_over_10m: false,
            limit: 200,
        }
    }
}

fn normalize_agent_track(mut track: Track) -> Track {
    if track.bpm.is_some_and(|b| b <= 0.0) {
        track.bpm = None;
    }
    if track.camelot.as_deref() == Some("") {
        track.camelot = None;
    }
    track
}

/// Counts workable library tracks (`scan_level >= 0`, `has_audio_skips = 0`, and `< 10m` unless
/// `include_mixes_over_10m` is requested) in `< 1 ms`.
pub async fn count_agent_workable_tracks(
    conn: &mut SqliteConnection,
    include_mixes_over_10m: bool,
) -> AnyResult<usize> {
    let sql = if include_mixes_over_10m {
        "SELECT COUNT(*) FROM tracks WHERE scan_level >= 0 AND has_audio_skips = 0"
    } else {
        "SELECT COUNT(*) FROM tracks WHERE scan_level >= 0 AND has_audio_skips = 0 AND is_mix_over_10m = 0 AND (duration = 0 OR duration <= 600)"
    };
    let (cnt,): (i64,) = sqlx::query_as(sql).fetch_one(conn).await?;
    Ok(cnt.max(0) as usize)
}

/// Executes `< 1 ms` indexed SQLite retrieval over `tracks` using `idx_tracks_harmonic (camelot, bpm)`
/// while strictly excluding quarantined (`scan_level < 0`) and scratched (`has_audio_skips = 1`) files.
pub async fn query_agent_tracks_filtered(
    conn: &mut SqliteConnection,
    filter: &AgentTrackFilter,
) -> AnyResult<Vec<Track>> {
    let mut qb: QueryBuilder<'_, Sqlite> = QueryBuilder::new(
        "SELECT
            id, path, title, album, album_artist, artists, genres, year, duration,
            track_no, track_of, disk_no, disk_of, is_compilation, bpm, camelot,
            energy, hera_seal, is_video, scan_level, is_transition_candidate,
            is_fake_320kbps, NULL AS waveform_peaks_blob, is_mix_over_10m,
            has_audio_skips, is_variable_bpm, entry_bpm, exit_bpm, hot_cues,
            replay_gain_db, loudness_lufs, has_intro_vocals, has_outro_vocals,
            is_own_production, has_bpm_trust_diff, waveform_peaks_b64,
            embedding_64d_b64, inferred_genre_family, inferred_era_year,
            partial_hash_64kb, file_size_bytes, legacy_bpm, bitrate_kbps, sample_rate_hz
         FROM tracks
         WHERE scan_level >= 0 AND has_audio_skips = 0",
    );

    if !filter.include_mixes_over_10m {
        qb.push(" AND is_mix_over_10m = 0 AND (duration = 0 OR duration <= 600)");
    }

    let clean_camelots: Vec<String> = filter
        .camelots
        .iter()
        .map(|c| c.trim().to_ascii_uppercase())
        .filter(|c| !c.is_empty())
        .collect();
    if clean_camelots.len() == 1 {
        qb.push(" AND camelot = ");
        qb.push_bind(clean_camelots[0].clone());
    } else if clean_camelots.len() > 1 {
        qb.push(" AND camelot IN (");
        let mut sep = qb.separated(", ");
        for cam in clean_camelots {
            sep.push_bind(cam);
        }
        sep.push_unseparated(")");
    }

    if let Some(min_bpm) = filter.bpm_min {
        qb.push(" AND bpm >= ");
        qb.push_bind(min_bpm);
    }
    if let Some(max_bpm) = filter.bpm_max {
        qb.push(" AND bpm <= ");
        qb.push_bind(max_bpm);
    }

    if let Some(ref gf) = filter.genre_family {
        let trimmed = gf.trim().to_lowercase();
        if !trimmed.is_empty() {
            let like_pat = format!("%{trimmed}%");
            qb.push(" AND (LOWER(COALESCE(inferred_genre_family, '')) = ");
            qb.push_bind(trimmed);
            qb.push(" OR (inferred_genre_family IS NULL AND LOWER(genres) LIKE ");
            qb.push_bind(like_pat);
            qb.push("))");
        }
    }

    if let Some(era_min) = filter.era_min {
        qb.push(" AND COALESCE(inferred_era_year, year) >= ");
        qb.push_bind(era_min);
    }
    if let Some(era_max) = filter.era_max {
        qb.push(" AND COALESCE(inferred_era_year, year) <= ");
        qb.push_bind(era_max);
    }

    if let Some(ref raw_q) = filter.query {
        let q = raw_q.trim().to_lowercase();
        if !q.is_empty() {
            let pat = format!("%{q}%");
            qb.push(" AND (LOWER(title) LIKE ");
            qb.push_bind(pat.clone());
            qb.push(" OR LOWER(album_artist) LIKE ");
            qb.push_bind(pat.clone());
            qb.push(" OR LOWER(artists) LIKE ");
            qb.push_bind(pat.clone());
            qb.push(" OR LOWER(album) LIKE ");
            qb.push_bind(pat.clone());
            qb.push(" OR LOWER(genres) LIKE ");
            qb.push_bind(pat);
            qb.push(")");
        }
    }

    let effective_limit = if filter.limit == 0 {
        200
    } else {
        filter.limit.clamp(1, 1000)
    } as i64;
    qb.push(" ORDER BY camelot ASC, bpm ASC LIMIT ");
    qb.push_bind(effective_limit);

    let rows: Vec<Track> = qb.build_query_as::<Track>().fetch_all(conn).await?;
    Ok(rows.into_iter().map(normalize_agent_track).collect())
}
