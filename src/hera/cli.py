"""Interfaz de línea de comandos CLI de Hera."""

import asyncio
from pathlib import Path
import shutil
import sys
import click
from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.jobs.runner import JobRunner
from hera.mcp.server import create_mcp_server


@click.group()
def main():
    """Hera — Capa inteligente local para música y DJs."""
    pass


@main.command()
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def setup(config: str):
    """Inicializa directorios, plantilla de configuración y base de datos SQLite."""
    cfg_path = Path(config)
    cfg_dir = cfg_path.parent
    cfg_dir.mkdir(parents=True, exist_ok=True)

    example_path = cfg_dir / "hera.toml.example"
    if not cfg_path.exists():
        if example_path.exists():
            shutil.copy2(example_path, cfg_path)
            click.echo(f"[OK] Creado archivo de configuracion desde plantilla: {cfg_path}")
        else:
            default_config = HeraConfig()
            default_config.save(cfg_path)
            click.echo(f"[OK] Creado archivo de configuracion por defecto: {cfg_path}")
    else:
        click.echo(f"[OK] Configuracion existente detectada: {cfg_path}")

    # Cargar y resolver rutas
    cfg = HeraConfig.load(cfg_path).resolve_paths(cfg_dir.parent)

    # Crear directorios operativos
    for d_name, d_path in [
        ("Cuarentena", Path(cfg.quarantine_dir)),
        ("Biblioteca", Path(cfg.library_dir)),
        ("Exportaciones", Path(cfg.exports_dir)),
        ("Logs", Path(cfg.logs_dir)),
    ]:
        d_path.mkdir(parents=True, exist_ok=True)
        click.echo(f"[OK] Directorio de {d_name}: {d_path}")

    # Inicializar base de datos
    db = Database(cfg.db_path)
    asyncio.run(db.init_schema())
    click.echo(f"[OK] Base de datos SQLite inicializada: {cfg.db_path}")

    click.echo("\n[SUCCESS] Configuracion completada con exito. Ejecuta 'hera doctor' para verificar.")


@main.command()
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def doctor(config: str):
    """Diagnostica y verifica la salud de dependencias y componentes."""
    click.echo("[*] Ejecutando diagnostico de salud de Hera...\n")
    all_ok = True

    # 1. Versión de Python
    py_ver = sys.version_info
    if py_ver >= (3, 11):
        click.echo(f"[OK] Python: {py_ver.major}.{py_ver.minor}.{py_ver.micro} (>= 3.11)")
    else:
        click.echo(f"[FAIL] Python: {py_ver.major}.{py_ver.minor} (se requiere >= 3.11)")
        all_ok = False

    # 2. Configuración
    cfg_path = Path(config)
    if cfg_path.exists():
        click.echo(f"[OK] Archivo de configuracion: {cfg_path}")
        cfg = HeraConfig.load(cfg_path).resolve_paths(cfg_path.parent.parent)
    else:
        click.echo(f"[FAIL] Archivo de configuracion no encontrado: {cfg_path} (ejecuta 'hera setup')")
        return

    # 3. Base de datos
    db_p = Path(cfg.db_path)
    if db_p.exists():
        click.echo(f"[OK] Base de datos hera.db accesible: {db_p}")
    else:
        click.echo(f"[FAIL] Base de datos no encontrada: {db_p} (ejecuta 'hera setup')")
        all_ok = False

    # 4. Binarios de sistema
    for tool_name, tool_path in [
        ("ffmpeg", cfg.analysis.ffmpeg_path),
        ("ffprobe", cfg.analysis.ffprobe_path),
        ("fpcalc", cfg.analysis.fpcalc_path),
    ]:
        found = shutil.which(tool_path)
        if found:
            click.echo(f"[OK] Binario {tool_name}: disponible ({found})")
        else:
            click.echo(f"[WARN] Binario {tool_name}: no encontrado en PATH (se usara fallback)")

    # 5. Librerías de análisis
    try:
        import librosa
        click.echo(f"[OK] Motor acustico librosa: instalado ({librosa.__version__})")
    except ImportError:
        click.echo("[WARN] Motor acustico librosa: no instalado (opcional, fallback activo)")

    try:
        import mcp
        click.echo("[OK] MCP Python SDK: instalado")
    except ImportError:
        click.echo("[FAIL] MCP Python SDK: no instalado (requerido)")
        all_ok = False

    click.echo("\n" + ("[SUCCESS] Todos los chequeos criticos pasaron." if all_ok else "[WARN] Se detectaron advertencias o errores."))


@main.command()
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def serve(config: str):
    """Inicia el servidor MCP por stdio y el worker local de jobs."""
    cfg_path = Path(config)
    cfg = HeraConfig.load(cfg_path).resolve_paths(cfg_path.parent.parent if cfg_path.exists() else Path("."))
    db = Database(cfg.db_path)

    async def run_server_and_worker():
        await db.init_schema()
        runner = JobRunner(db, cfg)
        await runner.start()

        server = create_mcp_server(cfg_path)
        try:
            # Ejecutar transporte stdio
            await server.run_stdio_async()
        finally:
            await runner.stop()
            await db.close()

    asyncio.run(run_server_and_worker())


if __name__ == "__main__":
    main()
