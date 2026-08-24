"""Pruebas unitarias de contratos y máquinas de estado."""

import pytest
from hera.contracts.authorization import Authorization, AuthorizationBasis
from hera.contracts.candidate import Candidate, ScoreComponents, AuthorizationState
from hera.contracts.job import Job, JobState, JobType
from hera.contracts.track import Track, TrackStatus


def test_track_state_transitions():
    track = Track(canonical_title="Drive", canonical_artist="Black Coffee")
    assert track.status == TrackStatus.CANDIDATE

    # Transición válida
    assert track.can_transition_to(TrackStatus.DOWNLOADING) is True
    # Transición inválida directa
    assert track.can_transition_to(TrackStatus.ORGANIZED) is False

    track.status = TrackStatus.DOWNLOADING
    assert track.can_transition_to(TrackStatus.QUARANTINED) is True

    track.status = TrackStatus.QUARANTINED
    assert track.can_transition_to(TrackStatus.VALIDATED) is True

    track.status = TrackStatus.VALIDATED
    assert track.can_transition_to(TrackStatus.IDENTIFIED) is True


def test_authorization_model():
    auth = Authorization(
        basis=AuthorizationBasis.PURCHASED_COPY,
        evidence_ref="receipt:order:12345",
        acknowledged_by="dj_user",
    )
    assert auth.basis == AuthorizationBasis.PURCHASED_COPY
    assert auth.evidence_ref == "receipt:order:12345"


def test_candidate_scoring_components():
    comps = ScoreComponents(
        identity=0.95,
        technical=0.90,
        source=0.85,
        availability=1.0,
        preference=0.90,
        metadata=0.80,
        risk=0.95,
    )
    cand = Candidate(
        search_id="srch_01",
        provider="local",
        native_ref="/music/test.flac",
        artist="Artist",
        title="Title",
        score=92.5,
        score_components=comps,
    )
    assert cand.score == 92.5
    assert cand.provider == "local"


def test_job_model():
    job = Job(
        type=JobType.DOWNLOAD,
        idempotency_key="idemp_123",
        input_json={"candidate_id": "cand_01"},
    )
    assert job.state == JobState.QUEUED
    assert job.progress == 0.0
    assert job.idempotency_key == "idemp_123"
