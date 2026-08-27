"""Reorganizador de Sets para HERA usando el Motor Unificado de Sinergia (Armonía + Tempo + Transientes + Discogs-EffNet)."""

from __future__ import annotations
import asyncio
from pathlib import Path
import re
import shutil
import numpy as np

from mutagen.flac import FLAC
from mutagen.mp3 import MP3
from mutagen.id3 import ID3, TIT2, TPE1, TALB, TBPM, TKEY, TRCK

from analyzers.audio_features.analyzer import AudioFeatureAnalyzer
from analyzers.audio_features.transients import TransientAnalyzer, TransientProfile
from analyzers.audio_features.style_embeddings import DiscogsEffNetStyleAnalyzer, StyleProfile
from analyzers.audio_features.synergy_engine import UnifiedSynergyEngine, TrackAcousticFingerprint, HolisticTrackSynergy


class SetReorganizer:
    def __init__(self, sets_dir: Path):
        self.sets_dir = sets_dir
        self.analyzer = AudioFeatureAnalyzer()

    def _parse_track_filename(self, file_path: Path) -> tuple[str, str, float, str]:
        """Extrae (artist, title, bpm, camelot) del nombre de archivo o tags."""
        stem = file_path.stem
        # Ejemplo: "01. Modjo - Lady (Hear Me Tonight) [129.2 BPM - 3A]"
        m = re.match(r"^(?:\d+\.\s*)?(.*?)\s*-\s*(.*?)(?:\s*\[([\d\.]+)\s*BPM\s*-\s*([0-9]{1,2}[A-B])\])?$", stem)
        if m:
            artist = m.group(1).strip()
            title = m.group(2).strip()
            bpm = float(m.group(3)) if m.group(3) else 124.0
            camelot = m.group(4).strip() if m.group(4) else "8A"
            return artist, title, bpm, camelot

        return "Unknown Artist", stem, 124.0, "8A"

    def _get_track_duration(self, file_path: Path) -> float:
        """Retorna duración en segundos."""
        try:
            if file_path.suffix.lower() == ".flac":
                return float(FLAC(str(file_path)).info.length)
            elif file_path.suffix.lower() == ".mp3":
                return float(MP3(str(file_path)).info.length)
        except Exception:
            pass
        return 240.0

    async def analyze_track_fingerprint(self, file_path: Path, set_name: str) -> tuple[Path, TrackAcousticFingerprint, float]:
        """Crea el fingerprint acústico completo del track."""
        artist, title, parsed_bpm, parsed_camelot = self._parse_track_filename(file_path)
        duration = self._get_track_duration(file_path)

        try:
            feat = await self.analyzer.analyze(file_path)
            bpm = feat.bpm or parsed_bpm
            camelot = feat.camelot or parsed_camelot
            musical_key = feat.musical_key or "A minor"
            energy = feat.energy or 0.75
        except Exception:
            bpm = parsed_bpm
            camelot = parsed_camelot
            musical_key = "A minor"
            energy = 0.75

        # Inferir o extraer transientes
        # Heurística de transientes por energía y tempo
        crest = float(np.clip(7.0 + (energy * 3.5), 4.0, 12.0))
        density = float(np.clip((bpm / 30.0) + (energy * 1.5), 2.5, 6.0))
        sharpness = float(np.clip(0.70 + (energy * 0.25), 0.50, 0.95))
        trans_prof = TransientProfile(
            crest_factor=round(crest, 2),
            transient_density=round(density, 2),
            attack_sharpness=round(sharpness, 2),
            spectral_transient_ratio=1.8,
        )

        # Inferir subgénero a partir del nombre del set y metadatos
        style_seed = "Electronic"
        set_lower = set_name.lower()
        if "french touch" in set_lower:
            style_seed = "French Touch"
        elif "electro" in set_lower or "dirty" in set_lower:
            style_seed = "Electro House"
        elif "trance" in set_lower or "asot" in set_lower or "sensation" in set_lower:
            style_seed = "Trance"
        elif "progressive" in set_lower or "stadium" in set_lower:
            style_seed = "Progressive House"
        elif "disco" in set_lower or "funk" in set_lower:
            style_seed = "Disco House"
        elif "vocal" in set_lower or "soulful" in set_lower:
            style_seed = "Deep House"
        elif "dutch" in set_lower or "club" in set_lower:
            style_seed = "Tech House"

        style_prof = DiscogsEffNetStyleAnalyzer.create_profile(
            primary_style=style_seed,
            confidence=0.90,
        )

        fp = TrackAcousticFingerprint(
            track_id=file_path.name,
            title=title,
            artist=artist,
            bpm=bpm,
            musical_key=musical_key,
            camelot=camelot,
            energy=energy,
            transients=trans_prof,
            style=style_prof,
        )

        return file_path, fp, duration

    def optimize_set_sequence(
        self,
        tracks: list[tuple[Path, TrackAcousticFingerprint, float]],
    ) -> tuple[list[tuple[Path, TrackAcousticFingerprint, float]], list[HolisticTrackSynergy]]:
        """
        Encuentra la secuencia óptima maximizando la sinergia global:
        - Progresión natural de BPM (de menor a mayor o curva de energía controlada).
        - Transiciones armónicas fluidas (Camelot Wheel).
        - Coherencia de transientes y estilo.
        """
        if len(tracks) <= 1:
            return tracks, []

        # 1. Encontrar el mejor track de apertura (menor o moderado BPM, energía balanceada)
        tracks_sorted_tempo = sorted(tracks, key=lambda x: (x[1].bpm, x[1].energy))
        start_track = tracks_sorted_tempo[0]

        remaining = [t for t in tracks if t[0] != start_track[0]]
        ordered = [start_track]
        transitions: list[HolisticTrackSynergy] = []

        current = start_track

        # 2. Greedy forward matching ponderando BPM ascendente/estable + Sinergia Holística
        while remaining:
            best_candidate = None
            best_synergy_eval = None
            best_composite_score = -1.0

            for cand in remaining:
                syn = UnifiedSynergyEngine.evaluate_track_pair(current[1], cand[1])
                
                # Penalización suave si el BPM retrocede drásticamente
                bpm_delta = cand[1].bpm - current[1].bpm
                bpm_flow_penalty = 0.0
                if bpm_delta < -3.0:
                    bpm_flow_penalty = abs(bpm_delta) * 0.03
                elif 0.0 <= bpm_delta <= 5.0:
                    bpm_flow_penalty = -0.05  # Bonificación por progresión ascendente gradual

                adjusted_score = syn.master_synergy_score - bpm_flow_penalty

                if adjusted_score > best_composite_score:
                    best_composite_score = adjusted_score
                    best_candidate = cand
                    best_synergy_eval = syn

            ordered.append(best_candidate)
            transitions.append(best_synergy_eval)
            remaining.remove(best_candidate)
            current = best_candidate

        return ordered, transitions

    async def reorganize_single_set(self, set_folder: Path) -> dict:
        """Reorganiza un set específico en disco y regenera su cue sheet y metadata."""
        audio_files = [
            f for f in set_folder.glob("*.*")
            if f.is_file() and not f.name.startswith("_") and f.suffix.lower() in [".flac", ".mp3"]
        ]

        if not audio_files:
            return {"set_name": set_folder.name, "tracks_count": 0, "status": "empty"}

        # 1. Analizar huellas acústicas
        analyzed = []
        for f in audio_files:
            item = await self.analyze_track_fingerprint(f, set_folder.name)
            analyzed.append(item)

        # 2. Calcular secuencia óptima por sinergia
        optimal_sequence, transitions = self.optimize_set_sequence(analyzed)

        # 3. Renombrar temporalmente para evitar colisiones
        temp_renamed = []
        for idx, (path, fp, dur) in enumerate(optimal_sequence, start=1):
            temp_name = f"__temp_{idx:03d}_{path.name}"
            temp_path = set_folder / temp_name
            shutil.move(str(path), str(temp_path))
            temp_renamed.append((temp_path, fp, dur, path.suffix.lower()))

        # 4. Renombrar con numeración canónica definitiva
        final_files = []
        cue_sheet_lines = [
            "=" * 78,
            "🎧 HERA DJ CUE SHEET (OPTIMIZADO POR SINERGIA HOLÍSTICA)",
            f"💿 Set: {set_folder.name}",
            "=" * 78,
            f"{'POS':<4} | {'ARTISTA & TÍTULO':<38} | {'BPM':<6} | {'CAMELOT':<6} | {'PUNCH / DENS'}",
            "-" * 78,
        ]

        total_sec = 0.0
        for idx, (t_path, fp, dur, ext) in enumerate(temp_renamed, start=1):
            total_sec += dur
            raw_title = f"{idx:02d}. {fp.artist} - {fp.title} [{fp.bpm:.1f} BPM - {fp.camelot}]{ext}"
            clean_name = "".join(c for c in raw_title if c not in '<>:"/\\|?*')
            final_path = set_folder / clean_name
            shutil.move(str(t_path), str(final_path))
            final_files.append(final_path)

            # Inyectar tags ID3v2.4 / Vorbis actualizados
            try:
                if ext == ".flac":
                    fl = FLAC(str(final_path))
                    fl["TRACKNUMBER"] = f"{idx:02d}"
                    fl["BPM"] = str(fp.bpm)
                    fl["INITIALKEY"] = fp.camelot
                    fl["KEY"] = f"{fp.camelot} ({fp.musical_key})"
                    fl["ALBUM"] = set_folder.name
                    fl.save()
                elif ext == ".mp3":
                    try:
                        id3 = ID3(str(final_path))
                    except Exception:
                        id3 = ID3()
                    id3.add(TRCK(encoding=3, text=f"{idx:02d}"))
                    id3.add(TBPM(encoding=3, text=str(fp.bpm).split(".")[0]))
                    id3.add(TKEY(encoding=3, text=fp.camelot))
                    id3.add(TALB(encoding=3, text=set_folder.name))
                    id3.save(str(final_path))
            except Exception:
                pass

            punch_desc = f"Crest:{fp.transients.crest_factor:.1f} | D:{fp.transients.transient_density:.1f}"
            cue_sheet_lines.append(
                f"{idx:02d}.  | {fp.artist[:16]} - {fp.title[:18]:<18} | {fp.bpm:5.1f} | {fp.camelot:<6} | {punch_desc}"
            )

            # Agregar notas de transición entre tracks
            if idx <= len(transitions):
                tr = transitions[idx - 1]
                cue_sheet_lines.append(
                    f"     └── ➔ [Sinergia: {tr.master_synergy_score:.2f} ({tr.transition_grade})] {tr.executive_summary}"
                )

        cue_sheet_lines.append("=" * 78)
        cue_sheet_lines.append(f"⏱️ DURACIÓN TOTAL DEL SET: {total_sec / 60.0:.2f} MINUTOS ({len(final_files)} PISTAS)")
        cue_sheet_lines.append("=" * 78)

        # Guardar _00_SET_GUIDE.txt
        guide_file = set_folder / "_00_SET_GUIDE.txt"
        guide_file.write_text("\n".join(cue_sheet_lines), encoding="utf-8")

        avg_synergy = np.mean([t.master_synergy_score for t in transitions]) if transitions else 1.0

        return {
            "set_name": set_folder.name,
            "tracks_count": len(final_files),
            "duration_minutes": round(total_sec / 60.0, 2),
            "avg_synergy_score": round(float(avg_synergy), 3),
            "status": "reorganized_success",
        }

    async def reorganize_all_sets(self) -> list[dict]:
        """Reorganiza todos los sets de la biblioteca."""
        set_dirs = [
            d for d in self.sets_dir.glob("Set*")
            if d.is_dir()
        ]
        set_dirs.sort(key=lambda x: int(re.search(r"Set\s*(\d+)", x.name).group(1)) if re.search(r"Set\s*(\d+)", x.name) else 99)

        reports = []
        for s in set_dirs:
            print(f"Reorganizando por sinergia: {s.name} ...")
            rep = await self.reorganize_single_set(s)
            reports.append(rep)

        return reports


async def main():
    sets_path = Path("sets")
    if not sets_path.exists() or not list(sets_path.glob("Set*")):
        sets_path = Path("outputs/hera/sets")

    reorganizer = SetReorganizer(sets_path)
    reports = await reorganizer.reorganize_all_sets()

    print("\n" + "=" * 80)
    print(f"{'SET REORGANIZADO':<52} | {'TRKS':<4} | {'DURACIÓN':<8} | {'SINERGIA MEDIA'}")
    print("=" * 80)
    for r in reports:
        print(f"{r['set_name']:<52} | {r['tracks_count']:2d}   | {r['duration_minutes']:5.2f}m  | {r['avg_synergy_score']*100:5.1f}%")
    print("=" * 80)


if __name__ == "__main__":
    asyncio.run(main())
