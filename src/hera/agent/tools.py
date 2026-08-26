"""Herramientas de nivel de dominio expuestas al agente de Hera."""

import asyncio
from pathlib import Path
import shutil
import time
import httpx

from mutagen.flac import FLAC
from mutagen.id3 import ID3, TIT2, TPE1, TALB, TBPM, TKEY, TCON, COMM, TRCK, TDRC
from analyzers.ffmpeg.validator import FFmpegValidator
from analyzers.audio_features.analyzer import AudioFeatureAnalyzer
from hera.domain.database import Database
from hera.domain.config import HeraConfig
from hera.contracts.track import Track, TrackStatus
from hera.domain.repositories import TrackRepository, CrateRepository
from hera.adapters.storage.rclone import RcloneStorageAdapter


async def search_and_acquire_tracks(queries: list[str]) -> dict:
    """Busca y descarga pistas reales de alta calidad (FLAC/MP3 320k) en Soulseek, las valida y analiza."""
    url = "http://localhost:5030/api/v0"
    validator = FFmpegValidator()
    analyzer = AudioFeatureAnalyzer()
    
    base_dir = Path(".").resolve()
    quarantine = base_dir / "quarantine"
    library = base_dir / "library"
    quarantine.mkdir(parents=True, exist_ok=True)
    library.mkdir(parents=True, exist_ok=True)

    acquired = []
    failed = []

    for q in queries:
        try:
            r = httpx.post(f"{url}/searches", json={"searchText": q}, timeout=5.0)
            if r.status_code != 200:
                failed.append({"query": q, "reason": "No se pudo iniciar búsqueda en slskd"})
                continue
            s_id = r.json()["id"]
            await asyncio.sleep(4.0)

            res = httpx.get(f"{url}/searches/{s_id}/responses", timeout=5.0).json()
            chosen_peer = None
            chosen_file = None

            for u in res:
                if u.get("locked", False):
                    continue
                for f in u.get("files", []):
                    fn = f.get("filename", "").lower()
                    if fn.endswith(".flac") or (fn.endswith(".mp3") and f.get("bitRate", 0) >= 256):
                        if not chosen_peer or u.get("hasFreeUploadSlot", False):
                            chosen_peer = u.get("username")
                            chosen_file = f
                            if u.get("hasFreeUploadSlot", False):
                                break
                if chosen_peer and u.get("hasFreeUploadSlot", False):
                    break

            if chosen_peer and chosen_file:
                fn_clean = chosen_file["filename"].replace("\\", "/").split("/")[-1]
                sz_mb = chosen_file["size"] / (1024 * 1024)
                dl = httpx.post(
                    f"{url}/transfers/downloads/{chosen_peer}",
                    json=[{"filename": chosen_file["filename"], "size": chosen_file["size"]}],
                    timeout=5.0,
                )
                if dl.status_code in [200, 201]:
                    acquired.append({"query": q, "file": fn_clean, "peer": chosen_peer, "size_mb": round(sz_mb, 2)})
            else:
                failed.append({"query": q, "reason": "No se encontraron peers con archivos de alta calidad disponibles"})
        except Exception as e:
            failed.append({"query": q, "reason": str(e)})

    return {"status": "success", "enqueued_downloads": acquired, "failed_queries": failed}


