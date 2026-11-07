"""Handler para la tool audit_repertoire y sincronización in-place de biblioteca."""

from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.domain.repertoire_intelligence import RepertoireAuditor, sync_local_library_to_db


async def handle_audit_repertoire(
    check_queries: list[str] | None,
    sync_db: bool,
    db: Database,
    config: HeraConfig,
) -> dict:
    """Ejecuta la auditoría completa del repertorio (Audio, Video-DJ, Producciones Propias),
    valida duplicados pre-descarga (`[SKIP - YA EXISTE]` vs `[MISSING]`) y sincroniza `hera.db`."""
    auditor = RepertoireAuditor(config)
    report = auditor.run_full_audit(check_queries=check_queries)

    if sync_db:
        sync_stats = await sync_local_library_to_db(db, config)
        report["database_sync"] = sync_stats

    return report
