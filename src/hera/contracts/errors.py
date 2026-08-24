"""Códigos de error tipados del sistema Hera."""

from enum import Enum


class HeraErrorCode(str, Enum):
    PROVIDER_UNAVAILABLE = "PROVIDER_UNAVAILABLE"
    RATE_LIMITED = "RATE_LIMITED"
    AUTH_REQUIRED = "AUTH_REQUIRED"
    POLICY_DENIED = "POLICY_DENIED"
    NO_SOURCES = "NO_SOURCES"
    TRANSFER_STALLED = "TRANSFER_STALLED"
    INVALID_MEDIA = "INVALID_MEDIA"
    IDENTITY_AMBIGUOUS = "IDENTITY_AMBIGUOUS"
    DUPLICATE_FOUND = "DUPLICATE_FOUND"
    STORAGE_FULL = "STORAGE_FULL"
    EXPORT_FAILED = "EXPORT_FAILED"


class HeraException(Exception):
    """Excepción base para errores de Hera con código tipado."""

    def __init__(self, code: HeraErrorCode, message: str, details: dict | None = None):
        super().__init__(message)
        self.code = code
        self.message = message
        self.details = details or {}

    def to_dict(self) -> dict:
        return {
            "error_code": self.code.value,
            "message": self.message,
            "details": self.details,
        }
