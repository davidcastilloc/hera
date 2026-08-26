"""Motor de orquestación de agentes en lenguaje natural para Hera."""

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
    """Orquestador del agente Hera impulsado por Antigravity y herramientas autónomas de DJ."""

    def __init__(self, api_key: str | None = None, model: str = "gemini-2.5-flash"):
        self.api_key = api_key or os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
        self.model = model
        self.agent = None

    async def initialize(self) -> bool:
        """Inicializa el agente si hay credenciales disponibles."""
        if not self.api_key:
            return False

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
        except Exception:
            return False

    async def close(self):
        """Cierra la sesión del agente."""
        if self.agent:
            try:
                await self.agent.__aexit__(None, None, None)
            except Exception:
                pass


async def run_hera_interactive_chat():
    """Consola conversacional interactiva en lenguaje natural para DJs (100% directa y sin prompts molestos)."""
    brain = HeraBrain()
    initialized = await brain.initialize()

    print("=" * 80)
    print(" 🎧 HERA AI AGENT — Consola Conversacional para DJs")
    print("=" * 80)
    if initialized:
        print("[+] Backend: Antigravity Agent Cloud Runtime Activo.")
    else:
        print("[+] Motor: Hera Autonomous DJ Engine (100% Local & Lenguaje Natural).")

    print("\nHabla con Hera naturalmente en español o inglés:")
    print("  * 'búscame temas de Sensation White como For An Angel y Universal Nation'")
    print("  * 'arma un set de 60 min con los temas a 128 bpm'")
    print("  * 'sincroniza todos mis sets a Google Drive'")
    print("  * '¿qué canciones tengo listas en mi biblioteca?'")
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

            if brain.agent:
                try:
                    response = await brain.agent.chat(user_input)
                    async for token in response:
                        sys.stdout.write(token)
                        sys.stdout.flush()
                    print("\n")
                except Exception:
                    await _handle_nlp(user_input)
            else:
                await _handle_nlp(user_input)

    finally:
        await brain.close()


async def _handle_nlp(user_input: str):
    """Interpreta y ejecuta cualquier petición en lenguaje natural sin requerir comandos fijos."""
    q_low = user_input.lower()

    # 1. Búsqueda y Adquisición P2P
    if any(k in q_low for k in ["busca", "buscar", "descarga", "descargar", "encuentra", "consigue", "search", "download", "get"]):
        raw_terms = user_input
        for prefix in [
            "búscame los mejores temas de", "buscame los mejores temas de",
            "búscame temas de", "buscame temas de", "busca temas de", "buscar temas de",
            "búscame", "buscame", "busca", "buscar",
            "descárgame", "descargame", "descarga", "descargar",
            "consígueme", "consigueme", "consigue",
            "search for", "download", "find"
        ]:
            if prefix in q_low:
                raw_terms = user_input[q_low.find(prefix) + len(prefix):].strip()
                break

        # Limpiar conectores
        terms = [t.strip() for t in raw_terms.replace(" como ", ",").replace(" y ", ",").replace(" and ", ",").split(",") if t.strip()]
        if not terms:
            terms = [raw_terms]

        print(f"[*] Buscando y adquiriendo en Soulseek P2P: {', '.join(terms)}...")
        res = search_and_acquire_tracks(terms)
        print(res + "\n")

    # 2. Creación y organización de Sets
    elif any(k in q_low for k in ["arma", "armar", "crea", "crear", "haz", "hacer", "organiza", "organizar", "set", "crate"]):
        set_name = "DJ Live Set"
        if "sensation" in q_low:
            set_name = "Sensation White Live Crate"
        elif "french" in q_low or "house" in q_low:
            set_name = "French Touch & Vocal House"
        elif "trance" in q_low or "euro" in q_low:
            set_name = "Eurodance & Trance"

        print(f"[*] Analizando biblioteca, afinidad armónica Camelot y compilando '{set_name}'...")
        # Tomar tracks disponibles
        res = create_or_update_dj_set(set_name, ["Modjo", "Stardust", "Daft Punk", "Tiesto", "Armand", "Spiller"])
        print(res + "\n")

    # 3. Sincronización en la Nube
    elif any(k in q_low for k in ["drive", "google", "sync", "sincroniza", "sincronizar", "sube", "subir", "nube", "cloud"]):
        print("[*] Sincronizando sets con Google Drive vía rclone...")
        res = sync_sets_to_cloud()
        print(res + "\n")

    # 4. Estado de biblioteca y sets
    elif any(k in q_low for k in ["biblioteca", "canciones", "tracks", "inventario", "sets", "tengo", "list", "show"]):
        res = get_library_status()
        print(res + "\n")

    # 5. Consultas armónicas / Camelot
    elif any(k in q_low for k in ["camelot", "armonia", "armonica", "mezclar", "bpm", "tonalidad", "clave"]):
        print(recommend_harmonic_transitions("8A", 124.0) + "\n")

    else:
        print(f"Entendido: '{user_input}'. Puedes pedirme buscar artistas o canciones, armar un set armónico o sincronizar con tu Google Drive.\n")
