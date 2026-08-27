-- Migración 001: Esquema inicial de Hera
CREATE TABLE IF NOT EXISTS tracks (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    canonical_title TEXT NOT NULL,
    canonical_artist TEXT NOT NULL,
    version TEXT,
    duration_ms INTEGER,
    recording_mbid TEXT,
    release_mbid TEXT,
    isrc TEXT,
    fingerprint TEXT,
    audio_hash_sha256 TEXT,
    perceptual_hash TEXT,
    codec TEXT,
    bitrate_kbps INTEGER,
    sample_rate_hz INTEGER,
    bit_depth INTEGER,
    channels INTEGER,
    file_size_bytes INTEGER,
    quarantine_path TEXT,
    library_path TEXT,
    bpm REAL,
    bpm_confidence REAL,
    musical_key TEXT,
    key_confidence REAL,
    camelot TEXT,
    energy REAL,
    danceability REAL,
    loudness_lufs REAL,
    embedding_ref TEXT,
    analysis_version TEXT,
    license_basis TEXT,
    authorization_evidence_ref TEXT,
    provenance_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_tracks_status ON tracks(status);
CREATE INDEX IF NOT EXISTS idx_tracks_sha256 ON tracks(audio_hash_sha256);
CREATE INDEX IF NOT EXISTS idx_tracks_artist_title ON tracks(canonical_artist, canonical_title);

CREATE TABLE IF NOT EXISTS candidates (
    candidate_id TEXT PRIMARY KEY,
    search_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    native_ref TEXT NOT NULL,
    artist TEXT NOT NULL,
    title TEXT NOT NULL,
    version TEXT,
    duration_ms INTEGER,
    format TEXT,
    bitrate_kbps INTEGER,
    file_size_bytes INTEGER,
    score REAL NOT NULL,
    score_components_json TEXT NOT NULL,
    score_reasons_json TEXT NOT NULL,
    availability TEXT NOT NULL,
    authorization_state TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_candidates_search_id ON candidates(search_id);

CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,
    state TEXT NOT NULL,
    progress REAL NOT NULL DEFAULT 0.0,
    attempts INTEGER NOT NULL DEFAULT 0,
    idempotency_key TEXT UNIQUE NOT NULL,
    correlation_id TEXT NOT NULL,
    input_json TEXT NOT NULL,
    result_json TEXT,
    error_code TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_jobs_state ON jobs(state);
CREATE INDEX IF NOT EXISTS idx_jobs_idempotency ON jobs(idempotency_key);

CREATE TABLE IF NOT EXISTS crates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    brief TEXT NOT NULL,
    duration_target_minutes INTEGER NOT NULL,
    constraints_json TEXT NOT NULL,
    scoring_version TEXT NOT NULL,
    exports_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS crate_tracks (
    crate_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    transition_notes TEXT,
    PRIMARY KEY (crate_id, track_id),
    FOREIGN KEY (crate_id) REFERENCES crates(id) ON DELETE CASCADE,
    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS dj_preferences (
    profile_id TEXT NOT NULL,
    subject TEXT NOT NULL,
    feature TEXT NOT NULL,
    value TEXT NOT NULL,
    weight REAL NOT NULL,
    evidence TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (profile_id, subject, feature)
);

CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    actor TEXT NOT NULL,
    entity_id TEXT,
    policy_code TEXT,
    authorization_ref TEXT,
    details_json TEXT,
    created_at TEXT NOT NULL
);
