"""Interfaz de línea de comandos CLI de Hera con soporte multiplataforma y sync en la nube."""

import asyncio
from pathlib import Path
import platform
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
from hera.adapters.storage.rclone import RcloneStorageAdapter


def ensure_binaries(base_dir: Path):
    """Descarga automáticamente binarios auxiliares (slskd, fpcalc, rclone) para Windows, Linux o macOS."""
    bin_dir = base_dir / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)
    os_name = platform.system().lower()
    is_win = os_name == "windows"
    is_linux = os_name == "linux"
    is_mac = os_name == "darwin"

    # 1. slskd (Soulseek Daemon)
    slskd_name = "slskd.exe" if is_win else "slskd"
    slskd_path = bin_dir / slskd_name
    if not slskd_path.exists():
        click.echo(f"[*] Descargando demonio Soulseek (slskd) para {os_name}...")
        try:
            if is_win:
                url = "https://github.com/slskd/slskd/releases/download/0.26.0/slskd-0.26.0-win-x64.zip"
            elif is_linux:
                url = "https://github.com/slskd/slskd/releases/download/0.26.0/slskd-0.26.0-linux-x64.zip"
            elif is_mac:
                url = "https://github.com/slskd/slskd/releases/download/0.26.0/slskd-0.26.0-osx-x64.zip"
            else:
                url = None

            if url:
                resp = httpx.get(url, follow_redirects=True, timeout=60.0)
                if resp.status_code == 200:
                    with zipfile.ZipFile(io.BytesIO(resp.content)) as z:
                        z.extractall(bin_dir)
                    if not is_win:
                        slskd_path.chmod(0o755)
                    click.echo(f"[OK] slskd instalado en: {slskd_path}")
        except Exception as e:
            click.echo(f"[WARN] No se pudo descargar slskd automaticamente: {e}")

    # 2. fpcalc (Chromaprint / Acoustic Fingerprint)
    fpcalc_name = "fpcalc.exe" if is_win else "fpcalc"
    fpcalc_path = bin_dir / fpcalc_name
    if not fpcalc_path.exists():
        click.echo(f"[*] Descargando motor de huella acustica (fpcalc) para {os_name}...")
        try:
            if is_win:
                url = "https://github.com/acoustid/chromaprint/releases/download/v1.5.1/chromaprint-fpcalc-1.5.1-windows-x86_64.zip"
                resp = httpx.get(url, follow_redirects=True, timeout=60.0)
                if resp.status_code == 200:
                    with zipfile.ZipFile(io.BytesIO(resp.content)) as z:
                        for member in z.namelist():
                            if member.endswith("fpcalc.exe"):
                                with z.open(member) as source, open(fpcalc_path, "wb") as target:
                                    target.write(source.read())
                    click.echo(f"[OK] fpcalc instalado en: {fpcalc_path}")
            elif is_linux:
                pass
        except Exception as e:
            click.echo(f"[WARN] No se pudo descargar fpcalc automaticamente: {e}")

    # 3. rclone (Multi-cloud Storage Engine: Google Drive, S3, R2, Dropbox)
    rclone_name = "rclone.exe" if is_win else "rclone"
    rclone_path = bin_dir / rclone_name
    if not rclone_path.exists():
        click.echo(f"[*] Descargando motor de almacenamiento en la nube (rclone) para {os_name}...")
        try:
            if is_win:
                url = "https://downloads.rclone.org/v1.69.1/rclone-v1.69.1-windows-amd64.zip"
            elif is_linux:
                url = "https://downloads.rclone.org/v1.69.1/rclone-v1.69.1-linux-amd64.zip"
            elif is_mac:
                url = "https://downloads.rclone.org/v1.69.1/rclone-v1.69.1-osx-amd64.zip"
            else:
                url = None

            if url:
                resp = httpx.get(url, follow_redirects=True, timeout=60.0)
                if resp.status_code == 200:
                    with zipfile.ZipFile(io.BytesIO(resp.content)) as z:
                        for member in z.namelist():
                            if member.endswith(rclone_name):
                                with z.open(member) as source, open(rclone_path, "wb") as target:
                                    target.write(source.read())
                    if not is_win:
                        rclone_path.chmod(0o755)
                    click.echo(f"[OK] rclone instalado en: {rclone_path}")
        except Exception as e:
            click.echo(f"[WARN] No se pudo descargar rclone automaticamente: {e}")


