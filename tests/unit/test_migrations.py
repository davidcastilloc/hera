"""Tests unitarios para el sistema de migraciones versionadas de SQLite."""

import pytest
from pathlib import Path
import aiosqlite
from hera.domain.database import Database
from hera.infrastructure.db.migrations.runner import MigrationRunner


@pytest.mark.asyncio
async def test_migration_runner_applies_incremental_versions(tmp_path: Path):
    """Verifica que MigrationRunner aplica scripts .sql y registra en hera_schema_migrations."""
    db_file = tmp_path / "test_migration.db"
    conn = await aiosqlite.connect(str(db_file))
    conn.row_factory = aiosqlite.Row

    runner = MigrationRunner()
    applied = await runner.run_migrations(conn)
    assert len(applied) >= 1
    assert "001_initial_schema.sql" in applied

    # Verificar que hera_schema_migrations contiene la versión 1
    cursor = await conn.execute("SELECT version, name FROM hera_schema_migrations")
    rows = await cursor.fetchall()
    assert len(rows) == 1
    assert rows[0][0] == 1
    assert rows[0][1] == "001_initial_schema.sql"

    # Segunda ejecución: no debe aplicar nada nuevo (idempotente)
    applied_second = await runner.run_migrations(conn)
    assert len(applied_second) == 0

    await conn.close()


@pytest.mark.asyncio
async def test_database_init_schema_uses_migration_runner(tmp_path: Path):
    """Verifica que Database.init_schema() inicializa correctamente las tablas vía MigrationRunner."""
    db_file = tmp_path / "test_db_init.db"
    db = Database(db_file)
    await db.init_schema()

    conn = await db.connect()
    cursor = await conn.execute("SELECT name FROM sqlite_master WHERE type='table'")
    tables = {r[0] for r in await cursor.fetchall()}
    assert "tracks" in tables
    assert "candidates" in tables
    assert "jobs" in tables
    assert "crates" in tables
    assert "hera_schema_migrations" in tables

    await db.close()
