"""Generación de exportaciones DJ: M3U8, Rekordbox XML y Manifiesto JSON."""

from pathlib import Path
import json
import xml.etree.ElementTree as ET
from urllib.parse import quote
from hera.contracts.crate import Crate
from hera.contracts.track import Track
from hera.domain.repositories import TrackRepository


class CrateExporter:
    def __init__(self, track_repo: TrackRepository, exports_dir: Path | str):
        self.track_repo = track_repo
        self.exports_dir = Path(exports_dir).resolve()
        self.exports_dir.mkdir(parents=True, exist_ok=True)

    async def export_m3u8(self, crate: Crate, tracks: list[Track]) -> Path:
        """Genera una playlist M3U8 UTF-8 con rutas verificadas."""
        clean_name = "".join(c if c.isalnum() or c in "._- " else "_" for c in crate.name).strip()
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
        """Genera un archivo Rekordbox XML compatible con Pioneer DJ."""
        clean_name = "".join(c if c.isalnum() or c in "._- " else "_" for c in crate.name).strip()
        out_path = self.exports_dir / f"{clean_name}_rekordbox.xml"

        root = ET.Element("DJ_PLAYLISTS", Version="1.0.0")
        ET.SubElement(root, "PRODUCT", Name="Hera", Version="0.1.0", Company="Hera DJ")

        # Collection
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
            ET.SubElement(collection, "TRACK", track_attrs)

        # Playlists
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

    async def export_manifest_json(self, crate: Crate, tracks: list[Track]) -> Path:
        """Genera un manifiesto JSON con procedencia y hashes de verificación."""
        clean_name = "".join(c if c.isalnum() or c in "._- " else "_" for c in crate.name).strip()
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
                    "sha256": t.audio_hash_sha256,
                    "path": t.library_path or t.quarantine_path,
                }
                for t in tracks
            ],
        }

        with open(out_path, "w", encoding="utf-8") as f:
            json.dump(manifest_data, f, indent=2, ensure_ascii=False)

        return out_path
