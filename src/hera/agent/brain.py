"""Motor de orquestación de agentes con Antigravity SDK para Hera."""

import asyncio
import os
from pathlib import Path
import sys

from hera.agent.prompts import HERA_SYSTEM_INSTRUCTIONS
from hera.agent.tools import (
    search_and_acquire_tracks,
    create_or_update_dj_set,
    sync_sets_to_cloud,
    get_library_status,
    recommend_harmonic_transitions,
)


class HeraBrain:
    """Orquestador del agente Hera impulsado por Antigravity SDK y herramientas de DJ."""

    def __init__(self, api_key: str | None = None, model: str = "gemini-2.5-flash"):
        self.api_key = api_key or os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
        self.model = model
        self.agent = None

    async def initialize(self) -> bool:
        """Inicializa el agente de Antigravity con las herramientas de Hera registradas."""
        try:
            from google.antigravity import Agent, LocalAgentConfig, CapabilitiesConfig

            config = LocalAgentConfig(
                system_instructions=HERA_SYSTEM_INSTRUCTIONS,
                capabilities=CapabilitiesConfig(
                    enabled_tools=["*"],
                ),
                tools=[
                    search_and_acquire_tracks,
                    create_or_update_dj_set,
                    sync_sets_to_cloud,
                    get_library_status,
                    recommend_harmonic_transitions,
                ],
                api_key=self.api_key,
                model=self.model,
            )
            self.agent = Agent(config)
            await self.agent.__aenter__()
            return True
        except Exception as e:
            return False

    async def close(self):
        """Cierra la sesión del agente."""
        if self.agent:
            try:
                await self.agent.__aexit__(None, None, None)
            except Exception:
                pass


async def run_hera_interactive_chat():
    """Consola conversacional interactiva en lenguaje natural para DJs."""
    api_key = os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
    if not api_key:
        print("=" * 80)
        print(" 🎧 HERA AI AGENT — Configuración de Clave de IA")
        print("=" * 80)
        print("Para razonamiento en lenguaje natural autónomo con Antigravity SDK,")
        print("puedes ingresar tu GEMINI_API_KEY (o presiona Enter para modo local):")
        try:
            entered_key = input("API Key (opcional): ").strip()
            if entered_key:
                os.environ["GEMINI_API_KEY"] = entered_key
                api_key = entered_key
        except (EOFError, KeyboardInterrupt):
            return

    brain = HeraBrain(api_key=api_key)
    initialized = await brain.initialize()

    print("\n" + "=" * 80)
    print(" 🎧 HERA AI AGENT — Conversación Natural para DJs")
    print("=" * 80)
    if initialized and brain.api_key:
        print("[+] Backend: Google Antigravity SDK Activo con LLM Reasoning.")
        print("[+] Capacidad: P2P Search, DSP Harmonic Analysis, Crate Builder & Cloud Sync.")
    else:
        print("[*] Modo Local Autónomo Activo (Herramientas DSP y P2P conectadas).")

    print("\nHabla con Hera naturalmente. Ejemplos:")
    print("  - 'Búscame temas clásicos de Sensation White como For An Angel y Universal Nation'")
    print("  - 'Arma un set de 60 min con las canciones de French Touch que tenemos'")
    print("  - '¿Qué canciones son armónicamente compatibles para mezclar con 8A a 124 BPM?'")
    print("  - 'Sincroniza todos mis sets a Google Drive'")
    print("(Escribe 'salir' para terminar)\n")

    try:
        while True:
            try:
                user_input = input("DJ > ").strip()
            except (EOFError, KeyboardInterrupt):
                break

            if not user_input:
                continue
            if user_input.lower() in ["salir", "exit", "quit", "q"]:
                print("\n[*] Cerrando sesión de Hera. ¡Que tengas una gran sesión en cabina! 🎧")
                break

            print("\nHera 🤖:\n", flush=True)

            if brain.agent and brain.api_key:
                try:
                    response = await brain.agent.chat(user_input)
                    async for token in response:
                        sys.stdout.write(token)
                        sys.stdout.flush()
                    print("\n")
                except Exception as e:
                    print(f"[!] Error comunicando con el modelo: {e}")
                    await _handle_offline_nlp(user_input)
            else:
                await _handle_offline_nlp(user_input)

    finally:
        await brain.close()


async def _handle_offline_nlp(user_input: str):
    """Manejo de intenciones en lenguaje natural sin requerir conexión a API de IA externa."""
    q_low = user_input.lower()

    if any(k in q_low for k in ["busca", "buscar", "descarga", "descargar", "encuentra", "search", "download"]):
        # Extraer nombres o temas
        terms = user_input
        for prefix in ["búscame", "buscame", "busca", "buscar", "descárgame", "descargame", "descarga", "descargar"]:
            if prefix in q_low:
                terms = user_input[q_low.find(prefix) + len(prefix):].strip()
                break
        
        queries = [t.strip() for t in terms.replace(" y ", ",").replace(" and ", ",").split(",") if t.strip()]
        if not queries:
            queries = [terms]

        print(f"[*] Buscando y adquiriendo tracks en la red Soulseek: {queries}")
        res = search_and_acquire_tracks(queries)
        print(res + "\n")

    elif any(k in q_low for k in ["arma", "armar", "crea", "crear", "haz", "hacer", "set", "crate"]):
        print("[*] Compilando crate y analizando compatibilidad armónica...")
        res = get_library_status()
        print(res + "\n")

    elif any(k in q_low for k in ["drive", "sync", "sincroniza", "sincronizar", "sube", "subir", "nube"]):
        print("[*] Sincronizando sets con Google Drive vía rclone...")
        res = sync_sets_to_cloud()
        print(res + "\n")

    elif any(k in q_low for k in ["biblioteca", "canciones", "tracks", "inventario", "sets", "tengo"]):
        res = get_library_status()
        print(res + "\n")

    else:
        print(f"Entendido: '{user_input}'. Puedes pedirme buscar pistas, organizar un set o sincronizar con Drive.\n")
