"""Repositorios de acceso a datos para modelos de Hera."""

from datetime import datetime, timezone
import json
import aiosqlite
from hera.contracts.candidate import Candidate, ScoreComponents, AuthorizationState
from hera.contracts.crate import Crate, CrateConstraints, CrateTrack
from hera.contracts.errors import HeraErrorCode, HeraException
from hera.contracts.job import Job, JobState, JobType
from hera.contracts.preference import DjPreference
from hera.contracts.track import Track, TrackStatus


class TrackRepository:
    def __init__(self, conn: aiosqlite.Connection):
        self.conn = conn

    async def save(self, track: Track) -> Track:
        query = """
        INSERT INTO tracks (
            id, status, canonical_title, canonical_artist, version,
            duration_ms, recording_mbid, release_mbid, isrc,
            fingerprint, audio_hash_sha256, perceptual_hash,
            codec, bitrate_kbps, sample_rate_hz, bit_depth, channels, file_size_bytes,
            quarantine_path, library_path, bpm, bpm_confidence, musical_key, key_confidence,
            camelot, energy, danceability, loudness_lufs, embedding_ref, analysis_version,
            license_basis, authorization_evidence_ref, provenance_json,
            created_at, updated_at, deleted_at
        ) VALUES (
            ?, ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?,
            ?, ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?, ?,
            ?, ?, ?,
            ?, ?, ?
        )
        ON CONFLICT(id) DO UPDATE SET
            status = excluded.status,
            canonical_title = excluded.canonical_title,
            canonical_artist = excluded.canonical_artist,
            version = excluded.version,
            duration_ms = excluded.duration_ms,
            recording_mbid = excluded.recording_mbid,
            release_mbid = excluded.release_mbid,
            isrc = excluded.isrc,
            fingerprint = excluded.fingerprint,
            audio_hash_sha256 = excluded.audio_hash_sha256,
            perceptual_hash = excluded.perceptual_hash,
            codec = excluded.codec,
            bitrate_kbps = excluded.bitrate_kbps,
            sample_rate_hz = excluded.sample_rate_hz,
            bit_depth = excluded.bit_depth,
            channels = excluded.channels,
            file_size_bytes = excluded.file_size_bytes,
            quarantine_path = excluded.quarantine_path,
            library_path = excluded.library_path,
            bpm = excluded.bpm,
            bpm_confidence = excluded.bpm_confidence,
            musical_key = excluded.musical_key,
            key_confidence = excluded.key_confidence,
            camelot = excluded.camelot,
            energy = excluded.energy,
            danceability = excluded.danceability,
            loudness_lufs = excluded.loudness_lufs,
            embedding_ref = excluded.embedding_ref,
            analysis_version = excluded.analysis_version,
            license_basis = excluded.license_basis,
            authorization_evidence_ref = excluded.authorization_evidence_ref,
            provenance_json = excluded.provenance_json,
            updated_at = excluded.updated_at,
            deleted_at = excluded.deleted_at;
        """
        now_str = datetime.now(timezone.utc).isoformat()
        provenance = json.dumps(track.provenance_json) if track.provenance_json else None
        await self.conn.execute(
            query,
            (
                track.id, track.status.value, track.canonical_title, track.canonical_artist, track.version,
                track.duration_ms, track.recording_mbid, track.release_mbid, track.isrc,
                track.fingerprint, track.audio_hash_sha256, track.perceptual_hash,
                track.codec, track.bitrate_kbps, track.sample_rate_hz, track.bit_depth, track.channels, track.file_size_bytes,
                track.quarantine_path, track.library_path, track.bpm, track.bpm_confidence, track.musical_key, track.key_confidence,
                track.camelot, track.energy, track.danceability, track.loudness_lufs, track.embedding_ref, track.analysis_version,
                track.license_basis, track.authorization_evidence_ref, provenance,
                track.created_at.isoformat(), now_str, track.deleted_at.isoformat() if track.deleted_at else None,
            ),
        )
        await self.conn.commit()
        return track

    async def get_by_id(self, track_id: str) -> Track | None:
        cursor = await self.conn.execute("SELECT * FROM tracks WHERE id = ? AND deleted_at IS NULL", (track_id,))
        row = await cursor.fetchone()
        if not row:
            return None
        return self._row_to_track(row)

    async def find_by_sha256(self, sha256: str) -> Track | None:
        cursor = await self.conn.execute(
            "SELECT * FROM tracks WHERE audio_hash_sha256 = ? AND deleted_at IS NULL", (sha256,)
        )
        row = await cursor.fetchone()
        if not row:
            return None
        return self._row_to_track(row)

    async def list_all(self, status: TrackStatus | None = None, limit: int = 100) -> list[Track]:
        if status:
            cursor = await self.conn.execute(
                "SELECT * FROM tracks WHERE status = ? AND deleted_at IS NULL ORDER BY created_at DESC LIMIT ?",
                (status.value, limit),
            )
        else:
            cursor = await self.conn.execute(
                "SELECT * FROM tracks WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT ?",
                (limit,),
            )
        rows = await cursor.fetchall()
        return [self._row_to_track(r) for r in rows]

    async def update_status(self, track_id: str, new_status: TrackStatus) -> Track:
        track = await self.get_by_id(track_id)
        if not track:
            raise HeraException(HeraErrorCode.INVALID_MEDIA, f"Track {track_id} no encontrado")
        if not track.can_transition_to(new_status):
            raise HeraException(
                HeraErrorCode.INVALID_MEDIA,
                f"Transición inválida de {track.status.value} a {new_status.value} para track {track_id}",
            )
        track.status = new_status
        track.updated_at = datetime.now(timezone.utc)
        return await self.save(track)

    def _row_to_track(self, row: aiosqlite.Row) -> Track:
        provenance = json.loads(row["provenance_json"]) if row["provenance_json"] else None
        return Track(
            id=row["id"],
            status=TrackStatus(row["status"]),
            canonical_title=row["canonical_title"],
            canonical_artist=row["canonical_artist"],
            version=row["version"],
            duration_ms=row["duration_ms"],
            recording_mbid=row["recording_mbid"],
            release_mbid=row["release_mbid"],
            isrc=row["isrc"],
            fingerprint=row["fingerprint"],
            audio_hash_sha256=row["audio_hash_sha256"],
            perceptual_hash=row["perceptual_hash"],
            codec=row["codec"],
            bitrate_kbps=row["bitrate_kbps"],
            sample_rate_hz=row["sample_rate_hz"],
            bit_depth=row["bit_depth"],
            channels=row["channels"],
            file_size_bytes=row["file_size_bytes"],
            quarantine_path=row["quarantine_path"],
            library_path=row["library_path"],
            bpm=row["bpm"],
            bpm_confidence=row["bpm_confidence"],
            musical_key=row["musical_key"],
            key_confidence=row["key_confidence"],
            camelot=row["camelot"],
            energy=row["energy"],
            danceability=row["danceability"],
            loudness_lufs=row["loudness_lufs"],
            embedding_ref=row["embedding_ref"],
            analysis_version=row["analysis_version"],
            license_basis=row["license_basis"],
            authorization_evidence_ref=row["authorization_evidence_ref"],
            provenance_json=provenance,
            created_at=datetime.fromisoformat(row["created_at"]),
            updated_at=datetime.fromisoformat(row["updated_at"]),
            deleted_at=datetime.fromisoformat(row["deleted_at"]) if row["deleted_at"] else None,
        )


