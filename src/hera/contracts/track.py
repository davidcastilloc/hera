"""Contratos del modelo TRACK y ciclo de vida."""

from datetime import datetime, timezone
from enum import Enum
import uuid
from pydantic import BaseModel, Field


class TrackStatus(str, Enum):
    CANDIDATE = "candidate"
    DOWNLOADING = "downloading"
    QUARANTINED = "quarantined"
    VALIDATED = "validated"
    IDENTIFIED = "identified"
    ANALYZED = "analyzed"
    ORGANIZED = "organized"
    NEEDS_REVIEW = "needs_review"
    REJECTED = "rejected"
    DUPLICATE = "duplicate"
    DELETED = "deleted"


# Mapa de transiciones de estado permitidas
ALLOWED_TRANSITIONS: dict[TrackStatus, set[TrackStatus]] = {
    TrackStatus.CANDIDATE: {TrackStatus.DOWNLOADING, TrackStatus.REJECTED},
    TrackStatus.DOWNLOADING: {TrackStatus.QUARANTINED, TrackStatus.REJECTED, TrackStatus.NEEDS_REVIEW},
    TrackStatus.QUARANTINED: {TrackStatus.VALIDATED, TrackStatus.REJECTED, TrackStatus.NEEDS_REVIEW},
    TrackStatus.VALIDATED: {TrackStatus.IDENTIFIED, TrackStatus.REJECTED, TrackStatus.NEEDS_REVIEW},
    TrackStatus.IDENTIFIED: {TrackStatus.ANALYZED, TrackStatus.NEEDS_REVIEW, TrackStatus.DUPLICATE},
    TrackStatus.ANALYZED: {TrackStatus.ORGANIZED, TrackStatus.NEEDS_REVIEW, TrackStatus.DUPLICATE},
    TrackStatus.ORGANIZED: {TrackStatus.DELETED, TrackStatus.NEEDS_REVIEW},
    TrackStatus.NEEDS_REVIEW: {TrackStatus.IDENTIFIED, TrackStatus.ANALYZED, TrackStatus.ORGANIZED, TrackStatus.REJECTED, TrackStatus.DELETED},
    TrackStatus.REJECTED: {TrackStatus.DELETED},
    TrackStatus.DUPLICATE: {TrackStatus.DELETED, TrackStatus.NEEDS_REVIEW},
    TrackStatus.DELETED: set(),
}


class Track(BaseModel):
    id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    status: TrackStatus = TrackStatus.CANDIDATE
    canonical_title: str
    canonical_artist: str
    version: str | None = None

    # Duración e identificadores
    duration_ms: int | None = None
    recording_mbid: str | None = None
    release_mbid: str | None = None
    isrc: str | None = None

    # Huellas y hashes
    fingerprint: str | None = None
    audio_hash_sha256: str | None = None
    perceptual_hash: str | None = None

    # Formato técnico
    codec: str | None = None
    bitrate_kbps: int | None = None
    sample_rate_hz: int | None = None
    bit_depth: int | None = None
    channels: int | None = None
    file_size_bytes: int | None = None

    # Ubicación física
    quarantine_path: str | None = None
    library_path: str | None = None

    # Análisis acústico / DJ
    bpm: float | None = None
    bpm_confidence: float | None = None
    musical_key: str | None = None
    key_confidence: float | None = None
    camelot: str | None = None
    energy: float | None = None
    danceability: float | None = None
    loudness_lufs: float | None = None
    embedding_ref: str | None = None
    analysis_version: str | None = None

    # Autorización y procedencia
    license_basis: str | None = None
    authorization_evidence_ref: str | None = None
    provenance_json: dict | None = None

    # Fechas
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    updated_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    deleted_at: datetime | None = None

    def can_transition_to(self, new_status: TrackStatus) -> bool:
        return new_status in ALLOWED_TRANSITIONS.get(self.status, set())
