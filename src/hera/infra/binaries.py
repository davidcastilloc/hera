"""Gestión y descarga automática de binarios externos auxiliares (slskd, fpcalc, rclone)."""

from pathlib import Path
import platform
import io
import zipfile
import httpx
import click


def ensure_binaries(base_dir: Path) -> None:
    """Descarga automáticamente binarios auxiliares (slskd, fpcalc, rclone) para Windows, Linux (x64/ARM64) o macOS."""
    bin_dir = base_dir / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)
    os_name = platform.system().lower()
    machine = platform.machine().lower()
    
    is_win = os_name == "windows"
    is_linux = os_name == "linux"
    is_mac = os_name == "darwin"
    is_arm64 = machine in {"aarch64", "arm64"}
    is_arm32 = machine.startswith("arm") and not is_arm64

    # 1. slskd (Soulseek Daemon)
    slskd_name = "slskd.exe" if is_win else "slskd"
    slskd_path = bin_dir / slskd_name
    if not slskd_path.exists():
        click.echo(f"[*] Descargando demonio Soulseek (slskd) para {os_name} ({machine})...")
        try:
            if is_win:
                url = "https://github.com/slskd/slskd/releases/download/0.26.0/slskd-0.26.0-win-x64.zip"
            elif is_linux:
                if is_arm64:
                    url = "https://github.com/slskd/slskd/releases/download/0.26.0/slskd-0.26.0-linux-arm64.zip"
                elif is_arm32:
                    url = "https://github.com/slskd/slskd/releases/download/0.26.0/slskd-0.26.0-linux-arm.zip"
                else:
                    url = "https://github.com/slskd/slskd/releases/download/0.26.0/slskd-0.26.0-linux-x64.zip"
            elif is_mac:
                url = "https://github.com/slskd/slskd/releases/download/0.26.0/slskd-0.26.0-osx-arm64.zip" if is_arm64 else "https://github.com/slskd/slskd/releases/download/0.26.0/slskd-0.26.0-osx-x64.zip"
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
        click.echo(f"[*] Verificando motor de huella acustica (fpcalc) para {os_name}...")
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
        click.echo(f"[*] Descargando motor de almacenamiento en la nube (rclone) para {os_name} ({machine})...")
        try:
            if is_win:
                url = "https://downloads.rclone.org/v1.69.1/rclone-v1.69.1-windows-amd64.zip"
            elif is_linux:
                if is_arm64:
                    url = "https://downloads.rclone.org/v1.69.1/rclone-v1.69.1-linux-arm64.zip"
                elif is_arm32:
                    url = "https://downloads.rclone.org/v1.69.1/rclone-v1.69.1-linux-arm-v7.zip"
                else:
                    url = "https://downloads.rclone.org/v1.69.1/rclone-v1.69.1-linux-amd64.zip"
            elif is_mac:
                url = "https://downloads.rclone.org/v1.69.1/rclone-v1.69.1-osx-arm64.zip" if is_arm64 else "https://downloads.rclone.org/v1.69.1/rclone-v1.69.1-osx-amd64.zip"
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

