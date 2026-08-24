"""Pruebas unitarias del motor de políticas y seguridad."""

from pathlib import Path
import pytest
from hera.contracts.authorization import Authorization, AuthorizationBasis
from hera.contracts.candidate import Candidate, ScoreComponents
from hera.domain.config import PolicyConfig
from hera.policy.engine import PolicyEngine
from hera.policy.path_validator import sanitize_filename, validate_path_safety


def test_sanitize_filename():
    dangerous = 'My Track: Extended/Mix (2024)?*<>|'
    clean = sanitize_filename(dangerous)
    assert ":" not in clean
    assert "/" not in clean
    assert "?" not in clean
    assert "<" not in clean


def test_validate_path_safety():
    base = Path("/music/library")
    safe = Path("/music/library/Artist/Album/track.flac")
    traversal = Path("/music/library/../../etc/passwd")

    assert validate_path_safety(base, safe) is True
    assert validate_path_safety(base, traversal) is False


def test_policy_engine_authorizations():
    config = PolicyConfig(
        require_approval=True,
        allowed_bases=["purchased_copy", "owned_original"],
        max_file_size_mb=100,
    )
    engine = PolicyEngine(config)

    comps = ScoreComponents(
        identity=0.9, technical=0.9, source=0.8, availability=1.0, preference=0.8, metadata=0.7, risk=0.9
    )
    cand = Candidate(
        search_id="s1", provider="local", native_ref="ref", artist="A", title="T",
        score=90.0, score_components=comps, file_size_bytes=50_000_000,
    )

    # 1. Autorización aprobada con token
    auth = Authorization(basis=AuthorizationBasis.PURCHASED_COPY, evidence_ref="receipt:123")
    res = engine.authorize_download(cand, auth, approval_token="token_abc")
    assert res.approved is True

    # 2. Denegada por falta de approval_token
    res_no_tok = engine.authorize_download(cand, auth, approval_token=None)
    assert res_no_tok.approved is False
    assert "aprobación" in res_no_tok.reason.lower()

    # 3. Denegada por base no permitida
    auth_open = Authorization(basis=AuthorizationBasis.OPEN_LICENSE, evidence_ref="url:license")
    res_disallowed = engine.authorize_download(cand, auth_open, approval_token="tok")
    assert res_disallowed.approved is False
