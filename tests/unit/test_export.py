"""Pruebas unitarias de exportación (M3U8 y Rekordbox XML)."""

from pathlib import Path
import xml.etree.ElementTree as ET
import pytest
from hera.contracts.crate import Crate, CrateTrack
from hera.contracts.track import Track
from hera.domain.database import Database
from hera.domain.export import CrateExporter
from hera.domain.repositories import TrackRepository


@pytest.mark.asyncio
async def test_crate_export_m3u8_and_rekordbox(tmp_path: Path):
    db_file = tmp_path / "test.db"
    db = Database(db_file)
    await db.init_schema()
    conn = await db.connect()
    track_repo = TrackRepository(conn)

    # Crear archivo dummy
    audio_file = tmp_path / "track1.flac"
    audio_file.write_bytes(b"DUMMY_AUDIO_DATA_FOR_TESTING")

    track = Track(
        canonical_title="Drive",
        canonical_artist="Black Coffee",
        version="Extended Mix",
        duration_ms=412000,
        bpm=122.1,
        camelot="8A",
        bitrate_kbps=1411,
        library_path=str(audio_file),
    )
    await track_repo.save(track)

    crate = Crate(
        name="Afro House Set",
        brief="Deep Afro House",
        duration_target_minutes=60,
        tracks=[CrateTrack(track_id=track.id, position=1)],
    )

    exports_dir = tmp_path / "exports"
    exporter = CrateExporter(track_repo, exports_dir)

    # 1. M3U8
    m3u8_p = await exporter.export_m3u8(crate, [track])
    assert m3u8_p.exists()
    content = m3u8_p.read_text(encoding="utf-8")
    assert "#EXTM3U" in content
    assert "Black Coffee - Drive (Extended Mix)" in content

    # 2. Rekordbox XML
    rb_p = await exporter.export_rekordbox_xml(crate, [track])
    assert rb_p.exists()
    tree = ET.parse(rb_p)
    root = tree.getroot()
    assert root.tag == "DJ_PLAYLISTS"
    collection = root.find("COLLECTION")
    assert collection is not None
    track_node = collection.find("TRACK")
    assert track_node is not None
    assert track_node.get("Name") == "Drive [Extended Mix]"
    assert track_node.get("AverageBpm") == "122.10"
    assert track_node.get("Tonality") == "8A"

    # 3. VirtualDJ (.vdjfolder + _vdj_database.xml con 4 Hot Cues <Poi>)
    vdj_p = await exporter.export_virtualdj_folder(crate, [track])
    assert vdj_p.exists()
    vdj_tree = ET.parse(vdj_p)
    vdj_root = vdj_tree.getroot()
    assert vdj_root.tag == "VirtualFolder"
    song_el = vdj_root.find("Song")
    assert song_el is not None
    assert song_el.get("Artist") == "Black Coffee"
    assert song_el.get("Key") == "8A"

    vdj_db_p = exports_dir / "Afro House Set_vdj_database.xml"
    assert vdj_db_p.exists()
    vdj_db_root = ET.parse(vdj_db_p).getroot()
    assert vdj_db_root.tag == "VirtualDJ_Database"
    db_song = vdj_db_root.find("Song")
    assert db_song is not None
    cue_pois = [p for p in db_song.findall("Poi") if p.get("Type") == "cue"]
    assert len(cue_pois) == 4
    assert [p.get("Num") for p in cue_pois] == ["1", "2", "3", "4"]

    # 4. Native Instruments Traktor Pro (_traktor.nml con 4 Hot Cues <CUE_V2>)
    traktor_p = await exporter.export_traktor_nml(crate, [track])
    assert traktor_p.exists()
    nml_root = ET.parse(traktor_p).getroot()
    assert nml_root.tag == "NML"
    assert nml_root.get("VERSION") == "19"
    nml_entry = nml_root.find("./COLLECTION/ENTRY")
    assert nml_entry is not None
    assert nml_entry.get("ARTIST") == "Black Coffee"
    cues_v2 = nml_entry.findall("CUE_V2")
    assert len(cues_v2) == 4
    assert [c.get("HOTCUE") for c in cues_v2] == ["0", "1", "2", "3"]

    await db.close()


@pytest.mark.asyncio
async def test_dj_style_dna_and_brain_persistence(tmp_path: Path):
    from hera.domain.config import HeraConfig, ProvidersConfig
    from hera.domain.repertoire_intelligence import (
        RepertoireAuditor,
        load_dj_brain_context,
        persist_dj_style_dna_to_db,
        sync_local_library_to_db,
    )

    music_dir = tmp_path / "DJ_Library"
    music_dir.mkdir(parents=True, exist_ok=True)
    own_dir = tmp_path / "_MIS_PRODUCCIONES"
    own_dir.mkdir(parents=True, exist_ok=True)

    (music_dir / "124_8A_Keinemusik - Move.flac").write_bytes(b"X" * 3_000_000)
    (music_dir / "125_9A_Keinemusik - Les Gout.flac").write_bytes(b"X" * 3_000_000)
    (own_dir / "126_8A_David Castillo - Caracas Dub.flac").write_bytes(b"X" * 3_000_000)

    cfg = HeraConfig(
        db_path=str(tmp_path / "hera_brain.db"),
        providers=ProvidersConfig(local_folders=[str(music_dir), str(own_dir)]),
    )
    db = Database(cfg.db_path)
    await db.init_schema()

    auditor = RepertoireAuditor(cfg)
    report = auditor.run_full_audit()
    dna = report["dj_style_dna"]
    assert dna["dominant_bpm_zone"] == "club_afro_guaracha_122_132"
    assert "8A" in dna["dominant_camelot_keys"]
    assert dna["top_anchor_artists"][0]["artist"] == "Keinemusik"
    assert dna["top_anchor_artists"][0]["track_count"] == 2

    await sync_local_library_to_db(db, cfg)
    sync_res = await persist_dj_style_dna_to_db(
        db,
        report,
        onboarding_answers={
            "dj_handle": "DaviDJ",
            "preferred_dj_software": "VirtualDJ",
            "golden_rules": "Siempre incluir 1 produccion propia y no bajar de 320 kbps",
        },
    )
    assert sync_res["persisted_features"] >= 10

    ctx = await load_dj_brain_context(db)
    assert ctx["total_tracks_in_db"] == 3
    assert "DaviDJ" in ctx["prompt_block"]
    assert "VirtualDJ" in ctx["prompt_block"]
    assert "club_afro_guaracha_122_132" in ctx["prompt_block"]
    await db.close()

