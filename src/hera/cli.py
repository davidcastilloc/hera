"""Interfaz de línea de comandos CLI de Hera con soporte multiplataforma, sync en la nube y zero-touch P2P."""

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

# Asegurar codificación UTF-8 en consolas Windows
if sys.stdout and hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
if sys.stderr and hasattr(sys.stderr, "reconfigure"):
    try:
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass


from hera.domain.config import HeraConfig
from hera.domain.database import Database
from hera.domain.community import CommunityStats
from hera.infra.lifecycle import SlskdLifecycle
from hera.infra.slskd_config import generate_slskd_config, update_shared_directories
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
    """Hera — Super-agente inteligente, zero-touch y multiplataforma para DJs y música."""
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

    # 1. Configurar slskd.yml con Auto-Sharing
    slskd_yml = base_dir / "bin" / "slskd.yml"
    if not slskd_yml.exists():
        generate_slskd_config(cfg, target_path=slskd_yml)
        click.echo(f"[OK] Configuración Soulseek creada con Auto-Sharing: {slskd_yml}")
    else:
        update_shared_directories(slskd_yml)
        click.echo("[OK] Configuración Soulseek verificada con Auto-Sharing de biblioteca.")

    for d_name, d_path in [
        ("Cuarentena", Path(cfg.quarantine_dir)),
        ("Biblioteca", Path(cfg.library_dir)),
        ("Exportaciones", Path(cfg.exports_dir)),
        ("Logs", Path(cfg.logs_dir)),
        ("Sets", base_dir / "sets"),
        ("Music Inbox", base_dir / "music_inbox"),
    ]:
        d_path.mkdir(parents=True, exist_ok=True)
        click.echo(f"[OK] Directorio de {d_name}: {d_path}")

    db = Database(cfg.db_path)
    asyncio.run(db.init_schema())
    click.echo(f"[OK] Base de datos SQLite inicializada: {cfg.db_path}")

    click.echo("\n[SUCCESS] Configuración e inicialización completada al 100%.")
    click.echo("💡 Puedes iniciar el agente conversacional directamente con: 'hera chat'")


@main.command()
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def status(config: str):
    """Muestra el estado consolidado de salud, backend IA y estadísticas de comunidad P2P."""
    cfg_path = Path(config)
    cfg = HeraConfig.load(cfg_path).resolve_paths(Path("."))

    click.echo("=" * 80)
    click.echo(" 🎧 HERA AI SYSTEM & COMMUNITY DASHBOARD")
    click.echo("=" * 80)

    # 1. Sistema
    py_ver = sys.version_info
    click.echo(f"🖥️  OS: {platform.system()} {platform.release()} ({platform.machine()}) | Python {py_ver.major}.{py_ver.minor}.{py_ver.micro}")

    # 2. Base de datos
    db_p = Path(cfg.db_path)
    db_ok = "🟢 Conectada" if db_p.exists() else "🔴 No encontrada"
    click.echo(f"🗄️  Base de Datos: {db_p.name} ({db_ok})")

    # 3. Soulseek & Comunidad
    stats = CommunityStats(cfg.providers.slskd_url or "http://localhost:5030")
    summary = asyncio.run(stats.get_sharing_summary(cfg.library_dir, Path(cfg.data_dir) / "sets"))

    slskd_state = "🟢 EN LÍNEA (5030)" if summary["is_live"] else "🟡 LOCAL (inactivo)"
    click.echo(f"🌐 Soulseek P2P: {slskd_state}")
    click.echo(f"📦 Biblioteca Compartida: {summary['tracks_shared']} tracks curados ({summary['total_size_gb']:.2f} GB)")
    if summary["uploads_count"] > 0:
        click.echo(f"🤝 Colaboración: {summary['uploads_count']} transferencias ({summary['uploads_gb']:.2f} GB) a {summary['unique_peers_served']} DJs")

    # 4. Cloud
    rclone = RcloneStorageAdapter(cfg.storage.rclone_path, cfg.storage.config_path)
    if rclone.is_available():
        remotes = rclone.list_remotes()
        cloud_str = f"🟢 Disponible ({', '.join(remotes) if remotes else 'sin remotes'})"
    else:
        cloud_str = "🟡 No instalado"
    click.echo(f"☁️  Cloud Sync (rclone): {cloud_str}")

    click.echo("=" * 80)



