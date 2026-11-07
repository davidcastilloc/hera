"""Motor de deduplicación multi-nivel y auditoría difusa pre-descarga (catalog_index.json >= 85%)."""

from difflib import SequenceMatcher
from pathlib import Path
import json
import os
import re
from pydantic import BaseModel
from hera.contracts.track import Track
from hera.domain.repositories import TrackRepository
from providers.local.scanner import VIDEO_EXTENSIONS, parse_dj_filename, strip_accents


def normalize_dedup_slug(raw: str) -> str:
    """Normaliza artista/título eliminando tildes, etiquetas de BPM/Camelot y sufijos de video/audio."""
    s = strip_accents((raw or "").lower())
    s = re.sub(r"\[\s*\d+(?:\.\d+)?\s*bpm\s*-\s*[0-9]{1,2}[ab]\s*\]", " ", s)
    s = re.sub(r"^(?:y2mate\.com|ytconvert\.in)\s*-\s*", " ", s)
    s = re.sub(r"^\d{2,3}_[0-9]{1,2}[ab]_", " ", s)
    s = re.sub(r"\((?:official|lyric|visualizer|audio|video|hd|oficial|letra)[^)]*\)", " ", s)
    s = re.sub(r"\[(?:official|lyric|visualizer|audio|video|hd|oficial|letra)[^\]]*\]", " ", s)
    s = re.sub(r"([a-z])\1{2,}", r"\1", s)
    s = re.sub(r"[^a-z0-9]+", " ", s)
    return re.sub(r"\s+", " ", s).strip()


class DedupResult(BaseModel):
    is_duplicate: bool
    duplicate_type: str | None = None  # exact_sha256 | fingerprint | mbid_isrc | near_metadata | fuzzy_catalog
    existing_track_id: str | None = None
    existing_path: str | None = None
    confidence: float = 0.0
    action_suggested: str = "allow"  # allow | skip | review


