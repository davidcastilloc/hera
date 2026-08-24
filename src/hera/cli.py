"""Interfaz de línea de comandos CLI de Hera."""

import asyncio
from pathlib import Path
import shutil
import subprocess
import sys
import click
import httpx
import zipfile
import io

from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.jobs.runner import JobRunner
from hera.mcp.server import create_mcp_server


def ensure_binaries(base_dir: Path):
    """Descarga automáticamente binarios auxiliares (slskd, fpcalc) si no existen."""
    bin_dir = base_dir / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)

    # 1. slskd
    slskd_exe = bin_dir / "slskd.exe"
    if not slskd_exe.exists():
        click.echo("[*] Descargando demonio Soulseek (slskd) para Windows...")
        try:
            url = "https://github.com/slskd/slskd/releases/download/0.26.0/slskd-0.26.0-win-x64.zip"
            resp = httpx.get(url, follow_redirects=True, timeout=60.0)
            if resp.status_code == 200:
                with zipfile.ZipFile(io.BytesIO(resp.content)) as z:
                    z.extractall(bin_dir)
                click.echo(f"[OK] slskd instalado en: {slskd_exe}")
        except Exception as e:
            click.echo(f"[WARN] No se pudo descargar slskd automaticamente: {e}")

    # 2. fpcalc
    fpcalc_exe = bin_dir / "fpcalc.exe"
    if not fpcalc_exe.exists():
        click.echo("[*] Descargando motor de huella acustica (fpcalc) para Windows...")
        try:
            url = "https://github.com/acoustid/chromaprint/releases/download/v1.5.1/chromaprint-fpcalc-1.5.1-windows-x86_64.zip"
            resp = httpx.get(url, follow_redirects=True, timeout=60.0)
            if resp.status_code == 200:
                with zipfile.ZipFile(io.BytesIO(resp.content)) as z:
                    for member in z.namelist():
                        if member.endswith("fpcalc.exe"):
                            with z.open(member) as source, open(fpcalc_exe, "wb") as target:
                                target.write(source.read())
                click.echo(f"[OK] fpcalc instalado en: {fpcalc_exe}")
        except Exception as e:
            click.echo(f"[WARN] No se pudo descargar fpcalc automaticamente: {e}")


@click.group()
def main():
    """Hera — Capa inteligente local para música y DJs."""
    pass


@main.command()
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
@click.option("--no-binaries", is_flag=True, help="Omitir descarga automática de slskd y fpcalc")
def setup(config: str, no_binaries: bool):
    """Inicializa directorios, plantilla de configuración, base de datos SQLite y herramientas externas."""
    cfg_path = Path(config)
    cfg_dir = cfg_path.parent
    base_dir = cfg_dir.parent if cfg_path.is_relative_to(Path(".")) else Path(".")
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

    # Descargar binarios auxiliares (slskd, fpcalc) si es necesario
    if not no_binaries:
        ensure_binaries(base_dir)

    # Cargar y resolver rutas
    cfg = HeraConfig.load(cfg_path).resolve_paths(base_dir)

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

    click.echo("\n[SUCCESS] Configuracion e inicializacion completada al 100%. Ejecuta 'hera doctor' para verificar.")


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
        found = shutil.which(tool_path) or (Path(tool_path).exists() and str(Path(tool_path).resolve()))
        if found:
            click.echo(f"[OK] Binario {tool_name}: disponible ({found})")
        else:
            click.echo(f"[WARN] Binario {tool_name}: no encontrado ({tool_path})")

    # 5. slskd (Soulseek Daemon)
    slskd_exe = Path("bin/slskd.exe")
    if slskd_exe.exists():
        click.echo(f"[OK] Demonio Soulseek (slskd): instalado en ({slskd_exe.resolve()})")
    else:
        click.echo("[WARN] Demonio Soulseek (slskd): no encontrado en bin/ (ejecuta 'hera setup')")

    # 6. Librerías de análisis
    try:
        import librosa
        click.echo(f"[OK] Motor acustico librosa: instalado ({librosa.__version__})")
    except ImportError:
        click.echo("[WARN] Motor acustico librosa: no instalado (opcional)")

    try:
        import mcp
        click.echo("[OK] MCP Python SDK: instalado")
    except ImportError:
        click.echo("[FAIL] MCP Python SDK: no instalado (requerido)")
        all_ok = False

    click.echo("\n" + ("[SUCCESS] Todos los chequeos criticos y herramientas auxiliares estan listos." if all_ok else "[WARN] Se detectaron advertencias."))


@main.command()
def slskd():
    """Inicia el demonio local de Soulseek (slskd)."""
    slskd_exe = Path("bin/slskd.exe")
    if not slskd_exe.exists():
        click.echo("[!] bin/slskd.exe no encontrado. Ejecuta 'hera setup' para descargarlo automáticamente.")
        return

    click.echo("[*] Iniciando slskd (Soulseek Daemon) en http://localhost:5030 ...")
    subprocess.run([str(slskd_exe.resolve())], cwd=str(slskd_exe.parent))


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
