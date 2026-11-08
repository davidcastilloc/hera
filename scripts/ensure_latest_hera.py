#!/usr/bin/env python3
"""
🎧 HERA — Auto-Verificador e Instalador/Actualizador Automático (Paso 0)
Verifica si Hera, el entorno Python (.venv), el binario nativo Rust (hera-node),
los binarios de soporte (slskd, rclone, ffmpeg, fpcalc) y las Skills globales del
agente están instalados y en su última versión, actualizándolos automáticamente.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


REPO_URL = "https://github.com/davidcastilloc/hera.git"


def _run_cmd(
    cmd: list[str],
    cwd: Path | None = None,
    timeout: int = 120,
) -> tuple[int, str, str]:
    try:
        proc = subprocess.run(
            cmd,
            cwd=str(cwd) if cwd else None,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
        )
        return proc.returncode, proc.stdout.strip(), proc.stderr.strip()
    except Exception as exc:
        return 1, "", str(exc)


def detect_hera_dir(explicit_dir: str | None = None) -> Path:
    if explicit_dir:
        return Path(explicit_dir).expanduser().resolve()
    env_dir = os.environ.get("HERA_DIR")
    if env_dir:
        return Path(env_dir).expanduser().resolve()
    # Check relative to this script (scripts/ensure_latest_hera.py -> repo root)
    script_root = Path(__file__).resolve().parent.parent
    if (script_root / "pyproject.toml").exists():
        return script_root
    # Common locations
    home = Path.home()
    candidates = [
        home / "Git-Repos" / "hera",
        home / "hera",
        Path.cwd(),
    ]
    for cand in candidates:
        if (cand / "pyproject.toml").exists():
            return cand.resolve()
    return (home / "hera").resolve()


def ensure_git_repo_updated(hera_dir: Path, pull_remote: bool = True) -> dict[str, Any]:
    info: dict[str, Any] = {
        "hera_dir": str(hera_dir),
        "installed": hera_dir.exists(),
        "git_present": (hera_dir / ".git").exists(),
        "updated": False,
        "local_commit": None,
        "remote_commit": None,
        "dirty_worktree_preserved": False,
        "message": "",
    }

    if not hera_dir.exists() or not (hera_dir / "pyproject.toml").exists():
        hera_dir.parent.mkdir(parents=True, exist_ok=True)
        rc, out, err = _run_cmd(["git", "clone", REPO_URL, str(hera_dir)], timeout=180)
        if rc != 0:
            info["message"] = f"Error clonando repositorio: {err or out}"
            return info
        info["installed"] = True
        info["git_present"] = True
        info["updated"] = True

    rc, local_rev, _ = _run_cmd(["git", "rev-parse", "--short", "HEAD"], cwd=hera_dir, timeout=15)
    if rc == 0:
        info["local_commit"] = local_rev

    # Check if working tree has local modifications (preserve them untouched!)
    rc_stat, stat_out, _ = _run_cmd(["git", "status", "--porcelain"], cwd=hera_dir, timeout=15)
    has_local_edits = bool(stat_out.strip()) if rc_stat == 0 else False
    info["dirty_worktree_preserved"] = has_local_edits

    if not pull_remote or not info["git_present"]:
        info["message"] = "Verificación local completada (sin fetch remoto)."
        return info

    rc_fetch, _, fetch_err = _run_cmd(
        ["git", "fetch", "origin", "main", "--quiet"], cwd=hera_dir, timeout=30
    )
    if rc_fetch != 0:
        info["message"] = f"Modo offline o remoto no alcanzable ({fetch_err}); usando versión local {local_rev}."
        return info

    _, local_full, _ = _run_cmd(["git", "rev-parse", "HEAD"], cwd=hera_dir, timeout=10)
    rc_rem, remote_full, _ = _run_cmd(["git", "rev-parse", "origin/main"], cwd=hera_dir, timeout=10)
    if rc_rem == 0 and remote_full:
        info["remote_commit"] = remote_full[:7]

    if not remote_full or local_full == remote_full:
        info["message"] = f"Repositorio Git ya está en la última versión ({info['local_commit']})."
        return info

    # Check if local is ahead of origin/main
    rc_anc, _, _ = _run_cmd(
        ["git", "merge-base", "--is-ancestor", "origin/main", "HEAD"],
        cwd=hera_dir,
        timeout=10,
    )
    if rc_anc == 0:
        info["message"] = f"Repositorio local ({info['local_commit']}) incluye commits más recientes que origin/main ({info['remote_commit']})."
        return info

    # Need to pull updates from origin/main
    stashed = False
    if has_local_edits:
        rc_stash, stash_out, _ = _run_cmd(
            ["git", "stash", "push", "-u", "-m", "hera-auto-updater-safeguard"],
            cwd=hera_dir,
            timeout=30,
        )
        stashed = rc_stash == 0 and "No local changes" not in stash_out

    rc_pull, pull_out, pull_err = _run_cmd(
        ["git", "pull", "--rebase", "origin", "main", "--quiet"],
        cwd=hera_dir,
        timeout=60,
    )
    if stashed:
        _run_cmd(["git", "stash", "pop"], cwd=hera_dir, timeout=30)

    _, new_rev, _ = _run_cmd(["git", "rev-parse", "--short", "HEAD"], cwd=hera_dir, timeout=10)
    info["local_commit"] = new_rev
    if rc_pull == 0:
        info["updated"] = True
        info["message"] = f"Actualizado exitosamente a commit {new_rev}."
    else:
        info["message"] = f"Advertencia en git pull: {pull_err or pull_out}"

    return info


def ensure_rust_binary_updated(hera_dir: Path, auto_compile: bool = True) -> dict[str, Any]:
    is_win = platform.system().lower().startswith("win")
    exe_name = "hera-node.exe" if is_win else "hera-node"
    hera_rs_dir = hera_dir / "hera-rs"
    release_bin = hera_rs_dir / "target" / "release" / exe_name
    bin_dir = hera_dir / "bin"
    bin_copy = bin_dir / exe_name

    result: dict[str, Any] = {
        "binary_name": exe_name,
        "release_binary_path": str(release_bin),
        "bin_path": str(bin_copy),
        "exists": release_bin.exists(),
        "needs_rebuild": False,
        "rebuilt": False,
        "version": None,
        "message": "",
    }

    if not hera_rs_dir.exists():
        result["message"] = "Directorio hera-rs no encontrado."
        return result

    # Compare mtime of release binary vs all Rust source files & Cargo.toml
    newest_src_mtime = 0.0
    for src_file in list((hera_rs_dir / "src").rglob("*.rs")) + [hera_rs_dir / "Cargo.toml"]:
        if src_file.exists():
            newest_src_mtime = max(newest_src_mtime, src_file.stat().st_mtime)

    bin_mtime = release_bin.stat().st_mtime if release_bin.exists() else 0.0
    needs_rebuild = (not release_bin.exists()) or (newest_src_mtime > bin_mtime + 1.0)
    result["needs_rebuild"] = needs_rebuild

    if needs_rebuild and auto_compile:
        cargo_bin = shutil.which("cargo")
        if not cargo_bin:
            home_cargo = Path.home() / ".cargo" / "bin" / ("cargo.exe" if is_win else "cargo")
            if home_cargo.exists():
                cargo_bin = str(home_cargo)

        if cargo_bin:
            rc, out, err = _run_cmd(
                [cargo_bin, "build", "--release", "--manifest-path", str(hera_rs_dir / "Cargo.toml")],
                cwd=hera_rs_dir,
                timeout=300,
            )
            if rc == 0 and release_bin.exists():
                result["rebuilt"] = True
                result["exists"] = True
                result["needs_rebuild"] = False
                result["message"] = "Binario Rust hera-node compilado en modo release con las últimas mejoras."
            else:
                result["message"] = f"Error compilando hera-rs: {err[-500:] if err else out[-500:]}"
        else:
            result["message"] = "Cargo no detectado en PATH; usando binario preexistente."
    else:
        result["message"] = "Binario Rust hera-node ya está compilado y al día con el código fuente."

    # Always sync release binary into hera_dir/bin/hera-node(.exe)
    if release_bin.exists():
        bin_dir.mkdir(parents=True, exist_ok=True)
        try:
            if (not bin_copy.exists()) or (
                release_bin.stat().st_mtime > bin_copy.stat().st_mtime + 0.5
                or release_bin.stat().st_size != bin_copy.stat().st_size
            ):
                shutil.copy2(release_bin, bin_copy)
        except OSError:
            # Might be in use by a running process; ignore copy error
            pass

    # Read version from Cargo.toml
    cargo_toml = hera_rs_dir / "Cargo.toml"
    if cargo_toml.exists():
        for line in cargo_toml.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.strip().startswith("version"):
                result["version"] = line.split("=", 1)[1].strip().strip('"').strip("'")
                break

    return result


def ensure_python_env_updated(hera_dir: Path) -> dict[str, Any]:
    is_win = platform.system().lower().startswith("win")
    venv_dir = hera_dir / ".venv"
    py_bin = venv_dir / ("Scripts/python.exe" if is_win else "bin/python")
    hera_cli = venv_dir / ("Scripts/hera.exe" if is_win else "bin/hera")

    res: dict[str, Any] = {
        "venv_path": str(venv_dir),
        "python_bin": str(py_bin),
        "hera_cli_bin": str(hera_cli),
        "venv_ok": py_bin.exists() and hera_cli.exists(),
        "updated": False,
        "message": "",
    }

    uv_bin = shutil.which("uv")
    if not res["venv_ok"] and uv_bin:
        _run_cmd([uv_bin, "venv", str(venv_dir)], cwd=hera_dir, timeout=60)
        rc, _, err = _run_cmd([uv_bin, "pip", "install", "-e", ".", "--quiet"], cwd=hera_dir, timeout=180)
        res["venv_ok"] = py_bin.exists()
        res["updated"] = rc == 0
        res["message"] = "Entorno Python .venv instalado/actualizado." if rc == 0 else f"Error uv pip: {err}"
    else:
        res["message"] = "Entorno Python .venv y CLI hera verificados."

    return res


def check_companion_binaries(hera_dir: Path) -> dict[str, Any]:
    is_win = platform.system().lower().startswith("win")
    bin_dir = hera_dir / "bin"
    companions = ["slskd", "rclone", "ffmpeg", "fpcalc"]
    status: dict[str, Any] = {}

    for name in companions:
        exe = f"{name}.exe" if is_win else name
        local_candidate = bin_dir / exe
        sys_candidate = shutil.which(name)
        resolved = str(local_candidate) if local_candidate.exists() else sys_candidate
        status[name] = {
            "installed": bool(resolved),
            "path": resolved,
        }
    return status


def _hash_file(path: Path) -> str:
    if not path.exists() or not path.is_file():
        return ""
    h = hashlib.sha256()
    h.update(path.read_bytes())
    return h.hexdigest()


def sync_global_skills(hera_dir: Path) -> dict[str, Any]:
    """
    Sincroniza automáticamente skills/hera y skills/hera-dj-set-curator desde el
    repositorio hacia los directorios globales de skills del agente (.gemini, .claude, .agents).
    """
    src_skills_root = hera_dir / "skills"
    if not src_skills_root.exists():
        return {"synced": False, "reason": "No existe carpeta skills/ en el repositorio."}

    home = Path.home()
    target_roots: list[Path] = [
        hera_dir / ".agent" / "skills",
        hera_dir / ".claude" / "skills",
    ]
    for candidate_root in [
        home / ".gemini" / "config" / "skills",
        home / ".claude" / "skills",
        home / ".agents" / "skills",
    ]:
        if candidate_root.exists() or candidate_root.parent.exists():
            target_roots.append(candidate_root)

    synced_targets: list[dict[str, Any]] = []
    skill_names = ["hera", "hera-dj-set-curator"]

    for skill_name in skill_names:
        src_skill = src_skills_root / skill_name
        if not src_skill.exists():
            continue

        for root in target_roots:
            dst_skill = root / skill_name
            files_updated = 0
            files_checked = 0
            for src_file in src_skill.rglob("*"):
                if src_file.is_dir() or "__pycache__" in src_file.parts:
                    continue
                rel = src_file.relative_to(src_skill)
                dst_file = dst_skill / rel
                files_checked += 1
                if _hash_file(src_file) != _hash_file(dst_file):
                    dst_file.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(src_file, dst_file)
                    files_updated += 1

            synced_targets.append(
                {
                    "skill": skill_name,
                    "destination": str(dst_skill),
                    "files_checked": files_checked,
                    "files_updated": files_updated,
                }
            )

    return {
        "synced": True,
        "targets": synced_targets,
    }


def run_ensure_latest_hera(
    hera_dir: Path | None = None,
    pull_remote: bool = True,
    auto_compile_rust: bool = True,
    sync_skills: bool = True,
) -> dict[str, Any]:
    resolved_dir = detect_hera_dir(str(hera_dir) if hera_dir else None)
    git_report = ensure_git_repo_updated(resolved_dir, pull_remote=pull_remote)
    rust_report = ensure_rust_binary_updated(resolved_dir, auto_compile=auto_compile_rust)
    py_report = ensure_python_env_updated(resolved_dir)
    bins_report = check_companion_binaries(resolved_dir)
    skills_report = sync_global_skills(resolved_dir) if sync_skills else {"synced": False}

    return {
        "status": "ok",
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "hera_dir": str(resolved_dir),
        "git": git_report,
        "rust_binary": rust_report,
        "python_env": py_report,
        "companion_binaries": bins_report,
        "skills_sync": skills_report,
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Verifica si Hera, sus binarios y sus skills están instalados y actualizados a la última versión."
    )
    parser.add_argument("--hera-dir", type=str, default=None, help="Ruta al directorio de Hera")
    parser.add_argument("--no-pull", action="store_true", help="Omitir git fetch/pull remoto")
    parser.add_argument("--no-rust-build", action="store_true", help="Omitir compilación automática de hera-rs")
    parser.add_argument("--no-skill-sync", action="store_true", help="Omitir sincronización de skills globales")
    parser.add_argument("--json", action="store_true", help="Imprimir únicamente JSON estructurado")
    args = parser.parse_args()

    report = run_ensure_latest_hera(
        hera_dir=Path(args.hera_dir) if args.hera_dir else None,
        pull_remote=not args.no_pull,
        auto_compile_rust=not args.no_rust_build,
        sync_skills=not args.no_skill_sync,
    )

    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        print("================================================================")
        print(" 🎧 HERA — AUTO-VERIFICACIÓN Y ACTUALIZACIÓN AUTOMÁTICA (PASO 0)")
        print("================================================================")
        print(f"📍 Directorio Hera : {report['hera_dir']}")
        print(f"🔄 Git Commit      : {report['git']['local_commit']} ({report['git']['message']})")
        print(
            f"🦀 Binario Rust    : {report['rust_binary']['binary_name']} v{report['rust_binary']['version']} "
            f"({'RECOMPILADO' if report['rust_binary']['rebuilt'] else 'AL DÍA'}) -> {report['rust_binary']['release_binary_path']}"
        )
        print(f"🐍 Entorno Python  : {'OK' if report['python_env']['venv_ok'] else 'PENDIENTE'} ({report['python_env']['hera_cli_bin']})")
        if report["skills_sync"].get("synced"):
            for t in report["skills_sync"].get("targets", []):
                print(
                    f"🧠 Skill Sync      : {t['skill']} -> {t['destination']} "
                    f"({t['files_updated']} archivos actualizados / {t['files_checked']} verificados)"
                )
        print("================================================================")
        print(json.dumps(report, indent=2, ensure_ascii=False))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