class CandidateRepository:
    def __init__(self, conn: aiosqlite.Connection):
        self.conn = conn

    async def save_many(self, candidates: list[Candidate]) -> None:
        query = """
        INSERT OR REPLACE INTO candidates (
            candidate_id, search_id, provider, native_ref, artist, title, version,
            duration_ms, format, bitrate_kbps, file_size_bytes, score,
            score_components_json, score_reasons_json, availability, authorization_state,
            created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """
        now = datetime.now(timezone.utc).isoformat()
        params = [
            (
                c.candidate_id, c.search_id, c.provider, c.native_ref, c.artist, c.title, c.version,
                c.duration_ms, c.format, c.bitrate_kbps, c.file_size_bytes, c.score,
                json.dumps(c.score_components.model_dump()), json.dumps(c.score_reasons),
                c.availability, c.authorization_state.value, now,
            )
            for c in candidates
        ]
        await self.conn.executemany(query, params)
        await self.conn.commit()

    async def get_by_id(self, candidate_id: str) -> Candidate | None:
        cursor = await self.conn.execute("SELECT * FROM candidates WHERE candidate_id = ?", (candidate_id,))
        row = await cursor.fetchone()
        if not row:
            return None
        return self._row_to_candidate(row)

    async def list_by_search_id(self, search_id: str, limit: int = 10) -> list[Candidate]:
        cursor = await self.conn.execute(
            "SELECT * FROM candidates WHERE search_id = ? ORDER BY score DESC LIMIT ?",
            (search_id, limit),
        )
        rows = await cursor.fetchall()
        return [self._row_to_candidate(r) for r in rows]

    def _row_to_candidate(self, row: aiosqlite.Row) -> Candidate:
        components = ScoreComponents(**json.loads(row["score_components_json"]))
        reasons = json.loads(row["score_reasons_json"])
        return Candidate(
            candidate_id=row["candidate_id"],
            search_id=row["search_id"],
            provider=row["provider"],
            native_ref=row["native_ref"],
            artist=row["artist"],
            title=row["title"],
            version=row["version"],
            duration_ms=row["duration_ms"],
            format=row["format"],
            bitrate_kbps=row["bitrate_kbps"],
            file_size_bytes=row["file_size_bytes"],
            score=row["score"],
            score_components=components,
            score_reasons=reasons,
            availability=row["availability"],
            authorization_state=AuthorizationState(row["authorization_state"]),
        )


