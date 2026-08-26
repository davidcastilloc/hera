"""Cálculo de métricas de contribución a la comunidad P2P — Buen Ciudadano Soulseek."""

from pathlib import Path
import httpx
import os


class CommunityStats:
    """Calcula y formatea el impacto colaborativo del usuario en la red Soulseek."""

    def __init__(self, base_url: str = "http://localhost:5030", api_key: str | None = None):
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key or os.environ.get("HERA_SLSKD_API_KEY", "")

    def _headers(self) -> dict[str, str]:
        headers = {"Content-Type": "application/json"}
        if self.api_key:
            headers["X-API-Key"] = self.api_key
        return headers

    async def get_sharing_summary(
        self,
        library_dir: Path | str,
        sets_dir: Path | str | None = None,
    ) -> dict:
        """Calcula el inventario compartido local y las métricas de transferencia desde slskd."""
        lib_path = Path(library_dir)
        sets_path = Path(sets_dir) if sets_dir else None

        valid_exts = {".flac", ".mp3", ".wav", ".aif", ".aiff", ".m4a", ".alac", ".ogg"}

        # 1. Contar archivos locales compartidos
        shared_files = []
        if lib_path.exists():
            for f in lib_path.rglob("*.*"):
                if f.is_file() and f.suffix.lower() in valid_exts and f.name != ".gitkeep":
                    shared_files.append(f)

        sets_files = []
        if sets_path and sets_path.exists():
            for f in sets_path.rglob("*.*"):
                if f.is_file() and f.suffix.lower() in valid_exts and f.name != ".gitkeep":
                    sets_files.append(f)

        all_shared = list({f.resolve(): f for f in (shared_files + sets_files)}.values())
        total_bytes = sum(f.stat().st_size for f in all_shared if f.exists())
        total_gb = total_bytes / (1024 ** 3)

        # 2. Consultar slskd API para métricas en vivo
        uploads_count = 0
        uploads_bytes = 0
        peers_served = set()
        is_live = False

        try:
            async with httpx.AsyncClient(timeout=2.0) as client:
                health_resp = await client.get(f"{self.base_url}/api/v0/application", headers=self._headers())
                if health_resp.status_code == 200:
                    is_live = True

                    up_resp = await client.get(f"{self.base_url}/api/v0/transfers/uploads/all/completed", headers=self._headers())
                    if up_resp.status_code == 200:
                        uploads_data = up_resp.json()
                        if isinstance(uploads_data, list):
                            uploads_count = len(uploads_data)
                            for item in uploads_data:
                                uploads_bytes += item.get("size", 0)
                                if "username" in item:
                                    peers_served.add(item["username"])
        except Exception:
            pass

        uploads_gb = uploads_bytes / (1024 ** 3) if uploads_bytes else 0.0

        if is_live:
            msg = (
                f"🌍 Estado: EN LÍNEA en Soulseek\n"
                f"📤 Compartiendo: {len(all_shared)} tracks curados ({total_gb:.2f} GB) con la comunidad.\n"
                f"🤝 Colaboración: {uploads_count} transferencias ({uploads_gb:.2f} GB subidos) a {len(peers_served)} DJs."
            )
        else:
            msg = (
                f"🟡 Estado: LOCAL (slskd en pausa)\n"
                f"📦 Biblioteca curada lista para compartir: {len(all_shared)} tracks ({total_gb:.2f} GB).\n"
                f"💡 Inicia Hera para conectar y colaborar con la red."
            )

        return {
            "tracks_shared": len(all_shared),
            "library_tracks": len(shared_files),
            "sets_tracks": len(sets_files),
            "total_size_bytes": total_bytes,
            "total_size_gb": round(total_gb, 2),
            "uploads_count": uploads_count,
            "uploads_bytes": uploads_bytes,
            "uploads_gb": round(uploads_gb, 2),
            "unique_peers_served": len(peers_served),
            "is_live": is_live,
            "community_message": msg,
        }

