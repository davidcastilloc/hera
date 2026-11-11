-- Migration 06: Add columns for video, mixes > 10m, audio skips, variable BPM, and DJ learning tables
ALTER TABLE tracks ADD COLUMN is_video BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE tracks ADD COLUMN is_mix_over_10m BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE tracks ADD COLUMN has_audio_skips BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE tracks ADD COLUMN is_variable_bpm BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE tracks ADD COLUMN entry_bpm REAL;
ALTER TABLE tracks ADD COLUMN exit_bpm REAL;

CREATE TABLE IF NOT EXISTS dj_golden_pairs (
    id TEXT PRIMARY KEY NOT NULL,
    from_track_id TEXT NOT NULL,
    to_track_id TEXT NOT NULL,
    from_artist TEXT NOT NULL DEFAULT '',
    from_title TEXT NOT NULL DEFAULT '',
    to_artist TEXT NOT NULL DEFAULT '',
    to_title TEXT NOT NULL DEFAULT '',
    custom_bonus REAL NOT NULL DEFAULT 0.38,
    note TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS dj_excluded_tracks (
    id TEXT PRIMARY KEY NOT NULL,
    track_id TEXT,
    title_pattern TEXT,
    reason TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS dj_feedback_memory (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT,
    category TEXT NOT NULL,
    rule_text TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tracks_mix_over_10m
    ON tracks(is_mix_over_10m);

CREATE INDEX IF NOT EXISTS idx_tracks_has_audio_skips
    ON tracks(has_audio_skips);

CREATE INDEX IF NOT EXISTS idx_golden_pairs_from
    ON dj_golden_pairs(from_track_id);
