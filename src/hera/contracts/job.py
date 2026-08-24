"""Contratos del sistema de persistencia y cola de trabajos (JOB)."""

from datetime import datetime, timezone
from enum import Enum
import uuid
from pydantic import BaseModel, Field


class JobType(str, Enum):
    DOWNLOAD = "download"
    VALIDATE = "validate"
    IDENTIFY = "identify"
    ANALYZE = "analyze"
    ORGANIZE = "organize"
    EXPORT = "export"


class JobState(str, Enum):
    QUEUED = "queued"
    RESOLVING = "resolving"
    RUNNING = "running"
    VERIFYING = "verifying"
    QUARANTINED = "quarantined"
    COMPLETED = "completed"
    FAILED = "failed"
    STALLED = "stalled"
    CANCELLED = "cancelled"
    POLICY_DENIED = "policy_denied"


class Job(BaseModel):
    id: str = Field(default_factory=lambda: f"job_{uuid.uuid4().hex[:8]}")
    type: JobType
    state: JobState = JobState.QUEUED
    progress: float = Field(default=0.0, ge=0.0, le=1.0)
    attempts: int = Field(default=0)
    idempotency_key: str
    correlation_id: str = Field(default_factory=lambda: f"corr_{uuid.uuid4().hex[:8]}")

    input_json: dict = Field(default_factory=dict)
    result_json: dict | None = None
    error_code: str | None = None
    error_message: str | None = None

    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    updated_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class JobCreate(BaseModel):
    type: JobType
    idempotency_key: str
    input_json: dict
    correlation_id: str | None = None
