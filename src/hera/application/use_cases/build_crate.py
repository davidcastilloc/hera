"""Caso de uso: Construir un crate / playlist para DJ con secuenciación armónica y sinergia holística."""

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
    """Orquesta la creación de DJ Sets / Crates armónicos y holísticamente optimizados por sinergia."""

    def __init__(self, track_repo: ITrackRepository | None = None, crate_repo: ICrateRepository | None = None):
        self.track_repo = track_repo
        self.crate_repo = crate_repo

    def _track_to_fingerprint(self, t: Track) -> TrackAcousticFingerprint:
        """Convierte una entidad Track en un TrackAcousticFingerprint con perfiles acústicos y de estilo."""
        energy_val = t.energy or 0.75
        bpm_val = t.bpm or 124.0
        camelot_val = t.camelot or "8A"
        key_val = t.musical_key or "A minor"

        # Perfil de transientes
        crest = float(np.clip(7.0 + (energy_val * 3.5), 4.0, 12.0))
        density = float(np.clip((bpm_val / 30.0) + (energy_val * 1.5), 2.5, 6.0))
        sharpness = float(np.clip(0.70 + (energy_val * 0.25), 0.50, 0.95))
        trans_prof = TransientProfile(
            crest_factor=round(crest, 2),
            transient_density=round(density, 2),
            attack_sharpness=round(sharpness, 2),
            spectral_transient_ratio=1.8,
        )

        # Perfil de estilo Discogs
        style_seed = "Electronic"
        art_l = (t.canonical_artist or "").lower()
        if any(a in art_l for a in ["daft punk", "modjo", "stardust", "cassius", "sinclar"]):
            style_seed = "French Touch"

        style_prof = DiscogsEffNetStyleAnalyzer.create_profile(
            primary_style=style_seed,
            confidence=0.90,
        )

        return TrackAcousticFingerprint(
            track_id=t.id,
            title=t.canonical_title or "Unknown Title",
            artist=t.canonical_artist or "Unknown Artist",
            bpm=bpm_val,
            musical_key=key_val,
            camelot=camelot_val,
            energy=energy_val,
            transients=trans_prof,
            style=style_prof,
        )

    def sequence_tracks_harmonically(self, tracks: list[Track]) -> list[Track]:
        """Ordena una lista de tracks maximizando la sinergia holística (Camelot + Tempo + Transientes + Estilo)."""
        if len(tracks) <= 1:
            return tracks

        # 1. Mapear tracks a fingerprints
        track_map = {t.id: (t, self._track_to_fingerprint(t)) for t in tracks}

        # 2. Empezar con el track de tempo y energía más estable/moderada
        sorted_candidates = sorted(tracks, key=lambda t: (t.bpm or 120.0, t.energy or 0.5))
        start_track = sorted_candidates[0]

        ordered_tracks = [start_track]
        remaining = [t for t in tracks if t.id != start_track.id]
        current_fp = track_map[start_track.id][1]

        # 3. Greedy path finding con ponderación de BPM y sinergia holística
        while remaining:
            best_track = None
            best_score = -1.0

            for cand in remaining:
                cand_fp = track_map[cand.id][1]
                syn = UnifiedSynergyEngine.evaluate_track_pair(current_fp, cand_fp)

                # Penalización suave por saltos hacia atrás en BPM
                bpm_delta = cand_fp.bpm - current_fp.bpm
                bpm_penalty = abs(bpm_delta) * 0.03 if bpm_delta < -3.0 else (-0.05 if 0.0 <= bpm_delta <= 5.0 else 0.0)
                adjusted_score = syn.master_synergy_score - bpm_penalty

                if adjusted_score > best_score:
                    best_score = adjusted_score
                    best_track = cand

            ordered_tracks.append(best_track)
            current_fp = track_map[best_track.id][1]
            remaining.remove(best_track)

        return ordered_tracks

    def generate_cue_sheet(self, set_name: str, tracks: list[Track]) -> str:
        """Genera el contenido ASCII del cue sheet para cabina de DJ con notas de sinergia paso a paso."""
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

            # Transición hacia el siguiente tema
            if idx < len(fps):
                next_fp = fps[idx]
                syn = UnifiedSynergyEngine.evaluate_track_pair(fp, next_fp)
                lines.append(f"     └── ➔ [Sinergia: {syn.master_synergy_score:.2f} ({syn.transition_grade})] {syn.executive_summary}")

        lines.append("=" * 78)
        return "\n".join(lines)