async def build_dj_set(name: str, track_matches: list[str]) -> dict:
    """Compila y secuencia pistas de la biblioteca hacia una carpeta de set simple con tags ID3/Vorbis y Cheat Sheet."""
    base_dir = Path(".").resolve()
    library = base_dir / "library"
    quarantine = base_dir / "quarantine"
    set_folder = base_dir / "sets" / name
    set_folder.mkdir(parents=True, exist_ok=True)

    validator = FFmpegValidator()
    analyzer = AudioFeatureAnalyzer()

    all_files = list(library.rglob("*.*")) + list(quarantine.rglob("*.*"))
    all_files = [f for f in all_files if f.is_file() and f.suffix.lower() in [".flac", ".mp3"] and f.stat().st_size > 1.5 * 1024 * 1024]
    all_files.sort(key=lambda f: f.stat().st_size, reverse=True)

    added_tracks = []
    cue_lines = [
        "=" * 64,
        f"🎧 DJ SET GUIDE: {name}",
        "=" * 64,
    ]

    for idx, match_word in enumerate(track_matches, start=1):
        matched_f = None
        for f in all_files:
            if match_word.lower() in f.name.lower() or match_word.lower() in f.parent.name.lower():
                matched_f = f
                break

        if matched_f:
            val = await validator.validate_media(matched_f)
            if val.is_valid:
                feat = await analyzer.analyze(matched_f)
                artist = matched_f.parent.parent.name if matched_f.parent.parent != library else "DJ Track"
                title = matched_f.stem.replace("01 - ", "").split(" [")[0]

                clean_name = f"{idx:02d}. {artist} - {title} [{feat.bpm} BPM - {feat.camelot}]{matched_f.suffix.lower()}"
                dest = set_folder / clean_name
                shutil.copy2(matched_f, dest)

                # Inyectar tags ID3 / Vorbis
                try:
                    if dest.suffix.lower() == ".flac":
                        fl = FLAC(str(dest))
                        fl["TITLE"] = title
                        fl["ARTIST"] = artist
                        fl["ALBUM"] = name
                        fl["BPM"] = str(feat.bpm)
                        fl["INITIALKEY"] = feat.camelot
                        fl["KEY"] = f"{feat.camelot} ({feat.musical_key})"
                        fl["TRACKNUMBER"] = f"{idx:02d}"
                        fl.save()
                    elif dest.suffix.lower() == ".mp3":
                        try:
                            id3 = ID3(str(dest))
                        except Exception:
                            id3 = ID3()
                        id3.add(TIT2(encoding=3, text=title))
                        id3.add(TPE1(encoding=3, text=artist))
                        id3.add(TALB(encoding=3, text=name))
                        id3.add(TBPM(encoding=3, text=str(feat.bpm).split(".")[0]))
                        id3.add(TKEY(encoding=3, text=feat.camelot))
                        id3.add(TRCK(encoding=3, text=f"{idx:02d}"))
                        id3.save(str(dest))
                except Exception:
                    pass

                added_tracks.append({"position": idx, "filename": clean_name, "bpm": feat.bpm, "camelot": feat.camelot})
                cue_lines.append(f"{idx:02d}. {artist} - {title} | {feat.bpm} BPM | {feat.camelot} ({feat.musical_key})")

    # Guardar _00_SET_GUIDE.txt
    guide_path = set_folder / "_00_SET_GUIDE.txt"
    with open(guide_path, "w", encoding="utf-8") as f:
        f.write("\n".join(cue_lines) + "\n")

    return {"status": "success", "set_folder": str(set_folder), "tracks_count": len(added_tracks), "tracks": added_tracks}


async def sync_to_cloud(folder: str | None = None, remote: str = "gdrive", dry_run: bool = False) -> dict:
    """Sincroniza los sets con Google Drive u otra nube configurada en rclone."""
    config = HeraConfig.load("config/hera.toml").resolve_paths(Path("."))
    rclone = RcloneStorageAdapter(config.storage.rclone_path, config.storage.config_path)
    
    if not rclone.is_available():
        return {"status": "error", "message": "rclone no está disponible en bin/"}

    remote_name = remote if remote.endswith(":") else f"{remote}:"
    remote_dest = f"{remote_name}{folder or config.storage.remote_folder}"
    local_sets = Path(config.data_dir) / "sets"

    res = await rclone.copy(local_sets, remote_dest, dry_run=dry_run)
    return {
        "status": "success" if res.success else "error",
        "remote_destination": remote_dest,
        "dry_run": dry_run,
        "error": res.error,
    }


async def get_library_inventory() -> dict:
    """Obtiene el inventario completo de archivos reales de audio en la biblioteca."""
    base_dir = Path(".").resolve()
    library = base_dir / "library"
    files = [f for f in library.rglob("*.*") if f.is_file() and f.name != ".gitkeep" and f.stat().st_size > 1.5 * 1024 * 1024]
    
    inventory = []
    for f in sorted(files):
        inventory.append({
            "artist": f.parent.parent.name if f.parent.parent != library else f.parent.name,
            "filename": f.name,
            "size_mb": round(f.stat().st_size / (1024 * 1024), 2),
            "format": f.suffix.upper().replace(".", "")
        })
    return {"total_tracks": len(inventory), "tracks": inventory}


async def get_sets_inventory() -> dict:
    """Obtiene la lista de todas las carpetas de sets DJ existentes con sus canciones."""
    base_dir = Path(".").resolve()
    sets_dir = base_dir / "sets"
    if not sets_dir.exists():
        return {"total_sets": 0, "sets": []}

    sets_list = []
    for s in sorted(sets_dir.iterdir()):
        if s.is_dir():
            files = [f.name for f in sorted(s.glob("*.*")) if f.suffix.lower() in [".flac", ".mp3"]]
            sets_list.append({
                "set_name": s.name,
                "track_count": len(files),
                "tracks": files
            })
    return {"total_sets": len(sets_list), "sets": sets_list}
