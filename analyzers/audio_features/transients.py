"""Módulo de análisis de transientes y cálculo de sinergia rítmico-dinámica para HERA."""

from __future__ import annotations
import math
from typing import Sequence
import numpy as np
from pydantic import BaseModel, Field


class TransientProfile(BaseModel):
    """Firma de transientes de un track de audio."""
    crest_factor: float = Field(..., description="Relación Peak / RMS (rango dinámico del impacto / pegada)")
    transient_density: float = Field(..., description="Onsets por segundo detectados en la señal")
    attack_sharpness: float = Field(..., description="Pendiente / velocidad de subida promedio de los ataques (0.0 a 1.0)")
    spectral_transient_ratio: float = Field(
        ..., description="Proporción de energía de transiente en graves (<250Hz) vs agudos (>2kHz)"
    )
    onset_envelope_summary: list[float] | None = Field(
        default=None, description="Resumen normalizado del pulso rítmico (16 bins)"
    )


class TransientSynergyScore(BaseModel):
    """Evaluación de sinergia entre dos perfiles de transientes."""
    punch_synergy: float = Field(..., ge=0.0, le=1.0, description="Compatibilidad de pegada y compresión dinámica")
    density_synergy: float = Field(..., ge=0.0, le=1.0, description="Compatibilidad de densidad y groove rítmico")
    attack_synergy: float = Field(..., ge=0.0, le=1.0, description="Compatibilidad de textura y nitidez de ataque")
    spectral_synergy: float = Field(..., ge=0.0, le=1.0, description="Equilibrio espectral en los impactos (Kick vs Hi-Hat)")
    overall_synergy: float = Field(..., ge=0.0, le=1.0, description="Puntuación ponderada global de sinergia")
    verdict: str = Field(..., description="Veredicto cualitativo de la transición")
    recommendations: list[str] = Field(default_factory=list, description="Recomendaciones operativas para la mezcla DJ")


