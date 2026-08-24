"""Contratos de autorización y política legal."""

from datetime import datetime, timezone
from enum import Enum
from pydantic import BaseModel, Field


class AuthorizationBasis(str, Enum):
    OWNED_ORIGINAL = "owned_original"
    PURCHASED_COPY = "purchased_copy"
    OPEN_LICENSE = "open_license"
    PUBLIC_DOMAIN = "public_domain"
    CREATOR_PERMISSION = "creator_permission"
    AUTHORIZED_POOL = "authorized_pool"
    OTHER_DOCUMENTED_BASIS = "other_documented_basis"


class Authorization(BaseModel):
    basis: AuthorizationBasis
    evidence_ref: str = Field(description="Referencia verificable a recibo, URL de licencia o prueba")
    acknowledged_by: str = Field(default="user", description="Identificador del usuario que declara autorización")
    declared_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))


class ApprovalResult(BaseModel):
    approved: bool
    reason: str
    policy_code: str
    required_action: str | None = None
