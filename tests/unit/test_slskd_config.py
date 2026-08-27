"""Unit tests for slskd_config generator."""

import pytest
from pathlib import Path
import yaml
from hera.domain.config import HeraConfig, SharingConfig
from hera.infra.slskd_config import generate_slskd_config, update_shared_directories


def test_generate_slskd_config_with_shares(tmp_path):
    cfg = HeraConfig(
        sharing=SharingConfig(
            enabled=True,
            share_library=True,
            share_sets=True,
            max_upload_speed_kbps=4096,
            max_upload_slots=8,
        )
    )
    target = tmp_path / "slskd.yml"
    text = generate_slskd_config(cfg, {"username": "test_dj", "password": "secret"}, target_path=target)

    assert target.exists()
    data = yaml.safe_load(text)
    assert data["soulseek"]["username"] == "test_dj"
    assert len(data["shares"]["directories"]) == 2
    assert data["shares"]["directories"][0]["path"] == "../library"
    assert data["shares"]["directories"][1]["path"] == "../sets"
    assert data["transfers"]["global"]["upload"]["slots"] == 8
    assert data["transfers"]["global"]["upload"]["speed_limit"] == 4096


def test_update_shared_directories(tmp_path):
    target = tmp_path / "slskd.yml"
    target.write_text("soulseek:\n  username: existing_user\n", encoding="utf-8")

    ok = update_shared_directories(target, library_path="/custom/lib", sets_path="/custom/sets")
    assert ok is True

    data = yaml.safe_load(target.read_text(encoding="utf-8"))
    assert data["soulseek"]["username"] == "existing_user"
    assert data["shares"]["directories"][0]["path"] == "/custom/lib"
    assert data["shares"]["directories"][1]["path"] == "/custom/sets"

