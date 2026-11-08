"""Caso de uso: Construir un crate / playlist para DJ con secuenciación armónica global (2-Opt TSP + Curvas de Energía) y sinergia holística."""

from pathlib import Path
import shutil
from typing import Sequence
import numpy as np
from hera.domain.ports.repositories import ITrackRepository, ICrateRepository
from hera.contracts.crate import Crate, CrateTrack
from hera.contracts.track import Track
from analyzers.audio_features.analyzer import KEY_TO_CAMELOT
from analyzers.audio_features.transients import TransientProfile, TransientAnalyzer
from analyzers.audio_features.style_embeddings import DiscogsEffNetStyleAnalyzer, StyleProfile
from analyzers.audio_features.synergy_engine import UnifiedSynergyEngine, TrackAcousticFingerprint, HolisticTrackSynergy


class BuildHarmonicCrateUseCase:
    """Orquesta la creación de DJ Sets / Crates armónicos y globalmente optimizados por sinergia."""

    def __init__(self, track_repo: ITrackRepository | None = None, crate_repo: ICrateRepository | None = None):
        self.track_repo = track_repo
        self.crate_repo = crate_repo

    def _track_to_fingerprint(self, t: Track) -> TrackAcousticFingerprint:
        """Convierte una entidad Track en un TrackAcousticFingerprint usando telemetría DSP real si existe en `provenance_json`."""
        energy_val = t.energy or 0.75
        bpm_val = t.bpm or 124.0
        camelot_val = t.camelot or "8A"
        key_val = t.musical_key or "A minor"

        telem: dict = {}
        if isinstance(t.provenance_json, dict):
            raw_t = t.provenance_json.get("hera_dsp_telemetry")
            if isinstance(raw_t, dict):
                telem = raw_t

        alt_bpm_val: float | None = telem.get("alt_bpm")

        # 1. Perfil de transientes (Real desde SQLite o estimado de respaldo)
        if isinstance(telem.get("transients"), dict):
            trans_prof = TransientProfile(**telem["transients"])
        else:
            crest = float(np.clip(7.0 + (energy_val * 3.5), 4.0, 12.0))
            density = float(np.clip((bpm_val / 30.0) + (energy_val * 1.5), 2.5, 6.0))
            sharpness = float(np.clip(0.70 + (energy_val * 0.25), 0.50, 0.95))
            trans_prof = TransientProfile(
                crest_factor=round(crest, 2),
                transient_density=round(density, 2),
                attack_sharpness=round(sharpness, 2),
                spectral_transient_ratio=1.8,
            )

        # 2. Perfil de estilo Discogs (Real desde SQLite o inferido por contexto)
        if isinstance(telem.get("style"), dict):
            style_prof = StyleProfile(**telem["style"])
        else:
            style_seed = "Electronic"
            ctx_l = f"{t.library_path or ''} {t.canonical_artist or ''} {t.canonical_title or ''}".lower()
            if any(a in ctx_l for a in ("daft punk", "modjo", "stardust", "cassius", "sinclar", "french")):
                style_seed = "French Touch"
            elif any(a in ctx_l for a in ("afro", "keinemusik", "hugel", "black coffee")):
                style_seed = "Afro House"
            elif any(a in ctx_l for a in ("tech_house", "tech house", "house_tech", "pawsa", "fisher", "cloonee", "san pacho")):
                style_seed = "Tech House"
            elif any(a in ctx_l for a in ("guaracha", "aleteo", "tribal")):
                style_seed = "Guaracha"
            elif any(a in ctx_l for a in ("reggaeton", "urbano", "feid", "karol", "bad bunny", "maluma", "beele")):
                style_seed = "Reggaeton"
            elif any(a in ctx_l for a in ("dembow", "teteo", "reparto", "alfa")):
                style_seed = "Dembow"
            elif any(a in ctx_l for a in ("salsa", "tropical", "cumbia", "merengue")):
                style_seed = "Salsa"

            style_prof = DiscogsEffNetStyleAnalyzer.create_profile(
                primary_style=style_seed,
                confidence=0.90,
            )

        # 3. Actividad vocal en Intro/Outro desde estructura acústica
        raw_struct = telem.get("structure")
        struct_dict: dict = raw_struct if isinstance(raw_struct, dict) else {}
        vocal_in = bool(struct_dict.get("vocal_intro_active", False))
        vocal_out = bool(struct_dict.get("vocal_outro_active", False))

        return TrackAcousticFingerprint(
            track_id=t.id,
            title=t.canonical_title or "Unknown Title",
            artist=t.canonical_artist or "Unknown Artist",
            bpm=bpm_val,
            alt_bpm=alt_bpm_val,
            musical_key=key_val,
            camelot=camelot_val,
            energy=energy_val,
            transients=trans_prof,
            style=style_prof,
            vocal_intro_active=vocal_in,
            vocal_outro_active=vocal_out,
        )

    @staticmethod
    def _transition_objective(
        fp_a: TrackAcousticFingerprint,
        fp_b: TrackAcousticFingerprint,
        step_idx: int = 0,
        total_steps: int = 1,
        energy_arc: str = "steady_climb",
    ) -> float:
        """Evalúa la utilidad de una transición individual incluyendo sinergia maestra, dirección de BPM y arco de energía."""
        syn = UnifiedSynergyEngine.evaluate_track_pair(fp_a, fp_b)
        bpm_delta = fp_b.bpm - fp_a.bpm
        if syn.polyrhythmic_bridge:
            bpm_penalty = -0.04  # Premiar puentes polirrítmicos 3:4 limpios
        elif bpm_delta < -3.0:
            bpm_penalty = abs(bpm_delta) * 0.03
        elif 0.0 <= bpm_delta <= 5.0:
            bpm_penalty = -0.05
        else:
            bpm_penalty = 0.0

        # Afinidad con la curva de energía deseada
        prog = step_idx / max(1, total_steps)
        if energy_arc == "rollercoaster":
            target_e = 0.72 + 0.20 * float(np.sin(prog * 2.0 * np.pi))
        elif energy_arc == "peak_hour":
            target_e = 0.88 if 0.15 <= prog <= 0.85 else 0.75
        else:  # steady_climb
            target_e = 0.68 + 0.25 * prog

        arc_bonus = max(-0.04, 0.03 - abs(fp_b.energy - target_e) * 0.08)
        return float(syn.master_synergy_score - bpm_penalty + arc_bonus)

    @classmethod
    def _total_path_objective(
        cls,
        fp_seq: list[TrackAcousticFingerprint],
        energy_arc: str = "steady_climb",
    ) -> tuple[float, float]:
        """Retorna (puntuación_acumulada, peor_transición_cuello_de_botella) para una secuencia completa."""
        if len(fp_seq) <= 1:
            return 1.0, 1.0
        n_steps = len(fp_seq) - 1
        scores = [
            cls._transition_objective(fp_seq[i], fp_seq[i + 1], i, n_steps, energy_arc)
            for i in range(n_steps)
        ]
        min_edge = min(scores)
        mean_edge = sum(scores) / len(scores)
        # Ponderar la media global + protección contra cuellos de botella (peor transición)
        return (mean_edge + 0.25 * min_edge), min_edge

    def sequence_tracks_harmonically(
        self,
        tracks: list[Track],
        energy_arc: str = "steady_climb",
    ) -> list[Track]:
        """Ordena una lista de tracks maximizando la sinergia holística global (Greedy Inicial + Refinamiento 2-Opt TSP)."""
        if len(tracks) <= 1:
            return tracks

        # 1. Mapear tracks a fingerprints reales
        track_map = {t.id: (t, self._track_to_fingerprint(t)) for t in tracks}

        # 2. Construcción inicial Greedy empezando con el track de tempo y energía más moderada
        sorted_candidates = sorted(tracks, key=lambda t: (t.bpm or 120.0, t.energy or 0.5))
        start_track = sorted_candidates[0]

        ordered_tracks = [start_track]
        remaining = [t for t in tracks if t.id != start_track.id]
        current_fp = track_map[start_track.id][1]
        n_total_steps = max(1, len(tracks) - 1)

        step_idx = 0
        while remaining:
            best_track: Track = remaining[0]
            best_score = -1e9

            for cand in remaining:
                cand_fp = track_map[cand.id][1]
                adjusted_score = self._transition_objective(
                    current_fp, cand_fp, step_idx, n_total_steps, energy_arc
                )
                if adjusted_score > best_score:
                    best_score = adjusted_score
                    best_track = cand

            ordered_tracks.append(best_track)
            current_fp = track_map[best_track.id][1]
            remaining.remove(best_track)
            step_idx += 1

        # 3. Optimización Global 2-Opt TSP (elimina pistas huérfanas y maximiza flujo de principio a fin)
        if len(ordered_tracks) >= 4:
            best_seq = list(ordered_tracks)
            best_fps = [track_map[t.id][1] for t in best_seq]
            best_obj, _ = self._total_path_objective(best_fps, energy_arc)

            improved = True
            max_passes = 6
            passes = 0
            n = len(best_seq)

            while improved and passes < max_passes:
                improved = False
                passes += 1
                for i in range(1, n - 2):
                    for j in range(i + 1, n):
                        cand_seq = best_seq[:i] + list(reversed(best_seq[i : j + 1])) + best_seq[j + 1 :]
                        cand_fps = [track_map[t.id][1] for t in cand_seq]
                        cand_obj, _ = self._total_path_objective(cand_fps, energy_arc)
                        if cand_obj > best_obj + 1e-4:
                            best_seq = cand_seq
                            best_fps = cand_fps
                            best_obj = cand_obj
                            improved = True
            ordered_tracks = best_seq

        return ordered_tracks

    def evaluate_set_metrics(self, tracks: list[Track]) -> dict:
        """Calcula métricas globales de calidad de un DJ Set secuenciado."""
        if len(tracks) <= 1:
            return {
                "mean_synergy_score": 1.0,
                "min_synergy_score": 1.0,
                "grade_s_a_count": 0,
                "polyrhythmic_bridges_count": 0,
                "vocal_clash_warnings_count": 0,
            }
        fps = [self._track_to_fingerprint(t) for t in tracks]
        synergies = [
            UnifiedSynergyEngine.evaluate_track_pair(fps[i], fps[i + 1])
            for i in range(len(fps) - 1)
        ]
        scores = [s.master_synergy_score for s in synergies]
        return {
            "mean_synergy_score": round(float(np.mean(scores)), 3),
            "min_synergy_score": round(float(np.min(scores)), 3),
            "grade_s_a_count": sum(1 for s in synergies if s.transition_grade in ("S", "A")),
            "polyrhythmic_bridges_count": sum(1 for s in synergies if s.polyrhythmic_bridge),
            "vocal_clash_warnings_count": sum(1 for s in synergies if s.vocal_clash_warning),
        }

    def generate_cue_sheet(self, set_name: str, tracks: list[Track]) -> str:
        """Genera el contenido ASCII del cue sheet para cabina de DJ con notas de sinergia y Hot Cues reales paso a paso."""
        lines = [
            "=" * 78,
            f"🎧 HERA DJ CUE SHEET: {set_name}",
            "=" * 78,
        ]

        if not tracks:
            lines.append("No hay pistas en este set.")
            lines.append("=" * 78)
            return "\n".join(lines)

        fps = [self._track_to_fingerprint(t) for t in tracks]

        for idx, (t, fp) in enumerate(zip(tracks, fps), start=1):
            camelot_val = fp.camelot
            bpm_val = fp.bpm
            key_val = fp.musical_key
            artist = fp.artist
            title = fp.title
            punch_desc = f"Crest:{fp.transients.crest_factor:.1f} | D:{fp.transients.transient_density:.1f}"

            lines.append(f"{idx:02d}. {artist} - {title} | {bpm_val} BPM | Camelot: {camelot_val} ({key_val}) | {punch_desc}")

            # Si tiene estructura de Hot Cues reales en provenance_json, mostrarla
            if isinstance(t.provenance_json, dict):
                telem = t.provenance_json.get("hera_dsp_telemetry") or {}
                struct = telem.get("structure") if isinstance(telem, dict) else None
                if isinstance(struct, dict) and struct.get("hot_cues"):
                    cues_str = " | ".join(
                        f"C{c['num']}:{c['position_sec']:.1f}s" for c in struct["hot_cues"][:4]
                    )
                    lines.append(f"     ├── 🎯 [Hot Cues Acústicos] {cues_str}")

            # Transición hacia el siguiente tema
            if idx < len(fps):
                next_fp = fps[idx]
                syn = UnifiedSynergyEngine.evaluate_track_pair(fp, next_fp)
                lines.append(f"     └── ➔ [Sinergia: {syn.master_synergy_score:.2f} ({syn.transition_grade})] {syn.executive_summary}")

        if len(fps) > 1:
            metrics = self.evaluate_set_metrics(tracks)
            lines.append("-" * 78)
            lines.append(
                f"📊 SINERGIA MEDIA DEL SET: {metrics['mean_synergy_score']:.3f} | "
                f"MÍNIMA (Cuello de Botella): {metrics['min_synergy_score']:.3f} | "
                f"Transiciones S/A: {metrics['grade_s_a_count']}/{len(fps) - 1}"
            )

        lines.append("=" * 78)
        return "\n".join(lines)
