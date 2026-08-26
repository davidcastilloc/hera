"""Hera Agent Brain — Powered by Google Antigravity SDK.

This module delegates ALL natural language understanding and tool selection
to the LLM via the Antigravity SDK Agent. No hardcoded keyword matching.
The model reasons about the user's intent and autonomously decides which
tools to call, exactly like a real AI agent.
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


class HeraBrain:
    """Real AI Agent orchestrator — the LLM decides everything."""

    def __init__(self, api_key: str | None = None, model: str = "gemini-2.5-flash"):
        self.api_key = api_key or os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
        self.model = model
        self.agent = None
        self._initialized = False

    async def initialize(self) -> bool:
        """Spawn the Antigravity Agent with Hera's DJ tools registered."""
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
            self._initialized = True
            return True
        except Exception as e:
            print(f"[!] Error initializing Antigravity Agent: {e}")
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
    """Interactive conversational console for DJs — 100% LLM-driven, zero hardcoded logic."""

    # Resolve API key: env var first, then prompt user
    api_key = os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")

    if not api_key:
        print("=" * 80)
        print(" HERA AI AGENT - Setup")
        print("=" * 80)
        print()
        print("Hera needs a Gemini API key to reason autonomously with the LLM.")
        print("Get one for FREE in 30 seconds at: https://aistudio.google.com/apikey")
        print()
        print("Once you have it, you can either:")
        print("  1. Set it as env var:  set GEMINI_API_KEY=your_key_here")
        print("  2. Paste it below (it won't be stored anywhere):")
        print()
        try:
            entered = input("GEMINI_API_KEY: ").strip()
        except (EOFError, KeyboardInterrupt):
            return
        if not entered:
            print("\n[!] No API key provided. Hera requires a Gemini API key to function as a real agent.")
            print("    Run: set GEMINI_API_KEY=your_key_here")
            print("    Then: uv run hera chat")
            return
        api_key = entered
        os.environ["GEMINI_API_KEY"] = api_key

    # Initialize the real Antigravity Agent
    brain = HeraBrain(api_key=api_key)
    initialized = await brain.initialize()

    if not initialized:
        print("\n[!] Could not initialize the Antigravity Agent.")
        print("    Check your API key and internet connection.")
        return

    print()
    print("=" * 80)
    print(" HERA AI AGENT - Consola Conversacional para DJs")
    print("=" * 80)
    print("[+] Backend: Google Antigravity SDK (LLM Reasoning + Tool Calling)")
    print("[+] Model:", brain.model)
    print("[+] Tools: search_and_acquire_tracks, create_or_update_dj_set,")
    print("           sync_sets_to_cloud, get_library_status, recommend_harmonic_transitions")
    print()
    print("Habla con Hera naturalmente. El LLM decide que herramientas ejecutar.")
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
                print("\n[*] Cerrando sesion. Que tengas una gran sesion en cabina!")
                break

            print("\nHera:\n", flush=True)

            try:
                await brain.chat(user_input)
                print("\n")
            except Exception as e:
                print(f"\n[!] Error: {e}\n")

    finally:
        await brain.close()
