"""Contratos de preferencias del DJ y aprendizaje explícito."""

from datetime import datetime, timezone
from pydantic import BaseModel, Field


class DjPreference(BaseModel):
    profile_id: str = Field(default="default", description="Identificador de perfil / alias DJ")
    subject: str = Field(description="Entidad o dimensión (format, label, version, bpm, camelot)")
    feature: str = Field(description="Característica específica (e.g. FLAC, Radio Edit, Afterlife)")
    value: str | float | bool
    weight: float = Field(default=1.0, ge=-1.0, le=1.0, description="Peso de preferencia (-1 = vetado, +1 = favorito)")
    evidence: str | None = Field(default="explicit", description="Origen de la preferencia (explicit, crate_selection, candidate_override)")
    updated_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class PreferenceProfile(BaseModel):
    profile_id: str = "default"
    name: str = "Perfil Principal"
    preferred_formats: list[str] = Field(default_factory=lambda: ["FLAC", "ALAC"])
    excluded_versions: list[str] = Field(default_factory=lambda: ["radio edit", "clean"])
    preferred_labels: list[str] = Field(default_factory=list)
    bpm_range: tuple[float, float] | None = None
    preferences: list[DjPreference] = Field(default_factory=list)
