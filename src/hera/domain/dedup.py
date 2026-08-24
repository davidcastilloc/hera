"""Motor de deduplicación multi-nivel para activos musicales."""

from pydantic import BaseModel
from hera.contracts.track import Track
from hera.domain.repositories import TrackRepository


class DedupResult(BaseModel):
    is_duplicate: bool
    duplicate_type: str | None = None  # exact_sha256 | fingerprint | mbid_isrc | near_metadata
    existing_track_id: str | None = None
    confidence: float = 0.0
    action_suggested: str = "allow"  # allow | skip | review


class DeduplicationEngine:
    def __init__(self, track_repo: TrackRepository, duration_tolerance_ms: int = 3000):
        self.track_repo = track_repo
        self.duration_tolerance_ms = duration_tolerance_ms

    async def check_duplicates(self, track: Track) -> DedupResult:
        """Verifica si el track ya existe en la base de datos según 4 niveles de certeza."""

        # Nivel 1: SHA-256 Exacto
        if track.audio_hash_sha256:
            existing = await self.track_repo.find_by_sha256(track.audio_hash_sha256)
            if existing and existing.id != track.id:
                return DedupResult(
                    is_duplicate=True,
                    duplicate_type="exact_sha256",
                    existing_track_id=existing.id,
                    confidence=1.0,
                    action_suggested="skip",
                )

        # Nivel 2: Huella acústica idéntica
        if track.fingerprint:
            all_tracks = await self.track_repo.list_all(limit=500)
            for existing in all_tracks:
                if existing.id == track.id:
                    continue
                if existing.fingerprint and existing.fingerprint == track.fingerprint:
                    return DedupResult(
                        is_duplicate=True,
                        duplicate_type="fingerprint",
                        existing_track_id=existing.id,
                        confidence=0.95,
                        action_suggested="review",
                    )

        # Nivel 3: Mismo MusicBrainz Recording MBID o ISRC
        if track.recording_mbid or track.isrc:
            all_tracks = await self.track_repo.list_all(limit=500)
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
                        confidence=0.90,
                        action_suggested="review",
                    )

        # Nivel 4: Artista + Título + Duración cercana (± 3s)
        all_tracks = await self.track_repo.list_all(limit=500)
        t_artist = track.canonical_artist.lower().strip()
        t_title = track.canonical_title.lower().strip()

        for existing in all_tracks:
            if existing.id == track.id:
                continue
            e_artist = existing.canonical_artist.lower().strip()
            e_title = existing.canonical_title.lower().strip()

            if t_artist == e_artist and t_title == e_title:
                if track.duration_ms and existing.duration_ms:
                    diff = abs(track.duration_ms - existing.duration_ms)
                    if diff <= self.duration_tolerance_ms:
                        return DedupResult(
                            is_duplicate=True,
                            duplicate_type="near_metadata",
                            existing_track_id=existing.id,
                            confidence=0.85,
                            action_suggested="review",
                        )

        return DedupResult(
            is_duplicate=False,
            action_suggested="allow",
        )
