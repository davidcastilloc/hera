"""Gestor del ciclo de vida y orquestación automática de demonios y servicios externos (slskd)."""

import asyncio
from pathlib import Path
import platform
import subprocess
import time
import httpx
import structlog

from hera.domain.config import HeraConfig

logger = structlog.get_logger(__name__)


class SlskdLifecycle:
    """Administra el arranque, monitoreo y detención limpia de slskd en segundo plano."""

    def __init__(self, config: HeraConfig | None = None):
        self.config = config or HeraConfig()
        self._process: subprocess.Popen | None = None
        self._log_file = None

    @staticmethod
    def is_running_sync(base_url: str = "http://localhost:5030", timeout: float = 1.5) -> bool:
        """Comprueba de forma sincrónica si la API de slskd responde en base_url."""
        url = base_url.rstrip("/") + "/api/v0/application"
        try:
            with httpx.Client(timeout=timeout) as client:
                resp = client.get(url)
                return resp.status_code == 200
        except Exception:
            return False

    @staticmethod
    async def is_running_async(base_url: str = "http://localhost:5030", timeout: float = 1.5) -> bool:
        """Comprueba de forma asíncrona si la API de slskd responde en base_url."""
        url = base_url.rstrip("/") + "/api/v0/application"
        try:
            async with httpx.AsyncClient(timeout=timeout) as client:
                resp = await client.get(url)
                return resp.status_code == 200
        except Exception:
            return False

    def find_binary(self, base_dir: Path | None = None) -> Path | None:
        """Busca el binario de slskd según el sistema operativo."""
        root = base_dir or Path(self.config.data_dir).resolve()
        is_win = platform.system().lower() == "windows"
        bin_name = "slskd.exe" if is_win else "slskd"

        candidate_paths = [
            root / "bin" / bin_name,
            Path("bin") / bin_name,
            Path.cwd() / "bin" / bin_name,
        ]
        for p in candidate_paths:
            if p.exists() and p.is_file():
                return p.resolve()
        return None

    def start_background(self, base_dir: Path | None = None) -> bool:
        """Inicia slskd en segundo plano desacoplado de la terminal."""
        binary = self.find_binary(base_dir)
        if not binary:
            return False

        root = base_dir or Path(self.config.data_dir).resolve()
        logs_dir = Path(self.config.logs_dir)
        if not logs_dir.is_absolute():
            logs_dir = root / logs_dir
        logs_dir.mkdir(parents=True, exist_ok=True)
        log_path = logs_dir / "slskd.log"

        self._log_file = open(log_path, "a", encoding="utf-8")

        flags = 0
        if platform.system().lower() == "windows":
            flags = 0x00000008 | 0x00000200

        try:
            self._process = subprocess.Popen(
                [str(binary)],
                cwd=str(binary.parent),
                stdout=self._log_file,
                stderr=subprocess.STDOUT,
                creationflags=flags,
            )
            return True
        except Exception as e:
            logger.warning("Error iniciando subproceso slskd", error=str(e))
            if self._log_file:
                self._log_file.close()
                self._log_file = None
            return False

    async def ensure_running(
        self,
        base_dir: Path | None = None,
        timeout_sec: float = 8.0,
    ) -> bool:
        """Garantiza que slskd esté en ejecución, iniciándolo automáticamente si es necesario."""
        url = self.config.providers.slskd_url or "http://localhost:5030"

        # 1. Comprobar si ya está activo
        if await self.is_running_async(url, timeout=1.0):
            return True

        # 2. Iniciar en segundo plano
        started = self.start_background(base_dir)
        if not started:
            return False

        # 3. Esperar a que el health check responda
        start_time = time.monotonic()
        while time.monotonic() - start_time < timeout_sec:
            await asyncio.sleep(0.4)
            if await self.is_running_async(url, timeout=1.0):
                return True

        return False

    def ensure_running_sync(
        self,
        base_dir: Path | None = None,
        timeout_sec: float = 8.0,
    ) -> bool:
        """Versión sincrónica de ensure_running."""
        url = self.config.providers.slskd_url or "http://localhost:5030"

        if self.is_running_sync(url, timeout=1.0):
            return True

        started = self.start_background(base_dir)
        if not started:
            return False

        start_time = time.monotonic()
        while time.monotonic() - start_time < timeout_sec:
            time.sleep(0.4)
            if self.is_running_sync(url, timeout=1.0):
                return True

        return False

    def stop(self) -> None:
        """Detiene el proceso hijo si fue iniciado por esta instancia."""
        if self._process:
            try:
                self._process.terminate()
                self._process.wait(timeout=3.0)
            except Exception:
                try:
                    self._process.kill()
                except Exception:
                    pass
            finally:
                self._process = None

        if self._log_file:
            try:
                self._log_file.close()
            except Exception:
                pass
            self._log_file = None

