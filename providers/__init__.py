"""Proveedores de música para Hera y Registro Federado."""

import asyncio
from typing import Sequence
from hera.contracts.candidate import Candidate
from hera.contracts.provider import Provider
from hera.contracts.search import SearchFilters
from hera.domain.config import HeraConfig


class ProviderRegistry:
    """Registro dinámico y motor de búsqueda federada multi-proveedor."""

    def __init__(self):
        self._providers: dict[str, Provider] = {}

    def register(self, name: str, provider: Provider) -> None:
        """Registra una instancia de Provider."""
        self._providers[name.lower()] = provider

    def unregister(self, name: str) -> None:
        """Elimina un provider registrado."""
        self._providers.pop(name.lower(), None)

    def get(self, name: str) -> Provider | None:
        """Obtiene un provider por nombre."""
        return self._providers.get(name.lower())

    def list_available(self) -> list[str]:
        """Retorna la lista de nombres de proveedores registrados."""
        return list(self._providers.keys())

    @classmethod
    def from_config(cls, config: HeraConfig) -> "ProviderRegistry":
        """Instancia e inicializa los providers activos configurados en HeraConfig."""
        registry = cls()

        # 1. Slskd Provider (Soulseek P2P)
        if config.providers.slskd_url:
            try:
                from providers.slskd.client import SlskdProvider
                registry.register("slskd", SlskdProvider(config.providers.slskd_url))
            except Exception:
                pass

        # 2. Prowlarr Provider (BitTorrent / Usenet Trackers)
        if getattr(config.providers, "prowlarr_enabled", False):
            try:
                from providers.prowlarr.client import ProwlarrProvider
                registry.register(
                    "prowlarr",
                    ProwlarrProvider(
                        base_url=getattr(config.providers, "prowlarr_url", "http://localhost:9696"),
                        api_key=getattr(config.providers, "prowlarr_api_key", None),
                        qbittorrent_url=getattr(config.providers, "qbittorrent_url", "http://localhost:8080"),
                        qbittorrent_user=getattr(config.providers, "qbittorrent_user", "admin"),
                        qbittorrent_pass=getattr(config.providers, "qbittorrent_pass", None),
                    ),
                )
            except Exception:
                pass

        # 3. yt-dlp Provider (YouTube, SoundCloud, etc.)
        if getattr(config.providers, "ytdlp_enabled", False):
            try:
                from providers.ytdlp.client import YtdlpProvider
                registry.register(
                    "ytdlp",
                    YtdlpProvider(
                        max_results=getattr(config.providers, "ytdlp_max_results", 10),
                        preferred_quality=getattr(config.providers, "ytdlp_preferred_quality", "320"),
                    ),
                )
            except Exception:
                pass

        # 4. Lidarr Provider (Gestión automatizada & MusicBrainz)
        if getattr(config.providers, "lidarr_enabled", False):
            try:
                from providers.lidarr.client import LidarrProvider
                registry.register(
                    "lidarr",
                    LidarrProvider(
                        base_url=getattr(config.providers, "lidarr_url", "http://localhost:8686"),
                        api_key=getattr(config.providers, "lidarr_api_key", None),
                    ),
                )
            except Exception:
                pass

        
        # 5. Local Provider (Carpetas locales)
        if config.providers.local_folders:
            try:
                from providers.local.scanner import LocalProvider
                registry.register(
                    "local",
                    LocalProvider(folders=config.providers.local_folders)
                )
            except Exception as e:
                pass

        # 7. Internet Archive Provider (Dominio público)
        if getattr(config.providers, "archive_enabled", False):
            try:
                from providers.archive.client import ArchiveProvider
                registry.register(
                    "archive",
                    ArchiveProvider(
                        collections=getattr(config.providers, "archive_collections", ["audio"]),
                    ),
                )
            except Exception:
                pass

        return registry

    async def search_all(
        self,
        query: str,
        filters: SearchFilters | None = None,
        requested_providers: Sequence[str] | None = None,
        timeout_seconds: float = 15.0,
    ) -> tuple[list[Candidate], list[str], list[str]]:
        """
        Ejecuta la búsqueda en paralelo sobre los proveedores solicitados (o todos los registrados).
        Retorna (todos_los_candidatos, providers_completados, providers_fallidos).
        """
        # Orden de prioridad: slskd (Soulseek FLAC) -> prowlarr (Torrents) -> archive -> ytdlp (fallback)
        priority_order = ["local", "slskd", "prowlarr", "lidarr", "archive", "ytdlp"]
        if requested_providers:
            target_names = [p.lower() for p in requested_providers]
        else:
            target_names = [p for p in priority_order if p in self._providers]
            # Agregar otros si existen
            target_names.extend([p for p in self.list_available() if p not in target_names])
        tasks = []
        names = []

        for name in target_names:
            prov = self.get(name)
            if prov:
                names.append(name)
                tasks.append(asyncio.wait_for(prov.search(query, filters), timeout=timeout_seconds))

        if not tasks:
            return [], [], []

        results = await asyncio.gather(*tasks, return_exceptions=True)

        completed: list[str] = []
        failed: list[str] = []
        all_candidates: list[Candidate] = []

        for name, res in zip(names, results):
            if isinstance(res, Exception):
                failed.append(name)
            else:
                completed.append(name)
                if isinstance(res, list):
                    all_candidates.extend(res)

        return all_candidates, completed, failed
