"""Pruebas integrales de las 7 mejoras arquitectónicas extraídas del caso de uso 'Hera MCP Music Analysis'."""

from pathlib import Path
import json
import xml.etree.ElementTree as ET
import numpy as np
import pytest

from analyzers.audio_features.analyzer import AudioFeatureAnalyzer
from hera.contracts.candidate import Candidate, ScoreComponents, AuthorizationState
from hera.contracts.track import Track, TrackStatus
from hera.domain.config import HeraConfig, ProvidersConfig
from hera.domain.database import Database
from hera.domain.dedup import DeduplicationEngine
from hera.domain.repositories import CandidateRepository, TrackRepository
from hera.mcp.handlers.analyze import handle_analyze_batch, handle_analyze_track
from hera.mcp.handlers.audit import handle_audit_repertoire
from hera.mcp.handlers.crate import handle_build_dj_crate
from providers.local.scanner import LocalProvider, parse_dj_filename


def test_parse_dj_filename_and_local_wildcard_video(tmp_path: Path):
    """Verifica el parser inteligente de nombres DJ/Ableton, soporte de comodín '*' y archivos de Video-DJ (.mp4)."""
    artist, title, ver, bpm, cam = parse_dj_filename(
        "01 - Calvin Harris & Clementine Douglas - Blessings (Extended Mix) [124 BPM - 10B].wav"
    )
    assert artist == "Calvin Harris & Clementine Douglas"
    assert title == "Blessings"
    assert ver == "Extended Mix"
    assert bpm == 124.0
    assert cam == "10B"

    # Formato Ableton: 102_8A_Beele - La Plena.mp3
    a2, t2, v2, b2, c2 = parse_dj_filename("102_8A_Beele - La Plena.mp3")
    assert a2 == "Beele"
    assert t2 == "La Plena"
    assert b2 == 102.0
    assert c2 == "8A"

    # Crear archivos de prueba de audio y video HD
    repertorio = tmp_path / "03_Reggaeton_y_Urbano"
    repertorio.mkdir(parents=True)
    audio_f = repertorio / "01 - Feid - Ferxxo 100 [96 BPM - 4A].mp3"
    video_f = repertorio / "y2mate.com - Karol G - Si Antes Te Hubiera Conocido [128 BPM - 11B].mp4"
    audio_f.write_bytes(b"ID3" + b"\x00" * 4096)
    video_f.write_bytes(b"\x00\x00\x00\x18ftypmp42" + b"\x00" * 8192)

    # Carpeta de crate generada que debe ser ignorada para evitar duplicados
    crate_dup = repertorio / "00_CRATES_DE_COMBATE_ARMONICOS"
    crate_dup.mkdir()
    (crate_dup / "01 - Dup.mp3").write_bytes(b"ID3" + b"\x00" * 1024)

    provider = LocalProvider(folders=[tmp_path])

    import asyncio
    all_cands = asyncio.run(provider.search("*"))
    assert len(all_cands) == 2

    video_cands = asyncio.run(provider.search("Karol", filters=None))
    assert len(video_cands) == 1
    assert video_cands[0].format == "MP4"
    assert video_cands[0].artist == "Karol G"
    assert any("128 BPM - 11B" in r for r in video_cands[0].score_reasons)


def test_genre_bpm_disambiguation_and_lufs(tmp_path: Path):
    """Verifica la corrección Half/Double-Time por género y el cálculo BS.1770 de LUFS y True Peak (dBTP)."""
    reggaeton_path = tmp_path / "03_Reggaeton_y_Urbano" / "W Sound, Beele - La Plena.mp3"
    house_path = tmp_path / "01_Afro_House_y_Afrobeats" / "Keinemusik - Move.mp3"

    # 192 BPM (octava doble) en carpeta urbana se corrige a 96 BPM, pero un track real de 130 BPM NO se deforma por 2/3
    adj_urb = AudioFeatureAnalyzer._adjust_bpm_for_genre_context(192.0, reggaeton_path)
    assert adj_urb == 96.0
    adj_crossover = AudioFeatureAnalyzer._adjust_bpm_for_genre_context(130.0, reggaeton_path)
    assert adj_crossover == 130.0

    # House a mitad de tiempo (62 BPM) se duplica a 124 BPM
    adj_house = AudioFeatureAnalyzer._adjust_bpm_for_genre_context(62.0, house_path)
    assert adj_house == 124.0

    # Prueba sintética del motor HERA-PRE v3.0: patrón Dembow/Tresillo a 96.0 BPM (con snare en 0.75 beat = 128 AltBpm)
    sr = 22050
    dur = 25.0
    y_dembow = np.zeros(int(sr * dur), dtype=np.float32)
    beat_sec = 60.0 / 96.0
    t_impulse = np.arange(int(0.04 * sr), dtype=np.float32) / sr
    kick_wave = np.sin(2 * np.pi * 65.0 * t_impulse) * np.exp(-t_impulse * 40.0)
    snare_wave = np.sin(2 * np.pi * 1800.0 * t_impulse) * np.exp(-t_impulse * 60.0)
    for beat_idx in range(int(dur / beat_sec)):
        k_pos = int(beat_idx * beat_sec * sr)
        if k_pos + len(kick_wave) < len(y_dembow):
            y_dembow[k_pos : k_pos + len(kick_wave)] += 0.8 * kick_wave
        # Tresillo snare en 0.75 beats (que genera el falso pico de 128 BPM = 4/3 * 96)
        s_pos = int((beat_idx + 0.75) * beat_sec * sr)
        if s_pos + len(snare_wave) < len(y_dembow):
            y_dembow[s_pos : s_pos + len(snare_wave)] += 0.9 * snare_wave

    detected_bpm, _ = AudioFeatureAnalyzer._compute_exact_bpm(y_dembow, sr=sr)
    assert abs(detected_bpm - 96.0) <= 0.5

    # Medición de LUFS y True Peak sobre onda senoidal conocida
    sr = 22050
    t = np.linspace(0, 2.0, sr * 2, endpoint=False, dtype=np.float32)
    sine = 0.5 * np.sin(2 * np.pi * 440.0 * t)
    lufs, dbtp = AudioFeatureAnalyzer._measure_lufs_and_true_peak(sine, sr=sr)
    assert -15.0 <= lufs <= -5.0
    assert -7.0 <= dbtp <= -5.5


