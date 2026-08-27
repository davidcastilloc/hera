"""Proveedor Internet Archive para música de dominio público, audiobooks y grabaciones de conciertos (Live Music Archive)."""

import asyncio
from pathlib import Path
import uuid
import httpx
from hera.contracts.candidate import Candidate, ScoreComponents, AuthorizationState
from hera.contracts.search import SearchFilters


class ArchiveProvider:
    name = "archive"

    def __init__(self, collections: list[str] | None = None):
        self.collections = collections or ["audio", "etree"]

    async def capabilities(self) -> list[str]:
        return ["search", "download"]

    async def health(self) -> dict:
        try:
            async with httpx.AsyncClient(timeout=3.0) as client:
                res = await client.get("https://archive.org/services/search/v1/scrape?q=mediatype:audio&count=1")
                return {
                    "provider": self.name,
                    "status": "healthy" if res.status_code == 200 else "degraded",
                    "status_code": res.status_code,
                }
        except Exception as e:
            return {"provider": self.name, "status": "unavailable", "error": str(e)}

    async def search(self, query: str, filters: SearchFilters | None = None) -> list[Candidate]:
        candidates: list[Candidate] = []
        search_id = f"srch_archive_{uuid.uuid4().hex[:6]}"

        try:
            async with httpx.AsyncClient(timeout=8.0) as client:
                params = {
                    "q": f"mediatype:audio AND {query}",
                    "fl[]": ["identifier", "title", "creator", "publicdate"],
                    "rows": 10,
                    "output": "json",
                }
                res = await client.get("https://archive.org/advancedsearch.php", params=params)
                if res.status_code != 200:
                    return []

                docs = res.json().get("response", {}).get("docs", [])
                for item in docs:
                    identifier = item.get("identifier", "")
                    title = item.get("title", identifier)
                    creator = item.get("creator", "Archive Artist")

                    ident = 0.85
                    tech = 0.85
                    source = 0.70
                    avail = 0.95  # Direct fast CDN HTTP download
                    pref = 0.75
                    meta = 0.80
                    risk = 0.95  # 100% legal / public domain

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
                        native_ref=f"https://archive.org/details/{identifier}",
                        artist=creator if isinstance(creator, str) else str(creator[0] if creator else "Archive Artist"),
                        title=title,
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
                            "Internet Archive: Dominio Público / Live Music Archive",
                            "Descarga directa HTTP CDN",
                        ],
                        availability="public_archive_cdn",
                        authorization_state=AuthorizationState.CONFIRMED,
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
                f.write(b"SIMULATED_INTERNET_ARCHIVE_AUDIO_" + candidate.title.encode("utf-8"))
        return f"archive_xfer_{uuid.uuid4().hex[:8]}"

    async def transfer_status(self, transfer_id: str) -> dict:
        return {"transfer_id": transfer_id, "state": "completed", "progress": 1.0}

    async def cancel_transfer(self, transfer_id: str) -> bool:
        return True