class JobRepository:
    def __init__(self, conn: aiosqlite.Connection):
        self.conn = conn

    async def create_job(self, job: Job) -> Job:
        query = """
        INSERT INTO jobs (
            id, type, state, progress, attempts, idempotency_key, correlation_id,
            input_json, result_json, error_code, error_message, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(idempotency_key) DO UPDATE SET
            updated_at = excluded.updated_at
        RETURNING id, type, state, progress, attempts, idempotency_key, correlation_id,
                  input_json, result_json, error_code, error_message, created_at, updated_at;
        """
        cursor = await self.conn.execute(
            query,
            (
                job.id, job.type.value, job.state.value, job.progress, job.attempts,
                job.idempotency_key, job.correlation_id, json.dumps(job.input_json),
                json.dumps(job.result_json) if job.result_json else None,
                job.error_code, job.error_message,
                job.created_at.isoformat(), job.updated_at.isoformat(),
            ),
        )
        row = await cursor.fetchone()
        await self.conn.commit()
        if not row:
            return job
        return self._row_to_job(row)

    async def get_by_id(self, job_id: str) -> Job | None:
        cursor = await self.conn.execute("SELECT * FROM jobs WHERE id = ?", (job_id,))
        row = await cursor.fetchone()
        if not row:
            return None
        return self._row_to_job(row)

    async def get_by_idempotency_key(self, key: str) -> Job | None:
        cursor = await self.conn.execute("SELECT * FROM jobs WHERE idempotency_key = ?", (key,))
        row = await cursor.fetchone()
        if not row:
            return None
        return self._row_to_job(row)

    async def update_state(
        self,
        job_id: str,
        state: JobState,
        progress: float | None = None,
        attempts: int | None = None,
        result_json: dict | None = None,
        error_code: str | None = None,
        error_message: str | None = None,
    ) -> None:
        now = datetime.now(timezone.utc).isoformat()
        updates = ["state = ?", "updated_at = ?"]
        params: list[object] = [state.value, now]

        if progress is not None:
            updates.append("progress = ?")
            params.append(progress)
        if attempts is not None:
            updates.append("attempts = ?")
            params.append(attempts)
        if result_json is not None:
            updates.append("result_json = ?")
            params.append(json.dumps(result_json))
        if error_code is not None:
            updates.append("error_code = ?")
            params.append(error_code)
        if error_message is not None:
            updates.append("error_message = ?")
            params.append(error_message)

        params.append(job_id)
        query = f"UPDATE jobs SET {', '.join(updates)} WHERE id = ?"
        await self.conn.execute(query, params)
        await self.conn.commit()

    async def claim_next_queued(self) -> Job | None:
        cursor = await self.conn.execute(
            "SELECT * FROM jobs WHERE state = ? ORDER BY created_at ASC LIMIT 1",
            (JobState.QUEUED.value,),
        )
        row = await cursor.fetchone()
        if not row:
            return None
        job = self._row_to_job(row)
        await self.update_state(job.id, JobState.RUNNING)
        job.state = JobState.RUNNING
        return job

    def _row_to_job(self, row: aiosqlite.Row) -> Job:
        return Job(
            id=row["id"],
            type=JobType(row["type"]),
            state=JobState(row["state"]),
            progress=row["progress"],
            attempts=row["attempts"],
            idempotency_key=row["idempotency_key"],
            correlation_id=row["correlation_id"],
            input_json=json.loads(row["input_json"]),
            result_json=json.loads(row["result_json"]) if row["result_json"] else None,
            error_code=row["error_code"],
            error_message=row["error_message"],
            created_at=datetime.fromisoformat(row["created_at"]),
            updated_at=datetime.fromisoformat(row["updated_at"]),
        )


