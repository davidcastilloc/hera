from pathlib import Path
from mcp.server.fastmcp import FastMCP
from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.mcp.handlers.search import handle_search_music
from hera.mcp.handlers.candidates import handle_get_track_candidates
from hera.mcp.handlers.download import handle_download_track
from hera.mcp.handlers.status import handle_download_status
from hera.mcp.handlers.identify import handle_identify_track
from hera.mcp.handlers.analyze import handle_analyze_track
from hera.mcp.handlers.organize import handle_organize_track
from hera.mcp.handlers.crate import handle_build_dj_crate


def create_mcp_server(config_path: Path | str | None = None) -> FastMCP:
    """Instancia y configura el servidor FastMCP de Hera."""
    cfg_p = Path(config_path or "config/hera.toml")
    config = HeraConfig.load(cfg_p).resolve_paths(cfg_p.parent.parent if cfg_p.exists() else Path("."))
    db = Database(config.db_path)

    mcp = FastMCP(
        name="hera",
        instructions="Capa inteligente para búsqueda, adquisición autorizada, validación y organización de música para DJs.",
    )


    @mcp.tool()
    async def search_music(
        query: str,
        filters: dict | None = None,
        providers: list[str] | None = None,
    ) -> dict:
        """Busca temas musicales en providers locales y autorizados (local, slskd)."""
        await db.init_schema()
        return await handle_search_music(query, filters, providers, db, config)

    @mcp.tool()
    async def get_track_candidates(
        search_id: str,
        limit: int = 10,
    ) -> list[dict]:
        """Obtiene candidatos normalizados y ordenados por score explicable para una búsqueda previa."""
        await db.init_schema()
        return await handle_get_track_candidates(search_id, limit, db)

    @mcp.tool()
    async def download_track(
        candidate_id: str,
        authorization: dict,
        idempotency_key: str,
        approval_token: str | None = None,
    ) -> dict:
        """Inicia la adquisición autorizada de un candidato hacia el directorio aislado de cuarentena."""
        await db.init_schema()
        return await handle_download_track(
            candidate_id, authorization, approval_token, idempotency_key, db, config
        )

    @mcp.tool()
    async def download_status(
        job_id: str,
    ) -> dict:
        """Consulta el estado, progreso y errores de un trabajo de descarga o procesamiento."""
        await db.init_schema()
        return await handle_download_status(job_id, db)

    @mcp.tool()
    async def identify_track(
        asset_id: str,
    ) -> dict:
        """Calcula la huella acústica de un activo en cuarentena y genera hipótesis de identidad con confianza."""
        await db.init_schema()
        return await handle_identify_track(asset_id, db, config)

    @mcp.tool()
    async def analyze_track(
        track_id: str,
        profile: str = "dj-standard",
    ) -> dict:
        """Extrae características musicales acústicas del track: BPM, clave musical, Camelot y energía."""
        await db.init_schema()
        return await handle_analyze_track(track_id, profile, db)

    @mcp.tool()
    async def organize_track(
        track_id: str,
        template: str | None = None,
        collision_policy: str | None = None,
    ) -> dict:
        """Promueve de forma segura y transaccional un track validado desde cuarentena a la biblioteca organizada."""
        await db.init_schema()
        return await handle_organize_track(track_id, template, collision_policy, db, config)

    @mcp.tool()
    async def build_dj_crate(
        brief: str,
        duration_minutes: int = 60,
        constraints: dict | None = None,
        export: list[str] | None = None,
    ) -> dict:
        """Construye un crate/playlist curado para DJ y genera exportaciones en M3U8 y Rekordbox XML."""
        await db.init_schema()
        return await handle_build_dj_crate(brief, duration_minutes, constraints, export, db, config)

    return mcp


if __name__ == "__main__":
    server = create_mcp_server()
    server.run(transport="stdio")
