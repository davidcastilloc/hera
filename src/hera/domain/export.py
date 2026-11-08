"""Generación de exportaciones DJ: M3U8, Rekordbox XML (con Hot Cues acústicos reales), Ableton Live y Manifiesto JSON."""

from pathlib import Path
import json
import os
import shutil
import xml.etree.ElementTree as ET
from urllib.parse import quote
from hera.contracts.crate import Crate
from hera.contracts.track import Track
from hera.domain.repositories import TrackRepository

CAMELOT_TO_MUSICAL: dict[str, str] = {
    "8B": "C", "3B": "Db", "10B": "D", "5B": "Eb", "12B": "E", "7B": "F",
    "2B": "F#", "9B": "G", "4B": "Ab", "11B": "A", "6B": "Bb", "1B": "B",
    "5A": "Cm", "12A": "C#m", "7A": "Dm", "2A": "D#m", "9A": "Em", "4A": "Fm",
    "11A": "F#m", "6A": "Gm", "1A": "G#m", "8A": "Am", "3A": "Bbm", "10A": "Bm",
}


class CrateExporter:
    def __init__(self, track_repo: TrackRepository, exports_dir: Path | str):
        self.track_repo = track_repo
        self.exports_dir = Path(exports_dir).resolve()
        self.exports_dir.mkdir(parents=True, exist_ok=True)

    @staticmethod
    def _sanitize_name(name: str) -> str:
        return "".join(c if c.isalnum() or c in "._- " else "_" for c in name).strip()

    @staticmethod
    def _resolve_structural_cues(t: Track, bpm: float, dur_sec: float) -> list[dict]:
        """Obtiene los 4 Hot Cues acústicos reales de la telemetría DSP del track o construye el perfil cuantizado."""
        from analyzers.audio_features.structure import StructuralCueAnalyzer

        if isinstance(t.provenance_json, dict):
            telem = t.provenance_json.get("hera_dsp_telemetry") or {}
            struct = telem.get("structure") if isinstance(telem, dict) else None
            if isinstance(struct, dict) and isinstance(struct.get("hot_cues"), list) and len(struct["hot_cues"]) >= 4:
                return struct["hot_cues"][:4]

        prof = StructuralCueAnalyzer.build_fallback_profile(bpm=bpm, duration_sec=dur_sec)
        return [c.model_dump() for c in prof.hot_cues]

    async def export_m3u8(self, crate: Crate, tracks: list[Track]) -> Path:
        """Genera una playlist M3U8 UTF-8 con rutas verificadas."""
        clean_name = self._sanitize_name(crate.name)
        out_path = self.exports_dir / f"{clean_name}.m3u8"

        lines = ["#EXTM3U\n"]
        for t in tracks:
            path_str = t.library_path or t.quarantine_path
            if not path_str or not Path(path_str).exists():
                continue
            duration_sec = (t.duration_ms // 1000) if t.duration_ms else -1
            title_part = f"{t.canonical_artist} - {t.canonical_title}"
            if t.version:
                title_part += f" ({t.version})"
            lines.append(f"#EXTINF:{duration_sec},{title_part}\n")
            lines.append(f"{path_str}\n")

        with open(out_path, "w", encoding="utf-8") as f:
            f.writelines(lines)

        return out_path

    async def export_rekordbox_xml(self, crate: Crate, tracks: list[Track]) -> Path:
        """Genera un archivo Rekordbox XML compatible con Pioneer DJ inyectando Beatgrid y los 4 Hot Cues estructurales."""
        clean_name = self._sanitize_name(crate.name)
        out_path = self.exports_dir / f"{clean_name}_rekordbox.xml"

        root = ET.Element("DJ_PLAYLISTS", Version="1.0.0")
        ET.SubElement(root, "PRODUCT", Name="Hera", Version="0.1.0", Company="Hera DJ")

        collection = ET.SubElement(root, "COLLECTION", Entries=str(len(tracks)))

        for i, t in enumerate(tracks, start=1):
            path_str = t.library_path or t.quarantine_path
            if not path_str:
                continue
            file_p = Path(path_str).resolve()
            cleaned_path = str(file_p).replace("\\", "/")
            loc_url = f"file://localhost/{quote(cleaned_path)}"

            track_attrs = {
                "TrackID": str(i),
                "Name": t.canonical_title + (f" [{t.version}]" if t.version else ""),
                "Artist": t.canonical_artist,
                "TotalTime": str((t.duration_ms // 1000) if t.duration_ms else 0),
                "AverageBpm": f"{t.bpm:.2f}" if t.bpm else "120.00",
                "Tonality": t.camelot or "8A",
                "BitRate": str(t.bitrate_kbps or 1411),
                "Location": loc_url,
                "Comments": f"Energy:{t.energy:.2f}" if t.energy else "Hera Curated",
            }
            trk_el = ET.SubElement(collection, "TRACK", track_attrs)
            bpm_val = float(t.bpm or 120.0)
            dur_sec = (t.duration_ms / 1000.0) if t.duration_ms else 210.0
            cues = self._resolve_structural_cues(t, bpm_val, dur_sec)
            cue1_pos = float(cues[0]["position_sec"])
            ET.SubElement(
                trk_el,
                "TEMPO",
                {
                    "Inizio": f"{cue1_pos:.3f}",
                    "Bpm": f"{bpm_val:.2f}",
                    "Metro": "4/4",
                    "Battito": "1",
                },
            )
            rb_colors = [
                ("40", "226", "20"),   # Cue 1: Green (Intro Beat)
                ("0", "210", "255"),   # Cue 2: Cyan (Phrase / Vocal In)
                ("255", "45", "45"),   # Cue 3: Red (Drop / Climax)
                ("255", "190", "0"),   # Cue 4: Amber (Mix Out)
            ]
            for idx_c, c_data in enumerate(cues[:4]):
                r_col, g_col, b_col = rb_colors[idx_c]
                ET.SubElement(
                    trk_el,
                    "POSITION_MARK",
                    {
                        "Name": str(c_data["name"]),
                        "Type": "0",
                        "Start": f"{float(c_data['position_sec']):.3f}",
                        "Num": str(idx_c),
                        "Red": r_col,
                        "Green": g_col,
                        "Blue": b_col,
                    },
                )

        playlists = ET.SubElement(root, "PLAYLISTS")
        root_node = ET.SubElement(playlists, "NODE", Type="0", Name="ROOT")
        crate_node = ET.SubElement(
            root_node, "NODE", Type="1", Name=crate.name, KeyType="0", Entries=str(len(tracks))
        )

        for i in range(1, len(tracks) + 1):
            ET.SubElement(crate_node, "TRACK", Key=str(i))

        tree = ET.ElementTree(root)
        tree.write(out_path, encoding="UTF-8", xml_declaration=True)

        return out_path

    async def export_cue_sheet(self, crate: Crate, tracks: list[Track]) -> Path:
        """Genera la guía de mezcla (_SET_GUIDE.txt) con notas de transición armónica Camelot y sinergia."""
        from hera.application.use_cases.build_crate import BuildHarmonicCrateUseCase

        clean_name = self._sanitize_name(crate.name)
        out_path = self.exports_dir / f"{clean_name}_SET_GUIDE.txt"
        uc = BuildHarmonicCrateUseCase()
        content = uc.generate_cue_sheet(crate.name, tracks)
        out_path.write_text(content + "\n", encoding="utf-8")
        return out_path

    async def export_ableton_curated(
        self,
        crate: Crate,
        tracks: list[Track],
        curated_dir: Path | str | None = None,
    ) -> Path:
        """Exporta pistas renombradas con el estándar técnico de Ableton Live: `[BPM]_[Camelot]_[Artista] - [Titulo].ext`
        y genera un índice JSON con LUFS Integrados, BPM, Tonalidad y Hot Cues estructurales."""
        clean_name = self._sanitize_name(crate.name)
        out_dir = Path(curated_dir).resolve() if curated_dir else (self.exports_dir / f"{clean_name}_ableton")
        out_dir.mkdir(parents=True, exist_ok=True)

        ableton_entries = []
        for idx, t in enumerate(tracks, start=1):
            path_str = t.library_path or t.quarantine_path
            if not path_str or not Path(path_str).exists():
                continue
            src_p = Path(path_str).resolve()
            bpm_int = int(round(t.bpm or 124.0))
            cam_str = (t.camelot or "8A").upper()
            artist_clean = self._sanitize_name(t.canonical_artist or "Unknown")
            title_clean = self._sanitize_name(t.canonical_title or src_p.stem)
            ableton_fname = f"{bpm_int}_{cam_str}_{artist_clean} - {title_clean}{src_p.suffix.lower()}"
            dst_p = out_dir / ableton_fname

            if not dst_p.exists():
                try:
                    os.link(src_p, dst_p)
                except OSError:
                    shutil.copy2(src_p, dst_p)

            dur_sec = (t.duration_ms / 1000.0) if t.duration_ms else 210.0
            ableton_entries.append({
                "position": idx,
                "track_id": t.id,
                "ableton_filename": ableton_fname,
                "ableton_path": str(dst_p),
                "source_path": str(src_p),
                "bpm": t.bpm or 124.0,
                "camelot": cam_str,
                "musical_key": t.musical_key or CAMELOT_TO_MUSICAL.get(cam_str, "Am"),
                "loudness_lufs": t.loudness_lufs if t.loudness_lufs is not None else -9.5,
                "energy": t.energy or 0.80,
                "hot_cues": self._resolve_structural_cues(t, float(t.bpm or 124.0), dur_sec),
            })

        index_path = out_dir / "_ABLETON_CURATION_INDEX.json"
        with open(index_path, "w", encoding="utf-8") as f:
            json.dump(
                {
                    "crate_id": crate.id,
                    "crate_name": crate.name,
                    "naming_standard": "[BPM]_[Camelot]_[Artist] - [Title].ext",
                    "track_count": len(ableton_entries),
                    "tracks": ableton_entries,
                },
                f,
                indent=2,
                ensure_ascii=False,
            )
        return index_path

    async def export_manifest_json(self, crate: Crate, tracks: list[Track]) -> Path:
        """Genera un manifiesto JSON con procedencia y hashes de verificación."""
        clean_name = self._sanitize_name(crate.name)
        out_path = self.exports_dir / f"{clean_name}_manifest.json"

        manifest_data = {
            "crate_id": crate.id,
            "crate_name": crate.name,
            "brief": crate.brief,
            "duration_target_minutes": crate.duration_target_minutes,
            "track_count": len(tracks),
            "tracks": [
                {
                    "track_id": t.id,
                    "artist": t.canonical_artist,
                    "title": t.canonical_title,
                    "version": t.version,
                    "bpm": t.bpm,
                    "camelot": t.camelot,
                    "energy": t.energy,
                    "loudness_lufs": t.loudness_lufs,
                    "sha256": t.audio_hash_sha256,
                    "path": t.library_path or t.quarantine_path,
                }
                for t in tracks
            ],
        }

        with open(out_path, "w", encoding="utf-8") as f:
            json.dump(manifest_data, f, indent=2, ensure_ascii=False)

        return out_path

    async def export_virtualdj_folder(self, crate: Crate, tracks: list[Track]) -> Path:
        """Genera un archivo `.vdjfolder` nativo de VirtualDJ junto con su sidecar `_vdj_database.xml`
        conteniendo el Beatgrid y los 4 Hot Cues acústicos (`<Poi Type="cue" Num="1..4">`)."""
        clean_name = self._sanitize_name(crate.name)
        vdjfolder_path = self.exports_dir / f"{clean_name}.vdjfolder"
        vdj_db_path = self.exports_dir / f"{clean_name}_vdj_database.xml"

        folder_root = ET.Element("VirtualFolder", Name=crate.name)
        db_root = ET.Element("VirtualDJ_Database", Version="2026")

        vdj_colors = [
            "#28E214",  # Cue 1: Green (Intro Beat)
            "#00D2FF",  # Cue 2: Cyan (Phrase / Vocal In)
            "#FF2D2D",  # Cue 3: Red (Drop / Climax)
            "#FFBE00",  # Cue 4: Amber (Mix Out)
        ]

        for idx, t in enumerate(tracks, start=1):
            path_str = t.library_path or t.quarantine_path
            if not path_str:
                continue
            file_p = Path(path_str).resolve()
            bpm_val = float(t.bpm or 124.0)
            dur_sec = (t.duration_ms / 1000.0) if t.duration_ms else 210.0
            cam_str = (t.camelot or "8A").upper()
            full_title = t.canonical_title + (f" ({t.version})" if t.version else "")

            ET.SubElement(
                folder_root,
                "Song",
                {
                    "Path": str(file_p),
                    "Idx": str(idx),
                    "Artist": t.canonical_artist,
                    "Title": full_title,
                    "Bpm": f"{bpm_val:.2f}",
                    "Key": cam_str,
                },
            )

            song_el = ET.SubElement(
                db_root,
                "Song",
                {
                    "FilePath": str(file_p),
                    "FileSize": str(t.file_size_bytes or 0),
                },
            )
            ET.SubElement(
                song_el,
                "Tags",
                {
                    "Author": t.canonical_artist,
                    "Title": full_title,
                    "Bpm": f"{bpm_val:.2f}",
                    "Key": cam_str,
                    "Comment": f"Hera Set: {crate.name}",
                },
            )
            ET.SubElement(
                song_el,
                "Infos",
                {
                    "SongLength": f"{dur_sec:.3f}",
                    "Bitrate": str(t.bitrate_kbps or 1411),
                },
            )
            ET.SubElement(
                song_el,
                "Scan",
                {
                    "Version": "801",
                    "Bpm": f"{60.0 / max(1.0, bpm_val):.6f}",
                    "Key": cam_str,
                },
            )

            cues = self._resolve_structural_cues(t, bpm_val, dur_sec)
            cue1_pos = float(cues[0]["position_sec"])
            ET.SubElement(
                song_el,
                "Poi",
                {
                    "Pos": f"{cue1_pos:.3f}",
                    "Type": "beatgrid",
                    "Bpm": f"{60.0 / max(1.0, bpm_val):.6f}",
                },
            )
            for idx_c, c_data in enumerate(cues[:4]):
                ET.SubElement(
                    song_el,
                    "Poi",
                    {
                        "Name": str(c_data["name"]),
                        "Pos": f"{float(c_data['position_sec']):.3f}",
                        "Num": str(idx_c + 1),
                        "Color": vdj_colors[idx_c],
                        "Type": "cue",
                    },
                )

        ET.ElementTree(folder_root).write(vdjfolder_path, encoding="UTF-8", xml_declaration=True)
        ET.ElementTree(db_root).write(vdj_db_path, encoding="UTF-8", xml_declaration=True)
        return vdjfolder_path

    async def export_traktor_nml(self, crate: Crate, tracks: list[Track]) -> Path:
        """Genera un archivo Native Instruments Traktor Pro (`_traktor.nml`, `VERSION="19"`)
        con `<TEMPO>`, `<MUSICAL_KEY>` y los 4 Hot Cues estructurales (`<CUE_V2 HOTCUE="0..3">`)."""
        clean_name = self._sanitize_name(crate.name)
        out_path = self.exports_dir / f"{clean_name}_traktor.nml"

        nml_root = ET.Element("NML", VERSION="19")
        ET.SubElement(nml_root, "HEAD", COMPANY="Hera DJ", PROGRAM="Hera")
        collection = ET.SubElement(nml_root, "COLLECTION", ENTRIES=str(len(tracks)))

        playlist_keys: list[str] = []

        for t in tracks:
            path_str = t.library_path or t.quarantine_path
            if not path_str:
                continue
            file_p = Path(path_str).resolve()
            bpm_val = float(t.bpm or 124.0)
            dur_sec = (t.duration_ms / 1000.0) if t.duration_ms else 210.0
            cam_str = (t.camelot or "8A").upper()
            full_title = t.canonical_title + (f" [{t.version}]" if t.version else "")

            parts = [p for p in file_p.parent.parts if p not in ("/", "\\")]
            traktor_dir = "/:" + "/:".join(parts) + "/:" if parts else "/:"
            traktor_key = f"LocalDisk{traktor_dir}{file_p.name}"
            playlist_keys.append(traktor_key)

            entry = ET.SubElement(
                collection,
                "ENTRY",
                {
                    "TITLE": full_title,
                    "ARTIST": t.canonical_artist,
                },
            )
            ET.SubElement(
                entry,
                "LOCATION",
                {
                    "DIR": traktor_dir,
                    "FILE": file_p.name,
                    "VOLUME": "LocalDisk",
                    "VOLUMEID": "LocalDisk",
                },
            )
            ET.SubElement(
                entry,
                "INFO",
                {
                    "BITRATE": str((t.bitrate_kbps or 1411) * 1000),
                    "PLAYTIME": str(int(round(dur_sec))),
                    "PLAYTIME_FLOAT": f"{dur_sec:.3f}",
                    "COMMENT": f"Camelot:{cam_str} | Energy:{(t.energy or 0.80):.2f}",
                },
            )
            ET.SubElement(
                entry,
                "TEMPO",
                {
                    "BPM": f"{bpm_val:.2f}",
                    "BPM_QUALITY": "100.000000",
                },
            )
            ET.SubElement(
                entry,
                "MUSICAL_KEY",
                {
                    "VALUE": cam_str,
                },
            )

            cues = self._resolve_structural_cues(t, bpm_val, dur_sec)
            for idx_c, c_data in enumerate(cues[:4]):
                start_ms = float(c_data["position_sec"]) * 1000.0
                ET.SubElement(
                    entry,
                    "CUE_V2",
                    {
                        "NAME": str(c_data["name"]),
                        "DISPL_ORDER": "0",
                        "TYPE": "0",
                        "START": f"{start_ms:.3f}",
                        "LEN": "0.000000",
                        "REPEATS": "-1",
                        "HOTCUE": str(idx_c),
                    },
                )

        playlists = ET.SubElement(nml_root, "PLAYLISTS")
        root_node = ET.SubElement(playlists, "NODE", TYPE="FOLDER", NAME="$ROOT")
        subnodes = ET.SubElement(root_node, "SUBNODES", COUNT="1")
        pl_node = ET.SubElement(subnodes, "NODE", TYPE="PLAYLIST", NAME=crate.name)
        pl_el = ET.SubElement(pl_node, "PLAYLIST", ENTRIES=str(len(playlist_keys)), TYPE="LIST", UUID=crate.id)
        for pk in playlist_keys:
            pl_entry = ET.SubElement(pl_el, "ENTRY")
            ET.SubElement(pl_entry, "PRIMARYKEY", TYPE="TRACK", KEY=pk)

        ET.ElementTree(nml_root).write(out_path, encoding="UTF-8", xml_declaration=True)
        return out_path
