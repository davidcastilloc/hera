"""Motor unificado de Sinergia Musical para HERA: Armonía + Tempo + Transientes + Estilo Discogs."""

from __future__ import annotations
from pydantic import BaseModel, Field
from analyzers.audio_features.transients import TransientProfile, TransientSynergyScore, TransientAnalyzer
from analyzers.audio_features.style_embeddings import StyleProfile, StyleSynergyScore, DiscogsEffNetStyleAnalyzer


class TrackAcousticFingerprint(BaseModel):
    """Firma acústica completa de una canción."""
    track_id: str
    title: str
    artist: str
    bpm: float
    musical_key: str
    camelot: str
    energy: float
    transients: TransientProfile
    style: StyleProfile


class HolisticTrackSynergy(BaseModel):
    """Evaluación holística de sinergia entre dos canciones."""
    track_a_id: str
    track_b_id: str
    harmonic_synergy: float = Field(..., ge=0.0, le=1.0, description="Compatibilidad en la Rueda Camelot")
    tempo_synergy: float = Field(..., ge=0.0, le=1.0, description="Alineación y delta de BPM")
    transient_synergy: TransientSynergyScore = Field(..., description="Sinergia de pegada y transientes")
    style_synergy: StyleSynergyScore = Field(..., description="Sinergia de estilo y subgénero Discogs")
    master_synergy_score: float = Field(..., ge=0.0, le=1.0, description="Puntuación maestra ponderada de sinergia")
    transition_grade: str = Field(..., description="Calificación (S, A, B, C, F)")
    executive_summary: str = Field(..., description="Resumen ejecutivo para la sesión DJ")


class UnifiedSynergyEngine:
    """Motor de cálculo de sinergia multidimensional."""

    @staticmethod
    def calculate_camelot_distance(cam_a: str, cam_b: str) -> float:
        """Calcula la distancia armónica en la rueda Camelot (1.0 = idéntica o vecina, <0.4 = choque)."""
        if not cam_a or not cam_b:
            return 0.5

        try:
            num_a, letter_a = int(cam_a[:-1]), cam_a[-1].upper()
            num_b, letter_b = int(cam_b[:-1]), cam_b[-1].upper()
        except Exception:
            return 0.5

        # Mismo tono exacto (ej. 8A -> 8A)
        if cam_a == cam_b:
            return 1.0

        # Relativa mayor / menor (ej. 8A [A menor] <-> 8B [C mayor])
        if num_a == num_b and letter_a != letter_b:
            return 0.95

        # Adyacente (+1 / -1) en la misma letra (ej. 8A -> 9A o 8A -> 7A)
        diff = abs(num_a - num_b)
        circ_diff = min(diff, 12 - diff)

        if letter_a == letter_b:
            if circ_diff == 1:
                return 0.90
            elif circ_diff == 2:
                return 0.75  # Energy boost (+2)
            elif circ_diff == 7:
                return 0.70  # Dominant leap (+7)
            else:
                return max(0.1, 1.0 - (circ_diff * 0.15))

        # Adyacente con cambio de letra (ej. 8A -> 9B o 8B -> 7A)
        if circ_diff == 1 and letter_a != letter_b:
            return 0.70

        # Diagonal lejana
        return max(0.10, 0.60 - (circ_diff * 0.10))

    @staticmethod
    def calculate_bpm_synergy(bpm_a: float, bpm_b: float, max_stretch_pct: float = 0.06) -> float:
        """Calcula compatibilidad de tempo permitiendo hasta 6% de pitch stretch estándar de DJ."""
        if bpm_a <= 0 or bpm_b <= 0:
            return 0.5

        ratio = bpm_b / bpm_a
        possible_ratios = [ratio, ratio * 2.0, ratio / 2.0]
        best_diff = min(abs(1.0 - r) for r in possible_ratios)

        if best_diff <= 0.01:
            return 1.0
        elif best_diff <= max_stretch_pct:
            return 1.0 - (best_diff / max_stretch_pct) * 0.3
        else:
            return max(0.0, 1.0 - (best_diff / 0.20))

    @classmethod
    def evaluate_track_pair(
        cls,
        track_a: TrackAcousticFingerprint,
        track_b: TrackAcousticFingerprint,
        weights: dict[str, float] | None = None,
    ) -> HolisticTrackSynergy:
        """Calcula la sinergia completa unificada."""
        w = weights or {
            "harmonic": 0.25,
            "tempo": 0.20,
            "transient": 0.30,
            "style": 0.25,
        }

        # 1. Armonía
        harm_score = cls.calculate_camelot_distance(track_a.camelot, track_b.camelot)

        # 2. BPM
        tempo_score = cls.calculate_bpm_synergy(track_a.bpm, track_b.bpm)

        # 3. Transientes
        trans_res = TransientAnalyzer.calculate_synergy(track_a.transients, track_b.transients)

        # 4. Estilo / Subgénero Discogs
        style_res = DiscogsEffNetStyleAnalyzer.calculate_style_synergy(track_a.style, track_b.style)

        # 5. Master Score
        master = (
            w["harmonic"] * harm_score
            + w["tempo"] * tempo_score
            + w["transient"] * trans_res.overall_synergy
            + w["style"] * style_res.overall_style_synergy
        )
        master = round(min(1.0, max(0.0, master)), 3)

        # 6. Calificación y resumen
        styles_context = f"'{track_a.style.primary_style}' ➔ '{track_b.style.primary_style}'"
        keys_context = f"{track_a.camelot} ➔ {track_b.camelot}"

        if master >= 0.88:
            grade = "S"
            summary = f"Transición Maestra (S): Sinergia óptima en estilo ({styles_context}), armonía ({keys_context}) y pegada de transientes."
        elif master >= 0.75:
            grade = "A"
            summary = f"Excelente compatibilidad (A): flujo armónico y groove alineado ({styles_context} a {track_a.bpm:.0f} ➔ {track_b.bpm:.0f} BPM)."
        elif master >= 0.60:
            grade = "B"
            summary = f"Transición buena (B): contraste moderado entre {styles_context}. Recomendado ajustar EQ o usar transiciones de 16 compases."
        elif master >= 0.45:
            grade = "C"
            summary = f"Transición exigente (C): disparidad en transientes o subgénero ({styles_context}). Requiere puente instrumental."
        else:
            grade = "F"
            summary = f"Incompatibilidad crítica (F): choque estilístico o tonal ({styles_context}, {keys_context})."

        return HolisticTrackSynergy(
            track_a_id=track_a.track_id,
            track_b_id=track_b.track_id,
            harmonic_synergy=round(harm_score, 3),
            tempo_synergy=round(tempo_score, 3),
            transient_synergy=trans_res,
            style_synergy=style_res,
            master_synergy_score=master,
            transition_grade=grade,
            executive_summary=summary,
        )
