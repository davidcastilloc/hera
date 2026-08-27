"""Proveedor yt-dlp para extracción de audio de YouTube, SoundCloud y 1700+ fuentes."""

import asyncio
from pathlib import Path
import uuid
from hera.contracts.candidate import Candidate, ScoreComponents, AuthorizationState
from hera.contracts.search import SearchFilters


class YtdlpProvider:
    name = "ytdlp"

    def __init__(self, max_results: int = 10, preferred_quality: str = "320"):
        self.max_results = max_results
        self.preferred_quality = preferred_quality

    async def capabilities(self) -> list[str]:
        return ["search", "download"]

    async def health(self) -> dict:
        try:
            import yt_dlp
            return {
                "provider": self.name,
                "status": "healthy",
                "version": getattr(yt_dlp, "__version__", "unknown"),
            }
        except ImportError:
            return {"provider": self.name, "status": "unavailable", "error": "yt-dlp package not installed"}

    def _sync_search(self, query: str) -> list[dict]:
        import yt_dlp
        ydl_opts = {
            "skip_download": True,
            "extract_flat": True,
            "quiet": True,
            "no_warnings": True,
        }
        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            res = ydl.extract_info(f"ytsearch{self.max_results}:{query}", download=False)
            return res.get("entries", []) if res else []

    async def search(self, query: str, filters: SearchFilters | None = None) -> list[Candidate]:
        candidates: list[Candidate] = []
        search_id = f"srch_ytdlp_{uuid.uuid4().hex[:6]}"

        try:
            loop = asyncio.get_running_loop()
            entries = await loop.run_in_executor(None, self._sync_search, query)

            tokens = query.lower().split()

            for entry in entries:
                if not entry:
                    continue

                title = entry.get("title", "")
                url = entry.get("url") or entry.get("webpage_url") or entry.get("id")
                if not url:
                    continue

                if not url.startswith("http"):
                    url = f"https://www.youtube.com/watch?v={url}"

                duration_s = entry.get("duration") or 0
                duration_ms = int(duration_s * 1000) if duration_s else None
                uploader = entry.get("uploader") or entry.get("channel") or "Web Source"

                # Parse de artista / título
                artist = uploader
                clean_title = title
                version = None

                if " - " in title:
                    parts = title.split(" - ", 1)
                    artist = parts[0].strip()
                    clean_title = parts[1].strip()

                for delimiter in [("[", "]"), ("(", ")")]:
                    if delimiter[0] in clean_title and delimiter[1] in clean_title:
                        start = clean_title.find(delimiter[0])
                        end = clean_title.find(delimiter[1])
                        if start < end:
                            version = clean_title[start + 1 : end].strip()
                            clean_title = (clean_title[:start] + clean_title[end + 1 :]).strip()

                # Scoring: YouTube es lossy re-encoded, source=0.60, tech=0.65
                matches = sum(1 for token in tokens if token in title.lower())
                ident = min(1.0, max(0.4, matches / max(1, len(tokens))))
                tech = 0.65  # Opus/AAC stream re-encode
                source = 0.60
                avail = 0.90
                pref = 0.60
                meta = 0.70 if duration_ms else 0.50
                risk = 0.75

                score = (
                    30 * ident
                    + 25 * tech
                    + 15 * source
                    + 10 * avail
                    + 10 * pref
                    + 5 * meta
                    + 5 * risk
                )

                score_comps = ScoreComponents(
                    identity=round(ident, 2),
                    technical=round(tech, 2),
                    source=round(source, 2),
                    availability=round(avail, 2),
                    preference=round(pref, 2),
                    metadata=round(meta, 2),
                    risk=round(risk, 2),
                )

                cand = Candidate(
                    search_id=search_id,
                    provider=self.name,
                    native_ref=url,
                    artist=artist,
                    title=clean_title,
                    version=version,
                    duration_ms=duration_ms,
                    format="MP3",
                    bitrate_kbps=int(self.preferred_quality) if self.preferred_quality.isdigit() else 320,
                    score=round(score, 1),
                    score_components=score_comps,
                    score_reasons=[
                        f"yt-dlp Stream Web ({uploader})",
                        f"Audio recodificado a MP3 {self.preferred_quality}k",
                    ],
                    availability="streaming_download",
                    authorization_state=AuthorizationState.USER_CONFIRMATION_REQUIRED,
                )
                candidates.append(cand)

        except Exception:
            return []

        return candidates

    async def resolve(self, native_ref: str) -> dict:
        return {"native_ref": native_ref, "resolved": True}

    def _sync_download(self, url: str, target_path: Path) -> None:
        import yt_dlp
        target_path.parent.mkdir(parents=True, exist_ok=True)
        base_out = str(target_path.with_suffix(""))

        ydl_opts = {
            "format": "bestaudio/best",
            "outtmpl": base_out + ".%(ext)s",
            "postprocessors": [
                {
                    "key": "FFmpegExtractAudio",
                    "preferredcodec": "mp3",
                    "preferredquality": self.preferred_quality,
                }
            ],
            "quiet": True,
            "no_warnings": True,
        }
        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            ydl.download([url])

        # Asegurar que el archivo de salida final coincida con target_path
        expected_mp3 = Path(base_out + ".mp3")
        if expected_mp3.exists() and expected_mp3 != target_path:
            if target_path.exists():
                target_path.unlink()
            expected_mp3.rename(target_path)

    async def start_transfer(self, candidate: Candidate, target_path: str) -> str:
        dst = Path(target_path)
        loop = asyncio.get_running_loop()
        await loop.run_in_executor(None, self._sync_download, candidate.native_ref, dst)
        return f"ytdlp_xfer_{uuid.uuid4().hex[:8]}"

    async def transfer_status(self, transfer_id: str) -> dict:
        return {"transfer_id": transfer_id, "state": "completed", "progress": 1.0}

    async def cancel_transfer(self, transfer_id: str) -> bool:
        return True
