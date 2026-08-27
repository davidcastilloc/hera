"""Caso de uso: Sincronización de DJ Sets y música con nubes (Google Drive, R2, S3) vía rclone."""

from pathlib import Path
from dataclasses import dataclass
from hera.domain.config import HeraConfig
from hera.adapters.storage.rclone import RcloneStorageAdapter, SyncResult


@dataclass
class SyncCloudResult:
    success: bool
    message: str
    transferred_files: int = 0
    error: str | None = None


class SyncCloudUseCase:
    """Orquesta la sincronización bidireccional (Push / Pull) de sets locales con remotes en la nube."""

    def __init__(self, config: HeraConfig):
        self.config = config
        self.rclone = RcloneStorageAdapter(config.storage.rclone_path, config.storage.config_path)

    async def push_sets(self, remote: str | None = None, folder: str | None = None, dry_run: bool = False) -> SyncCloudResult:
        if not self.rclone.is_available():
            return SyncCloudResult(
                success=False,
                message="rclone no está instalado o no disponible.",
                error="RCLONE_NOT_FOUND",
            )

        remote_name = remote or self.config.storage.default_remote
        if not remote_name.endswith(":"):
            remote_name += ":"
        remote_dest = f"{remote_name}{folder or self.config.storage.remote_folder}"

        local_sets_dir = Path(self.config.data_dir) / "sets"
        local_sets_dir.mkdir(parents=True, exist_ok=True)

        res: SyncResult = await self.rclone.copy(local_sets_dir, remote_dest, dry_run=dry_run)
        if res.success:
            return SyncCloudResult(
                success=True,
                message=f"Sincronización exitosa hacia '{remote_dest}' ({res.transferred_files} archivos).",
                transferred_files=res.transferred_files,
            )
        return SyncCloudResult(
            success=False,
            message=f"Error en la sincronización hacia '{remote_dest}'.",
            error=res.error,
        )

    async def pull_sets(self, remote: str | None = None, folder: str | None = None, dry_run: bool = False) -> SyncCloudResult:
        if not self.rclone.is_available():
            return SyncCloudResult(
                success=False,
                message="rclone no está instalado o no disponible.",
                error="RCLONE_NOT_FOUND",
            )

        remote_name = remote or self.config.storage.default_remote
        if not remote_name.endswith(":"):
            remote_name += ":"
        remote_src = f"{remote_name}{folder or self.config.storage.remote_folder}"

        local_sets_dir = Path(self.config.data_dir) / "sets"
        local_sets_dir.mkdir(parents=True, exist_ok=True)

        res: SyncResult = await self.rclone.copy(remote_src, local_sets_dir, dry_run=dry_run)
        if res.success:
            return SyncCloudResult(
                success=True,
                message=f"Descarga exitosa desde '{remote_src}' ({res.transferred_files} archivos).",
                transferred_files=res.transferred_files,
            )
        return SyncCloudResult(
            success=False,
            message=f"Error en la descarga desde '{remote_src}'.",
            error=res.error,
        )