class DeduplicationEngine:
    def __init__(
        self,
        track_repo: TrackRepository,
        duration_tolerance_ms: int = 3000,
        fuzzy_threshold: float = 0.85,
    ):
        self.track_repo = track_repo
        self.duration_tolerance_ms = duration_tolerance_ms
        self.fuzzy_threshold = fuzzy_threshold

    async def check_duplicates(self, track: Track) -> DedupResult:
        """Verifica si el track ya existe en la base de datos según 5 niveles de certeza (incluyendo fuzzy >= 85%)."""

        # Nivel 1: SHA-256 Exacto
        if track.audio_hash_sha256:
            existing = await self.track_repo.find_by_sha256(track.audio_hash_sha256)
            if existing and existing.id != track.id:
                return DedupResult(
                    is_duplicate=True,
                    duplicate_type="exact_sha256",
                    existing_track_id=existing.id,
                    existing_path=existing.library_path or existing.quarantine_path,
                    confidence=1.0,
                    action_suggested="skip",
                )

        all_tracks = await self.track_repo.list_all(limit=20000)

        # Nivel 2: Huella acústica idéntica
        if track.fingerprint:
            for existing in all_tracks:
                if existing.id == track.id:
                    continue
                if existing.fingerprint and existing.fingerprint == track.fingerprint:
                    return DedupResult(
                        is_duplicate=True,
                        duplicate_type="fingerprint",
                        existing_track_id=existing.id,
                        existing_path=existing.library_path or existing.quarantine_path,
                        confidence=0.95,
                        action_suggested="review",
                    )

        # Nivel 3: Mismo MusicBrainz Recording MBID o ISRC
        if track.recording_mbid or track.isrc:
            for existing in all_tracks:
                if existing.id == track.id:
                    continue
                if (track.recording_mbid and existing.recording_mbid == track.recording_mbid) or (
                    track.isrc and existing.isrc == track.isrc
                ):
                    return DedupResult(
                        is_duplicate=True,
                        duplicate_type="mbid_isrc",
                        existing_track_id=existing.id,
                        existing_path=existing.library_path or existing.quarantine_path,
                        confidence=0.90,
                        action_suggested="review",
                    )

        # Nivel 4: Artista + Título exacto + Duración cercana (± 3s)
        t_artist = (track.canonical_artist or "").lower().strip()
        t_title = (track.canonical_title or "").lower().strip()
        t_slug = normalize_dedup_slug(f"{t_artist} {t_title}")

        for existing in all_tracks:
            if existing.id == track.id:
                continue
            e_artist = (existing.canonical_artist or "").lower().strip()
            e_title = (existing.canonical_title or "").lower().strip()

            if t_artist == e_artist and t_title == e_title:
                if track.duration_ms and existing.duration_ms:
                    diff = abs(track.duration_ms - existing.duration_ms)
                    if diff <= self.duration_tolerance_ms:
                        return DedupResult(
                            is_duplicate=True,
                            duplicate_type="near_metadata",
                            existing_track_id=existing.id,
                            existing_path=existing.library_path or existing.quarantine_path,
                            confidence=0.88,
                            action_suggested="review",
                        )
                else:
                    return DedupResult(
                        is_duplicate=True,
                        duplicate_type="near_metadata",
                        existing_track_id=existing.id,
                        existing_path=existing.library_path or existing.quarantine_path,
                        confidence=0.86,
                        action_suggested="review",
                    )

            # Nivel 5: Coincidencia difusa normalizada (>= 85%)
            if len(t_slug) >= 6:
                e_slug = normalize_dedup_slug(f"{e_artist} {e_title}")
                if e_slug and SequenceMatcher(None, t_slug, e_slug).ratio() >= self.fuzzy_threshold:
                    return DedupResult(
                        is_duplicate=True,
                        duplicate_type="fuzzy_catalog",
                        existing_track_id=existing.id,
                        existing_path=existing.library_path or existing.quarantine_path,
                        confidence=0.85,
                        action_suggested="skip",
                    )

        return DedupResult(
            is_duplicate=False,
            action_suggested="allow",
        )

    @staticmethod
    def audit_queries_against_folders(
        queries: list[str],
        local_folders: list[str | Path],
        fuzzy_threshold: float = 0.85,
        catalog_index_path: Path | str | None = None,
    ) -> dict:
        """Fase 1 Anti-Duplicados: Escanea carpetas locales (Audio + Video HD), actualiza `catalog_index.json`
        y clasifica cada consulta en `[SKIP - YA EXISTE]` o `[MISSING]` antes de iniciar cualquier descarga."""
        valid_exts = {
            ".mp3", ".flac", ".wav", ".m4a", ".aiff", ".aif", ".ogg",
            ".mp4", ".webm", ".mkv", ".mov",
        }
        skip_prefixes = ("00_CRATES_DE_COMBATE", "00_SETS_ESTRATEGICOS", "curated_library")

        catalog_entries: list[dict] = []
        seen_paths: set[str] = set()

        for folder in local_folders:
            fp = Path(folder)
            if not fp.exists() or not fp.is_dir():
                continue
            for root, dirs, files in os.walk(fp):
                dirs[:] = [d for d in dirs if not d.startswith(skip_prefixes)]
                for fname in files:
                    dot = fname.rfind(".")
                    if dot == -1:
                        continue
                    ext = fname[dot:].lower()
                    if ext not in valid_exts:
                        continue
                    full_p = os.path.join(root, fname)
                    norm_p = os.path.normcase(full_p)
                    if norm_p in seen_paths:
                        continue
                    seen_paths.add(norm_p)

                    artist, title, version, bpm, camelot = parse_dj_filename(fname)
                    slug = normalize_dedup_slug(f"{artist} {title}")
                    title_slug = normalize_dedup_slug(title)
                    is_video = ext.lstrip(".") in VIDEO_EXTENSIONS
                    catalog_entries.append({
                        "path": full_p,
                        "filename": fname,
                        "artist": artist,
                        "title": title,
                        "version": version,
                        "bpm": bpm,
                        "camelot": camelot,
                        "media_type": "video" if is_video else "audio",
                        "slug": slug,
                        "title_slug": title_slug,
                    })

        if catalog_index_path:
            idx_p = Path(catalog_index_path)
            try:
                idx_p.parent.mkdir(parents=True, exist_ok=True)
                with open(idx_p, "w", encoding="utf-8") as f:
                    json.dump(
                        {
                            "total_indexed": len(catalog_entries),
                            "entries": catalog_entries[:5000],
                        },
                        f,
                        indent=2,
                        ensure_ascii=False,
                    )
            except Exception:
                pass

        existing_matches: list[dict] = []
        missing_queries: list[str] = []

        for q in queries:
            q_artist, q_title, _, _, _ = parse_dj_filename(q)
            q_slug = normalize_dedup_slug(q)
            q_title_slug = normalize_dedup_slug(q_title)
            main_artist_tok = normalize_dedup_slug(q_artist.split(",")[0].split("&")[0])

            best_audio = None
            best_video = None
            best_score = 0.0

            for entry in catalog_entries:
                e_slug = entry["slug"]
                if not e_slug:
                    continue
                ratio = SequenceMatcher(None, q_slug, e_slug).ratio()
                if q_title_slug and q_title_slug in e_slug and (not main_artist_tok or main_artist_tok in e_slug):
                    ratio = max(ratio, 0.90)

                if ratio >= fuzzy_threshold:
                    if ratio > best_score:
                        best_score = ratio
                    if entry["media_type"] == "video" and best_video is None:
                        best_video = entry["path"]
                    elif entry["media_type"] == "audio" and best_audio is None:
                        best_audio = entry["path"]

            if best_audio or best_video:
                existing_matches.append({
                    "query": q,
                    "status": "[SKIP - YA EXISTE]",
                    "confidence": round(best_score, 3),
                    "audio_path": best_audio,
                    "video_path": best_video,
                })
            else:
                missing_queries.append(q)

        return {
            "total_catalog_files": len(catalog_entries),
            "queries_checked": len(queries),
            "skip_existing_count": len(existing_matches),
            "missing_count": len(missing_queries),
            "existing": existing_matches,
            "missing": missing_queries,
        }
