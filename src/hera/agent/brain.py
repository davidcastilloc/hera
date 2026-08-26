"""Hera Agent Brain — Powered by Google Antigravity SDK.

This module delegates ALL natural language understanding and tool selection
to the LLM via the Antigravity SDK Agent. No hardcoded keyword matching.
The model reasons about the user's intent and autonomously decides which
tools to call, exactly like a real AI agent.

Auth strategy (in priority order):
  1. Vertex AI with Application Default Credentials (ADC) — zero config
  2. GEMINI_API_KEY env var — manual setup
  3. Interactive prompt for API key — last resort
"""

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

# Default Vertex AI config (works with ADC out of the box)
DEFAULT_VERTEX_PROJECT = "suite-aerya"
DEFAULT_VERTEX_LOCATION = "us-central1"
DEFAULT_MODEL = "gemini-2.5-flash"

HERA_TOOLS = [
    search_and_acquire_tracks,
    create_or_update_dj_set,
    sync_sets_to_cloud,
    get_library_status,
    recommend_harmonic_transitions,
]


class HeraBrain:
    """Real AI Agent orchestrator — the LLM decides everything."""

    def __init__(
        self,
        api_key: str | None = None,
        model: str = DEFAULT_MODEL,
        vertex_project: str | None = None,
        vertex_location: str = DEFAULT_VERTEX_LOCATION,
    ):
        self.api_key = api_key or os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
        self.model = model
        self.vertex_project = vertex_project
        self.vertex_location = vertex_location
        self.agent = None
        self._initialized = False
        self._auth_method = None

    async def initialize(self) -> bool:
        """Spawn the Antigravity Agent — tries Vertex AI ADC first, then API key."""
        from google.antigravity import Agent, LocalAgentConfig, CapabilitiesConfig

        # Strategy 1: Vertex AI with ADC (auto-discovers Google Cloud credentials)
        if not self.api_key:
            project = self.vertex_project
            if not project:
                try:
                    import google.auth
                    _, project = google.auth.default()
                except Exception:
                    project = None
            if not project:
                project = DEFAULT_VERTEX_PROJECT

            try:
                config = LocalAgentConfig(
                    system_instructions=HERA_SYSTEM_INSTRUCTIONS,
                    capabilities=CapabilitiesConfig(),
                    tools=HERA_TOOLS,
                    vertex=True,
                    project=project,
                    location=self.vertex_location,
                    model=self.model,
                )
                self.agent = Agent(config)
                await self.agent.__aenter__()
                self._initialized = True
                self._auth_method = f"Vertex AI (project: {project})"
                return True
            except Exception as e:
                print(f"[!] Vertex AI failed: {e}")

        # Strategy 2: Direct Gemini API key
        if self.api_key:
            try:
                config = LocalAgentConfig(
                    system_instructions=HERA_SYSTEM_INSTRUCTIONS,
                    capabilities=CapabilitiesConfig(),
                    tools=HERA_TOOLS,
                    api_key=self.api_key,
                    model=self.model,
                )
                self.agent = Agent(config)
                await self.agent.__aenter__()
                self._initialized = True
                self._auth_method = "Gemini API Key"
                return True
            except Exception as e:
                print(f"[!] API Key auth failed: {e}")

        return False

    async def chat(self, user_input: str) -> str:
        """Send a message to the agent and stream the response."""
        if not self.agent:
            return "[!] Agent not initialized."

        response = await self.agent.chat(user_input)
        full_response = []
        async for token in response:
            sys.stdout.write(token)
            sys.stdout.flush()
            full_response.append(token)
        return "".join(full_response)

    async def close(self):
        """Shut down the agent session."""
        if self.agent:
            try:
                await self.agent.__aexit__(None, None, None)
            except Exception:
                pass


async def run_hera_interactive_chat():
    """Interactive conversational console for DJs — 100% LLM-driven."""

    print()
    print("=" * 80)
    print(" HERA AI AGENT - Consola Conversacional para DJs")
    print("=" * 80)
    print("[*] Initializing agent...")

    brain = HeraBrain()
    initialized = await brain.initialize()

    if not initialized:
        # Last resort: ask for API key interactively
        print()
        print("[!] Could not auto-detect credentials.")
        print("    Get a free Gemini API key at: https://aistudio.google.com/apikey")
        print()
        try:
            entered = input("GEMINI_API_KEY: ").strip()
        except (EOFError, KeyboardInterrupt):
            return
        if not entered:
            print("[!] No API key provided. Cannot start agent.")
            return

        brain = HeraBrain(api_key=entered)
        initialized = await brain.initialize()
        if not initialized:
            print("[!] Authentication failed. Check your API key.")
            return

    print(f"[+] Backend: Google Antigravity SDK ({brain._auth_method})")
    print(f"[+] Model: {brain.model}")
    print(f"[+] Tools: {len(HERA_TOOLS)} registered (LLM selects autonomously)")
    print()
    print("Habla con Hera naturalmente en espanol o ingles.")
    print("(Escribe 'salir' para terminar)")
    print()

    try:
        while True:
            try:
                user_input = input("DJ > ").strip()
            except (EOFError, KeyboardInterrupt):
                break

            if not user_input:
                continue
            if user_input.lower() in ["salir", "exit", "quit", "q"]:
                print("\n[*] Cerrando sesion. Buena sesion en cabina!")
                break

            print("\nHera:\n", flush=True)

            try:
                await brain.chat(user_input)
                print("\n")
            except Exception as e:
                print(f"\n[!] Error: {e}\n")

    finally:
        await brain.close()
