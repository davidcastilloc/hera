ALTER TABLE tracks ADD COLUMN scan_level INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tracks ADD COLUMN partial_hash_64kb TEXT;
ALTER TABLE tracks ADD COLUMN file_size_bytes INTEGER;
ALTER TABLE tracks ADD COLUMN is_transition_candidate INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tracks ADD COLUMN is_fake_320kbps INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tracks ADD COLUMN waveform_peaks_blob BLOB;
ALTER TABLE tracks ADD COLUMN user_locked_bpm_key INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_tracks_hash_size
  ON tracks (partial_hash_64kb, file_size_bytes);

CREATE INDEX IF NOT EXISTS idx_tracks_scan_level
  ON tracks (scan_level);

CREATE TABLE IF NOT EXISTS library_doctor_issues (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  track_id TEXT NOT NULL,
  file_path TEXT NOT NULL,
  issue_kind TEXT NOT NULL,
  severity TEXT NOT NULL,
  details TEXT NOT NULL,
  resolved INTEGER NOT NULL DEFAULT 0,
  detected_at_unix_ms INTEGER NOT NULL
);