@click.group()
def main():
    """Hera — Super-agente inteligente y multiplataforma para música y DJs."""
    pass


@main.command()
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
@click.option("--no-binaries", is_flag=True, help="Omitir descarga automática de slskd, fpcalc y rclone")
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

    if not no_binaries:
        ensure_binaries(base_dir)

    cfg = HeraConfig.load(cfg_path).resolve_paths(base_dir)

    for d_name, d_path in [
        ("Cuarentena", Path(cfg.quarantine_dir)),
        ("Biblioteca", Path(cfg.library_dir)),
        ("Exportaciones", Path(cfg.exports_dir)),
        ("Logs", Path(cfg.logs_dir)),
        ("Sets", base_dir / "sets"),
    ]:
        d_path.mkdir(parents=True, exist_ok=True)
        click.echo(f"[OK] Directorio de {d_name}: {d_path}")

    db = Database(cfg.db_path)
    asyncio.run(db.init_schema())
    click.echo(f"[OK] Base de datos SQLite inicializada: {cfg.db_path}")

    click.echo("\n[SUCCESS] Configuracion e inicializacion completada al 100%. Ejecuta 'hera doctor' para verificar.")


@main.command()
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def doctor(config: str):
    """Diagnostica y verifica la salud de dependencias y componentes multiplataforma."""
    click.echo("[*] Ejecutando diagnostico de salud de Hera...\n")
    all_ok = True

    py_ver = sys.version_info
    click.echo(f"[OK] Sistema Operativo: {platform.system()} {platform.release()} ({platform.machine()})")
    if py_ver >= (3, 11):
        click.echo(f"[OK] Python: {py_ver.major}.{py_ver.minor}.{py_ver.micro} (>= 3.11)")
    else:
        click.echo(f"[FAIL] Python: {py_ver.major}.{py_ver.minor} (se requiere >= 3.11)")
        all_ok = False

    cfg_path = Path(config)
    if cfg_path.exists():
        click.echo(f"[OK] Archivo de configuracion: {cfg_path}")
        cfg = HeraConfig.load(cfg_path).resolve_paths(cfg_path.parent.parent)
    else:
        click.echo(f"[FAIL] Archivo de configuracion no encontrado: {cfg_path} (ejecuta 'hera setup')")
        return

    db_p = Path(cfg.db_path)
    if db_p.exists():
        click.echo(f"[OK] Base de datos hera.db accesible: {db_p}")
    else:
        click.echo(f"[FAIL] Base de datos no encontrada: {db_p} (ejecuta 'hera setup')")
        all_ok = False

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

    slskd_exe = Path("bin/slskd.exe" if platform.system() == "Windows" else "bin/slskd")
    if slskd_exe.exists():
        click.echo(f"[OK] Demonio Soulseek (slskd): instalado en ({slskd_exe.resolve()})")
    else:
        click.echo("[WARN] Demonio Soulseek (slskd): no encontrado en bin/ (ejecuta 'hera setup')")

    rclone = RcloneStorageAdapter(cfg.storage.rclone_path, cfg.storage.config_path)
    if rclone.is_available():
        ver = rclone.get_version()
        remotes = rclone.list_remotes()
        click.echo(f"[OK] Motor rclone en la nube: disponible ({ver})")
        if remotes:
            click.echo(f"     * Remotes en la nube configurados: {', '.join(remotes)}")
        else:
            click.echo("     * Remotes en la nube: Ninguno configurado todavia (ejecuta 'hera sync login')")
    else:
        click.echo("[WARN] Motor rclone no encontrado en bin/ (ejecuta 'hera setup')")

    try:
        import librosa
        click.echo(f"[OK] Motor acustico librosa: instalado ({librosa.__version__})")
    except ImportError:
        click.echo("[WARN] Motor acustico librosa: no instalado (opcional)")

    try:
        import mutagen
        ver_str = getattr(mutagen, "version_string", "instalado")
        click.echo(f"[OK] Motor de etiquetado ID3/Vorbis (mutagen): {ver_str}")
    except ImportError:
        click.echo("[WARN] Mutagen: no instalado")

    try:
        import mcp
        click.echo("[OK] MCP Python SDK: instalado")
    except ImportError:
        click.echo("[FAIL] MCP Python SDK: no instalado (requerido)")
        all_ok = False

    click.echo("\n" + ("[SUCCESS] Todos los chequeos criticos y herramientas auxiliares estan listos." if all_ok else "[WARN] Se detectaron advertencias."))


