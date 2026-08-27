"""Bandeja de sistema (System Tray Icon) para Hera — Control visual 100% human-friendly."""

import sys
import webbrowser
from pathlib import Path
import threading
import subprocess

try:
    from PIL import Image, ImageDraw
    import pystray
    HAS_TRAY = True
except ImportError:
    HAS_TRAY = False

from hera.domain.config import HeraConfig
from hera.infra.lifecycle import SlskdLifecycle
from hera.domain.community import CommunityStats


def create_tray_image():
    """Genera un icono de auriculares 🎧 estilizado para la barra de tareas."""
    if not HAS_TRAY:
        return None
    width = 64
    height = 64
    image = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)

    # Círculo de fondo oscuro / turquesa
    draw.ellipse((4, 4, 60, 60), fill=(24, 24, 27, 255), outline=(16, 185, 129, 255), width=3)

    # Diadema de auriculares
    draw.arc((14, 12, 50, 48), start=180, end=0, fill=(16, 185, 129, 255), width=4)

    # Almohadillas izquierda y derecha
    draw.rounded_rectangle((12, 30, 22, 48), radius=3, fill=(16, 185, 129, 255))
    draw.rounded_rectangle((42, 30, 52, 48), radius=3, fill=(16, 185, 129, 255))

    return image


def run_tray_app(config_path: Path | str = "config/hera.toml"):
    """Inicia la aplicación de bandeja del sistema de Hera."""
    if not HAS_TRAY:
        print("[!] Las dependencias de System Tray no estan instaladas. Ejecuta 'uv sync --extra desktop'")
        return

    cfg = HeraConfig.load(config_path).resolve_paths(Path("."))
    lifecycle = SlskdLifecycle(cfg)

    # Iniciar slskd de fondo
    lifecycle.ensure_running_sync()

    def open_folder(folder_path: Path):
        folder_path.mkdir(parents=True, exist_ok=True)
        if sys.platform == "win32":
            subprocess.run(["explorer", str(folder_path.resolve())])
        elif sys.platform == "darwin":
            subprocess.run(["open", str(folder_path.resolve())])
        else:
            subprocess.run(["xdg-open", str(folder_path.resolve())])

    def open_web_dashboard(icon, item):
        webbrowser.open(cfg.providers.slskd_url or "http://localhost:5030")

    def open_inbox(icon, item):
        open_folder(Path(cfg.data_dir) / "music_inbox")

    def open_library(icon, item):
        open_folder(Path(cfg.library_dir))

    def open_sets(icon, item):
        open_folder(Path(cfg.data_dir) / "sets")

    def quit_app(icon, item):
        lifecycle.stop()
        icon.stop()

    menu_items = [
        pystray.MenuItem("🎧 Hera AI — En línea", None, enabled=False),
        pystray.Menu.SEPARATOR,
        pystray.MenuItem("📥 Abrir Music Inbox", open_inbox),
        pystray.MenuItem("🎵 Abrir Biblioteca Curada", open_library),
        pystray.MenuItem("🎛️ Abrir DJ Sets / Crates", open_sets),
        pystray.MenuItem("🌐 Abrir Panel Web Soulseek", open_web_dashboard),
        pystray.Menu.SEPARATOR,
        pystray.MenuItem("❌ Salir de Hera", quit_app),
    ]

    img = create_tray_image()
    if img is not None:
        icon = pystray.Icon("hera", img, "Hera — Super-Agente DJ", menu=pystray.Menu(*menu_items))
        print("[*] Hera System Tray iniciado. Icono disponible en la barra de tareas.")
        icon.run()

