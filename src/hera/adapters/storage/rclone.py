"""Adaptador de almacenamiento en la nube multiplataforma basado en rclone."""

import asyncio
from dataclasses import dataclass
import os
from pathlib import Path
import platform
import shutil
import subprocess


@dataclass
class SyncResult:
    success: bool
    transferred_files: int
    transferred_bytes: int
    output: str
    error: str | None = None


class RcloneStorageAdapter:
    """Gestiona la sincronización con Google Drive, S3, R2, Dropbox, etc. mediante rclone."""

    def __init__(self, rclone_path: str = "bin/rclone.exe", config_path: str | None = None):
        self.rclone_path = self._resolve_binary(rclone_path)
        self.config_path = config_path

    @staticmethod
    def _resolve_binary(path_str: str) -> str:
        """Resuelve la ruta del binario rclone según el sistema operativo."""
        # 1. PATH del sistema
        which_path = shutil.which("rclone")
        if which_path:
            return which_path

        # 2. Ruta local en bin/
        p = Path(path_str)
        if p.exists():
            return str(p.resolve())

        # 3. Intentar con o sin .exe según OS
        if platform.system() == "Windows" and not path_str.endswith(".exe"):
            p_win = Path(f"{path_str}.exe")
            if p_win.exists():
                return str(p_win.resolve())
        elif platform.system() != "Windows" and path_str.endswith(".exe"):
            p_nix = Path(path_str[:-4])
            if p_nix.exists():
                return str(p_nix.resolve())

        return path_str

    def is_available(self) -> bool:
        """Comprueba si el binario de rclone está disponible y es ejecutable."""
        try:
            res = subprocess.run([self.rclone_path, "version"], capture_output=True, text=True, timeout=5)
            return res.returncode == 0
        except Exception:
            return False

    def get_version(self) -> str | None:
        """Obtiene la versión instalada de rclone."""
        try:
            res = subprocess.run([self.rclone_path, "version"], capture_output=True, text=True, timeout=5)
            if res.returncode == 0:
                return res.stdout.splitlines()[0].strip()
        except Exception:
            pass
        return None

    def list_remotes(self) -> list[str]:
        """Devuelve la lista de servicios remotos configurados (e.g. ['gdrive:', 'r2:'])."""
        cmd = [self.rclone_path, "listremotes"]
        if self.config_path:
            cmd.extend(["--config", self.config_path])
        try:
            res = subprocess.run(cmd, capture_output=True, text=True, timeout=5)
            if res.returncode == 0:
                return [line.strip() for line in res.stdout.splitlines() if line.strip()]
        except Exception:
            pass
        return []

    async def sync(
        self,
        source: str | Path,
        destination: str,
        dry_run: bool = False,
        extra_args: list[str] | None = None,
    ) -> SyncResult:
        """Ejecuta una sincronización unidireccional (sync) entre origen y destino."""
        cmd = [self.rclone_path, "sync", str(source), destination, "--stats=1s", "--stats-one-line"]
        if dry_run:
            cmd.append("--dry-run")
        if self.config_path:
            cmd.extend(["--config", self.config_path])
        if extra_args:
            cmd.extend(extra_args)

        try:
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await proc.communicate()
            out_str = stdout.decode("utf-8", errors="replace")
            err_str = stderr.decode("utf-8", errors="replace")

            return SyncResult(
                success=(proc.returncode == 0),
                transferred_files=0,
                transferred_bytes=0,
                output=out_str,
                error=err_str if proc.returncode != 0 else None,
            )
        except Exception as e:
            return SyncResult(
                success=False,
                transferred_files=0,
                transferred_bytes=0,
                output="",
                error=str(e),
            )

    async def copy(
        self,
        source: str | Path,
        destination: str,
        dry_run: bool = False,
        extra_args: list[str] | None = None,
    ) -> SyncResult:
        """Copia archivos recursivamente sin borrar archivos en el destino."""
        cmd = [self.rclone_path, "copy", str(source), destination, "--stats=1s", "--stats-one-line"]
        if dry_run:
            cmd.append("--dry-run")
        if self.config_path:
            cmd.extend(["--config", self.config_path])
        if extra_args:
            cmd.extend(extra_args)

        try:
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await proc.communicate()
            out_str = stdout.decode("utf-8", errors="replace")
            err_str = stderr.decode("utf-8", errors="replace")

            return SyncResult(
                success=(proc.returncode == 0),
                transferred_files=0,
                transferred_bytes=0,
                output=out_str,
                error=err_str if proc.returncode != 0 else None,
            )
        except Exception as e:
            return SyncResult(
                success=False,
                transferred_files=0,
                transferred_bytes=0,
                output="",
                error=str(e),
            )