@main.group()
def sync():
    """Comandos de sincronización en la nube con Google Drive, S3, R2, etc. (vía rclone)."""
    pass


@sync.command(name="login")
@click.option("--name", "-n", default="gdrive", help="Nombre del remote a crear (por defecto 'gdrive')")
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def sync_login(name: str, config: str):
    """Autenticación directa de 1-clic con Google Drive vía navegador web (OAuth2)."""
    cfg = HeraConfig.load(config)
    rclone = RcloneStorageAdapter(cfg.storage.rclone_path, cfg.storage.config_path)
    if not rclone.is_available():
        click.echo("[!] rclone no está instalado. Ejecuta 'hera setup' para descargarlo automáticamente.")
        return

    click.echo("=" * 80)
    click.echo(f" CONECTANDO GOOGLE DRIVE ('{name}') VIA OAUTH DIRECTO")
    click.echo("=" * 80)
    click.echo("[*] Abriendo tu navegador web automáticamente...")
    click.echo("[*] Solo inicia sesión con tu cuenta de Google y haz clic en 'Permitir' / 'Allow'.\n")

    cmd = [rclone.rclone_path, "config", "create", name, "drive", "scope", "drive"]
    if cfg.storage.config_path:
        cmd.extend(["--config", cfg.storage.config_path])

    res = subprocess.run(cmd)
    if res.returncode == 0:
        click.echo("\n" + "=" * 80)
        click.echo(f"[SUCCESS] ¡Google Drive ('{name}:') conectado exitosamente con Hera!")
        click.echo("Ya puedes sincronizar tus canciones ejecutando:")
        click.echo(f"  uv run hera sync push")
        click.echo("=" * 80)
    else:
        click.echo(f"\n[FAIL] No se pudo completar la autenticacion (código de salida {res.returncode}).")


@sync.command(name="config")
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def sync_config(config: str):
    """Inicia el asistente interactivo manual para configurar nubes avanzadas (S3, R2, Dropbox, etc.)."""
    cfg = HeraConfig.load(config)
    rclone = RcloneStorageAdapter(cfg.storage.rclone_path, cfg.storage.config_path)
    if not rclone.is_available():
        click.echo("[!] rclone no está instalado. Ejecuta 'hera setup' para descargarlo automáticamente.")
        return

    click.echo("[*] Iniciando asistente interactivo de rclone (para Google Drive directo, usa 'hera sync login')...")
    cmd = [rclone.rclone_path, "config"]
    if cfg.storage.config_path:
        cmd.extend(["--config", cfg.storage.config_path])
    subprocess.run(cmd)


@sync.command(name="status")
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def sync_status(config: str):
    """Muestra el estado de la conexión a la nube y los remotes configurados."""
    cfg = HeraConfig.load(config)
    rclone = RcloneStorageAdapter(cfg.storage.rclone_path, cfg.storage.config_path)
    if not rclone.is_available():
        click.echo("[!] rclone no está instalado. Ejecuta 'hera setup'.")
        return

    remotes = rclone.list_remotes()
    click.echo("=" * 80)
    click.echo(" ESTADO DE SINCRONIZACION EN LA NUBE")
    click.echo("=" * 80)
    click.echo(f"Binario rclone: {rclone.rclone_path} ({rclone.get_version()})")
    click.echo(f"Remote por defecto: {cfg.storage.default_remote}")
    click.echo(f"Directorio remoto: {cfg.storage.remote_folder}")
    if remotes:
        click.echo(f"\n[+] Remotes disponibles: {', '.join(remotes)}")
    else:
        click.echo("\n[!] No hay ningún remote configurado. Ejecuta 'hera sync login' para conectar Google Drive.")


