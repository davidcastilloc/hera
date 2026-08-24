"""Handler para la tool get_track_candidates."""

from hera.domain.database import Database
from hera.domain.repositories import CandidateRepository


async def handle_get_track_candidates(
    search_id: str,
    limit: int = 10,
    db: Database | None = None,
) -> list[dict]:
    if not db:
        return []
    conn = await db.connect()
    cand_repo = CandidateRepository(conn)
    candidates = await cand_repo.list_by_search_id(search_id, limit=limit)
    return [c.model_dump() for c in candidates]
