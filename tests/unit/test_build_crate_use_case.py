"""Tests unitarios para BuildHarmonicCrateUseCase."""

import pytest
from datetime import datetime, timezone
from hera.contracts.track import Track, TrackStatus
from hera.application.use_cases.build_crate import BuildHarmonicCrateUseCase


def test_sequence_tracks_harmonically():
    use_case = BuildHarmonicCrateUseCase()

    t1 = Track(
        id="t1",
        status=TrackStatus.ORGANIZED,
        canonical_title="One More Time",
        canonical_artist="Daft Punk",
        bpm=128.0,
        camelot="8B",
        energy=0.85,
    )
    t2 = Track(
        id="t2",
        status=TrackStatus.ORGANIZED,
        canonical_title="Lady",
        canonical_artist="Modjo",
        bpm=124.0,
        camelot="8A",
        energy=0.75,
    )

    sequenced = use_case.sequence_tracks_harmonically([t1, t2])
    assert len(sequenced) == 2
    # Debe ordenar por BPM ascendente (124.0 -> 128.0)
    assert sequenced[0].id == "t2"
    assert sequenced[1].id == "t1"


def test_generate_cue_sheet():
    use_case = BuildHarmonicCrateUseCase()
    t = Track(
        id="t1",
        status=TrackStatus.ORGANIZED,
        canonical_title="Lady",
        canonical_artist="Modjo",
        bpm=124.0,
        camelot="8A",
        musical_key="A minor",
    )

    sheet = use_case.generate_cue_sheet("French Touch 2000", [t])
    assert "HERA DJ CUE SHEET: French Touch 2000" in sheet
    assert "01. Modjo - Lady | 124.0 BPM | Camelot: 8A (A minor)" in sheet
