"""Tests unitarios para SyncCloudUseCase."""

import pytest
from pathlib import Path
from unittest.mock import AsyncMock, patch, MagicMock
from hera.domain.config import HeraConfig
from hera.application.use_cases.sync_cloud import SyncCloudUseCase, SyncCloudResult
from hera.adapters.storage.rclone import SyncResult


@pytest.mark.asyncio
async def test_sync_cloud_push_success(tmp_path: Path):
    cfg = HeraConfig()
    cfg.data_dir = str(tmp_path)
    use_case = SyncCloudUseCase(cfg)

    with patch.object(use_case.rclone, "is_available", return_value=True):
        with patch.object(
            use_case.rclone,
            "copy",
            new_callable=AsyncMock,
            return_value=SyncResult(success=True, transferred_files=5, transferred_bytes=1000, output="OK"),
        ):
            res = await use_case.push_sets(remote="gdrive", folder="Hera_Music/sets")
            assert res.success is True
            assert res.transferred_files == 5
            assert "Sincronización exitosa" in res.message


@pytest.mark.asyncio
async def test_sync_cloud_pull_rclone_unavailable():
    cfg = HeraConfig()
    use_case = SyncCloudUseCase(cfg)

    with patch.object(use_case.rclone, "is_available", return_value=False):
        res = await use_case.pull_sets()
        assert res.success is False
        assert res.error == "RCLONE_NOT_FOUND"
