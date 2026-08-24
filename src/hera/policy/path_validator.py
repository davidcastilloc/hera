"""Validación y sanitización de rutas para evitar path traversal y caracteres peligrosos."""

from pathlib import Path
import re


# Caracteres prohibidos en nombres de archivo (Windows + POSIX)
FORBIDDEN_CHARS_REGEX = re.compile(r'[<>:"/\\|?*\x00-\x1f]')


def sanitize_filename(name: str, max_length: int = 255) -> str:
    """Limpia caracteres peligrosos de un nombre de archivo."""
    sanitized = FORBIDDEN_CHARS_REGEX.sub("_", name)
    sanitized = sanitized.strip(". ")
    if not sanitized:
        sanitized = "unnamed_track"
    return sanitized[:max_length]


def validate_path_safety(base_directory: Path | str, target_path: Path | str) -> bool:
    """Verifica que target_path se encuentre estrictamente dentro de base_directory."""
    base = Path(base_directory).resolve()
    target = Path(target_path).resolve()
    try:
        target.relative_to(base)
        return True
    except ValueError:
        return False
