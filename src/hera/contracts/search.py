"""Contratos de búsqueda federada."""

from pydantic import BaseModel, Field


class SearchFilters(BaseModel):
    format: list[str] | None = None
    version: list[str] | None = None
    min_bitrate_kbps: int | None = None
    min_duration_ms: int | None = None
    max_duration_ms: int | None = None


class SearchRequest(BaseModel):
    query: str
    filters: SearchFilters | None = None
    providers: list[str] | None = None


class SearchResult(BaseModel):
    search_id: str
    providers_completed: list[str] = Field(default_factory=list)
    providers_failed: list[str] = Field(default_factory=list)
    candidate_count: int = 0
