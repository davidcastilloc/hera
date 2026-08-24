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
    requested_providers = providers or ["local"]

    completed = []
    failed = []
    all_candidates: list[Candidate] = []

    # 1. Provider Local
    if "local" in requested_providers:
        try:
            local_p = LocalProvider(config.providers.local_folders)
            cands = await local_p.search(query, search_filters)
            for c in cands:
                c.search_id = search_id
            all_candidates.extend(cands)
            completed.append("local")
        except Exception:
            failed.append("local")

    # 2. Provider slskd
    if "slskd" in requested_providers:
        try:
            slskd_p = SlskdProvider(config.providers.slskd_url)
            cands = await slskd_p.search(query, search_filters)
            for c in cands:
                c.search_id = search_id
            all_candidates.extend(cands)
            completed.append("slskd")
        except Exception:
            failed.append("slskd")

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