class CrateRepository:
    def __init__(self, conn: aiosqlite.Connection):
        self.conn = conn

    async def save(self, crate: Crate) -> Crate:
        query = """
        INSERT OR REPLACE INTO crates (
            id, name, brief, duration_target_minutes, constraints_json,
            scoring_version, exports_json, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        """
        await self.conn.execute(
            query,
            (
                crate.id, crate.name, crate.brief, crate.duration_target_minutes,
                json.dumps(crate.constraints.model_dump()), crate.scoring_version,
                json.dumps(crate.exports), crate.created_at.isoformat(),
                datetime.now(timezone.utc).isoformat(),
            ),
        )
        # Reemplazar tracks de crate
        await self.conn.execute("DELETE FROM crate_tracks WHERE crate_id = ?", (crate.id,))
        track_params = [
            (crate.id, ct.track_id, ct.position, ct.transition_notes)
            for ct in crate.tracks
        ]
        if track_params:
            await self.conn.executemany(
                "INSERT INTO crate_tracks (crate_id, track_id, position, transition_notes) VALUES (?, ?, ?, ?)",
                track_params,
            )
        await self.conn.commit()
        return crate

    async def get_by_id(self, crate_id: str) -> Crate | None:
        cursor = await self.conn.execute("SELECT * FROM crates WHERE id = ?", (crate_id,))
        row = await cursor.fetchone()
        if not row:
            return None

        # Obtener tracks
        t_cursor = await self.conn.execute(
            "SELECT * FROM crate_tracks WHERE crate_id = ? ORDER BY position ASC",
            (crate_id,),
        )
        t_rows = await t_cursor.fetchall()
        tracks = [
            CrateTrack(
                track_id=tr["track_id"],
                position=tr["position"],
                transition_notes=tr["transition_notes"],
            )
            for tr in t_rows
        ]

        return Crate(
            id=row["id"],
            name=row["name"],
            brief=row["brief"],
            duration_target_minutes=row["duration_target_minutes"],
            constraints=CrateConstraints(**json.loads(row["constraints_json"])),
            scoring_version=row["scoring_version"],
            exports=json.loads(row["exports_json"]),
            tracks=tracks,
            created_at=datetime.fromisoformat(row["created_at"]),
            updated_at=datetime.fromisoformat(row["updated_at"]),
        )


class PreferenceRepository:
    def __init__(self, conn: aiosqlite.Connection):
        self.conn = conn

    async def set_preference(self, pref: DjPreference) -> None:
        query = """
        INSERT OR REPLACE INTO dj_preferences (
            profile_id, subject, feature, value, weight, evidence, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        """
        await self.conn.execute(
            query,
            (
                pref.profile_id, pref.subject, pref.feature, str(pref.value),
                pref.weight, pref.evidence, datetime.now(timezone.utc).isoformat(),
            ),
        )
        await self.conn.commit()

    async def list_preferences(self, profile_id: str = "default") -> list[DjPreference]:
        cursor = await self.conn.execute(
            "SELECT * FROM dj_preferences WHERE profile_id = ?",
            (profile_id,),
        )
        rows = await cursor.fetchall()
        return [
            DjPreference(
                profile_id=r["profile_id"],
                subject=r["subject"],
                feature=r["feature"],
                value=r["value"],
                weight=r["weight"],
                evidence=r["evidence"],
                updated_at=datetime.fromisoformat(r["updated_at"]),
            )
            for r in rows
        ]


class AuditRepository:
    def __init__(self, conn: aiosqlite.Connection):
        self.conn = conn

    async def record_event(
        self,
        event_type: str,
        actor: str,
        entity_id: str | None = None,
        policy_code: str | None = None,
        authorization_ref: str | None = None,
        details: dict | None = None,
    ) -> None:
        query = """
        INSERT INTO audit_log (
            event_type, actor, entity_id, policy_code, authorization_ref, details_json, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        """
        await self.conn.execute(
            query,
            (
                event_type, actor, entity_id, policy_code, authorization_ref,
                json.dumps(details) if details else None,
                datetime.now(timezone.utc).isoformat(),
            ),
        )
        await self.conn.commit()
