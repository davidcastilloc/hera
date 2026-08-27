"""Ejecutor asíncrono de migraciones de esquema versionadas para SQLite."""

from pathlib import Path
import aiosqlite
from datetime import datetime, timezone


MIGRATIONS_TABLE_DDL = """
CREATE TABLE IF NOT EXISTS hera_schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL
);
"""


class MigrationRunner:
    def __init__(self, migrations_dir: Path | str | None = None):
        if migrations_dir:
            self.migrations_dir = Path(migrations_dir)
        else:
            self.migrations_dir = Path(__file__).parent

    async def run_migrations(self, conn: aiosqlite.Connection) -> list[str]:
        """Aplica todas las migraciones pendientes en orden secuencial."""
        await conn.execute(MIGRATIONS_TABLE_DDL)
        await conn.commit()

        # Obtener versiones ya aplicadas
        cursor = await conn.execute("SELECT version FROM hera_schema_migrations ORDER BY version ASC")
        rows = await cursor.fetchall()
        applied_versions = {r[0] for r in rows}

        # Listar archivos .sql en migrations_dir
        sql_files = sorted(self.migrations_dir.glob("*.sql"))
        applied_now = []

        for sql_file in sql_files:
            # Convención de nombre: 001_nombre.sql -> versión = 1
            parts = sql_file.stem.split("_", 1)
            try:
                version = int(parts[0])
            except ValueError:
                continue

            if version not in applied_versions:
                sql_content = sql_file.read_text(encoding="utf-8")
                await conn.executescript(sql_content)
                now_str = datetime.now(timezone.utc).isoformat()
                await conn.execute(
                    "INSERT INTO hera_schema_migrations (version, name, applied_at) VALUES (?, ?, ?)",
                    (version, sql_file.name, now_str),
                )
                await conn.commit()
                applied_now.append(sql_file.name)

        return applied_now
