"""Proveedor Lidarr para búsqueda y gestión automatizada de música con MusicBrainz."""

import asyncio
from pathlib import Path
import uuid
import httpx
from hera.contracts.candidate import Candidate, ScoreComponents, AuthorizationState
from hera.contracts.search import SearchFilters


class LidarrProvider:
    name = "lidarr"

    def __init__(
        self,
        base_url: str = "http://localhost:8686",
        api_key: str | None = None,
    ):
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key

    def _headers(self) -> dict[str, str]:
        headers = {"Content-Type": "application/json"}
        if self.api_key:
            headers["X-Api-Key"] = self.api_key
        return headers

    async def capabilities(self) -> list[str]:
        return ["search", "download"]

    async def health(self) -> dict:
        try:
            async with httpx.AsyncClient(timeout=3.0) as client:
                res = await client.get(f"{self.base_url}/api/v1/system/status", headers=self._headers())
                return {
                    "provider": self.name,
                    "status": "healthy" if res.status_code == 200 else "degraded",
                    "status_code": res.status_code,
                }
        except Exception as e:
            return {"provider": self.name, "status": "unavailable", "error": str(e)}

    async def search(self, query: str, filters: SearchFilters | None = None) -> list[Candidate]:
        candidates: list[Candidate] = []
        search_id = f"srch_lidarr_{uuid.uuid4().hex[:6]}"

        try:
            async with httpx.AsyncClient(timeout=8.0) as client:
                params = {"term": query}
                res = await client.get(f"{self.base_url}/api/v1/album/lookup", headers=self._headers(), params=params)
                if res.status_code != 200:
                    return []

                albums = res.json()
                for album in albums:
                    title = album.get("title", "")
                    artist = album.get("artist", {}).get("artistName", "Unknown Artist")
                    foreign_album_id = album.get("foreignAlbumId", "")
                    release_date = album.get("releaseDate", "")
                    year = release_date[:4] if release_date else ""

                    ident = 0.90
                    tech = 0.90
                    source = 0.85
                    avail = 0.80
                    pref = 0.80
                    meta = 0.95
                    risk = 0.90

                    score = (
                        30 * ident
                        + 25 * tech
                        + 15 * source
                        + 10 * avail
                        + 10 * pref
                        + 5 * meta
                        + 5 * risk
                    )

                    cand = Candidate(
                        search_id=search_id,
                        provider=self.name,
                        native_ref=f"lidarr://album/{foreign_album_id}",
                        artist=artist,
                        title=title,
                        version=f"Release {year}" if year else None,
                        format="FLAC",
                        bitrate_kbps=1411,
                        score=round(score, 1),
                        score_components=ScoreComponents(
                            identity=ident,
                            technical=tech,
                            source=source,
                            availability=avail,
                            preference=pref,
                            metadata=meta,
                            risk=risk,
                        ),
                        score_reasons=[
                            "Catálogo MusicBrainz indexado por Lidarr",
                            "Álbum con metadatos enriquecidos",
                        ],
                        availability="lidarr_managed_release",
                        authorization_state=AuthorizationState.USER_CONFIRMATION_REQUIRED,
                    )
                    candidates.append(cand)
        except Exception:
            return []

        return candidates

    async def resolve(self, native_ref: str) -> dict:
        return {"native_ref": native_ref, "resolved": True}

    async def start_transfer(self, candidate: Candidate, target_path: str) -> str:
        path = Path(target_path)
        path.parent.mkdir(parents=True, exist_ok=True)
        if not path.exists():
            with open(path, "wb") as f:
                f.write(b"SIMULATED_LIDARR_MANAGED_AUDIO_" + candidate.title.encode("utf-8"))
        return f"lidarr_xfer_{uuid.uuid4().hex[:8]}"

    async def transfer_status(self, transfer_id: str) -> dict:
        return {"transfer_id": transfer_id, "state": "completed", "progress": 1.0}

    async def cancel_transfer(self, transfer_id: str) -> bool:
        return True
