"""Hera Agent Brain — Powered by Google Antigravity SDK.

All natural language understanding and tool selection is delegated to the LLM.
Supports 12 backends (4 cloud + 8 local) via the BackendRegistry.
"""

import asyncio
import os
import sys
from typing import Any
from pathlib import Path

# Cargar variables de entorno desde .env si existe
try:
    from dotenv import load_dotenv
    load_dotenv()
except Exception:
    pass

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

from hera.agent.backends import BackendRegistry, BACKENDS
from hera.agent.prompts import HERA_SYSTEM_INSTRUCTIONS
from hera.agent.tools import (
    search_and_acquire_tracks,
    create_or_update_dj_set,
    sync_sets_to_cloud,
    get_library_status,
    recommend_harmonic_transitions,
    get_community_status,
    get_session_cost_and_tokens,
)
from hera.domain.config import AgentConfig, HeraConfig
from hera.domain.cost import CostTracker
import contextvars

# Variable de contexto aislada para tracking de costos concurrente y seguro
_active_cost_tracker_ctx: contextvars.ContextVar[CostTracker | None] = contextvars.ContextVar(
    "active_cost_tracker", default=None
)

def get_active_cost_tracker() -> CostTracker | None:
    return _active_cost_tracker_ctx.get()

def set_active_cost_tracker(tracker: CostTracker | None) -> None:
    _active_cost_tracker_ctx.set(tracker)

# Compatibilidad con accesos directos
ACTIVE_COST_TRACKER: CostTracker | None = None

HERA_TOOLS = [
    search_and_acquire_tracks,
    create_or_update_dj_set,
    sync_sets_to_cloud,
    get_library_status,
    recommend_harmonic_transitions,
    get_community_status,
    get_session_cost_and_tokens,
]





def diagnose_llm_error(err: Exception) -> str:
    """Diagnostica de forma legible y propone soluciones ante errores de APIs LLM (429, 401, etc.)."""
    err_str = str(err)
    if "429" in err_str or "RESOURCE_EXHAUSTED" in err_str:
        if "prepayment credits are depleted" in err_str.lower():
            return (
                "⚠️ [Error 429 - Créditos Agotados en Google AI Studio]\n"
                "Los créditos prepagados de tu clave 'GEMINI_API_KEY' se han agotado.\n\n"
                "💡 Soluciones recomendadas:\n"
                "   1. Cambiar a Vertex AI: ejecuta 'hera chat --backend vertex'\n"
                "   2. Usar un motor local gratuito: inicia Ollama y ejecuta 'hera chat --backend ollama'\n"
                "   3. Recargar saldo en tu cuenta de Google AI Studio: https://ai.studio/projects"
            )
        return (
            "⚠️ [Error 429 - Límite de Cuota por Minuto Excedido]\n"
            "Has alcanzado el límite de peticiones por minuto de la API.\n\n"
            "💡 Espera unos segundos antes de enviar tu siguiente mensaje, o usa un backend local ('hera chat --backend ollama')."
        )
    if "401" in err_str or "403" in err_str or "UNAUTHENTICATED" in err_str or "PERMISSION_DENIED" in err_str:
        return (
            "⚠️ [Error de Autenticación / Permisos]\n"
            "La clave API o las credenciales no son válidas.\n"
            "💡 Verifica tu API Key o inicia sesión con Google Cloud: 'gcloud auth application-default login'."
        )
    return f"⚠️ [Error en modelo de lenguaje]: {err_str}"


