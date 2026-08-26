"""Unit tests for SlskdLifecycle."""

import pytest
from pathlib import Path
from hera.domain.config import HeraConfig
from hera.infra.lifecycle import SlskdLifecycle


def test_lifecycle_binary_discovery():
    cfg = HeraConfig()
    lifecycle = SlskdLifecycle(cfg)
    bin_path = lifecycle.find_binary(Path("."))
    if Path("bin/slskd.exe").exists() or Path("bin/slskd").exists():
        assert bin_path is not None
        assert bin_path.exists()


@pytest.mark.asyncio
async def test_lifecycle_health_check_offline():
    cfg = HeraConfig()
    lifecycle = SlskdLifecycle(cfg)
    is_live = await lifecycle.is_running_async("http://localhost:59999", timeout=0.2)
    assert is_live is False

