"""Worker local que consume la tabla SQLite durable de jobs."""

import asyncio
import traceback
from datetime import datetime, timezone
from hera.contracts.job import Job, JobState, JobType
from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.domain.repositories import JobRepository
from hera.jobs.handlers import HANDLERS


class JobRunner:
    def __init__(self, db: Database, config: HeraConfig, poll_interval_sec: float = 1.0, max_attempts: int = 3):
        self.db = db
        self.config = config
        self.poll_interval_sec = poll_interval_sec
        self.max_attempts = max_attempts
        self._running = False
        self._task: asyncio.Task | None = None

    async def start(self) -> None:
        """Inicia el worker en background."""
        self._running = True
        self._task = asyncio.create_task(self._worker_loop())

    async def stop(self) -> None:
        """Detiene el worker limpiamente."""
        self._running = False
        if self._task:
            self._task.cancel()
            try:
                await self._task
            except asyncio.CancelledError:
                pass

    async def run_once(self) -> bool:
        """Ejecuta un solo job si existe en la cola. Útil para tests y procesamiento puntual."""
        conn = await self.db.connect()
        job_repo = JobRepository(conn)

        job = await job_repo.claim_next_queued()
        if not job:
            return False

        handler = HANDLERS.get(job.type)
        if not handler:
            await job_repo.update_state(
                job.id,
                JobState.FAILED,
                error_code="NO_HANDLER",
                error_message=f"No existe handler registrado para el tipo de job {job.type.value}",
            )
            return True

        try:
            result = await handler(job, self.db, self.config)
            await job_repo.update_state(
                job.id,
                JobState.COMPLETED,
                progress=1.0,
                result_json=result,
            )
        except Exception as e:
            attempts = job.attempts + 1
            if attempts >= self.max_attempts:
                await job_repo.update_state(
                    job.id,
                    JobState.FAILED,
                    error_code="MAX_RETRIES_EXCEEDED",
                    error_message=f"{str(e)}\n{traceback.format_exc()}",
                )
            else:
                # Reencolar para reintento
                await job_repo.update_state(
                    job.id,
                    JobState.QUEUED,
                    error_code="RETRYING",
                    error_message=str(e),
                )
        return True

    async def _worker_loop(self) -> None:
        while self._running:
            try:
                worked = await self.run_once()
                if not worked:
                    await asyncio.sleep(self.poll_interval_sec)
            except asyncio.CancelledError:
                break
            except Exception:
                await asyncio.sleep(self.poll_interval_sec)
