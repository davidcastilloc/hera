"""Handler para la tool download_track."""

from datetime import datetime, timezone
from hera.contracts.authorization import Authorization
from hera.contracts.errors import HeraErrorCode, HeraException
from hera.contracts.job import Job, JobState, JobType
from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.domain.repositories import AuditRepository, CandidateRepository, JobRepository
from hera.policy.engine import PolicyEngine


async def handle_download_track(
    candidate_id: str,
    authorization: dict,
    approval_token: str | None,
    idempotency_key: str,
    db: Database,
    config: HeraConfig,
) -> dict:
    conn = await db.connect()
    cand_repo = CandidateRepository(conn)
    job_repo = JobRepository(conn)
    audit_repo = AuditRepository(conn)

    # 1. Comprobar si ya existe un job con esta clave de idempotencia
    existing_job = await job_repo.get_by_idempotency_key(idempotency_key)
    if existing_job:
        return {
            "job_id": existing_job.id,
            "status": existing_job.state.value,
            "idempotency_hit": True,
            "message": "Solicitud previa reanudada mediante clave de idempotencia.",
        }

    # 2. Obtener candidato
    candidate = await cand_repo.get_by_id(candidate_id)
    if not candidate:
        raise HeraException(HeraErrorCode.NO_SOURCES, f"Candidato {candidate_id} no encontrado.")

    # 3. Validar con PolicyEngine
    auth_obj = Authorization(**authorization)
    policy_engine = PolicyEngine(config.policy)
    approval = policy_engine.authorize_download(candidate, auth_obj, approval_token)

    if not approval.approved:
        # Registrar denegación en auditoría
        await audit_repo.record_event(
            event_type="PolicyDenied",
            actor="agent",
            entity_id=candidate_id,
            policy_code=approval.policy_code,
            authorization_ref=auth_obj.evidence_ref,
            details={"reason": approval.reason},
        )
        raise HeraException(
            HeraErrorCode(approval.policy_code) if approval.policy_code in HeraErrorCode.__members__ else HeraErrorCode.POLICY_DENIED,
            approval.reason,
            details={"required_action": approval.required_action},
        )

    # 4. Crear Job en SQLite
    job = Job(
        type=JobType.DOWNLOAD,
        state=JobState.QUEUED,
        idempotency_key=idempotency_key,
        input_json={
            "candidate_id": candidate_id,
            "authorization": authorization,
            "approval_token": approval_token,
        },
    )
    created_job = await job_repo.create_job(job)

    # 5. Registrar en auditoría
    await audit_repo.record_event(
        event_type="TransferQueued",
        actor="agent",
        entity_id=created_job.id,
        policy_code="POLICY_APPROVED",
        authorization_ref=auth_obj.evidence_ref,
        details={"candidate_id": candidate_id, "provider": candidate.provider},
    )

    return {
        "job_id": created_job.id,
        "status": created_job.state.value,
        "message": "Transferencia encolada hacia cuarentena.",
    }