@pytest.mark.asyncio
async def test_promote_candidate_and_build_rekordbox_ableton_crate(tmp_path: Path):
    """Verifica promoción in-place de `cand_*` a `trk_*`, batch analysis, y exportación a Rekordbox XML + Ableton."""
    db_path = tmp_path / "test_hera.db"
    db = Database(str(db_path))
    await db.init_schema()

    music_dir = tmp_path / "music"
    music_dir.mkdir()
    f1 = music_dir / "01 - Keinemusik - Move [122 BPM - 8A].mp3"
    f2 = music_dir / "02 - Hugel - Jamaican Bam Bam [124 BPM - 9A].mp3"
    f3 = music_dir / "03 - David Castillo - Orinoco Session [123 BPM - 8A].mp3"
    for f in (f1, f2, f3):
        f.write_bytes(b"ID3" + b"\x00" * 4096)

    conn = await db.connect()
    cand_repo = CandidateRepository(conn)
    cand = Candidate(
        candidate_id="cand_test1234",
        search_id="srch_01",
        provider="local",
        native_ref=str(f1.resolve()),
        artist="Keinemusik",
        title="Move",
        format="MP3",
        bitrate_kbps=320,
        file_size_bytes=4099,
        score=92.0,
        score_components=ScoreComponents(
            identity=1.0, technical=0.9, source=0.95, availability=1.0, preference=0.9, metadata=0.9, risk=0.95
        ),
        authorization_state=AuthorizationState.CONFIRMED,
    )
    await cand_repo.save_many([cand])

    # 1. Llamar analyze_track con cand_id directamente (sin error de 'Track no encontrado')
    res_single = await handle_analyze_track("cand_test1234", db=db)
    assert res_single["track_id"].startswith("trk_")
    assert "loudness_lufs" in res_single

    # 2. Llamar analyze_batch con rutas directas
    res_batch = await handle_analyze_batch([str(f2), str(f3)], max_workers=2, db=db)
    assert res_batch["analyzed_count"] == 2

    # 3. Construir crate con exportación a Rekordbox XML, Ableton y Cue Sheet
    cfg = HeraConfig(
        data_dir=str(tmp_path),
        exports_dir=str(tmp_path / "exports"),
        db_path=str(db_path),
        providers=ProvidersConfig(local_folders=[str(music_dir)]),
    )
    crate_res = await handle_build_dj_crate(
        brief="Afro_House_Showcase_David_Castillo",
        duration_minutes=15,
        constraints={"bpm": [120, 128], "include_own_productions": True},
        export=["m3u8", "rekordbox_xml", "cue_sheet", "ableton"],
        db=db,
        config=cfg,
    )

    assert crate_res["track_count"] == 3
    exports = crate_res["exports"]
    assert Path(exports["m3u8"]).exists()
    assert Path(exports["rekordbox_xml"]).exists()
    assert Path(exports["cue_sheet"]).exists()
    assert Path(exports["ableton_index"]).exists()

    # Verificar que Rekordbox XML contiene los 4 Hot Cues cuantizados (POSITION_MARK) por cada pista
    rb_tree = ET.parse(exports["rekordbox_xml"])
    pos_marks = rb_tree.getroot().findall(".//POSITION_MARK")
    assert len(pos_marks) == 12  # 4 Hot Cues * 3 tracks

    # Verificar el índice de Ableton Live
    abl_data = json.loads(Path(exports["ableton_index"]).read_text(encoding="utf-8"))
    assert abl_data["track_count"] == 3
    assert abl_data["tracks"][0]["ableton_filename"].endswith(".mp3")

    # 4. Ejecutar audit_repertoire con validación anti-duplicados pre-descarga
    audit_res = await handle_audit_repertoire(
        check_queries=["Keinemusik - Move", "Artista Nuevo - Cancion Inexistente 2026"],
        sync_db=True,
        db=db,
        config=cfg,
    )
    assert audit_res["summary"]["total_unique_files"] == 3
    dedup = audit_res["deduplication_audit"]
    assert dedup["skip_existing_count"] == 1
    assert dedup["missing_count"] == 1
    assert dedup["existing"][0]["status"] == "[SKIP - YA EXISTE]"

    await db.close()
