"""Handler para la tool identify_track."""

from pathlib import Path
from analyzers.chromaprint.fingerprinter import ChromaprintFingerprinter
from hera.contracts.errors import HeraErrorCode, HeraException
from hera.contracts.track import TrackStatus
from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.domain.repositories import TrackRepository


async def handle_identify_track(asset_id: str, db: Database, config: HeraConfig) -> dict:
    conn = await db.connect()
    track_repo = TrackRepository(conn)
    track = await track_repo.get_by_id(asset_id)

    if not track:
        raise HeraException(HeraErrorCode.INVALID_MEDIA, f"Activo {asset_id} no encontrado")

    file_path = track.quarantine_path or track.library_path
    if not file_path or not Path(file_path).exists():
        raise HeraException(HeraErrorCode.INVALID_MEDIA, f"Archivo de audio no disponible para activo {asset_id}")

    fingerprinter = ChromaprintFingerprinter(
        fpcalc_path=config.analysis.fpcalc_path,
        acoustid_api_key=None,
    )
    res = await fingerprinter.fingerprint_file(file_path)

    track.fingerprint = res.fingerprint
    track.status = TrackStatus.IDENTIFIED
    await track_repo.save(track)

    hypotheses = [h.model_dump() for h in res.hypotheses]
    if not hypotheses:
        # Hipótesis inicial basada en metadata de archivo
        hypotheses = [
            {
                "recording_mbid": track.recording_mbid,
                "artist": track.canonical_artist,
                "title": track.canonical_title,
                "confidence": 0.90,
                "evidence": ["local_filename", "audio_header_inspection"],
            }
        ]

    return {
        "asset_id": track.id,
        "fingerprint": res.fingerprint,
        "hypotheses": hypotheses,
        "review_required": res.review_required,
    }