class HeraBrain:
    """Real AI Agent orchestrator — the LLM decides everything."""

    def __init__(self, agent_config: AgentConfig | None = None):
        self.config = agent_config or AgentConfig()
        self.agent = None
        self._initialized = False
        self._display = ""
        self.cost_tracker: CostTracker | None = None

    async def initialize(self) -> bool:
        """Resolve backend and spawn the Antigravity Agent."""
        resolved = BackendRegistry.resolve(self.config)
        if not resolved:
            return False

        self._display = resolved["display"]

        # Inicializar el tracker de costos de la sesión
        backend_key = self.config.backend
        bdef = BACKENDS.get(backend_key, None)
        is_local = bdef.is_local if bdef else False
        model_name = resolved.get("model", self.config.model or "gemini-2.5-flash")

        self.cost_tracker = CostTracker(
            backend_name=resolved.get("type", backend_key),
            model_name=model_name,
            is_local=is_local,
            max_session_cost_usd=self.config.max_session_cost_usd,
        )
        set_active_cost_tracker(self.cost_tracker)
        global ACTIVE_COST_TRACKER
        ACTIVE_COST_TRACKER = self.cost_tracker

        try:
            if self.agent:
                try:
                    await self.close()
                except Exception:
                    pass

            from google.antigravity import Agent, LocalAgentConfig, CapabilitiesConfig

            backend_type = resolved["type"]
            caps = CapabilitiesConfig(enabled_tools=[])

            if backend_type == "gemini":
                sdk_config = LocalAgentConfig(
                    system_instructions=HERA_SYSTEM_INSTRUCTIONS,
                    capabilities=caps,
                    tools=HERA_TOOLS,
                    api_key=resolved["api_key"],
                    model=resolved["model"],
                )

            elif backend_type == "vertex":
                sdk_config = LocalAgentConfig(
                    system_instructions=HERA_SYSTEM_INSTRUCTIONS,
                    capabilities=caps,
                    tools=HERA_TOOLS,
                    vertex=True,
                    project=resolved["project"],
                    location=resolved["location"],
                    model=resolved["model"],
                )

            elif backend_type == "openai_compatible":
                from google.antigravity import LocalOpenAIAgentConfig
                env_dict = {}
                if resolved.get("api_key"):
                    env_dict["OPENAI_API_KEY"] = resolved["api_key"]
                sdk_config = LocalOpenAIAgentConfig(
                    system_instructions=HERA_SYSTEM_INSTRUCTIONS,
                    capabilities=caps,
                    tools=HERA_TOOLS,
                    model=resolved["model"],
                    base_url=resolved["base_url"],
                    env=env_dict if env_dict else None,
                )
            else:
                return False

            self.agent = Agent(sdk_config)
            await self.agent.__aenter__()
            self._initialized = True
            return True

        except Exception as e:
            print(f"[!] Error initializing agent: {e}")
            return False

    async def chat(
        self,
        user_input: str,
        on_token: Any = None,
        print_to_stdout: bool = True,
    ) -> str:
        """Send a message to the agent and stream the response, with automatic fallback and cost snapbar."""
        if not self.agent:
            return "[!] Agente no inicializado."

        try:
            response = await self.agent.chat(user_input)
            full_response = []
            async for token in response:
                if on_token and callable(on_token):
                    try:
                        on_token(token)
                    except Exception:
                        pass
                if print_to_stdout:
                    try:
                        sys.stdout.write(token)
                        sys.stdout.flush()
                    except Exception:
                        pass
                full_response.append(token)

            # Extraer consumo real de tokens desde usage_metadata
            prompt_toks = 0
            candidates_toks = 0
            thoughts_toks = 0

            if hasattr(response, "usage_metadata") and response.usage_metadata:
                prompt_toks = getattr(response.usage_metadata, "prompt_token_count", 0) or 0
                candidates_toks = getattr(response.usage_metadata, "candidates_token_count", 0) or 0
                thoughts_toks = getattr(response.usage_metadata, "thoughts_token_count", 0) or 0
            else:
                # Estimación heurística si el backend no emite metadatos
                prompt_toks = max(len(user_input) // 4, 10)
                candidates_toks = max(len("".join(full_response)) // 4, 10)

            # Registrar en el tracker de costos
            if self.cost_tracker:
                self.cost_tracker.record_turn(prompt_toks, candidates_toks, thoughts_toks)
                if print_to_stdout and getattr(self.config, "show_cost_snapbar", True):
                    snapbar = self.cost_tracker.format_snapbar()
                    sys.stdout.write(f"{snapbar}\n")
                    sys.stdout.flush()

            return "".join(full_response)


        except Exception as e:
            err_msg = str(e)
            # Intentar fallback automático transparente a Vertex AI si Gemini falló por 429/créditos
            if ("429" in err_msg or "RESOURCE_EXHAUSTED" in err_msg) and self.config.backend in ["auto", "gemini"]:
                try:
                    import google.auth
                    _, project = google.auth.default()
                    if project:
                        sys.stdout.write(
                            "\n\n🔄 [AUTO-FALLBACK] Error 429 en Gemini API (créditos agotados).\n"
                            f"   Cambiando automáticamente a Google Vertex AI (proyecto: {project})...\n\n"
                        )
                        sys.stdout.flush()
                        await self.close()
                        self.config.backend = "vertex"
                        ok = await self.initialize()
                        if ok:
                            return await self.chat(user_input)
                except Exception:
                    pass

            diagnosis = diagnose_llm_error(e)
            sys.stdout.write(f"\n\n{diagnosis}\n")
            sys.stdout.flush()
            return diagnosis

    async def close(self):
        """Shut down the agent session."""
        if self.agent:
            try:
                await self.agent.__aexit__(None, None, None)
            except Exception:
                pass



async def run_hera_interactive_chat(
    backend: str | None = None,
    model: str | None = None,
    base_url: str | None = None,
    config_path: str = "config/hera.toml",
):
    """Interactive conversational console for DJs — 100% LLM-driven."""

    # 1. Cargar configuración base desde hera.toml si existe
    cfg_file = Path(config_path)
    if cfg_file.exists():
        hera_cfg = HeraConfig.load(cfg_file)
        agent_config = hera_cfg.agent
    else:
        agent_config = AgentConfig()

    # 2. Sobrescribir con opciones explícitas pasadas por CLI
    if backend:
        agent_config.backend = backend
    if model:
        agent_config.model = model
    if base_url:
        agent_config.base_url = base_url

    print()
    print("=" * 80)
    print(" 🎧 HERA AI AGENT - Consola Conversacional para DJs")
    print("=" * 80)
    print("[*] Detecting backend...", flush=True)

    brain = HeraBrain(agent_config)
    initialized = await brain.initialize()

    if not initialized:
        # Fallback interactivo si no hay backend activo
        print()
        print("[!] No se detectó ningún backend de IA automáticamente.")
        print()
        print("Opciones para activar el cerebro de Hera:")
        print("  1. Inicia un motor local:      ollama serve")
        print("  2. Configura tu API Key:       set GEMINI_API_KEY=tu_clave")
        print("  3. Ingresa tu API Key aquí:")
        print()
        try:
            entered = input("GEMINI_API_KEY > ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\n[*] Cancelado.")
            return
        if not entered:
            print("[!] No se ingresó ninguna clave. Ejecuta 'hera chat --backend ollama' si usas Ollama.")
            return

        agent_config = AgentConfig(backend="gemini", api_key=entered, model="gemini-2.5-flash")
        brain = HeraBrain(agent_config)
        initialized = await brain.initialize()
        if not initialized:
            print("[!] Error de autenticación. Verifica tu API Key.")
            return

    print(f"[+] Backend: {brain._display}")
    print(f"[+] Tools: {len(HERA_TOOLS)} registradas (el LLM selecciona de forma autónoma)")
    print(f"[+] Snapbar de Costos: {'Activado 🟢' if agent_config.show_cost_snapbar else 'Desactivado'}")
    if agent_config.max_session_cost_usd:
        print(f"[+] Límite de Presupuesto: ${agent_config.max_session_cost_usd:.2f} USD")
    print()
    print("Habla con Hera naturalmente en español o inglés.")
    print("(Escribe 'salir' para terminar)")
    print()

    try:
        while True:
            try:
                user_input = input("DJ > ").strip()
            except (EOFError, KeyboardInterrupt):
                print("\n[*] Cerrando sesión de Hera. ¡Buena sesión en cabina!")
                break

            if not user_input:
                continue
            if user_input.lower() in ["salir", "exit", "quit", "q"]:
                print("\n[*] Cerrando sesión de Hera. ¡Buena sesión en cabina!")
                break

            print("\nHera:\n", flush=True)

            try:
                await brain.chat(user_input)
                print()
            except Exception as e:
                print(f"\n[!] Error en chat: {e}\n")

    finally:
        if brain.cost_tracker:
            summary = brain.cost_tracker.get_summary()
            print("\n" + "=" * 80)
            print(" 📊 RESUMEN FINAL DE SESIÓN:")
            print(f"    * Turnos: {summary['turns']}")
            print(f"    * Tokens consumidos: {summary['total_tokens']:,}")
            print(f"    * Gasto total estimado: {summary['cost_formatted']}")
            print("=" * 80 + "\n")
        await brain.close()


