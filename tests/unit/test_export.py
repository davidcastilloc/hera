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

    await db.close()