@main.command()
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def community(config: str):
    """Muestra el impacto y contribución de tu nodo a la red comunitaria Soulseek."""
    cfg = HeraConfig.load(config).resolve_paths(Path("."))
    stats = CommunityStats(cfg.providers.slskd_url or "http://localhost:5030")
    summary = asyncio.run(stats.get_sharing_summary(cfg.library_dir, Path(cfg.data_dir) / "sets"))
    click.echo("=" * 80)
    click.echo(" 🌍 HERA — IMPACTO COMUNITARIO (BUEN CIUDADANO P2P)")
    click.echo("=" * 80)
    click.echo(summary["community_message"])
    click.echo("=" * 80)


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



@main.group()
def slskd():
    """Gestión y control del demonio P2P Soulseek (slskd)."""
    pass


@slskd.command(name="start")
def slskd_start():
    """Inicia el demonio slskd en segundo plano."""
    cfg = HeraConfig.load("config/hera.toml").resolve_paths(Path("."))
    lifecycle = SlskdLifecycle(cfg)
    if lifecycle.is_running_sync():
        click.echo("[OK] slskd ya está en ejecución en http://localhost:5030")
        return
    click.echo("[*] Iniciando slskd en segundo plano...")
    ok = lifecycle.ensure_running_sync()
    if ok:
        click.echo("[OK] slskd iniciado con éxito en http://localhost:5030")
    else:
        click.echo("[FAIL] No se pudo iniciar slskd automáticamente.")


@slskd.command(name="stop")
def slskd_stop():
    """Detiene las instancias activas de slskd."""
    cfg = HeraConfig.load("config/hera.toml").resolve_paths(Path("."))
    lifecycle = SlskdLifecycle(cfg)
    lifecycle.stop()
    click.echo("[OK] Solicitud de parada enviada a slskd.")


@slskd.command(name="status")
def slskd_status():
    """Verifica si slskd está respondiendo en el puerto configurado."""
    cfg = HeraConfig.load("config/hera.toml").resolve_paths(Path("."))
    lifecycle = SlskdLifecycle(cfg)
    if lifecycle.is_running_sync():
        click.echo("🟢 slskd está ACTIVO y respondiendo en http://localhost:5030")
    else:
        click.echo("🟡 slskd está DETENIDO.")


@main.command()
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def desktop(config: str):
    """Inicia el icono de System Tray de Hera en la barra de tareas (100% visual y human-friendly)."""
    from hera.desktop.tray import run_tray_app
    run_tray_app(config)


@main.command()
@click.option("--config", "-c", default="config/hera.toml", help="Path to config file")
def serve(config: str):
    """Start the MCP stdio server (for AI agent integration). Auto-starts slskd if available."""
    import logging
    import warnings

    # MCP stdio requires clean stdout/stderr — suppress all logging noise
    logging.disable(logging.CRITICAL)
    warnings.filterwarnings("ignore")

    # Resolve project root from the config file location
    cfg_path = Path(config)
    if not cfg_path.is_absolute():
        # When invoked via `uv run --project <path>`, CWD is the project root
        cfg_path = Path.cwd() / config
    project_root = cfg_path.parent.parent if cfg_path.exists() else Path.cwd()

    cfg = HeraConfig.load(cfg_path).resolve_paths(project_root)
    db = Database(cfg.db_path)

    async def run_server_and_worker():
        await db.init_schema()

        # Try to start slskd, but don't fail if unavailable
        slskd_started = False
        try:
            lifecycle = SlskdLifecycle(cfg)
            slskd_started = await lifecycle.ensure_running(base_dir=project_root)
        except Exception:
            pass

        runner = JobRunner(db, cfg)
        await runner.start()

        server = create_mcp_server(cfg_path)
        try:
            await server.run_stdio_async()
        finally:
            await runner.stop()
            if slskd_started:
                try:
                    lifecycle.stop()
                except Exception:
                    pass
            await db.close()

    asyncio.run(run_server_and_worker())


# ─── Standalone Tool Commands (No LLM Required) ─────────────────────────────

@main.command()
@click.argument("queries", nargs=-1, required=True)
def search(queries):
    """Search & download tracks from Soulseek P2P (auto-starts backend if needed).

    Example: hera search "Daft Punk One More Time" "Modjo Lady"
    """
    cfg = HeraConfig.load("config/hera.toml").resolve_paths(Path("."))
    lifecycle = SlskdLifecycle(cfg)
    lifecycle.ensure_running_sync()

    from hera.agent.tools import search_and_acquire_tracks
    async def run():
        result = await search_and_acquire_tracks(list(queries))
        click.echo(result)
    asyncio.run(run())


@main.command()
def library():
    """Show inventory of all tracks and DJ sets on disk (no LLM needed)."""
    from hera.agent.tools import get_library_status
    click.echo(get_library_status())