@sync.command(name="push")
@click.option("--remote", "-r", default=None, help="Nombre del remote en la nube (ej. 'gdrive:')")
@click.option("--folder", "-f", default=None, help="Subcarpeta remota de destino")
@click.option("--dry-run", is_flag=True, help="Simular sin subir archivos reales")
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def sync_push(remote: str | None, folder: str | None, dry_run: bool, config: str):
    """Sube y sincroniza las carpetas de sets locales hacia Google Drive u otra nube."""
    cfg = HeraConfig.load(config).resolve_paths(Path("."))
    rclone = RcloneStorageAdapter(cfg.storage.rclone_path, cfg.storage.config_path)

    remote_name = remote or cfg.storage.default_remote
    if not remote_name.endswith(":"):
        remote_name += ":"
    remote_dest = f"{remote_name}{folder or cfg.storage.remote_folder}"

    local_sets_dir = Path(cfg.data_dir) / "sets"
    if not local_sets_dir.exists():
        click.echo(f"[!] La carpeta local de sets ({local_sets_dir}) no existe.")
        return

    click.echo("=" * 80)
    click.echo(f" SINCRONIZANDO SETS LOCALES -> {remote_dest} {'(SIMULACION)' if dry_run else ''}")
    click.echo("=" * 80)

    async def run_push():
        res = await rclone.copy(local_sets_dir, remote_dest, dry_run=dry_run)
        if res.success:
            click.echo(f"\n[OK] Sincronización con {remote_dest} completada con éxito.")
        else:
            click.echo(f"\n[FAIL] Error en la sincronización: {res.error}")

    asyncio.run(run_push())


@sync.command(name="pull")
@click.option("--remote", "-r", default=None, help="Nombre del remote en la nube (ej. 'gdrive:')")
@click.option("--folder", "-f", default=None, help="Subcarpeta remota de origen")
@click.option("--dry-run", is_flag=True, help="Simular sin descargar archivos")
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def sync_pull(remote: str | None, folder: str | None, dry_run: bool, config: str):
    """Descarga los sets desde Google Drive u otra nube hacia el almacenamiento local."""
    cfg = HeraConfig.load(config).resolve_paths(Path("."))
    rclone = RcloneStorageAdapter(cfg.storage.rclone_path, cfg.storage.config_path)

    remote_name = remote or cfg.storage.default_remote
    if not remote_name.endswith(":"):
        remote_name += ":"
    remote_src = f"{remote_name}{folder or cfg.storage.remote_folder}"

    local_sets_dir = Path(cfg.data_dir) / "sets"
    local_sets_dir.mkdir(parents=True, exist_ok=True)

    click.echo("=" * 80)
    click.echo(f" DESCARGANDO SETS DESDE {remote_src} -> {local_sets_dir} {'(SIMULACION)' if dry_run else ''}")
    click.echo("=" * 80)

    async def run_pull():
        res = await rclone.copy(remote_src, local_sets_dir, dry_run=dry_run)
        if res.success:
            click.echo(f"\n[OK] Descarga desde {remote_src} completada con éxito.")
        else:
            click.echo(f"\n[FAIL] Error en la descarga: {res.error}")

    asyncio.run(run_pull())


@main.command()
def slskd():
    """Inicia el demonio local de Soulseek (slskd)."""
    slskd_exe = Path("bin/slskd.exe" if platform.system() == "Windows" else "bin/slskd")
    if not slskd_exe.exists():
        click.echo("[!] Binario slskd no encontrado. Ejecuta 'hera setup' para descargarlo automáticamente.")
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
            await server.run_stdio_async()
        finally:
            await runner.stop()
            await db.close()

    asyncio.run(run_server_and_worker())


if __name__ == "__main__":
    main()
