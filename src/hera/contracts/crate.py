"""Contratos para gestión de crates (playlists DJ) y exportaciones."""

from datetime import datetime, timezone
from enum import Enum
import uuid
from pydantic import BaseModel, Field


class ExportFormat(str, Enum):
    M3U8 = "m3u8"
    REKORDBOX_XML = "rekordbox_xml"
    JSON_MANIFEST = "json_manifest"


class CrateConstraints(BaseModel):
    bpm: tuple[float, float] | list[float] | None = None
    camelot_max_step: int | None = None
    exclude_versions: list[str] = Field(default_factory=list)
    required_tracks: list[str] = Field(default_factory=list)


class CrateTrack(BaseModel):
    track_id: str
    position: int
    transition_notes: str | None = None


class Crate(BaseModel):
    id: str = Field(default_factory=lambda: f"crate_{uuid.uuid4().hex[:8]}")
    name: str
    brief: str
    duration_target_minutes: int
    constraints: CrateConstraints = Field(default_factory=CrateConstraints)
    scoring_version: str = "crate/1.0"
    tracks: list[CrateTrack] = Field(default_factory=list)
    exports: dict[str, str] = Field(default_factory=dict)
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    updated_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
