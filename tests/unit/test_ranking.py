"""Pruebas unitarias del algoritmo de ranking explicable."""

import pytest
from hera.contracts.candidate import Candidate, ScoreComponents
from hera.contracts.preference import PreferenceProfile
from hera.domain.ranking import RankingEngine


def test_ranking_engine_weights_and_reasons():
    engine = RankingEngine()
    profile = PreferenceProfile(
        preferred_formats=["FLAC"],
        excluded_versions=["radio edit"],
    )

    comps = ScoreComponents(
        identity=0.95, technical=0.95, source=0.95, availability=1.0, preference=0.80, metadata=0.80, risk=0.90
    )
    cand_flac = Candidate(
        search_id="s1", provider="local", native_ref="ref1", artist="Black Coffee", title="Drive",
        version="Extended Mix", format="FLAC", bitrate_kbps=1411, score=90.0, score_components=comps,
    )

    score_flac, comps_flac, reasons_flac = engine.compute_score(cand_flac, profile)
    assert score_flac > 85.0
    assert any("lossless" in r.lower() for r in reasons_flac)

    # Candidato con radio edit penalizado
    cand_radio = Candidate(
        search_id="s1", provider="local", native_ref="ref2", artist="Black Coffee", title="Drive",
        version="Radio Edit", format="MP3", bitrate_kbps=192, score=70.0, score_components=comps,
    )
    score_radio, comps_radio, reasons_radio = engine.compute_score(cand_radio, profile)
    assert score_radio < score_flac
    assert any("no deseada" in r.lower() for r in reasons_radio)
