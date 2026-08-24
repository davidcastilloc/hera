"""Motor de políticas de autorización y guardrails de Hera."""

from pathlib import Path
from hera.contracts.authorization import ApprovalResult, Authorization, AuthorizationBasis
from hera.contracts.candidate import Candidate
from hera.contracts.errors import HeraErrorCode, HeraException
from hera.contracts.track import Track
from hera.domain.config import PolicyConfig
from hera.policy.path_validator import validate_path_safety


class PolicyEngine:
    """Valida reglas de autorización, seguridad y límites antes de cualquier efecto."""

    def __init__(self, config: PolicyConfig):
        self.config = config

    def authorize_download(
        self,
        candidate: Candidate,
        authorization: Authorization,
        approval_token: str | None = None,
    ) -> ApprovalResult:
        """Valida si una solicitud de descarga cumple con la política de autorización."""

        # 1. Validar que la base de autorización esté permitida
        if authorization.basis.value not in self.config.allowed_bases:
            return ApprovalResult(
                approved=False,
                reason=f"La base de autorización '{authorization.basis.value}' no está permitida en la configuración.",
                policy_code=HeraErrorCode.POLICY_DENIED.value,
                required_action="Proporcionar una base de autorización permitida (e.g. purchased_copy, owned_original)",
            )

        # 2. Validar que la evidencia no esté vacía
        if not authorization.evidence_ref or len(authorization.evidence_ref.strip()) < 3:
            return ApprovalResult(
                approved=False,
                reason="La referencia de evidencia de autorización es requerida y debe ser verificable.",
                policy_code=HeraErrorCode.POLICY_DENIED.value,
                required_action="Proporcionar un recibo, URL de licencia o referencia de prueba en evidence_ref",
            )

        # 3. Validar aprobación humana si se exige
        if self.config.require_approval and not approval_token:
            return ApprovalResult(
                approved=False,
                reason="Se requiere un token de aprobación explícito para iniciar la descarga.",
                policy_code=HeraErrorCode.AUTH_REQUIRED.value,
                required_action="Obtener aprobación del usuario y reenviar con approval_token",
            )

        # 4. Validar límite de tamaño si se conoce
        if candidate.file_size_bytes:
            max_bytes = self.config.max_file_size_mb * 1024 * 1024
            if candidate.file_size_bytes > max_bytes:
                return ApprovalResult(
                    approved=False,
                    reason=f"El tamaño del archivo ({candidate.file_size_bytes / 1024 / 1024:.1f} MB) excede el máximo permitido ({self.config.max_file_size_mb} MB).",
                    policy_code=HeraErrorCode.POLICY_DENIED.value,
                    required_action="Aumentar max_file_size_mb en la configuración si es un archivo legítimo",
                )

        return ApprovalResult(
            approved=True,
            reason="Autorización verificada y aprobada por política.",
            policy_code="POLICY_APPROVED",
        )

    def authorize_organize(
        self,
        track: Track,
        destination_path: Path | str,
        library_base_dir: Path | str,
    ) -> ApprovalResult:
        """Valida que la promoción y organización de un archivo sea segura."""

        # 1. Validar que el track esté en un estado elegible (validado / identificado / analizado)
        if track.status.value not in {"validated", "identified", "analyzed"}:
            return ApprovalResult(
                approved=False,
                reason=f"El track no puede promoverse desde el estado '{track.status.value}'. Debe estar validado o analizado.",
                policy_code=HeraErrorCode.POLICY_DENIED.value,
            )

        # 2. Validar que el destino no escape del directorio library
        if not validate_path_safety(library_base_dir, destination_path):
            return ApprovalResult(
                approved=False,
                reason="Intento de path traversal detectado hacia fuera del directorio de biblioteca.",
                policy_code=HeraErrorCode.POLICY_DENIED.value,
            )

        return ApprovalResult(
            approved=True,
            reason="Organización autorizada dentro de biblioteca.",
            policy_code="POLICY_APPROVED",
        )
