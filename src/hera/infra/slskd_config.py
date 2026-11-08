"""Generador y gestor de configuración slskd.yml con soporte para Auto-Sharing (Buen Ciudadano P2P)."""

from pathlib import Path
import uuid
import yaml  # type: ignore[import-untyped]
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
            "listen_port": 2234,
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
        "transfers": {
            "global": {
                "upload": {
                    "slots": upload_slots,
                    "speed_limit": upload_speed,
                },
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


def ensure_runtime_slskd_config(
    hera_config: HeraConfig,
    base_dir: Path | None = None,
    target_path: Path | None = None,
) -> Path:
    """Genera o actualiza un slskd.yml 100% compatible con slskd >= 0.26 usando rutas absolutas nativas."""
    root = (base_dir or Path(hera_config.data_dir)).resolve()
    resolved_cfg = hera_config.resolve_paths(root)
    target = (target_path or (root / "bin" / "slskd.yml")).resolve()
    target.parent.mkdir(parents=True, exist_ok=True)

    # Preservar credenciales existentes si ya había un slskd.yml
    username = "hera_dj_2026"
    password = "HeraDjGlobal2026!"
    api_key = "hera_key_e2f72121391c"
    if target.exists():
        try:
            existing = yaml.safe_load(target.read_text(encoding="utf-8")) or {}
            slsk_sec = existing.get("soulseek") or {}
            if slsk_sec.get("username"):
                username = slsk_sec["username"]
            if slsk_sec.get("password"):
                password = slsk_sec["password"]
        except Exception:
            pass

    quarantine_dir = Path(resolved_cfg.quarantine_dir).resolve()
    incomplete_dir = (quarantine_dir / "incomplete").resolve()
    library_dir = Path(resolved_cfg.library_dir).resolve()
    sets_dir = (root / "sets").resolve()

    for d in (quarantine_dir, incomplete_dir, library_dir, sets_dir, target.parent / "data"):
        d.mkdir(parents=True, exist_ok=True)

    share_paths: list[str] = []
    sharing = getattr(resolved_cfg, "sharing", None)
    if not sharing or sharing.enabled:
        if not sharing or sharing.share_library:
            share_paths.append(str(library_dir))
        if not sharing or sharing.share_sets:
            share_paths.append(str(sets_dir))
        for folder in resolved_cfg.providers.local_folders:
            cand = Path(folder)
            if not cand.is_absolute():
                cand = (root / cand).resolve()
            else:
                cand = cand.resolve()
            if cand.exists() and cand.is_dir():
                cand_str = str(cand)
                if cand_str not in share_paths:
                    share_paths.append(cand_str)

    upload_slots = sharing.max_upload_slots if sharing else 5
    upload_speed = sharing.max_upload_speed_kbps if sharing else 2048
    description = (
        sharing.share_description
        if sharing and getattr(sharing, "share_description", None)
        else "Hera Curated Library — AI-organized, lossless verified"
    )

    runtime_dict = {
        "soulseek": {
            "username": username,
            "password": password,
            "listen_port": 50300,
            "description": description,
        },
        "web": {
            "port": 5030,
            "authentication": {
                "disabled": True,
                "api_keys": {
                    "hera": {
                        "key": api_key,
                        "role": "administrator",
                        "cidr": "0.0.0.0/0,::/0",
                    }
                },
            },
        },
        "directories": {
            "downloads": str(quarantine_dir),
            "incomplete": str(incomplete_dir),
        },
        "shares": {
            "directories": share_paths,
        },
        "transfers": {
            "upload": {
                "slots": upload_slots,
                "speed_limit": upload_speed,
            },
        },
    }

    yaml_text = yaml.dump(runtime_dict, sort_keys=False, default_flow_style=False, allow_unicode=True)
    target.write_text(yaml_text, encoding="utf-8")
    return target