class TransientAnalyzer:
    """Extractor DSP de transientes y comparador de sinergia acústica."""

    @staticmethod
    def extract_from_audio(y: np.ndarray, sr: int = 22050) -> TransientProfile:
        """Extrae el perfil de transientes a partir de una señal de audio flotante mono."""
        if y is None or len(y) == 0:
            return TransientProfile(
                crest_factor=3.0,
                transient_density=2.0,
                attack_sharpness=0.5,
                spectral_transient_ratio=1.0,
                onset_envelope_summary=[0.5] * 16,
            )

        import librosa
        from scipy import signal

        # 1. Crest Factor = Peak / (RMS + epsilon)
        peak = float(np.max(np.abs(y)))
        rms = float(np.sqrt(np.mean(y**2)))
        crest = float(np.clip(peak / max(rms, 1e-6), 1.0, 30.0))

        # 2. Onset Strength & Density
        onset_env = librosa.onset.onset_strength(y=y, sr=sr)
        onsets = librosa.onset.onset_detect(onset_envelope=onset_env, sr=sr, backtrack=False)
        duration_sec = max(0.5, float(len(y)) / sr)
        density = float(len(onsets) / duration_sec)

        # 3. Attack Sharpness (Pendiente de subida de los onsets)
        if len(onset_env) > 1:
            diffs = np.diff(onset_env)
            positive_slopes = diffs[diffs > 0]
            if len(positive_slopes) > 0:
                mean_slope = float(np.mean(positive_slopes))
                sharpness = float(np.clip(mean_slope / (np.max(onset_env) + 1e-5), 0.05, 0.99))
            else:
                sharpness = 0.5
        else:
            sharpness = 0.5

        # 4. Spectral Band Split: Low-end (<250Hz) vs High-end (>2000Hz)
        try:
            # Low-pass filter (250Hz)
            b_low, a_low = signal.butter(2, min(250.0 / (sr / 2), 0.99), btype="low")
            y_low = signal.filtfilt(b_low, a_low, y)
            low_rms = float(np.sqrt(np.mean(y_low**2)))

            # High-pass filter (2000Hz)
            b_high, a_high = signal.butter(2, min(2000.0 / (sr / 2), 0.99), btype="high")
            y_high = signal.filtfilt(b_high, a_high, y)
            high_rms = float(np.sqrt(np.mean(y_high**2)))

            spectral_ratio = float(np.clip(low_rms / max(high_rms, 1e-6), 0.1, 10.0))
        except Exception:
            spectral_ratio = 1.0

        # 5. Resumen de envolvente (16 bins)
        if len(onset_env) >= 16:
            step = len(onset_env) // 16
            summary = [float(np.mean(onset_env[i * step : (i + 1) * step])) for i in range(16)]
            max_s = max(summary) if max(summary) > 0 else 1.0
            summary = [round(s / max_s, 3) for s in summary]
        else:
            summary = [0.5] * 16

        return TransientProfile(
            crest_factor=round(crest, 2),
            transient_density=round(density, 2),
            attack_sharpness=round(sharpness, 2),
            spectral_transient_ratio=round(spectral_ratio, 2),
            onset_envelope_summary=summary,
        )

    @classmethod
    def calculate_synergy(
        cls,
        track_a: TransientProfile,
        track_b: TransientProfile,
        weights: dict[str, float] | None = None,
    ) -> TransientSynergyScore:
        """Calcula la sinergia de transientes entre dos temas con métricas deterministas y acotadas [0, 1]."""
        w = weights or {
            "punch": 0.35,
            "density": 0.25,
            "attack": 0.20,
            "spectral": 0.20,
        }

        # 1. Punch Synergy (Basada en diferencia de Crest Factor)
        max_crest = max(track_a.crest_factor, track_b.crest_factor, 1.0)
        crest_diff = abs(track_a.crest_factor - track_b.crest_factor)
        punch_score = max(0.0, 1.0 - (crest_diff / max_crest))

        # 2. Density Synergy (Tolerancia normalizada a 4 onsets/seg de diferencia)
        density_diff = abs(track_a.transient_density - track_b.transient_density)
        density_score = max(0.0, 1.0 - (density_diff / 5.0))

        # 3. Attack Sharpness Synergy
        attack_diff = abs(track_a.attack_sharpness - track_b.attack_sharpness)
        attack_score = max(0.0, 1.0 - attack_diff)

        # 4. Spectral Transient Synergy (Relación grave/agudo)
        max_spec = max(track_a.spectral_transient_ratio, track_b.spectral_transient_ratio, 0.1)
        spec_diff = abs(track_a.spectral_transient_ratio - track_b.spectral_transient_ratio)
        spectral_score = max(0.0, 1.0 - (spec_diff / max_spec))

        # 5. Puntuación Global Ponderada
        overall = (
            w["punch"] * punch_score
            + w["density"] * density_score
            + w["attack"] * attack_score
            + w["spectral"] * spectral_score
        )
        overall = float(np.clip(overall, 0.0, 1.0))

        # 6. Veredicto y Recomendaciones Operativas para DJ
        recommendations = []
        if overall >= 0.85:
            verdict = "EXCELLENT_MATCH"
            recommendations.append("Sinergia de transientes óptima: pegada, dinámica y densidad rítmica se complementan de forma natural.")
        elif overall >= 0.70:
            verdict = "GOOD_MATCH"
            recommendations.append("Buena afinidad rítmica: transición fluida sin riesgo de colisión dinámica.")
        elif overall >= 0.50:
            verdict = "MODERATE_CONTRAST"
            if punch_score < 0.60:
                recommendations.append("Diferencia notable de pegada/rango dinámico: ecualizar o compensar ganancia durante la mezcla.")
            if density_score < 0.60:
                recommendations.append("Contraste de densidad rítmica (uno de los temas es más denso que el otro): usar filtros graduales.")
        else:
            verdict = "HIGH_RISK_MISMATCH"
            recommendations.append("Riesgo de choque dinámico o pérdida abrupta de pegada: evitar solapar intros/outros con transientes discordantes.")

        return TransientSynergyScore(
            punch_synergy=round(punch_score, 3),
            density_synergy=round(density_score, 3),
            attack_synergy=round(attack_score, 3),
            spectral_synergy=round(spectral_score, 3),
            overall_synergy=round(overall, 3),
            verdict=verdict,
            recommendations=recommendations,
        )
