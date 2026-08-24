"""Protocolos e interfaces para proveedores de música."""

from typing import Protocol, runtime_checkable
from hera.contracts.candidate import Candidate
from hera.contracts.search import SearchFilters


@runtime_checkable
class Provider(Protocol):
    """Protocolo base que todo provider de Hera debe implementar."""

    name: str

    async def capabilities(self) -> list[str]:
        """Devuelve capacidades: search, download, local_import, etc."""
        ...

    async def health(self) -> dict:
        """Verifica conectividad y estado del provider."""
        ...

    async def search(self, query: str, filters: SearchFilters | None = None) -> list[Candidate]:
        """Ejecuta una búsqueda y devuelve candidatos normalizados."""
        ...

    async def resolve(self, native_ref: str) -> dict:
        """Resuelve metadata extendida antes de transferir."""
        ...

    async def start_transfer(self, candidate: Candidate, target_path: str) -> str:
        """Inicia transferencia del activo hacia el target_path (cuarentena). Retorna transfer_id."""
        ...

    async def transfer_status(self, transfer_id: str) -> dict:
        """Consulta progreso (0.0 - 1.0), estado y velocidad."""
        ...

    async def cancel_transfer(self, transfer_id: str) -> bool:
        """Cancela una transferencia activa."""
        ...
