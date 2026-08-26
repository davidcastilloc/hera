"""Backend Registry — Resolves LLM backend names to Antigravity SDK configs.

Supports 12 backends (4 cloud + 8 local), all connected through the
Antigravity SDK's native OpenAI-compatible endpoint support.
"""

import os
import httpx
from dataclasses import dataclass

from hera.domain.config import AgentConfig


# ─── Backend Definitions ─────────────────────────────────────────────────────

@dataclass(frozen=True)
class BackendDef:
    """Static definition of a supported LLM backend."""
    name: str
    display_name: str
    default_base_url: str | None
    default_model: str
    api_key_env: str | None
    is_local: bool
    default_port: int | None = None


BACKENDS: dict[str, BackendDef] = {
    # ── Cloud Providers ──
    "gemini": BackendDef(
        name="gemini",
        display_name="Google Gemini API",
        default_base_url=None,  # Handled natively by SDK
        default_model="gemini-2.5-flash",
        api_key_env="GEMINI_API_KEY",
        is_local=False,
    ),
    "vertex": BackendDef(
        name="vertex",
        display_name="Google Vertex AI",
        default_base_url=None,  # Handled natively by SDK
        default_model="gemini-2.5-flash",
        api_key_env=None,
        is_local=False,
    ),
    "openai": BackendDef(
        name="openai",
        display_name="OpenAI",
        default_base_url="https://api.openai.com/v1",
        default_model="gpt-4o",
        api_key_env="OPENAI_API_KEY",
        is_local=False,
    ),
    "anthropic": BackendDef(
        name="anthropic",
        display_name="Anthropic Claude",
        default_base_url="https://api.anthropic.com/v1",
        default_model="claude-sonnet-4-20250514",
        api_key_env="ANTHROPIC_API_KEY",
        is_local=False,
    ),
    # ── Local Engines ──
    "ollama": BackendDef(
        name="ollama",
        display_name="Ollama",
        default_base_url="http://localhost:11434/v1",
        default_model="llama3.3",
        api_key_env=None,
        is_local=True,
        default_port=11434,
    ),
    "lmstudio": BackendDef(
        name="lmstudio",
        display_name="LM Studio",
        default_base_url="http://localhost:1234/v1",
        default_model="default",
        api_key_env=None,
        is_local=True,
        default_port=1234,
    ),
    "jan": BackendDef(
        name="jan",
        display_name="Jan",
        default_base_url="http://localhost:1337/v1",
        default_model="default",
        api_key_env=None,
        is_local=True,
        default_port=1337,
    ),
    "llamacpp": BackendDef(
        name="llamacpp",
        display_name="llama.cpp Server",
        default_base_url="http://localhost:8080/v1",
        default_model="default",
        api_key_env=None,
        is_local=True,
        default_port=8080,
    ),
    "vllm": BackendDef(
        name="vllm",
        display_name="vLLM",
        default_base_url="http://localhost:8000/v1",
        default_model="default",
        api_key_env=None,
        is_local=True,
        default_port=8000,
    ),
    "localai": BackendDef(
        name="localai",
        display_name="LocalAI",
        default_base_url="http://localhost:8080/v1",
        default_model="default",
        api_key_env=None,
        is_local=True,
        default_port=8080,
    ),
    "mlx": BackendDef(
        name="mlx",
        display_name="MLX (Apple Silicon)",
        default_base_url="http://localhost:8080/v1",
        default_model="default",
        api_key_env=None,
        is_local=True,
        default_port=8080,
    ),
    "custom": BackendDef(
        name="custom",
        display_name="Custom OpenAI-Compatible",
        default_base_url=None,
        default_model="default",
        api_key_env="CUSTOM_API_KEY",
        is_local=False,
    ),
}

# Ordered list for auto-detection (local engines by port)
LOCAL_PROBE_ORDER = [
    ("ollama", "http://localhost:11434"),
    ("lmstudio", "http://localhost:1234"),
    ("jan", "http://localhost:1337"),
    ("vllm", "http://localhost:8000"),
    ("llamacpp", "http://localhost:8080"),  # shares port with localai/mlx
]


# ─── Backend Registry ────────────────────────────────────────────────────────

class BackendRegistry:
    """Factory that resolves backend names to Antigravity SDK LocalAgentConfig."""

    @staticmethod
    def list_backends() -> list[BackendDef]:
        """Return all supported backends."""
        return list(BACKENDS.values())

    @staticmethod
    def probe_endpoint(url: str, timeout: float = 2.0) -> bool:
        """Check if a local endpoint is responding."""
        try:
            r = httpx.get(f"{url}/v1/models", timeout=timeout)
            return r.status_code in [200, 401, 403]
        except Exception:
            try:
                # Some engines respond at /api/tags (Ollama)
                r = httpx.get(f"{url}/api/tags", timeout=timeout)
                return r.status_code == 200
            except Exception:
                return False

    @staticmethod
    def auto_detect() -> str | None:
        """Probe for running backends. Returns the first responding backend name."""
        # 1. Check cloud env vars
        if os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY"):
            return "gemini"

        if os.environ.get("VERTEX_PROJECT"):
            return "vertex"

        # 2. Check ADC for Vertex AI
        try:
            import google.auth
            _, project = google.auth.default()
            if project:
                return "vertex"
        except Exception:
            pass

        # 3. Check cloud API keys
        if os.environ.get("OPENAI_API_KEY"):
            return "openai"
        if os.environ.get("ANTHROPIC_API_KEY"):
            return "anthropic"

        # 4. Probe local engines
        for name, url in LOCAL_PROBE_ORDER:
            if BackendRegistry.probe_endpoint(url):
                return name

        return None

    @staticmethod
    def resolve(config: AgentConfig):
        """Build an Antigravity SDK LocalAgentConfig for the given backend config.

        Returns a dict with the kwargs to pass to LocalAgentConfig.
        """
        backend = config.backend
        if backend == "auto":
            detected = BackendRegistry.auto_detect()
            if not detected:
                return None
            backend = detected

        if backend not in BACKENDS:
            raise ValueError(
                f"Unknown backend '{backend}'. "
                f"Supported: {', '.join(BACKENDS.keys())}"
            )

        bdef = BACKENDS[backend]

        # Resolve model
        model = config.model or bdef.default_model

        # Resolve API key
        api_key = config.api_key
        if not api_key and bdef.api_key_env:
            api_key = os.environ.get(bdef.api_key_env)

        # Resolve base URL
        base_url = config.base_url or bdef.default_base_url

        # ── Build SDK config kwargs ──

        # Native Gemini API
        if backend == "gemini":
            if not api_key:
                return None
            return {
                "type": "gemini",
                "api_key": api_key,
                "model": model,
                "display": f"{bdef.display_name} ({model})",
            }

        # Native Vertex AI
        if backend == "vertex":
            project = config.vertex_project or os.environ.get("VERTEX_PROJECT")
            if not project:
                try:
                    import google.auth
                    _, project = google.auth.default()
                except Exception:
                    pass
            if not project:
                return None
            return {
                "type": "vertex",
                "project": project,
                "location": config.vertex_location,
                "model": model,
                "display": f"{bdef.display_name} ({project}/{model})",
            }

        # OpenAI-compatible endpoints (all others)
        if not base_url and backend == "custom":
            return None

        return {
            "type": "openai_compatible",
            "base_url": base_url,
            "api_key": api_key or "local",
            "model": model,
            "display": f"{bdef.display_name} ({model} @ {base_url})",
        }
