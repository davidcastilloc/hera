"""CLI helper script for the hera-dj-set-curator skill."""

import argparse
import asyncio
import json
from pathlib import Path
import sys
import shutil
import librosa
import numpy as np
from mutagen.flac import FLAC
from mutagen.mp3 import MP3

from hera.agent.tools import search_and_acquire_tracks, create_or_update_dj_set
from hera.domain.config import HeraConfig
from hera.adapters.storage.rclone import RcloneStorageAdapter


def audit_file(f: Path) -> dict:
    """Verifica integridad acústica FFT y calcula duración."""
    try:
        y, sr = librosa.load(str(f), sr=22050, duration=20.0)
        if len(y) == 0:
            return {"file": f.name, "valid": False, "reason": "EMPTY_AUDIO"}
        flatness = float(np.mean(librosa.feature.spectral_flatness(y=y)))
        fft_vals = np.abs(np.fft.rfft(y))
        peak_ratio = float(np.max(fft_vals) / (np.sum(fft_vals) + 1e-9))
        is_synthetic = peak_ratio > 0.35 and flatness < 1e-4
    except Exception as e:
        return {"file": f.name, "valid": False, "reason": str(e)}

    dur = 0.0
    try:
        if f.suffix.lower() == ".flac":
            dur = FLAC(str(f)).info.length / 60.0
        elif f.suffix.lower() == ".mp3":
            dur = MP3(str(f)).info.length / 60.0
    except Exception:
        pass

    return {
        "file": f.name,
        "valid": not is_synthetic,
        "duration_mins": round(dur, 2),
        "size_mb": round(f.stat().st_size / (1024 * 1024), 2),
        "peak_ratio": round(peak_ratio, 4),
        "status": "SYNTHETIC_REJECTED" if is_synthetic else "AUTHENTIC_MASTER",
    }


def main():
    parser = argparse.ArgumentParser(description="HERA DJ Set Curator Helper CLI")
    subparsers = parser.add_subparsers(dest="command", required=True)

    # 1. ACQUIRE
    p_acq = subparsers.add_parser("acquire", help="Acquire tracks from Soulseek P2P")
    p_acq.add_argument("--queries", nargs="+", required=True, help="List of search queries")
    p_acq.add_argument("--output", required=True, help="Path to JSON output file")

    # 2. AUDIT
    p_aud = subparsers.add_parser("audit", help="Audit acoustic purity in a directory")
    p_aud.add_argument("--directory", default="quarantine", help="Directory to scan")
    p_aud.add_argument("--output", required=True, help="Path to JSON output file")

    # 3. BUILD
    p_bld = subparsers.add_parser("build", help="Build or update a DJ set crate")
    p_bld.add_argument("--name", required=True, help="Set name")
    p_bld.add_argument("--tracks", nargs="+", required=True, help="Track artist/title keywords")
    p_bld.add_argument("--output", required=True, help="Path to JSON output file")

    # 4. SYNC
    p_snc = subparsers.add_parser("sync", help="Push sets to Google Drive via rclone")
    p_snc.add_argument("--remote", default="gdrive:Hera_Music/sets", help="Remote destination")
    p_snc.add_argument("--output", required=True, help="Path to JSON output file")

    args = parser.parse_args()

    if args.command == "acquire":
        res = asyncio.run(search_and_acquire_tracks(args.queries))
        data = {"status": "completed", "queries": args.queries, "log": res.split("\n")}
        Path(args.output).write_text(json.dumps(data, indent=2), encoding="utf-8")
        print(f"[OK] Acquisition log written to {args.output}")

    elif args.command == "audit":
        p = Path(args.directory)
        results = [audit_file(f) for f in p.rglob("*.*") if f.is_file() and f.suffix.lower() in [".flac", ".mp3"]]
        data = {"directory": str(p), "total_scanned": len(results), "items": results}
        Path(args.output).write_text(json.dumps(data, indent=2), encoding="utf-8")
        print(f"[OK] Audit report written to {args.output}")

    elif args.command == "build":
        res = asyncio.run(create_or_update_dj_set(args.name, args.tracks))
        folder = Path("sets") / args.name
        dur = sum(audit_file(f).get("duration_mins", 0) for f in folder.glob("*.*") if f.name != "_00_SET_GUIDE.txt")
        data = {
            "set_name": args.name,
            "folder": str(folder),
            "duration_mins": round(dur, 2),
            "meets_60m_target": dur >= 60.0,
            "log": res,
        }
        Path(args.output).write_text(json.dumps(data, indent=2), encoding="utf-8")
        print(f"[OK] Set build manifest written to {args.output}")

    elif args.command == "sync":
        cfg = HeraConfig.load("config/hera.toml").resolve_paths(Path("."))
        rclone = RcloneStorageAdapter(cfg.storage.rclone_path, cfg.storage.config_path)
        res = asyncio.run(rclone.copy(Path("sets"), args.remote))
        data = {"remote": args.remote, "success": res.success, "output": res.output, "error": res.error}
        Path(args.output).write_text(json.dumps(data, indent=2), encoding="utf-8")
        print(f"[OK] Sync report written to {args.output}")


if __name__ == "__main__":
    main()
