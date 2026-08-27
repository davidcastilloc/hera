"""Suite de pruebas de regresión para análisis de transientes y sinergia acústica en HERA."""

import pytest
import numpy as np
from analyzers.audio_features.transients import (
    TransientProfile,
    TransientSynergyScore,
    TransientAnalyzer,
)


class TestTransientSynergyRegression:
    """Suite de regresión que valida el modelado de transientes y compatibilidad dinámica."""

    def test_identical_profiles_yield_perfect_synergy(self):
        """Regresión: Dos temas con perfiles idénticos deben tener score de 1.0."""
        profile_a = TransientProfile(
            crest_factor=8.5,
            transient_density=4.0,
            attack_sharpness=0.85,
            spectral_transient_ratio=1.8,
            onset_envelope_summary=[0.8] * 16,
        )
        profile_b = TransientProfile(
            crest_factor=8.5,
            transient_density=4.0,
            attack_sharpness=0.85,
            spectral_transient_ratio=1.8,
            onset_envelope_summary=[0.8] * 16,
        )

        result = TransientAnalyzer.calculate_synergy(profile_a, profile_b)

        assert result.overall_synergy == 1.0
        assert result.punch_synergy == 1.0
        assert result.density_synergy == 1.0
        assert result.attack_synergy == 1.0
        assert result.spectral_synergy == 1.0
        assert result.verdict == "EXCELLENT_MATCH"
        assert len(result.recommendations) > 0

    def test_punch_mismatch_brickwall_vs_dynamic(self):
        """Regresión: Track dinámico con pegada vs master hipercomprimido (brickwalled)."""
        dynamic_track = TransientProfile(
            crest_factor=10.0,  # Alta dinámica, bombo con pegada limpia
            transient_density=3.5,
            attack_sharpness=0.90,
            spectral_transient_ratio=2.5,
        )
        squashed_track = TransientProfile(
            crest_factor=2.0,   # Aplastado con limitador, sin transientes
            transient_density=3.5,
            attack_sharpness=0.30,
            spectral_transient_ratio=0.8,
        )

        result = TransientAnalyzer.calculate_synergy(dynamic_track, squashed_track)

        # La compatibilidad de pegada debe colapsar a 0.20
        assert result.punch_synergy == 0.20
        assert result.attack_synergy == 0.40
        assert result.overall_synergy < 0.60
        assert any("pegada" in rec.lower() or "dinámico" in rec.lower() for rec in result.recommendations)

    def test_rhythm_density_contrast_minimal_vs_dense(self):
        """Regresión: Track ambient/minimalista vs track de percusión saturada."""
        sparse_ambient = TransientProfile(
            crest_factor=6.0,
            transient_density=0.8,  # Muy pocos golpes por segundo
            attack_sharpness=0.40,
            spectral_transient_ratio=1.2,
        )
        dense_percussion = TransientProfile(
            crest_factor=6.0,
            transient_density=5.8,  # 5.8 golpes por segundo (shakers, congas, hats rápidos)
            attack_sharpness=0.75,
            spectral_transient_ratio=1.2,
        )

        result = TransientAnalyzer.calculate_synergy(sparse_ambient, dense_percussion)

        # La penalización por densidad rítmica debe ser severa (diff = 5.0 -> density_score = 0.0)
        assert result.density_synergy == 0.0
        assert result.punch_synergy == 1.0  # Misma pegada individual
        assert result.overall_synergy < 0.80
        assert any("densidad rítmica" in rec.lower() for rec in result.recommendations)

    def test_mathematical_symmetry_and_bounds_invariants(self):
        """Regresión: Simetría S(A, B) == S(B, A) y límites garantizados [0.0, 1.0]."""
        np.random.seed(42)

        for _ in range(50):
            p1 = TransientProfile(
                crest_factor=float(np.random.uniform(1.0, 25.0)),
                transient_density=float(np.random.uniform(0.1, 10.0)),
                attack_sharpness=float(np.random.uniform(0.05, 0.99)),
                spectral_transient_ratio=float(np.random.uniform(0.1, 8.0)),
            )
            p2 = TransientProfile(
                crest_factor=float(np.random.uniform(1.0, 25.0)),
                transient_density=float(np.random.uniform(0.1, 10.0)),
                attack_sharpness=float(np.random.uniform(0.05, 0.99)),
                spectral_transient_ratio=float(np.random.uniform(0.1, 8.0)),
            )

            res_ab = TransientAnalyzer.calculate_synergy(p1, p2)
            res_ba = TransientAnalyzer.calculate_synergy(p2, p1)

            # Invariante 1: Simetría estricta
            assert res_ab.overall_synergy == res_ba.overall_synergy
            assert res_ab.punch_synergy == res_ba.punch_synergy
            assert res_ab.density_synergy == res_ba.density_synergy

            # Invariante 2: Límites de probabilidad [0.0, 1.0]
            assert 0.0 <= res_ab.overall_synergy <= 1.0
            assert 0.0 <= res_ab.punch_synergy <= 1.0
            assert 0.0 <= res_ab.density_synergy <= 1.0
            assert 0.0 <= res_ab.attack_synergy <= 1.0
            assert 0.0 <= res_ab.spectral_synergy <= 1.0

    def test_dsp_audio_extraction_on_synthetic_signals(self):
        """Regresión: Extracción DSP sobre señales acústicas sintetizadas de control."""
        sr = 22050
        duration = 2.0
        t = np.linspace(0, duration, int(sr * duration), endpoint=False)

        # Señal 1: Tren de impulsos percusivos (Kick sintético con alta pegada)
        impulse_signal = np.zeros_like(t)
        # 4 golpes de bombo con decaimiento exponencial rápido (2 Hz = 120 BPM)
        for beat_idx in range(4):
            onset_idx = int(beat_idx * 0.5 * sr)
            decay_len = min(int(0.1 * sr), len(t) - onset_idx)
            decay_env = np.exp(-np.linspace(0, 15, decay_len))
            freq_sweep = np.sin(2 * np.pi * np.linspace(150, 40, decay_len) * np.linspace(0, 0.1, decay_len))
            impulse_signal[onset_idx : onset_idx + decay_len] = decay_env * freq_sweep

        # Señal 2: Onda senoidal continua pura (Sin transientes / sostenida)
        sine_signal = 0.5 * np.sin(2 * np.pi * 440 * t)

        profile_impulse = TransientAnalyzer.extract_from_audio(impulse_signal, sr=sr)
        profile_sine = TransientAnalyzer.extract_from_audio(sine_signal, sr=sr)

        # Verificaciones acústicas:
        # El tren de impulsos debe tener un Crest Factor significativamente mayor que la onda senoidal pura
        assert profile_impulse.crest_factor > profile_sine.crest_factor
        assert profile_impulse.crest_factor > 4.0
        # La onda senoidal pura teórica tiene Crest Factor = sqrt(2) ~ 1.41
        assert 1.0 <= profile_sine.crest_factor <= 2.0

        # La sinergia entre un golpe percusivo y una sinusoide continua debe detectar el fuerte contraste
        synergy = TransientAnalyzer.calculate_synergy(profile_impulse, profile_sine)
        assert synergy.overall_synergy < 0.65
        assert synergy.verdict in ["MODERATE_CONTRAST", "HIGH_RISK_MISMATCH"]
