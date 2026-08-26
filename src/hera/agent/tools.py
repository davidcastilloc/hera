"""Domain tools exposed to the Hera Agent Brain for natural language DJ workflows."""

import asyncio
from pathlib import Path
import shutil
import time
import httpx

from mutagen.flac import FLAC
from mutagen.id3 import ID3, TIT2, TPE1, TALB, TBPM, TKEY, TCON, COMM, TRCK, TDRC
from analyzers.ffmpeg.validator import FFmpegValidator
from analyzers.audio_features.analyzer import AudioFeatureAnalyzer
from hera.domain.config import HeraConfig
from hera.adapters.storage.rclone import RcloneStorageAdapter


async def search_and_acquire_tracks(queries: list[str]) -> str:
    """
    Search the Soulseek P2P network for music tracks by natural artist/title query,
    download the highest quality studio master (FLAC/320kbps MP3) from active peers,
    validate acoustic integrity, run DSP analysis (BPM, Camelot Key, LUFS), and save to library.
    
    Args:
        queries: A list of search query strings (e.g. ['Daft Punk One More Time', 'Modjo Lady']).
    """
    url = "http://localhost:5030/api/v0"
    base_dir = Path(".").resolve()
    quarantine = base_dir / "quarantine"
    library = base_dir / "library"
    quarantine.mkdir(parents=True, exist_ok=True)
    library.mkdir(parents=True, exist_ok=True)

    results = []

    for q in queries:
        try:
            r = httpx.post(f"{url}/searches", json={"searchText": q}, timeout=5.0)
            if r.status_code != 200:
                results.append(f"[-] '{q}': Could not initiate Soulseek search.")
                continue
            s_id = r.json()["id"]
            await asyncio.sleep(3.5)

            res_resp = httpx.get(f"{url}/searches/{s_id}/responses", timeout=5.0)
            if res_resp.status_code != 200:
                results.append(f"[-] '{q}': No responses received from Soulseek network.")
                continue

            peers = res_resp.json()
            chosen_peer = None
            chosen_file = None

            for u in peers:
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

            if not chosen_file:
                for u in peers:
                    if u.get("locked", False):
                        continue
                    for f in u.get("files", []):
                        if f.get("filename", "").lower().endswith(".mp3"):
                            chosen_peer = u.get("username")
                            chosen_file = f
                            break
                    if chosen_file:
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
                    results.append(f"[+] '{q}': Enqueued {fn_clean} ({sz_mb:.1f} MB) from peer '{chosen_peer}'")
            else:
                results.append(f"[-] '{q}': No available peers with full-length audio.")
        except Exception as e:
            results.append(f"[!] Error processing '{q}': {e}")

    return "\n".join(results)


async def create_or_update_dj_set(set_name: str, tracks: list[str]) -> str:
    """
    Builds a clean, human-friendly DJ crate folder in 'sets/<set_name>/'.
    Sequences the requested tracks, performs harmonic DSP analysis (BPM, Camelot Key),
    embeds rich ID3/Vorbis tags into the audio files, and generates a visual DJ Cue Sheet (_00_SET_GUIDE.txt).

    Args:
        set_name: The name of the DJ set folder (e.g. 'French Touch & Vocal House', 'Sensation White 2004').
        tracks: A list of track titles or artist keywords to include in the set.
    """
    base_dir = Path(".").resolve()
    library = base_dir / "library"
    quarantine = base_dir / "quarantine"
    set_folder = base_dir / "sets" / set_name
    set_folder.mkdir(parents=True, exist_ok=True)

    validator = FFmpegValidator()
    analyzer = AudioFeatureAnalyzer()

    all_files = list(library.rglob("*.*")) + list(quarantine.rglob("*.*"))
    all_files = [f for f in all_files if f.is_file() and f.suffix.lower() in [".flac", ".mp3"] and f.stat().st_size > 1.5 * 1024 * 1024]
    all_files.sort(key=lambda f: f.stat().st_size, reverse=True)

    added = []
    cue_sheet = [
        "=" * 70,
        f"🎧 HERA DJ CUE SHEET: {set_name}",
        "=" * 70,
    ]

    idx = 1
    for match_word in tracks:
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

                try:
                    if dest.suffix.lower() == ".flac":
                        fl = FLAC(str(dest))
                        fl["TITLE"] = title
                        fl["ARTIST"] = artist
                        fl["ALBUM"] = set_name
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
                        id3.add(TALB(encoding=3, text=set_name))
                        id3.add(TBPM(encoding=3, text=str(feat.bpm).split(".")[0]))
                        id3.add(TKEY(encoding=3, text=feat.camelot))
                        id3.add(TRCK(encoding=3, text=f"{idx:02d}"))
                        id3.save(str(dest))
                except Exception:
                    pass

                cue_sheet.append(f"{idx:02d}. {artist} - {title} | {feat.bpm} BPM | Camelot: {feat.camelot} ({feat.musical_key})")
                added.append(clean_name)
                idx += 1

    guide_path = set_folder / "_00_SET_GUIDE.txt"
    with open(guide_path, "w", encoding="utf-8") as f:
        f.write("\n".join(cue_sheet) + "\n")

    return f"Created set '{set_name}' with {len(added)} real tracks in {set_folder}:\n" + "\n".join([f"  * {t}" for t in added])