@main.command(name="set")
@click.argument("set_name")
@click.argument("tracks", nargs=-1, required=True)
def create_set(set_name, tracks):
    """Build a DJ set from library tracks (no LLM needed).

    Example: hera set "My Set" "Modjo" "Daft Punk" "Bob Sinclar"
    """
    from hera.agent.tools import create_or_update_dj_set
    async def run():
        result = await create_or_update_dj_set(set_name, list(tracks))
        click.echo(result)
    asyncio.run(run())


@main.command()
@click.argument("key")
@click.argument("bpm", type=float)
def camelot(key, bpm):
    """Get harmonic mixing recommendations from the Camelot Wheel (no LLM needed).

    Example: hera camelot 8A 128.0
    """
    from hera.agent.tools import recommend_harmonic_transitions
    click.echo(recommend_harmonic_transitions(key, bpm))


@main.command()
@click.option("--backend", "-b", default=None,
              type=click.Choice(["auto", "gemini", "vertex", "openai", "anthropic",
                                 "ollama", "lmstudio", "jan", "llamacpp", "vllm",
                                 "localai", "mlx", "custom"], case_sensitive=False),
              help="LLM backend to use (default: auto-detect)")
@click.option("--model", "-m", default=None, help="Model name (provider-specific)")
@click.option("--base-url", default=None, help="Custom endpoint URL (for 'custom' backend)")
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def chat(backend, model, base_url, config):
    """Start the interactive conversational agent powered by the Antigravity SDK."""
    cfg = HeraConfig.load(config).resolve_paths(Path("."))
    lifecycle = SlskdLifecycle(cfg)
    slskd_live = lifecycle.ensure_running_sync()
    if slskd_live:
        click.echo("🟢 Soulseek P2P conectado y auto-compartiendo biblioteca en segundo plano.")

    from hera.agent.brain import run_hera_interactive_chat
    try:
        asyncio.run(run_hera_interactive_chat(backend=backend, model=model, base_url=base_url, config_path=config))
    finally:
        pass


@main.command()
@click.option("--backend", "-b", default=None,
              type=click.Choice(["auto", "gemini", "vertex", "openai", "anthropic",
                                 "ollama", "lmstudio", "jan", "llamacpp", "vllm",
                                 "localai", "mlx", "custom"], case_sensitive=False),
              help="LLM backend to use (default: auto-detect)")
@click.option("--model", "-m", default=None, help="Model name (provider-specific)")
@click.option("--base-url", default=None, help="Custom endpoint URL (for 'custom' backend)")
@click.option("--config", "-c", default="config/hera.toml", help="Ruta al archivo de configuración")
def agent(backend, model, base_url, config):
    """Alias for 'hera chat' — Start the autonomous Hera agent."""
    cfg = HeraConfig.load(config).resolve_paths(Path("."))
    lifecycle = SlskdLifecycle(cfg)
    slskd_live = lifecycle.ensure_running_sync()
    if slskd_live:
        click.echo("🟢 Soulseek P2P conectado y auto-compartiendo biblioteca en segundo plano.")

    from hera.agent.brain import run_hera_interactive_chat
    try:
        asyncio.run(run_hera_interactive_chat(backend=backend, model=model, base_url=base_url, config_path=config))
    finally:
        pass


@main.command()
@click.option("--port", "-p", default=8501, help="Puerto para el servidor web de la UI (default: 8501)")
@click.option("--no-browser", is_flag=True, help="No abrir automáticamente el navegador")
def ui(port, no_browser):
    """Inicia la interfaz gráfica moderna para DJs en el navegador web."""
    import subprocess
    import sys

    cfg = HeraConfig.load("config/hera.toml").resolve_paths(Path("."))
    lifecycle = SlskdLifecycle(cfg)
    slskd_live = lifecycle.ensure_running_sync()
    if slskd_live:
        click.echo("🟢 Soulseek P2P conectado y auto-compartiendo biblioteca en segundo plano.")

    click.echo(f"🎧 Iniciando Hera DJ Studio Web UI en http://localhost:{port}...")

    app_path = Path(__file__).parent / "ui" / "app.py"
    cmd = [
        sys.executable,
        "-m",
        "streamlit",
        "run",
        str(app_path),
        f"--server.port={port}",
        "--browser.gatherUsageStats=false",
        "--theme.base=dark",
        "--theme.primaryColor=#00f2fe",
        "--theme.backgroundColor=#0b0e14",
        "--theme.secondaryBackgroundColor=#141b2d",
        "--theme.textColor=#e0e6ed",
    ]
    if no_browser:
        cmd.append("--server.headless=true")

    try:
        subprocess.run(cmd)
    except KeyboardInterrupt:
        click.echo("\n[*] Hera UI cerrada.")


if __name__ == "__main__":
    main()



