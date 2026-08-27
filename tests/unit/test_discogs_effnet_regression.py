"""Suite de pruebas de regresión para Discogs-EffNet Style Embeddings y Unified Synergy Engine."""

import pytest
import numpy as np
from analyzers.audio_features.style_embeddings import (
    StyleProfile,
    StyleSynergyScore,
    DiscogsEffNetStyleAnalyzer,
    DISCOGS_STYLE_TAXONOMY,
)
from analyzers.audio_features.transients import TransientProfile, TransientAnalyzer
from analyzers.audio_features.synergy_engine import (
    TrackAcousticFingerprint,
    UnifiedSynergyEngine,
)


class TestDiscogsEffNetRegression:
    """Suite de regresión para el clasificador de estilos y embeddings Discogs-EffNet."""

    def test_identical_and_sister_styles_yield_natural_flow(self):
        """Regresión: Estilos hermanos (ej. French Touch y Disco House) deben dar sinergia de estilo >= 0.85."""
        p_ft = DiscogsEffNetStyleAnalyzer.create_profile(
            primary_style="French Touch",
            secondary_styles=[("Disco House", 0.90), ("Filter House", 0.85)],
        )
        p_dh = DiscogsEffNetStyleAnalyzer.create_profile(
            primary_style="Disco House",
            secondary_styles=[("French Touch", 0.90), ("Nu-Disco", 0.80)],
        )

        synergy = DiscogsEffNetStyleAnalyzer.calculate_style_synergy(p_ft, p_dh)

        assert synergy.overall_style_synergy >= 0.85
        assert synergy.verdict == "NATURAL_FLOW"
        assert synergy.taxonomy_affinity >= 0.90
        assert any("flujo" in n.lower() or "universo" in n.lower() for n in synergy.transition_notes)

    def test_crossover_electronic_subgenres(self):
        """Regresión: Subgéneros puente (Deep House -> Afro House) producen crossover fluido."""
        p_deep = DiscogsEffNetStyleAnalyzer.create_profile(
            primary_style="Deep House",
            secondary_styles=[("Tech House", 0.70)],
        )
        p_afro = DiscogsEffNetStyleAnalyzer.create_profile(
            primary_style="Afro House",
            secondary_styles=[("Deep House", 0.75)],
        )

        synergy = DiscogsEffNetStyleAnalyzer.calculate_style_synergy(p_deep, p_afro)

        assert 0.70 <= synergy.overall_style_synergy < 0.92
        assert synergy.verdict in ["SMOOTH_CROSSOVER", "NATURAL_FLOW"]
        assert synergy.taxonomy_affinity == 0.86

    def test_extreme_genre_clash_detection(self):
        """Regresión: Choque estilístico extremo (French Touch vs Heavy Metal o Death Metal)."""
        p_ft = DiscogsEffNetStyleAnalyzer.create_profile(
            primary_style="French Touch",
            secondary_styles=[("Disco House", 0.80)],
        )
        p_metal = DiscogsEffNetStyleAnalyzer.create_profile(
            primary_style="Heavy Metal",
            secondary_styles=[("Death Metal", 0.80)],
        )

        synergy = DiscogsEffNetStyleAnalyzer.calculate_style_synergy(p_ft, p_metal)

        assert synergy.overall_style_synergy < 0.40
        assert synergy.verdict == "GENRE_CLASH"
        assert synergy.taxonomy_affinity <= 0.20
        assert any("choque" in n.lower() or "disparidad" in n.lower() for n in synergy.transition_notes)

    def test_embedding_invariants_and_symmetry(self):
        """Regresión: Invariantes matemáticos de simetría y normalización L2 de los vectores 512-D."""
        styles = ["French Touch", "Minimal Techno", "Dubstep", "Ambient", "Nu-Disco"]

        for s1 in styles:
            for s2 in styles:
                prof1 = DiscogsEffNetStyleAnalyzer.create_profile(s1)
                prof2 = DiscogsEffNetStyleAnalyzer.create_profile(s2)

                # Vector L2 Normalizado
                vec1 = np.array(prof1.style_embedding)
                assert abs(np.linalg.norm(vec1) - 1.0) < 1e-4

                # Simetría S(1, 2) == S(2, 1)
                syn12 = DiscogsEffNetStyleAnalyzer.calculate_style_synergy(prof1, prof2)
                syn21 = DiscogsEffNetStyleAnalyzer.calculate_style_synergy(prof2, prof1)

                assert syn12.overall_style_synergy == syn21.overall_style_synergy
                assert syn12.embedding_cosine_similarity == syn21.embedding_cosine_similarity
                assert 0.0 <= syn12.overall_style_synergy <= 1.0

    def test_dsp_audio_extraction_style_profile(self):
        """Regresión: Extracción de StyleProfile directamente de señal de audio flotante."""
        sr = 22050
        duration = 2.0
        t = np.linspace(0, duration, int(sr * duration), endpoint=False)
        # Señal sintética con armónicos tipo house (120 BPM + filtros)
        y = 0.6 * np.sin(2 * np.pi * 120 * t) + 0.3 * np.sin(2 * np.pi * 240 * t)

        profile = DiscogsEffNetStyleAnalyzer.extract_from_audio(y, sr=sr)

        assert isinstance(profile, StyleProfile)
        assert len(profile.style_embedding) == 512
        assert profile.confidence > 0.0
        assert len(profile.secondary_styles) > 0

    def test_unified_holistic_synergy_engine_full_dj_transition(self):
        """Regresión: Evaluación holística de transición DJ (Daft Punk -> Modjo)."""
        # Track 1: Daft Punk - One More Time
        t1 = TrackAcousticFingerprint(
            track_id="trk_daft_punk_01",
            title="One More Time",
            artist="Daft Punk",
            bpm=128.0,
            musical_key="D major",
            camelot="10B",
            energy=0.88,
            transients=TransientProfile(
                crest_factor=8.5,
                transient_density=4.1,
                attack_sharpness=0.85,
                spectral_transient_ratio=1.9,
            ),
            style=DiscogsEffNetStyleAnalyzer.create_profile("French Touch", [("Disco House", 0.90)]),
        )

        # Track 2: Modjo - Lady (Hear Me Tonight)
        t2 = TrackAcousticFingerprint(
            track_id="trk_modjo_02",
            title="Lady (Hear Me Tonight)",
            artist="Modjo",
            bpm=124.0,
            musical_key="A minor",
            camelot="8A",
            energy=0.82,
            transients=TransientProfile(
                crest_factor=8.2,
                transient_density=4.0,
                attack_sharpness=0.82,
                spectral_transient_ratio=1.8,
            ),
            style=DiscogsEffNetStyleAnalyzer.create_profile("French Touch", [("Filter House", 0.85)]),
        )

        holistic = UnifiedSynergyEngine.evaluate_track_pair(t1, t2)

        # Verificaciones holísticas:
        assert holistic.master_synergy_score >= 0.75
        assert holistic.transition_grade in ["S", "A", "B"]
        assert holistic.transient_synergy.overall_synergy >= 0.90
        assert holistic.style_synergy.overall_style_synergy >= 0.85
        assert "French Touch" in holistic.executive_summary or "compatibilidad" in holistic.executive_summary.lower()
