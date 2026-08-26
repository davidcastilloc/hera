"""Motor de orquestación de agentes con Antigravity SDK para Hera."""

import asyncio
import os
from pathlib import Path
import sys

from hera.agent.prompts import HERA_SYSTEM_INSTRUCTIONS
from hera.agent.tools import (
    search_and_acquire_tracks,
    build_dj_set,
    sync_to_cloud,
    get_library_inventory,
    get_sets_inventory,
)


class HeraBrain:
    """Orquestador del agente Hera impulsado por Antigravity SDK."""

    def __init__(self, api_key: str | None = None, model: str = "gemini-2.5-flash"):
        self.api_key = api_key or os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
        self.model = model
        self.agent = None

    async def initialize(self):
        """Inicializa el agente de Antigravity con las capacidades y herramientas de Hera."""
        try:
            from google.antigravity import Agent, LocalAgentConfig, CapabilitiesConfig

            config = LocalAgentConfig(
                system_instructions=HERA_SYSTEM_INSTRUCTIONS,
                capabilities=CapabilitiesConfig(
                    enabled_tools=["*"],
                ),
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
    """Bucle conversacional interactivo para terminal (DJ Friendly)."""
    brain = HeraBrain()
    initialized = await brain.initialize()

    print("=" * 80)
    print(" 🎧 HERA AI AGENT — Consola Interactiva para DJs")
    print("=" * 80)
    if initialized:
        print("[+] Backend: Google Antigravity SDK conectado.")
    else:
        print("[*] Modo Autónomo Local (Herramientas y DSP listos).")
    print("Escribe tus instrucciones o preguntas (o 'salir' para terminar):")
    print("Ejemplos:")
    print("  - '¿Qué canciones reales tengo en mi biblioteca?'")
    print("  - '¿Qué sets DJ tenemos listos en disco?'")
    print("  - 'Sincroniza todos mis sets con Google Drive'\n")

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

            print("\nHera 🤖:", end=" ", flush=True)

            # Si el SDK de Antigravity está conectado, enviamos el prompt al modelo
            if brain.agent:
                try:
                    response = await brain.agent.chat(user_input)
                    async for token in response:
                        sys.stdout.write(token)
                        sys.stdout.flush()
                    print("\n")
                except Exception as e:
                    # Fallback autónomo si no hay API key configurada
                    await _handle_local_direct_intent(user_input)
            else:
                await _handle_local_direct_intent(user_input)

    finally:
        await brain.close()


async def _handle_local_direct_intent(user_input: str):
    """Manejo de intenciones directas para control offline sin depender de API externa."""
    q_low = user_input.lower()

    if "biblioteca" in q_low or "canciones" in q_low or "libreria" in q_low or "tracks" in q_low:
        res = await get_library_inventory()
        print(f"\n[+] Tienes {res['total_tracks']} canciones 100% reales en tu biblioteca:")
        for t in res["tracks"]:
            print(f"  * {t['artist']} — {t['filename']} ({t['size_mb']} MB, {t['format']})")
        print()

    elif "sets" in q_low or "crates" in q_low or "carpetas" in q_low:
        res = await get_sets_inventory()
        print(f"\n[+] Tienes {res['total_sets']} sets DJ organizados en disco:")
        for s in res["sets"]:
            print(f"\n📂 {s['set_name']} ({s['track_count']} tracks):")
            for tr in s["tracks"][:6]:
                print(f"   -> {tr}")
            if s["track_count"] > 6:
                print(f"   ... y {s['track_count'] - 6} más.")
        print()

    elif "drive" in q_low or "sync" in q_low or "subir" in q_low or "nube" in q_low:
        print("[*] Sincronizando sets con Google Drive vía rclone...")
        res = await sync_to_cloud()
        if res["status"] == "success":
            print(f"[OK] ¡Sets sincronizados con éxito hacia {res['remote_destination']}!")
        else:
            print(f"[!] Error: {res.get('error') or res.get('message')}")
        print()

    else:
        print(f"He recibido tu instrucción: '{user_input}'. Puedes consultar 'biblioteca', 'sets' o 'sincronizar con drive'.\n")
