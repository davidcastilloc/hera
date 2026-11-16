-- Migration 08: Zero-IO Booth Metrics, 8-Cue Lock, Harmonic Index, Producer Folders & Shazam Landmarks
ALTER TABLE tracks ADD COLUMN sample_rate_hz INTEGER;
ALTER TABLE tracks ADD COLUMN bitrate_kbps INTEGER;
ALTER TABLE tracks ADD COLUMN legacy_bpm REAL;
ALTER TABLE tracks ADD COLUMN has_bpm_trust_diff INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tracks ADD COLUMN has_intro_vocals INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tracks ADD COLUMN has_outro_vocals INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tracks ADD COLUMN is_own_production INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tracks ADD COLUMN loudness_lufs REAL;
ALTER TABLE tracks ADD COLUMN true_peak_dbfs REAL;
ALTER TABLE tracks ADD COLUMN replay_gain_db REAL;
ALTER TABLE tracks ADD COLUMN sub_phase_corr REAL;
ALTER TABLE tracks ADD COLUMN waveform_peaks_b64 TEXT;
ALTER TABLE tracks ADD COLUMN embedding_64d_b64 TEXT;
ALTER TABLE tracks ADD COLUMN inferred_genre_family TEXT;
ALTER TABLE tracks ADD COLUMN inferred_era_year INTEGER;
ALTER TABLE tracks ADD COLUMN genre_confidence REAL;
ALTER TABLE tracks ADD COLUMN user_locked_cues INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_tracks_harmonic
  ON tracks (camelot, bpm);

CREATE TABLE IF NOT EXISTS producer_watch_folders (
  id TEXT PRIMARY KEY NOT NULL,
  folder_path TEXT NOT NULL UNIQUE,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS acoustic_landmarks (
  hash_u64 INTEGER NOT NULL,
  track_id TEXT NOT NULL,
  time_offset_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_acoustic_landmarks_hash
  ON acoustic_landmarks (hash_u64);

CREATE INDEX IF NOT EXISTS idx_acoustic_landmarks_track
  ON acoustic_landmarks (track_id);
