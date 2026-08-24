"""Prueba de integración End-to-End del pipeline de Hera."""

from pathlib import Path
import pytest
from hera.contracts.authorization import AuthorizationBasis
from hera.contracts.track import TrackStatus
from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.domain.dedup import DeduplicationEngine
from hera.domain.repositories import CandidateRepository, TrackRepository
from hera.jobs.runner import JobRunner
from hera.mcp.handlers.analyze import handle_analyze_track
from hera.mcp.handlers.candidates import handle_get_track_candidates
from hera.mcp.handlers.crate import handle_build_dj_crate
from hera.mcp.handlers.download import handle_download_track
from hera.mcp.handlers.identify import handle_identify_track
from hera.mcp.handlers.organize import handle_organize_track
from hera.mcp.handlers.search import handle_search_music


@pytest.mark.asyncio
async def test_full_pipeline_e2e(tmp_path: Path):
    # Setup de directorios
    inbox_dir = tmp_path / "inbox"
    quarantine_dir = tmp_path / "quarantine"
    library_dir = tmp_path / "library"
    exports_dir = tmp_path / "exports"
    db_file = tmp_path / "hera.db"

    for d in [inbox_dir, quarantine_dir, library_dir, exports_dir]:
        d.mkdir(parents=True, exist_ok=True)

    # Crear archivo de audio dummy en el inbox
    dummy_audio = inbox_dir / "Black Coffee - Drive (Extended Mix).flac"
    dummy_audio.write_bytes(b"RIFF" + b"\x00" * 1024 + b"WAVEfmt " + b"\x00" * 4096)

    # Configuración de prueba
    config = HeraConfig(
        data_dir=str(tmp_path),
        quarantine_dir=str(quarantine_dir),
        library_dir=str(library_dir),
        exports_dir=str(exports_dir),
        db_path=str(db_file),
    )
    config.providers.local_folders = [str(inbox_dir)]
    config.policy.require_approval = True

    db = Database(db_file)
    await db.init_schema()

    # 1. search_music
    search_res = await handle_search_music("Drive", None, ["local"], db, config)
    assert search_res["candidate_count"] >= 1
    search_id = search_res["search_id"]

    # 2. get_track_candidates
    candidates = await handle_get_track_candidates(search_id, limit=5, db=db)
    assert len(candidates) >= 1
    cand = candidates[0]
    assert "Drive" in cand["title"]
    cand_id = cand["candidate_id"]

    # 3. download_track con autorización
    auth_payload = {
        "basis": AuthorizationBasis.PURCHASED_COPY.value,
        "evidence_ref": "receipt:test:999",
        "acknowledged_by": "tester",
    }
    dl_res = await handle_download_track(
        candidate_id=cand_id,
        authorization=auth_payload,
        approval_token="appr_valid_token",
        idempotency_key="idemp_e2e_001",
        db=db,
        config=config,
    )
    assert "job_id" in dl_res
    job_id = dl_res["job_id"]

    # 4. JobRunner procesa la adquisición hacia cuarentena y auto-validación
    runner = JobRunner(db, config)
    worked = await runner.run_once()
    assert worked is True

    conn = await db.connect()
    track_repo = TrackRepository(conn)
    tracks = await track_repo.list_all(limit=10)
    assert len(tracks) >= 1
    track = tracks[0]
    assert track.status in {TrackStatus.VALIDATED, TrackStatus.QUARANTINED}

    # 5. identify_track
    ident_res = await handle_identify_track(track.id, db, config)
    assert "hypotheses" in ident_res
    assert len(ident_res["hypotheses"]) >= 1

    # 6. analyze_track
    analysis_res = await handle_analyze_track(track.id, "dj-standard", db=db)
    assert "bpm" in analysis_res
    assert "camelot" in analysis_res

    # 7. deduplicación
    dedup = DeduplicationEngine(track_repo)
    dedup_res = await dedup.check_duplicates(track)
    assert dedup_res.is_duplicate is False

    # 8. organize_track
    org_res = await handle_organize_track(track.id, None, "review", db, config)
    assert org_res["status"] == "organized"
    assert Path(org_res["destination_path"]).exists()

    # 9. build_dj_crate
    crate_res = await handle_build_dj_crate(
        brief="Afro House Warmup",
        duration_minutes=30,
        constraints={"bpm": [118, 126]},
        export=["m3u8", "rekordbox_xml"],
        db=db,
        config=config,
    )
    assert crate_res["track_count"] >= 1
    assert "m3u8" in crate_res["exports"]
    assert Path(crate_res["exports"]["m3u8"]).exists()
    assert Path(crate_res["exports"]["rekordbox_xml"]).exists()

    await db.close()
