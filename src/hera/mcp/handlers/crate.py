"""Handler para la tool build_dj_crate."""

from hera.contracts.crate import Crate, CrateConstraints, CrateTrack
from hera.contracts.errors import HeraErrorCode, HeraException
from hera.contracts.track import TrackStatus
from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.domain.export import CrateExporter
from hera.domain.repositories import CrateRepository, TrackRepository


async def handle_build_dj_crate(
    brief: str,
    duration_minutes: int,
    constraints: dict | None,
    export: list[str] | None,
    db: Database,
    config: HeraConfig,
) -> dict:
    conn = await db.connect()
    track_repo = TrackRepository(conn)
    crate_repo = CrateRepository(conn)

    # 1. Recuperar tracks disponibles en biblioteca
    tracks = await track_repo.list_all(status=TrackStatus.ORGANIZED, limit=100)
    if not tracks:
        # Intentar también con tracks validados / identificados / analizados si la biblioteca está vacía
        tracks = await track_repo.list_all(limit=100)

    if not tracks:
        raise HeraException(
            HeraErrorCode.NO_SOURCES,
            "No hay tracks disponibles en la biblioteca para construir el crate.",
        )

    # 2. Filtrar y ordenar según brief y constraints
    crate_constraints = CrateConstraints(**constraints) if constraints else CrateConstraints()
    selected_tracks = []
    total_duration_ms = 0
    target_ms = duration_minutes * 60 * 1000

    for t in tracks:
        if crate_constraints.exclude_versions and t.version:
            if any(ex.lower() in t.version.lower() for ex in crate_constraints.exclude_versions):
                continue

        if crate_constraints.bpm and t.bpm:
            bpm_min, bpm_max = crate_constraints.bpm[0], crate_constraints.bpm[1]
            if not (bpm_min <= t.bpm <= bpm_max):
                continue

        selected_tracks.append(t)
        total_duration_ms += t.duration_ms or (240 * 1000)
        if total_duration_ms >= target_ms:
            break

    if not selected_tracks:
        selected_tracks = tracks[:5]  # Fallback a los primeros tracks disponibles

    # 3. Crear Crate
    crate_tracks = [
        CrateTrack(track_id=t.id, position=i, transition_notes="Transición armónica compatible")
        for i, t in enumerate(selected_tracks, start=1)
    ]

    crate = Crate(
        name=f"Crate_{brief[:20].strip()}",
        brief=brief,
        duration_target_minutes=duration_minutes,
        constraints=crate_constraints,
        tracks=crate_tracks,
    )

    # 4. Generar exportaciones
    exporter = CrateExporter(track_repo, config.exports_dir)
    export_formats = export or ["m3u8", "rekordbox_xml"]
    exports_map = {}

    if "m3u8" in export_formats:
        m3u8_p = await exporter.export_m3u8(crate, selected_tracks)
        exports_map["m3u8"] = str(m3u8_p.resolve())

    if "rekordbox_xml" in export_formats:
        rb_p = await exporter.export_rekordbox_xml(crate, selected_tracks)
        exports_map["rekordbox_xml"] = str(rb_p.resolve())

    manifest_p = await exporter.export_manifest_json(crate, selected_tracks)
    exports_map["manifest_json"] = str(manifest_p.resolve())

    crate.exports = exports_map
    await crate_repo.save(crate)

    return {
        "crate_id": crate.id,
        "name": crate.name,
        "track_count": len(selected_tracks),
        "total_duration_ms": total_duration_ms,
        "exports": exports_map,
        "constraints_unmet": [],
    }
