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
    alt_bpm: float | None = None
    musical_key: str
    camelot: str
    energy: float
    transients: TransientProfile
    style: StyleProfile
    vocal_intro_active: bool = False
    vocal_outro_active: bool = False


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
    polyrhythmic_bridge: bool = Field(default=False, description="True si conecta vía puente polirrítmico 3:4 / 4:3")
    vocal_clash_warning: bool = Field(default=False, description="True si hay voz activa en Outro A e Intro B")
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
    def calculate_bpm_synergy(
        bpm_a: float,
        bpm_b: float,
        max_stretch_pct: float = 0.06,
        alt_bpm_a: float | None = None,
        alt_bpm_b: float | None = None,
    ) -> float:
        """Calcula compatibilidad de tempo permitiendo hasta 6% de pitch stretch estándar de DJ
        y puentes polirrítmicos de tresillo (3:4 / 4:3) vía AltBpm."""
        if bpm_a <= 0 or bpm_b <= 0:
            return 0.5

        ratio = bpm_b / bpm_a
        possible_ratios = [ratio, ratio * 2.0, ratio / 2.0]
        best_diff = min(abs(1.0 - r) for r in possible_ratios)

        if best_diff <= 0.01:
            base_score = 1.0
        elif best_diff <= max_stretch_pct:
            base_score = 1.0 - (best_diff / max_stretch_pct) * 0.3
        else:
            base_score = max(0.0, 1.0 - (best_diff / 0.20))

        # Verificar puente polirrítmico 3:4 / 4:3 si el tempo directo está lejano
        if base_score < 0.75:
            poly_candidates: list[float] = []
            if alt_bpm_a and alt_bpm_a > 0:
                poly_candidates.append(abs(1.0 - (bpm_b / alt_bpm_a)))
            if alt_bpm_b and alt_bpm_b > 0:
                poly_candidates.append(abs(1.0 - (alt_bpm_b / bpm_a)))
            # Relación matemática 4/3 (ej. 93 BPM Reggaetón <-> 124 BPM Club)
            poly_candidates.append(abs(1.0 - (bpm_b / (bpm_a * (4.0 / 3.0)))))
            poly_candidates.append(abs(1.0 - (bpm_b / (bpm_a * 0.75))))
            best_poly_diff = min(poly_candidates)
            if best_poly_diff <= 0.025:
                poly_score = 0.92 - (best_poly_diff / 0.025) * 0.10
                return round(max(base_score, poly_score), 4)

        return base_score

    @classmethod
    def is_polyrhythmic_bridge(cls, track_a: TrackAcousticFingerprint, track_b: TrackAcousticFingerprint) -> bool:
        """Determina si dos tracks conectan mediante puente polirrítmico 3:4 / 4:3."""
        if track_a.bpm <= 0 or track_b.bpm <= 0:
            return False
        direct_ratio = track_b.bpm / track_a.bpm
        if min(abs(1.0 - r) for r in (direct_ratio, direct_ratio * 2.0, direct_ratio / 2.0)) <= 0.08:
            return False
        checks = [
            abs(1.0 - (track_b.bpm / (track_a.bpm * (4.0 / 3.0)))),
            abs(1.0 - (track_b.bpm / (track_a.bpm * 0.75))),
        ]
        if track_a.alt_bpm and track_a.alt_bpm > 0:
            checks.append(abs(1.0 - (track_b.bpm / track_a.alt_bpm)))
        if track_b.alt_bpm and track_b.alt_bpm > 0:
            checks.append(abs(1.0 - (track_b.alt_bpm / track_a.bpm)))
        return min(checks) <= 0.025

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

        # 2. BPM (con soporte de puente polirrítmico 3:4)
        tempo_score = cls.calculate_bpm_synergy(
            track_a.bpm,
            track_b.bpm,
            alt_bpm_a=track_a.alt_bpm,
            alt_bpm_b=track_b.alt_bpm,
        )
        is_poly = cls.is_polyrhythmic_bridge(track_a, track_b)

        # 3. Transientes
        trans_res = TransientAnalyzer.calculate_synergy(track_a.transients, track_b.transients)

        # 4. Estilo / Subgénero Discogs
        style_res = DiscogsEffNetStyleAnalyzer.calculate_style_synergy(track_a.style, track_b.style)

        # 5. Master Score + Penalización por choque vocal en ventana de mezcla
        vocal_clash = bool(track_a.vocal_outro_active and track_b.vocal_intro_active)
        vocal_penalty = 0.04 if vocal_clash else 0.0

        master = (
            w["harmonic"] * harm_score
            + w["tempo"] * tempo_score
            + w["transient"] * trans_res.overall_synergy
            + w["style"] * style_res.overall_style_synergy
            - vocal_penalty
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

        if is_poly:
            summary += f" [🌉 Puente Polirrítmico 3:4: {track_a.bpm:g} ➔ {track_b.bpm:g} BPM]"
        if vocal_clash:
            summary += " [🎤 Aviso Vocal: Outro+Intro con voz; filtrar medios o mutear stem vocal]"

        return HolisticTrackSynergy(
            track_a_id=track_a.track_id,
            track_b_id=track_b.track_id,
            harmonic_synergy=round(harm_score, 3),
            tempo_synergy=round(tempo_score, 3),
            transient_synergy=trans_res,
            style_synergy=style_res,
            master_synergy_score=master,
            transition_grade=grade,
            polyrhythmic_bridge=is_poly,
            vocal_clash_warning=vocal_clash,
            executive_summary=summary,
        )
