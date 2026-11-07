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
        extra_dirs: list[Path | str] | None = None,
    ) -> dict:
        """Calcula el inventario compartido local y las métricas de transferencia desde slskd."""
        lib_path = Path(library_dir)
        sets_path = Path(sets_dir) if sets_dir else None

        valid_exts = {".flac", ".mp3", ".wav", ".aif", ".aiff", ".m4a", ".alac", ".ogg", ".mp4", ".webm", ".mkv", ".mov"}
        skip_prefixes = ("00_CRATES_DE_COMBATE", "00_SETS_ESTRATEGICOS", "curated_library")

        def _fast_scan(root_dir: Path, skip_crates: bool = False) -> tuple[int, int]:
            if not root_dir.exists() or not root_dir.is_dir():
                return 0, 0
            count = 0
            size_sum = 0
            stack = [str(root_dir)]
            while stack:
                curr = stack.pop()
                try:
                    with os.scandir(curr) as it:
                        for entry in it:
                            name = entry.name
                            if entry.is_dir(follow_symlinks=False):
                                if skip_crates and name.startswith(skip_prefixes):
                                    continue
                                if name.startswith("."):
                                    continue
                                stack.append(entry.path)
                            elif entry.is_file(follow_symlinks=False):
                                if name == ".gitkeep":
                                    continue
                                dot = name.rfind(".")
                                if dot == -1 or name[dot:].lower() not in valid_exts:
                                    continue
                                count += 1
                                try:
                                    size_sum += entry.stat(follow_symlinks=False).st_size
                                except OSError:
                                    pass
                except OSError:
                    pass
            return count, size_sum

        # 1. Contar archivos locales compartidos de forma ultrarrápida
        seen_roots: set[str] = set()
        lib_count, lib_bytes = 0, 0
        if lib_path.exists():
            seen_roots.add(os.path.normcase(os.path.abspath(str(lib_path))))
            lib_count, lib_bytes = _fast_scan(lib_path)

        sets_count, sets_bytes = 0, 0
        if sets_path and sets_path.exists():
            norm_s = os.path.normcase(os.path.abspath(str(sets_path)))
            if norm_s not in seen_roots:
                seen_roots.add(norm_s)
                sets_count, sets_bytes = _fast_scan(sets_path)

        extra_count, extra_bytes = 0, 0
        if extra_dirs:
            for d in extra_dirs:
                dp = Path(d)
                if dp.exists() and dp.is_dir():
                    norm_d = os.path.normcase(os.path.abspath(str(dp)))
                    if norm_d not in seen_roots:
                        seen_roots.add(norm_d)
                        c, b = _fast_scan(dp, skip_crates=True)
                        extra_count += c
                        extra_bytes += b

        total_local_tracks = lib_count + sets_count + extra_count
        total_bytes = lib_bytes + sets_bytes + extra_bytes
        total_gb = total_bytes / (1024 ** 3)

        # 2. Consultar slskd API para métricas en vivo
        uploads_count = 0
        uploads_bytes = 0
        peers_served = set()
        is_live = False
        is_logged_in = False
        slskd_shared_files = 0
        slskd_shared_dirs = 0

        try:
            async with httpx.AsyncClient(timeout=2.0) as client:
                health_resp = await client.get(f"{self.base_url}/api/v0/application", headers=self._headers())
                if health_resp.status_code == 200:
                    is_live = True
                    app_data = health_resp.json()
                    if isinstance(app_data, dict):
                        is_logged_in = bool((app_data.get("server") or {}).get("isLoggedIn", False))
                        shares_info = app_data.get("shares") or {}
                        slskd_shared_files = int(shares_info.get("files", 0))
                        slskd_shared_dirs = int(shares_info.get("directories", 0))

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
        effective_tracks = max(total_local_tracks, slskd_shared_files)

        if is_live:
            net_status = "EN LÍNEA en Soulseek (Conectado y Autenticado)" if is_logged_in else "EN LÍNEA (Conectando a Soulseek)"
            msg = (
                f"🌍 Estado: {net_status}\n"
                f"📤 Compartiendo: {effective_tracks} tracks curados ({total_gb:.2f} GB en {slskd_shared_dirs or 1} carpetas) con la comunidad.\n"
                f"🤝 Colaboración: {uploads_count} transferencias ({uploads_gb:.2f} GB subidos) a {len(peers_served)} DJs."
            )
        else:
            msg = (
                f"🟡 Estado: LOCAL (slskd en pausa)\n"
                f"📦 Biblioteca curada lista para compartir: {effective_tracks} tracks ({total_gb:.2f} GB).\n"
                f"💡 Inicia Hera para conectar y colaborar con la red."
            )

        return {
            "tracks_shared": effective_tracks,
            "library_tracks": lib_count,
            "sets_tracks": sets_count,
            "total_size_bytes": total_bytes,
            "total_size_gb": round(total_gb, 2),
            "uploads_count": uploads_count,
            "uploads_bytes": uploads_bytes,
            "uploads_gb": round(uploads_gb, 2),
            "unique_peers_served": len(peers_served),
            "is_live": is_live,
            "is_logged_in": is_logged_in,
            "slskd_shared_files": slskd_shared_files,
            "slskd_shared_dirs": slskd_shared_dirs,
            "community_message": msg,
        }

