"""Handler para la tool analyze_track y analyze_batch."""

from pathlib import Path
from analyzers.audio_features.analyzer import AudioFeatureAnalyzer
from hera.contracts.errors import HeraErrorCode, HeraException
from hera.contracts.track import TrackStatus
from hera.domain.database import Database
from hera.domain.repositories import CandidateRepository, TrackRepository


async def handle_analyze_track(track_id: str, profile: str = "dj-standard", db: Database | None = None) -> dict:
    if not db:
        raise HeraException(HeraErrorCode.INVALID_MEDIA, "Base de datos no inicializada")

    conn = await db.connect()
    track_repo = TrackRepository(conn)
    cand_repo = CandidateRepository(conn)
    track = await track_repo.resolve_or_promote(track_id, cand_repo=cand_repo)

    if not track:
        raise HeraException(HeraErrorCode.INVALID_MEDIA, f"Track {track_id} no encontrado")

    file_path = track.library_path or track.quarantine_path
    if not file_path or not Path(file_path).exists():
        raise HeraException(HeraErrorCode.INVALID_MEDIA, f"Archivo de audio no disponible para track {track_id}")

    analyzer = AudioFeatureAnalyzer()
    res = await analyzer.analyze(file_path, profile=profile)

    track.bpm = res.bpm
    track.bpm_confidence = res.bpm_confidence
    track.musical_key = res.musical_key
    track.key_confidence = res.key_confidence
    track.camelot = res.camelot
    track.energy = res.energy
    track.danceability = res.danceability
    track.loudness_lufs = res.loudness_lufs
    track.analysis_version = res.analysis_version
    track.embedding_ref = res.embedding_ref
    prov = dict(track.provenance_json) if isinstance(track.provenance_json, dict) else {}
    prov["hera_dsp_telemetry"] = {
        "alt_bpm": res.alt_bpm,
        "true_peak_dbtp": res.true_peak_dbtp,
        "transients": res.transients,
        "style": res.style,
        "structure": res.structure,
        "forensics": res.forensics,
    }
    track.provenance_json = prov
    if track.status != TrackStatus.ORGANIZED:
        track.status = TrackStatus.ANALYZED

    await track_repo.save(track)
    out = res.model_dump()
    out["track_id"] = track.id
    return out


async def handle_analyze_batch(
    paths_or_ids: list[str],
    profile: str = "dj-standard",
    max_workers: int = 6,
    db: Database | None = None,
) -> dict:
    """Analiza en paralelo un lote de tracks, candidatos o rutas físicas (Audio y Video-DJ) sin bloquear MCP."""
    if not db:
        raise HeraException(HeraErrorCode.INVALID_MEDIA, "Base de datos no inicializada")

    conn = await db.connect()
    track_repo = TrackRepository(conn)
    cand_repo = CandidateRepository(conn)

    resolved_tracks = []
    file_paths: list[Path] = []
    skipped: list[str] = []

    for item in paths_or_ids:
        t = await track_repo.resolve_or_promote(item, cand_repo=cand_repo)
        if not t:
            skipped.append(item)
            continue
        fp = t.library_path or t.quarantine_path
        if not fp or not Path(fp).exists():
            skipped.append(item)
            continue
        resolved_tracks.append(t)
        file_paths.append(Path(fp))

    analyzer = AudioFeatureAnalyzer()
    batch_results = await analyzer.analyze_batch(file_paths, profile=profile, max_workers=max_workers)

    analyzed_items = []
    for t, (_, res) in zip(resolved_tracks, batch_results):
        t.bpm = res.bpm
        t.bpm_confidence = res.bpm_confidence
        t.musical_key = res.musical_key
        t.key_confidence = res.key_confidence
        t.camelot = res.camelot
        t.energy = res.energy
        t.danceability = res.danceability
        t.loudness_lufs = res.loudness_lufs
        t.analysis_version = res.analysis_version
        t.embedding_ref = res.embedding_ref
        prov = dict(t.provenance_json) if isinstance(t.provenance_json, dict) else {}
        prov["hera_dsp_telemetry"] = {
            "alt_bpm": res.alt_bpm,
            "true_peak_dbtp": res.true_peak_dbtp,
            "transients": res.transients,
            "style": res.style,
            "structure": res.structure,
            "forensics": res.forensics,
        }
        t.provenance_json = prov
        if t.status != TrackStatus.ORGANIZED:
            t.status = TrackStatus.ANALYZED
        await track_repo.save(t)
        d = res.model_dump()
        d["track_id"] = t.id
        d["path"] = t.library_path or t.quarantine_path
        analyzed_items.append(d)

    return {
        "analyzed_count": len(analyzed_items),
        "skipped_count": len(skipped),
        "results": analyzed_items,
        "skipped": skipped,
    }
