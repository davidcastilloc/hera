"""Contratos del modelo CANDIDATE y puntuación."""

import uuid
from enum import Enum
from pydantic import BaseModel, Field


class AuthorizationState(str, Enum):
    CONFIRMED = "confirmed"
    USER_CONFIRMATION_REQUIRED = "user_confirmation_required"
    DENIED = "denied"


class ScoreComponents(BaseModel):
    identity: float = Field(ge=0.0, le=1.0, description="Coincidencia artista/título/versión/duración")
    technical: float = Field(ge=0.0, le=1.0, description="Calidad técnica: codec, bitrate, sample rate")
    source: float = Field(ge=0.0, le=1.0, description="Confianza y reputación del provider/fuente")
    availability: float = Field(ge=0.0, le=1.0, description="Salud de transferencia, slots, seeders")
    preference: float = Field(ge=0.0, le=1.0, description="Alineación con preferencias del DJ")
    metadata: float = Field(ge=0.0, le=1.0, description="Completitud de metadatos (ISRC, MBID, año)")
    risk: float = Field(ge=0.0, le=1.0, description="Bajo riesgo: ausencia de anomalías o transcodes sospechosos")


class Candidate(BaseModel):
    candidate_id: str = Field(default_factory=lambda: f"cand_{uuid.uuid4().hex[:8]}")
    search_id: str
    provider: str
    native_ref: str = Field(description="Referencia interna del provider para iniciar descarga o resolución")
    artist: str
    title: str
    version: str | None = None
    duration_ms: int | None = None
    format: str | None = None
    bitrate_kbps: int | None = None
    file_size_bytes: int | None = None

    score: float = Field(ge=0.0, le=100.0)
    score_components: ScoreComponents
    score_reasons: list[str] = Field(default_factory=list)

    availability: str = Field(default="available")
    authorization_state: AuthorizationState = AuthorizationState.USER_CONFIRMATION_REQUIRED
