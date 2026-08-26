"""Unit tests for CommunityStats."""

import pytest
from pathlib import Path
from hera.domain.community import CommunityStats


@pytest.mark.asyncio
async def test_community_summary_calculation(tmp_path):
    lib_dir = tmp_path / "library"
    sets_dir = tmp_path / "sets"
    lib_dir.mkdir()
    sets_dir.mkdir()

    # Create dummy audio files
    (lib_dir / "track1.flac").write_bytes(b"0" * 1024 * 1024)
    (lib_dir / "track2.mp3").write_bytes(b"0" * 2 * 1024 * 1024)
    (sets_dir / "set1_track.flac").write_bytes(b"0" * 1024 * 1024)

    stats = CommunityStats(base_url="http://localhost:59999")
    summary = await stats.get_sharing_summary(lib_dir, sets_dir)

    assert summary["tracks_shared"] == 3
    assert summary["library_tracks"] == 2
    assert summary["sets_tracks"] == 1
    assert summary["total_size_bytes"] == 4 * 1024 * 1024
    assert "tracks" in summary["community_message"]

