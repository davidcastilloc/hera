"""Handler para la tool analyze_track."""

from pathlib import Path
from analyzers.audio_features.analyzer import AudioFeatureAnalyzer
from hera.contracts.errors import HeraErrorCode, HeraException
from hera.contracts.track import TrackStatus
from hera.domain.database import Database
from hera.domain.repositories import TrackRepository


async def handle_analyze_track(track_id: str, profile: str = "dj-standard", db: Database | None = None) -> dict:
    if not db:
        raise HeraException(HeraErrorCode.INVALID_MEDIA, "Base de datos no inicializada")

    conn = await db.connect()
    track_repo = TrackRepository(conn)
    track = await track_repo.get_by_id(track_id)

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
    track.status = TrackStatus.ANALYZED

    await track_repo.save(track)
    return res.model_dump()
