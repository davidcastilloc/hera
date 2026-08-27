"""Handler para la tool search_music."""

import uuid
from hera.contracts.candidate import Candidate
from hera.contracts.search import SearchFilters
from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.domain.repositories import CandidateRepository
from providers.local.scanner import LocalProvider
from providers.slskd.client import SlskdProvider


async def handle_search_music(
    query: str,
    filters: dict | None,
    providers: list[str] | None,
    db: Database,
    config: HeraConfig,
) -> dict:
    search_id = f"srch_{uuid.uuid4().hex[:6]}"
    search_filters = SearchFilters(**filters) if filters else None
    requested_providers = providers  # Si es None, ProviderRegistry usará todos los activos

    # Orquestación federada mediante ProviderRegistry
    from providers import ProviderRegistry
    registry = ProviderRegistry.from_config(config)

    all_candidates, completed, failed = await registry.search_all(
        query=query,
        filters=search_filters,
        requested_providers=requested_providers,
        timeout_seconds=12.0,
    )

    for c in all_candidates:
        c.search_id = search_id

    # Persistir candidatos encontrados
    conn = await db.connect()
    cand_repo = CandidateRepository(conn)
    if all_candidates:
        await cand_repo.save_many(all_candidates)

    return {
        "search_id": search_id,
        "providers_completed": completed,
        "providers_failed": failed,
        "candidate_count": len(all_candidates),
    }
