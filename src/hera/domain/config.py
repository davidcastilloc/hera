"""Gestión de configuración basada en TOML para Hera."""

from pathlib import Path
import os
import tomllib
import tomli_w
from pydantic import BaseModel, Field


class ProvidersConfig(BaseModel):
    local_folders: list[str] = Field(default_factory=list, description="Carpetas locales a escanear")
    slskd_url: str | None = Field(default=None, description="URL base de slskd (e.g. http://localhost:5030)")
    slskd_api_key_env: str = Field(default="HERA_SLSKD_API_KEY", description="Nombre de variable de entorno con API Key de slskd")
    prowlarr_enabled: bool = False
    qbittorrent_enabled: bool = False


class AnalysisConfig(BaseModel):
    ffmpeg_path: str = "ffmpeg"
    ffprobe_path: str = "ffprobe"
    fpcalc_path: str = "fpcalc"
    acoustid_api_key_env: str = "HERA_ACOUSTID_API_KEY"
    acoustid_enabled: bool = False
    supported_formats: list[str] = Field(
        default_factory=lambda: ["flac", "alac", "mp3", "aac", "ogg", "opus", "wav", "aiff"]
    )


class StorageConfig(BaseModel):
    rclone_path: str = "bin/rclone.exe"
    config_path: str | None = None
    default_remote: str = "gdrive"
    remote_folder: str = "Hera_Music/sets"
    auto_sync: bool = False


class PolicyConfig(BaseModel):
    require_approval: bool = True
    allowed_bases: list[str] = Field(
        default_factory=lambda: [
            "owned_original",
            "purchased_copy",
            "open_license",
            "public_domain",
            "creator_permission",
            "authorized_pool",
            "other_documented_basis",
        ]
    )
    max_concurrent_downloads: int = 3
    max_file_size_mb: int = 500


class HeraConfig(BaseModel):
    data_dir: str = Field(default=".", description="Directorio raíz de Hera")
    quarantine_dir: str = Field(default="quarantine")
    library_dir: str = Field(default="library")
    exports_dir: str = Field(default="exports")
    logs_dir: str = Field(default="logs")
    db_path: str = Field(default="hera.db")

    organize_template: str = "{Artist}/{Year} - {Release}/{TrackNo} - {Title} [{Version}].{ext}"
    collision_policy: str = "review"  # review | suffix | skip
    identity_confidence_threshold: float = 0.85
    dedup_duration_tolerance_ms: int = 3000

    providers: ProvidersConfig = Field(default_factory=ProvidersConfig)
    analysis: AnalysisConfig = Field(default_factory=AnalysisConfig)
    storage: StorageConfig = Field(default_factory=StorageConfig)
    policy: PolicyConfig = Field(default_factory=PolicyConfig)

    def resolve_paths(self, base_path: Path | None = None) -> "HeraConfig":
        """Resuelve rutas relativas respecto a data_dir o base_path."""
        base = base_path or Path(self.data_dir).resolve()
        return HeraConfig(
            data_dir=str(base),
            quarantine_dir=str(base / self.quarantine_dir if not Path(self.quarantine_dir).is_absolute() else self.quarantine_dir),
            library_dir=str(base / self.library_dir if not Path(self.library_dir).is_absolute() else self.library_dir),
            exports_dir=str(base / self.exports_dir if not Path(self.exports_dir).is_absolute() else self.exports_dir),
            logs_dir=str(base / self.logs_dir if not Path(self.logs_dir).is_absolute() else self.logs_dir),
            db_path=str(base / self.db_path if not Path(self.db_path).is_absolute() else self.db_path),
            organize_template=self.organize_template,
            collision_policy=self.collision_policy,
            identity_confidence_threshold=self.identity_confidence_threshold,
            dedup_duration_tolerance_ms=self.dedup_duration_tolerance_ms,
            providers=self.providers,
            analysis=self.analysis,
            storage=self.storage,
            policy=self.policy,
        )

    @classmethod
    def load(cls, config_path: Path | str) -> "HeraConfig":
        path = Path(config_path)
        if not path.exists():
            return cls()
        with open(path, "rb") as f:
            data = tomllib.load(f)
        return cls(**data)

    def save(self, config_path: Path | str) -> None:
        path = Path(config_path)
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "wb") as f:
            tomli_w.dump(self.model_dump(), f)
