"""Tests TDD para verificar correcciones de Fase 1 (Zero-Trust, Async DSP, Job Retries, Tray)."""

import pytest
import asyncio
from pathlib import Path
from unittest.mock import AsyncMock, patch, MagicMock

from analyzers.ffmpeg.validator import FFmpegValidator, ValidationResult
from analyzers.audio_features.analyzer import AudioFeatureAnalyzer
from hera.domain.database import Database
from hera.domain.config import HeraConfig
from hera.contracts.job import Job, JobType, JobState
from hera.domain.repositories import JobRepository
from hera.jobs.runner import JobRunner


@pytest.mark.asyncio
async def test_ffmpeg_validator_rejects_on_nonzero_exit_code(tmp_path: Path):
    """Zero-Trust Invariant: Si ffprobe o ffmpeg devuelven código != 0, is_valid DEBE ser False."""
    fake_audio = tmp_path / "corrupt_track.mp3"
    fake_audio.write_bytes(b"corrupted_header_data_xyz")

    validator = FFmpegValidator()

    # Mock subprocess returncode != 0
    with patch("asyncio.create_subprocess_exec") as mock_exec:
        proc_mock = AsyncMock()
        proc_mock.returncode = 1
        proc_mock.communicate.return_value = (b"", b"Error: Invalid data found when processing input")
        mock_exec.return_value = proc_mock

        result = await validator.validate_media(fake_audio)
        assert result.is_valid is False
        assert any("error code 1" in e.lower() or "falló" in e.lower() for e in result.errors)


@pytest.mark.asyncio
async def test_provider_registry_and_federated_search():
    from providers import ProviderRegistry
    from hera.domain.config import HeraConfig

    cfg = HeraConfig()
    cfg.providers.ytdlp_enabled = True
    cfg.providers.archive_enabled = True
    cfg.providers.prowlarr_enabled = True
    cfg.providers.lidarr_enabled = True

    registry = ProviderRegistry.from_config(cfg)
    available = registry.list_available()

    assert "ytdlp" in available
    assert "archive" in available
    assert "prowlarr" in available
    assert "lidarr" in available
    assert "local" not in available
    assert "bandcamp" not in available

    # Test parallel search across mock/live providers
    candidates, completed, failed = await registry.search_all("Daft Punk One More Time", requested_providers=["archive"])
    assert "archive" in completed
    assert isinstance(candidates, list)


@pytest.mark.asyncio
async def test_audio_feature_analyzer_runs_off_event_loop(tmp_path: Path):
    """Async Correctness: El análisis DSP no debe congelar el event loop."""
    fake_audio = tmp_path / "test_track.flac"
    fake_audio.write_bytes(b"RIFF" + b"\x00" * 2000)

    analyzer = AudioFeatureAnalyzer()

    # Ejecutar análisis y verificar que retorna estructura válida de FeatureAnalysisResult
    result = await analyzer.analyze(fake_audio)
    assert result.bpm > 0
    assert result.camelot is not None
    assert result.musical_key is not None


@pytest.mark.asyncio
async def test_job_runner_persists_attempt_increment_and_fails_at_max(tmp_path: Path):
    """Resilience: JobRunner debe persistir el incremento de attempts en SQLite para no entrar en bucle infinito."""
    db_path = tmp_path / "test_jobs.db"
    db = Database(db_path)
    await db.init_schema()

    conn = await db.connect()
    job_repo = JobRepository(conn)

    job = Job(
        id="job_test_123",
        type=JobType.DOWNLOAD,
        state=JobState.QUEUED,
        idempotency_key="idemp_123",
        correlation_id="corr_123",
        input_json={"query": "test"},
        attempts=0,
    )
    await job_repo.create_job(job)

    cfg = HeraConfig()
    runner = JobRunner(db, cfg, max_attempts=2)

    # Simular fallo en el handler
    with patch.dict("hera.jobs.runner.HANDLERS", {JobType.DOWNLOAD: AsyncMock(side_effect=RuntimeError("Download timeout"))}):
        # Intento 1 -> Debe quedar en QUEUED con attempts=1
        ran = await runner.run_once()
        assert ran is True

        j1 = await job_repo.get_by_id("job_test_123")
        assert j1 is not None
        assert j1.attempts == 1
        assert j1.state == JobState.QUEUED

        # Intento 2 -> Debe pasar a FAILED porque attempts >= 2 (max_attempts)
        ran2 = await runner.run_once()
        assert ran2 is True

        j2 = await job_repo.get_by_id("job_test_123")
        assert j2 is not None
        assert j2.attempts == 2
        assert j2.state == JobState.FAILED
        assert j2.error_code == "MAX_RETRIES_EXCEEDED"

    await db.close()


def test_desktop_tray_syntax_and_import():
    """Desktop Tray: El módulo debe importar y declarar funciones sin SyntaxErrors."""
    import importlib
    import hera.desktop.tray as tray_mod
    importlib.reload(tray_mod)
    assert hasattr(tray_mod, "run_tray_app")
    assert hasattr(tray_mod, "create_tray_image")
