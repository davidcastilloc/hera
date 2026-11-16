-- Migration 09: OMKF v1 folder_crates index, tracks.crate_id, and learned genre cue templates
ALTER TABLE tracks ADD COLUMN crate_id TEXT;

CREATE INDEX IF NOT EXISTS idx_tracks_crate_id
  ON tracks (crate_id);

CREATE TABLE IF NOT EXISTS folder_crates (
  crate_id TEXT PRIMARY KEY NOT NULL,
  folder_path TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  tree_seal_64 TEXT NOT NULL,
  dir_mtime_ns INTEGER NOT NULL,
  track_count INTEGER NOT NULL DEFAULT 0,
  video_count INTEGER NOT NULL DEFAULT 0,
  full_count INTEGER NOT NULL DEFAULT 0,
  bpm_min REAL,
  bpm_max REAL,
  dominant_camelot TEXT,
  camelot_mask INTEGER NOT NULL DEFAULT 0,
  dominant_genre TEXT,
  era_span TEXT,
  lufs_avg REAL,
  centroid_64d_b64 TEXT,
  rules_markdown TEXT NOT NULL DEFAULT '',
  updated_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS dj_learned_cue_rules (
  genre_family TEXT PRIMARY KEY NOT NULL,
  pad5_ratio REAL NOT NULL DEFAULT 0.55,
  pad6_ratio REAL NOT NULL DEFAULT 0.66,
  pad7_ratio REAL NOT NULL DEFAULT 0.78,
  pad8_ratio REAL NOT NULL DEFAULT 0.88,
  sample_count INTEGER NOT NULL DEFAULT 1,
  updated_at INTEGER NOT NULL
);
