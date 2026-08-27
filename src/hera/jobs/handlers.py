"""Manejadores de tareas en background para el JobRunner."""

from pathlib import Path
import asyncio
from hera.contracts.job import Job, JobType
from hera.contracts.track import Track, TrackStatus
from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.domain.repositories import CandidateRepository, JobRepository, TrackRepository, AuditRepository
from analyzers.ffmpeg.validator import FFmpegValidator
from analyzers.chromaprint.fingerprinter import ChromaprintFingerprinter
from analyzers.audio_features.analyzer import AudioFeatureAnalyzer
from providers.local.scanner import LocalProvider


async def handle_download_job(job: Job, db: Database, config: HeraConfig) -> dict:
    """Ejecuta la adquisición del candidato y lo coloca en cuarentena, seguido de validación automática."""
    conn = await db.connect()
    cand_repo = CandidateRepository(conn)
    track_repo = TrackRepository(conn)
    audit_repo = AuditRepository(conn)

    cand_id = job.input_json.get("candidate_id")
    candidate = await cand_repo.get_by_id(cand_id)
    if not candidate:
        raise ValueError(f"Candidato {cand_id} no encontrado en base de datos.")

    quarantine_dir = Path(config.quarantine_dir).resolve()
    quarantine_dir.mkdir(parents=True, exist_ok=True)

    ext = candidate.format.lower() if candidate.format else "flac"
    target_filename = f"{candidate.candidate_id}.{ext}"
    target_path = quarantine_dir / target_filename

    # Transferencia federada según provider
    from providers import ProviderRegistry
    registry = ProviderRegistry.from_config(config)
    provider = registry.get(candidate.provider)
    if provider:
        await provider.start_transfer(candidate, str(target_path))
    else:
        # Fallback local o genérico
        from providers.local.scanner import LocalProvider
        p = LocalProvider(config.providers.local_folders)
        await p.start_transfer(candidate, str(target_path))

    # Crear registro TRACK en estado quarantined
    track = Track(
        status=TrackStatus.QUARANTINED,
        canonical_title=candidate.title,
        canonical_artist=candidate.artist,
        version=candidate.version,
        duration_ms=candidate.duration_ms,
        format=candidate.format,
        bitrate_kbps=candidate.bitrate_kbps,
        file_size_bytes=target_path.stat().st_size if target_path.exists() else candidate.file_size_bytes,
        quarantine_path=str(target_path.resolve()),
        license_basis=job.input_json.get("authorization", {}).get("basis"),
        authorization_evidence_ref=job.input_json.get("authorization", {}).get("evidence_ref"),
        provenance_json={
            "provider": candidate.provider,
            "candidate_id": candidate.candidate_id,
            "job_id": job.id,
            "native_ref": candidate.native_ref,
        },
    )
    await track_repo.save(track)

    # Validación técnica automática con FFmpeg/ffprobe
    validator = FFmpegValidator(config.analysis.ffmpeg_path, config.analysis.ffprobe_path)
    val_res = await validator.validate_media(target_path)

    if val_res.is_valid:
        track.status = TrackStatus.VALIDATED
        track.audio_hash_sha256 = val_res.sha256
        track.codec = val_res.codec or track.codec
        track.sample_rate_hz = val_res.sample_rate_hz
        track.bit_depth = val_res.bit_depth
        track.channels = val_res.channels
        if val_res.duration_ms:
            track.duration_ms = val_res.duration_ms
        await track_repo.save(track)
    else:
        track.status = TrackStatus.REJECTED
        await track_repo.save(track)

    await audit_repo.record_event(
        event_type="AssetQuarantinedAndValidated",
        actor="job_runner",
        entity_id=track.id,
        details={"path": str(target_path), "is_valid": val_res.is_valid},
    )

    return {
        "track_id": track.id,
        "status": track.status.value,
        "quarantine_path": str(target_path),
        "sha256": track.audio_hash_sha256,
        "is_valid": val_res.is_valid,
    }


async def handle_analyze_job(job: Job, db: Database, config: HeraConfig) -> dict:
    """Ejecuta análisis acústico en background."""
    conn = await db.connect()
    track_repo = TrackRepository(conn)

    track_id = job.input_json.get("track_id")
    track = await track_repo.get_by_id(track_id)
    if not track:
        raise ValueError(f"Track {track_id} no encontrado")

    file_path = track.library_path or track.quarantine_path
    if not file_path or not Path(file_path).exists():
        raise ValueError(f"Archivo de audio no disponible para track {track_id}")

    analyzer = AudioFeatureAnalyzer()
    features = await analyzer.analyze(file_path)

    track.bpm = features.bpm
    track.bpm_confidence = features.bpm_confidence
    track.musical_key = features.musical_key
    track.key_confidence = features.key_confidence
    track.camelot = features.camelot
    track.energy = features.energy
    track.danceability = features.danceability
    track.loudness_lufs = features.loudness_lufs
    track.analysis_version = features.analysis_version
    track.embedding_ref = features.embedding_ref
    track.status = TrackStatus.ANALYZED

    await track_repo.save(track)
    return features.model_dump()


HANDLERS = {
    JobType.DOWNLOAD: handle_download_job,
    JobType.ANALYZE: handle_analyze_job,
}
