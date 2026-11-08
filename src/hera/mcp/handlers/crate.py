"""Handler para la tool build_dj_crate con secuenciación armónica Camelot, soporte Video-DJ y exportación Rekordbox/Ableton."""

from pathlib import Path
from analyzers.audio_features.synergy_engine import UnifiedSynergyEngine
from hera.application.use_cases.build_crate import BuildHarmonicCrateUseCase
from hera.contracts.crate import Crate, CrateConstraints, CrateTrack
from hera.contracts.errors import HeraErrorCode, HeraException
from hera.contracts.track import Track, TrackStatus
from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.domain.export import CrateExporter
from hera.domain.repositories import CrateRepository, TrackRepository
from providers.local.scanner import VIDEO_EXTENSIONS, strip_accents


def _is_video_track(t: Track) -> bool:
    p = (t.library_path or t.quarantine_path or "").lower()
    ext = p.rsplit(".", 1)[-1] if "." in p else (t.codec or "").lower()
    return ext in VIDEO_EXTENSIONS


def _is_own_production(t: Track) -> bool:
    p = (t.library_path or t.quarantine_path or "").lower()
    art = (t.canonical_artist or "").lower()
    return "_mis_producciones" in p or "david castillo" in art or "david_castillo" in p


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
    tracks = await track_repo.list_all(status=TrackStatus.ORGANIZED, limit=15000)
    if not tracks:
        tracks = await track_repo.list_all(limit=15000)

    if not tracks:
        raise HeraException(
            HeraErrorCode.NO_SOURCES,
            "No hay tracks disponibles en la biblioteca para construir el crate.",
        )

    # 2. Filtrar según brief, media_type (audio/video/all), género y constraints
    crate_constraints = CrateConstraints(**constraints) if constraints else CrateConstraints()
    media_type = (crate_constraints.media_type or "all").lower()
    if media_type == "all":
        brief_l = brief.lower()
        if "video" in brief_l:
            media_type = "video"

    genre_kw = strip_accents((crate_constraints.genre or "").lower()).strip()

    filtered_pool: list[Track] = []
    own_productions_pool: list[Track] = []

    for t in tracks:
        if crate_constraints.exclude_versions and t.version:
            if any(ex.lower() in t.version.lower() for ex in crate_constraints.exclude_versions):
                continue

        if media_type == "video" and not _is_video_track(t):
            continue
        elif media_type == "audio" and _is_video_track(t):
            continue

        if crate_constraints.bpm and t.bpm:
            bpm_min, bpm_max = crate_constraints.bpm[0], crate_constraints.bpm[1]
            if not (bpm_min <= t.bpm <= bpm_max):
                continue

        if _is_own_production(t):
            own_productions_pool.append(t)

        if genre_kw:
            haystack = strip_accents(
                f"{t.library_path or ''} {t.canonical_artist or ''} {t.canonical_title or ''}".lower()
            )
            if not all(tok in haystack for tok in genre_kw.split()):
                continue

        filtered_pool.append(t)

    if not filtered_pool:
        # Fallback relajando filtro de género si era demasiado restrictivo
        for t in tracks:
            if crate_constraints.bpm and t.bpm:
                bpm_min, bpm_max = crate_constraints.bpm[0], crate_constraints.bpm[1]
                if not (bpm_min <= t.bpm <= bpm_max):
                    continue
            filtered_pool.append(t)

    if not filtered_pool:
        filtered_pool = tracks[:20]

    # 3. Seleccionar pool inicial hasta cubrir duration_minutes (con margen para secuenciación armónica)
    target_ms = duration_minutes * 60 * 1000
    candidate_pool: list[Track] = []
    total_duration_ms = 0

    # Priorizar target_camelot si se especificó
    if crate_constraints.target_camelot:
        target_cam = crate_constraints.target_camelot.strip().upper()
        filtered_pool.sort(
            key=lambda tr: UnifiedSynergyEngine.calculate_camelot_distance(target_cam, tr.camelot or "8A"),
            reverse=True,
        )

    # Incluir producciones propias si se solicitó
    want_own = crate_constraints.include_own_productions or any(
        kw in brief.lower() for kw in ("produccion", "david castillo", "showcase", "masters")
    )
    if want_own and own_productions_pool:
        for own_t in own_productions_pool[:4]:
            if own_t not in candidate_pool:
                candidate_pool.append(own_t)
                total_duration_ms += own_t.duration_ms or (210 * 1000)

    for t in filtered_pool:
        if t in candidate_pool:
            continue
        candidate_pool.append(t)
        total_duration_ms += t.duration_ms or (240 * 1000)
        if total_duration_ms >= target_ms:
            break

    # 4. Ordenar armónicamente mediante el motor de sinergia holística (Camelot + BPM + Transientes + Estilo)
    use_case = BuildHarmonicCrateUseCase(track_repo, crate_repo)
    selected_tracks = use_case.sequence_tracks_harmonically(candidate_pool)

    # 5. Calcular notas reales de transición entre pares consecutivos
    fps = [use_case._track_to_fingerprint(t) for t in selected_tracks]
    crate_tracks: list[CrateTrack] = []
    for i, (t, fp) in enumerate(zip(selected_tracks, fps), start=1):
        if i < len(fps):
            next_fp = fps[i]
            syn = UnifiedSynergyEngine.evaluate_track_pair(fp, next_fp)
            note = f"[{syn.transition_grade} | {syn.master_synergy_score:.2f}] {syn.executive_summary}"
        else:
            note = f"Cierre del Set en {fp.camelot} @ {fp.bpm:g} BPM"
        crate_tracks.append(
            CrateTrack(track_id=t.id, position=i, transition_notes=note)
        )

    crate = Crate(
        name=f"Crate_{brief[:32].strip()}",
        brief=brief,
        duration_target_minutes=duration_minutes,
        constraints=crate_constraints,
        scoring_version="synergy-holistic/2.0",
        tracks=crate_tracks,
    )

    # 6. Generar exportaciones (M3U8, Rekordbox XML, VirtualDJ, Traktor NML, Cue Sheet, Ableton Live, JSON Manifest)
    exporter = CrateExporter(track_repo, config.exports_dir)
    export_formats = [
        fmt.lower()
        for fmt in (export or ["m3u8", "rekordbox_xml", "virtualdj", "traktor_nml", "cue_sheet"])
    ]
    exports_map: dict[str, str] = {}

    if "m3u8" in export_formats or "all" in export_formats:
        m3u8_p = await exporter.export_m3u8(crate, selected_tracks)
        exports_map["m3u8"] = str(m3u8_p.resolve())

    if "rekordbox_xml" in export_formats or "rekordbox" in export_formats or "all" in export_formats:
        rb_p = await exporter.export_rekordbox_xml(crate, selected_tracks)
        exports_map["rekordbox_xml"] = str(rb_p.resolve())

    if "virtualdj" in export_formats or "vdj" in export_formats or "all" in export_formats:
        vdj_p = await exporter.export_virtualdj_folder(crate, selected_tracks)
        exports_map["virtualdj_folder"] = str(vdj_p.resolve())

    if "traktor_nml" in export_formats or "traktor" in export_formats or "all" in export_formats:
        trk_p = await exporter.export_traktor_nml(crate, selected_tracks)
        exports_map["traktor_nml"] = str(trk_p.resolve())

    if "cue_sheet" in export_formats or "all" in export_formats:
        cs_p = await exporter.export_cue_sheet(crate, selected_tracks)
        exports_map["cue_sheet"] = str(cs_p.resolve())

    if "ableton" in export_formats or "all" in export_formats:
        abl_p = await exporter.export_ableton_curated(crate, selected_tracks)
        exports_map["ableton_index"] = str(abl_p.resolve())

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