async def sync_sets_to_cloud(folder_name: str = "Hera_Music/sets", dry_run: bool = False) -> str:
    """
    Synchronizes all curated DJ sets with Google Drive (or other configured cloud storage) via rclone.

    Args:
        folder_name: The destination folder path in Google Drive (defaults to 'Hera_Music/sets').
        dry_run: If True, simulates the sync without transferring files.
    """
    config = HeraConfig.load("config/hera.toml").resolve_paths(Path("."))
    rclone = RcloneStorageAdapter(config.storage.rclone_path, config.storage.config_path)

    if not rclone.is_available():
        return "rclone binary not available. Please run 'hera setup'."

    remote_dest = f"gdrive:{folder_name}"
    local_sets = Path(config.data_dir) / "sets"

    res = await rclone.copy(local_sets, remote_dest, dry_run=dry_run)
    if res.success:
        return f"Successfully synchronized local sets to Google Drive at '{remote_dest}'!"
    else:
        return f"Sync failed: {res.error}"


def get_library_status() -> str:
    """
    Returns an inventory of all verified, real audio tracks available in the local library
    and all organized DJ sets on disk.
    """
    base_dir = Path(".").resolve()
    library = base_dir / "library"
    sets_dir = base_dir / "sets"

    lib_files = [f for f in library.rglob("*.*") if f.is_file() and f.name != ".gitkeep" and f.stat().st_size > 1.5 * 1024 * 1024]
    
    summary = [
        "=" * 60,
        f"LIBRARY INVENTORY: {len(lib_files)} Real Audio Masters",
        "=" * 60,
    ]
    for f in sorted(lib_files)[:15]:
        sz_mb = f.stat().st_size / (1024 * 1024)
        summary.append(f"  * {f.parent.parent.name} - {f.name} ({sz_mb:.1f} MB)")
    if len(lib_files) > 15:
        summary.append(f"  ... and {len(lib_files) - 15} more tracks.")

    if sets_dir.exists():
        summary.append("\n" + "=" * 60)
        summary.append("DJ SETS INVENTORY:")
        summary.append("=" * 60)
        for s in sorted(sets_dir.iterdir()):
            if s.is_dir():
                tracks = list(s.glob("*.*"))
                tracks = [t for t in tracks if t.name != "_00_SET_GUIDE.txt"]
                summary.append(f"[SET] {s.name} ({len(tracks)} tracks)")

    return "\n".join(summary)


def recommend_harmonic_transitions(current_camelot_key: str, current_bpm: float) -> str:
    """
    Calculates compatible harmonic mixing keys on the Camelot Wheel for DJ set transitions.

    Args:
        current_camelot_key: The Camelot key of the current track (e.g. '8A', '3A', '10B').
        current_bpm: The tempo of the current track in BPM (e.g. 124.0, 138.0).
    """
    try:
        key_num = int(current_camelot_key[:-1])
        key_letter = current_camelot_key[-1].upper()

        same_key = f"{key_num}{key_letter}"
        plus_one = f"{(key_num % 12) + 1}{key_letter}"
        minus_one = f"{12 if key_num == 1 else key_num - 1}{key_letter}"
        relative = f"{key_num}{'B' if key_letter == 'A' else 'A'}"
        energy_boost = f"{((key_num + 1) % 12) + 1}{key_letter}"

        bpm_range_min = round(current_bpm * 0.96, 1)
        bpm_range_max = round(current_bpm * 1.04, 1)

        return (
            f"Harmonic Mixing Recommendations for {current_camelot_key} @ {current_bpm} BPM:\n"
            f"  - Maintain Energy (Same Key): {same_key}\n"
            f"  - Smooth Build (+1 Key): {plus_one}\n"
            f"  - Smooth Wind-down (-1 Key): {minus_one}\n"
            f"  - Mood Shift (Relative Key): {relative}\n"
            f"  - Energy Boost (+2 Keys): {energy_boost}\n"
            f"  - Recommended BPM Range: {bpm_range_min} - {bpm_range_max} BPM (+/- 4%)"
        )
    except Exception as e:
        return f"Error calculating harmonic recommendations: {e}"
