"""Handler para la tool audit_repertoire y sincronización in-place de biblioteca."""

from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.domain.repertoire_intelligence import (
    RepertoireAuditor,
    persist_dj_style_dna_to_db,
    sync_local_library_to_db,
)


async def handle_audit_repertoire(
    check_queries: list[str] | None,
    sync_db: bool,
    db: Database,
    config: HeraConfig,
) -> dict:
    """Ejecuta la auditoría completa del repertorio (Audio, Video-DJ, Producciones Propias),
    valida duplicados pre-descarga (`[SKIP - YA EXISTE]` vs `[MISSING]`), sincroniza `hera.db`
    y persiste automáticamente el ADN Musical (`dj_style_dna`) en `dj_preferences` (`DJ Brain`)."""
    auditor = RepertoireAuditor(config)
    report = auditor.run_full_audit(check_queries=check_queries)

    if sync_db:
        sync_stats = await sync_local_library_to_db(db, config)
        report["database_sync"] = sync_stats
        dna_sync = await persist_dj_style_dna_to_db(db, report)
        report["dj_brain_sync"] = dna_sync

    return report
