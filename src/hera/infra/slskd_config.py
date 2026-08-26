"""Generador y gestor de configuración slskd.yml con soporte para Auto-Sharing (Buen Ciudadano P2P)."""

from pathlib import Path
import uuid
import yaml
from hera.domain.config import HeraConfig


def generate_slskd_config(
    hera_config: HeraConfig,
    credentials: dict | None = None,
    target_path: Path | None = None,
) -> str:
    """Genera la configuración slskd.yml con carpetas compartidas y límites de ancho de banda."""
    creds = credentials or {}
    username = creds.get("username", "hera_dj_2026")
    password = creds.get("password", "HeraDjGlobal2026!")
    description = creds.get(
        "description",
        hera_config.sharing.share_description if hasattr(hera_config, "sharing") else "Hera Curated Library"
    )

    api_key = creds.get("api_key", f"hera_key_{uuid.uuid4().hex[:12]}")

    shares_dirs = []
    sharing = getattr(hera_config, "sharing", None)
    if sharing and sharing.enabled:
        if sharing.share_library:
            shares_dirs.append({
                "alias": "Hera Curated Library",
                "path": "../library",
            })
        if sharing.share_sets:
            shares_dirs.append({
                "alias": "Hera DJ Sets",
                "path": "../sets",
            })
    else:
        # Default fallback shares
        shares_dirs.append({
            "alias": "Hera Curated Library",
            "path": "../library",
        })
        shares_dirs.append({
            "alias": "Hera DJ Sets",
            "path": "../sets",
        })

    upload_slots = sharing.max_upload_slots if sharing else 5
    upload_speed = sharing.max_upload_speed_kbps if sharing else 2048

    config_dict = {
        "soulseek": {
            "username": username,
            "password": password,
            "listen_port": 50300,
            "description": description,
        },
        "web": {
            "port": 5030,
            "authentication": {
                "disabled": False,
                "api_keys": [api_key],
            },
        },
        "directories": {
            "downloads": "../quarantine",
            "incomplete": "../quarantine/incomplete",
        },
        "shares": {
            "directories": shares_dirs,
        },
        "global": {
            "upload": {
                "slots": upload_slots,
                "speed_limit": upload_speed,
            },
        },
    }

    yaml_text = yaml.dump(config_dict, sort_keys=False, default_flow_style=False, allow_unicode=True)

    if target_path:
        target = Path(target_path)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(yaml_text, encoding="utf-8")

    return yaml_text


def update_shared_directories(
    target_path: Path,
    library_path: str = "../library",
    sets_path: str = "../sets",
) -> bool:
    """Actualiza o añade las carpetas de biblioteca y sets en un slskd.yml existente."""
    if not target_path.exists():
        return False

    try:
        content = target_path.read_text(encoding="utf-8")
        data = yaml.safe_load(content) or {}

        if "shares" not in data or not isinstance(data["shares"], dict):
            data["shares"] = {}

        data["shares"]["directories"] = [
            {"alias": "Hera Curated Library", "path": library_path},
            {"alias": "Hera DJ Sets", "path": sets_path},
        ]

        target_path.write_text(yaml.dump(data, sort_keys=False, default_flow_style=False), encoding="utf-8")
        return True
    except Exception:
        return False

