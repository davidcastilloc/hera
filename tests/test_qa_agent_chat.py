"""QA Test Suite for Hera Chat / Agent & Natural Language Tools."""

import asyncio
from pathlib import Path
import pytest

from hera.agent.backends import BackendRegistry
from hera.agent.brain import HeraBrain, HERA_TOOLS, ACTIVE_COST_TRACKER
from hera.agent.tools import (
    get_library_status,
    recommend_harmonic_transitions,
    create_or_update_dj_set,
    sync_sets_to_cloud,
    get_community_status,
    search_and_acquire_tracks,
    get_session_cost_and_tokens,
)
from hera.domain.config import AgentConfig, HeraConfig
from hera.domain.cost import CostTracker
from hera.infra.lifecycle import SlskdLifecycle


def test_qa_tool_registration():
    """QA 1: Verify all 7 tools are registered in HERA_TOOLS."""
    assert len(HERA_TOOLS) == 7
    tool_names = [t.__name__ for t in HERA_TOOLS]
    assert "search_and_acquire_tracks" in tool_names
    assert "create_or_update_dj_set" in tool_names
    assert "sync_sets_to_cloud" in tool_names
    assert "get_library_status" in tool_names
    assert "recommend_harmonic_transitions" in tool_names
    assert "get_community_status" in tool_names
    assert "get_session_cost_and_tokens" in tool_names


def test_qa_recommend_harmonic_transitions():
    """QA 2: Test Camelot harmonic mixing calculation."""
    rec = recommend_harmonic_transitions("8A", 124.0)
    assert "8A" in rec
    assert "9A" in rec
    assert "7A" in rec
    assert "8B" in rec
    assert "10A" in rec  # energy boost
    assert "BPM Range" in rec


def test_qa_get_library_status():
    """QA 3: Test library inventory summary tool."""
    status = get_library_status()
    assert "LIBRARY INVENTORY" in status


@pytest.mark.asyncio
async def test_qa_get_community_status():
    """QA 4: Test community collaboration impact reporting."""
    status = await get_community_status()
    assert "Estado:" in status
    assert "tracks" in status


@pytest.mark.asyncio
async def test_qa_create_and_update_dj_set(tmp_path):
    """QA 5: Test DJ set / crate creation tool."""
    res = await create_or_update_dj_set("QA Test Set", ["Modjo - Lady", "Daft Punk - One More Time"])
    assert "QA Test Set" in res
    assert "Successfully" in res or "Created" in res or "tracks" in res


@pytest.mark.asyncio
async def test_qa_sync_sets_to_cloud_dry_run():
    """QA 6: Test cloud sync tool in dry-run mode."""
    res = await sync_sets_to_cloud(folder_name="Hera_Music/sets", dry_run=True)
    assert isinstance(res, str)
    assert len(res) > 0


@pytest.mark.asyncio
async def test_qa_search_and_acquire_offline_fallback():
    """QA 7: Test search tool when slskd port is offline."""
    res = await search_and_acquire_tracks(["Test Artist - Test Track"])
    assert isinstance(res, str)
    assert "Test Artist - Test Track" in res


def test_qa_backend_registry_detection():
    """QA 8: Test LLM backend resolution."""
    backends = BackendRegistry.list_backends()
    assert isinstance(backends, list)
    assert len(backends) == 12

    cfg = AgentConfig(backend="custom", base_url="http://localhost:11434/v1", model="test-model")
    resolved = BackendRegistry.resolve(cfg)
    assert resolved is not None
    assert resolved["type"] == "openai_compatible"
    assert resolved["model"] == "test-model"


def test_qa_lifecycle_integration():
    """QA 9: Test SlskdLifecycle discovery and health check API."""
    cfg = HeraConfig()
    lifecycle = SlskdLifecycle(cfg)
    bin_path = lifecycle.find_binary()
    if bin_path:
        assert bin_path.exists()
    assert lifecycle.is_running_sync("http://localhost:59999", timeout=0.2) is False


def test_qa_cost_tracker_and_snapbar():
    """QA 10: Test token tracking, pricing calculations, and Snapbar formatting."""
    tracker = CostTracker(backend_name="vertex", model_name="gemini-2.5-flash", max_session_cost_usd=0.01)
    
    # Turn 1: 5000 prompt tokens, 100 completion tokens
    res = tracker.record_turn(5000, 100)
    assert tracker.total_tokens == 5100
    assert tracker.total_cost_usd > 0
    assert res["budget_exceeded"] is False

    snapbar = tracker.format_snapbar()
    assert "SNAPBAR DE CONSUMO" in snapbar
    assert "5,100 tokens" in snapbar

    # Turn 2: Big request triggering budget limit ($0.01)
    res2 = tracker.record_turn(200_000, 5000)
    assert res2["budget_exceeded"] is True
    snapbar2 = tracker.format_snapbar()
    assert "ALERTA DE PRESUPUESTO" in snapbar2


def test_qa_get_session_cost_tool():
    """QA 11: Test get_session_cost_and_tokens tool output."""
    report = get_session_cost_and_tokens()
    assert "REPORTE DE CONSUMO" in report or "Aún no hay" in report


def test_qa_streamlit_ui_app():
    """QA 12: Test Streamlit Web UI App compilation and initial rendering."""
    from streamlit.testing.v1 import AppTest
    app_path = Path(__file__).parent.parent / "src" / "hera" / "ui" / "app.py"
    at = AppTest.from_file(str(app_path.resolve()))
    at.run(timeout=10)
    assert len(at.exception) == 0
    assert len(at.sidebar) > 0






