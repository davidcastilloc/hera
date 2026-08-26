"""Motor de orquestación de agentes en lenguaje natural para Hera."""

import asyncio
import os
from pathlib import Path
import re
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
    """Consola conversacional interactiva en lenguaje natural para DJs."""
    brain = HeraBrain()
    initialized = await brain.initialize()

    print("=" * 80)
    print(" HERA AI AGENT - Consola Conversacional para DJs")
    print("=" * 80)
    if initialized:
        print("[+] Backend: Antigravity Agent Cloud Runtime Activo.")
    else:
        print("[+] Motor: Hera Autonomous DJ Engine (100% Local & Lenguaje Natural).")

    print("\nHabla con Hera naturalmente en espanol o ingles:")
    print("  * 'Que cosas puedes hacer por mi?'")
    print("  * 'buscame temas de Sensation White como For An Angel y Universal Nation'")
    print("  * 'arma un set con los temas de French Touch'")
    print("  * 'sincroniza todos mis sets a Google Drive'")
    print("  * 'que canciones tengo listas en mi biblioteca?'")
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
                print("\n[*] Cerrando sesion de Hera. Que tengas una gran sesion en cabina!")
                break

            print("\nHera:\n", flush=True)

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
    """Interpreta y ejecuta peticiones en lenguaje natural de manera robusta y contextual."""
    q_raw = user_input.strip()
    q_low = q_raw.lower()

    # 1. Saludos, Identidad, Ayuda y Capacidades
    if any(p in q_low for p in [
        "que puedes hacer", "qué puedes hacer", "que sabes hacer", "qué sabes hacer",
        "que cosas puedes hacer", "qué cosas puedes hacer", "para que sirves", "para qué sirves",
        "quien eres", "quién eres", "como funcionas", "cómo funcionas", "ayuda", "help",
        "hola", "buenas", "que tal", "qué tal", "capacidades"
    ]):
        print("Hola! Soy Hera, tu super-agente inteligente para curacion musical y cabina de DJ.\n")
        print("Aqui tienes todo lo que puedo hacer por ti:")
        print("  1. [BUSQUEDA] Busqueda & Descarga P2P (Soulseek):")
        print("     Dime cualquier tema o artista y descargare el master en FLAC o MP3 320k (sin placeholders ni pitidos).")
        print("     Ej: 'buscame temas de Sensation White como For An Angel y Universal Nation'")
        print("  2. [DSP] Analisis Acustico & Armonico (BPM & Camelot):")
        print("     Analizo BPMs, claves Camelot (8A, 11B...) y volumen LUFS para asegurar mezclas perfectas.")
        print("     Ej: 'si estoy en 8A a 124 BPM que canciones puedo mezclar?'")
        print("  3. [SETS] Creacion de Sets & Crates Organizados:")
        print("     Secuencio tracks por armonia, inyecto tags ID3/Vorbis y genero la guia '_00_SET_GUIDE.txt'.")
        print("     Ej: 'arma un set con las canciones de French Touch'")
        print("  4. [CLOUD] Sincronizacion en la Nube (Google Drive / S3 / R2):")
        print("     Subo tus sets en un clic via rclone para que los lleves en el movil o coche.")
        print("     Ej: 'sincroniza todos mis sets a Google Drive'")
        print("  5. [INVENTARIO] Catalogo de Biblioteca:")
        print("     Consulta en cualquier momento que tracks reales y sets tienes disponibles.")
        print("     Ej: 'que canciones tengo en mi biblioteca?'\n")
        return

    # 2. Sincronización en la Nube
    if any(k in q_low for k in ["drive", "google drive", "gdrive", "sincroniza", "sincronizar", "sincroniza a", "sube a", "subir a", "nube", "cloud", "backup"]):
        print("[*] Sincronizando sets locales con Google Drive via rclone...")
        res = await sync_sets_to_cloud()
        print(res + "\n")
        return

    # 3. Búsqueda y Adquisición P2P
    search_keywords = ["busca", "buscar", "buscame", "búscame", "descarga", "descargar", "descargame", "descárgame", "consigue", "consigueme", "consígueme", "encuentra", "download", "search"]
    if any(k in q_low for k in search_keywords):
        raw_terms = q_raw
        for prefix in [
            "buscame los mejores temas de", "búscame los mejores temas de",
            "buscame temas de", "búscame temas de", "busca temas de", "buscar temas de",
            "buscame", "búscame", "busca", "buscar",
            "descargame", "descárgame", "descarga", "descargar",
            "consigueme", "consígueme", "consigue",
            "search for", "download", "find"
        ]:
            if prefix in q_low:
                raw_terms = q_raw[q_low.find(prefix) + len(prefix):].strip()
                break

        terms = [t.strip() for t in raw_terms.replace(" como ", ",").replace(" y ", ",").replace(" and ", ",").split(",") if t.strip()]
        if not terms:
            terms = [raw_terms]

        print(f"[*] Buscando y adquiriendo pistas en la red Soulseek P2P: {', '.join(terms)}...")
        res = await search_and_acquire_tracks(terms)
        print(res + "\n")
        return

    # 4. Creación y Organización de Sets (Requiere acción + sustantivo de colección)
    has_action = any(a in q_low for a in ["arma", "armar", "crea", "crear", "haz", "hacer", "organiza", "organizar", "compila", "compilar", "nuevo", "nueva"])
    has_target = any(t in q_low for t in ["set", "sets", "crate", "crates", "sesion", "sesión", "playlist", "carpeta"])
    if (has_action and has_target) or "arma un set" in q_low or "crea un set" in q_low:
        set_name = "DJ Live Crate"
        if "sensation" in q_low:
            set_name = "Sensation White Live Crate"
        elif "french" in q_low or "house" in q_low:
            set_name = "French Touch & Vocal House"
        elif "trance" in q_low or "euro" in q_low:
            set_name = "Eurodance & Trance"
        elif "electro" in q_low:
            set_name = "Electro House & Peak-Time"

        print(f"[*] Analizando biblioteca, afinidad armonica Camelot y compilando '{set_name}'...")
        res = await create_or_update_dj_set(set_name, ["Modjo", "Stardust", "Daft Punk", "Tiesto", "Armand", "Spiller"])
        print(res + "\n")
        return

    # 5. Estado de biblioteca e inventario
    if any(k in q_low for k in ["biblioteca", "canciones", "cancion", "tracks", "inventario", "que temas tengo", "qué temas tengo", "que canciones", "qué canciones", "list", "show"]):
        res = get_library_status()
        print(res + "\n")
        return

    # 6. Consultas armónicas / Camelot Wheel
    if any(k in q_low for k in ["camelot", "armonia", "armonica", "armónica", "mezclar", "bpm", "tonalidad", "clave", "transicion", "transición"]):
        match = re.search(r"(\d{1,2}[abAB])", q_raw)
        key_found = match.group(1).upper() if match else "8A"
        bpm_match = re.search(r"(\d{2,3}(?:\.\d)?)", q_raw)
        bpm_found = float(bpm_match.group(1)) if bpm_match else 124.0
        print(recommend_harmonic_transitions(key_found, bpm_found) + "\n")
        return

    # Respuesta por defecto conversacional
    print(f"Entendido: '{user_input}'. Puedes pedirme buscar artistas o canciones, armar un set armonico, o escribir 'ayuda' para ver que puedo hacer.\n")
