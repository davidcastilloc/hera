"""Handler para la tool download_status."""

from hera.contracts.errors import HeraErrorCode, HeraException
from hera.domain.database import Database
from hera.domain.repositories import JobRepository


async def handle_download_status(job_id: str, db: Database) -> dict:
    conn = await db.connect()
    job_repo = JobRepository(conn)
    job = await job_repo.get_by_id(job_id)

    if not job:
        raise HeraException(HeraErrorCode.NO_SOURCES, f"Job {job_id} no encontrado")

    return {
        "job_id": job.id,
        "type": job.type.value,
        "state": job.state.value,
        "progress": job.progress,
        "attempts": job.attempts,
        "result": job.result_json,
        "error_code": job.error_code,
        "error_message": job.error_message,
        "updated_at": job.updated_at.isoformat(),
    }
