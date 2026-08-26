"""Hera Agent Brain — Powered by Google Antigravity SDK.

All natural language understanding and tool selection is delegated to the LLM.
Supports 12 backends (4 cloud + 8 local) via the BackendRegistry.
"""

import asyncio
import os
import sys

from hera.agent.backends import BackendRegistry, BACKENDS
from hera.agent.prompts import HERA_SYSTEM_INSTRUCTIONS
from hera.agent.tools import (
    search_and_acquire_tracks,
    create_or_update_dj_set,
    sync_sets_to_cloud,
    get_library_status,
    recommend_harmonic_transitions,
)
from hera.domain.config import AgentConfig

HERA_TOOLS = [
    search_and_acquire_tracks,
    create_or_update_dj_set,
    sync_sets_to_cloud,
    get_library_status,
    recommend_harmonic_transitions,
]


class HeraBrain:
    """Real AI Agent orchestrator — the LLM decides everything."""

    def __init__(self, agent_config: AgentConfig | None = None):
        self.config = agent_config or AgentConfig()
        self.agent = None
        self._initialized = False
        self._display = ""

    async def initialize(self) -> bool:
        """Resolve backend and spawn the Antigravity Agent."""
        resolved = BackendRegistry.resolve(self.config)
        if not resolved:
            return False

        self._display = resolved["display"]

        try:
            from google.antigravity import Agent, LocalAgentConfig, CapabilitiesConfig

            backend_type = resolved["type"]

            if backend_type == "gemini":
                sdk_config = LocalAgentConfig(
                    system_instructions=HERA_SYSTEM_INSTRUCTIONS,
                    capabilities=CapabilitiesConfig(),
                    tools=HERA_TOOLS,
                    api_key=resolved["api_key"],
                    model=resolved["model"],
                )

            elif backend_type == "vertex":
                sdk_config = LocalAgentConfig(
                    system_instructions=HERA_SYSTEM_INSTRUCTIONS,
                    capabilities=CapabilitiesConfig(),
                    tools=HERA_TOOLS,
                    vertex=True,
                    project=resolved["project"],
                    location=resolved["location"],
                    model=resolved["model"],
                )

            elif backend_type == "openai_compatible":
                from google.antigravity.models import ModelTarget, OpenAICompatibleEndpoint
                sdk_config = LocalAgentConfig(
                    system_instructions=HERA_SYSTEM_INSTRUCTIONS,
                    capabilities=CapabilitiesConfig(),
                    tools=HERA_TOOLS,
                    model=ModelTarget(
                        model=resolved["model"],
                        endpoint=OpenAICompatibleEndpoint(
                            base_url=resolved["base_url"],
                            api_key=resolved["api_key"],
                        ),
                    ),
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


async def run_hera_interactive_chat(
    backend: str | None = None,
    model: str | None = None,
    base_url: str | None = None,
):
    """Interactive conversational console for DJs — 100% LLM-driven."""

    # Build AgentConfig from CLI args or defaults
    agent_config = AgentConfig(
        backend=backend or "auto",
        model=model,
        base_url=base_url,
    )

    print()
    print("=" * 80)
    print(" HERA AI AGENT - Consola Conversacional para DJs")
    print("=" * 80)
    print("[*] Detecting backend...", flush=True)

    brain = HeraBrain(agent_config)
    initialized = await brain.initialize()

    if not initialized:
        # Last resort: ask for API key interactively
        print()
        print("[!] No backend detected automatically.")
        print()
        print("Options:")
        print("  1. Start a local engine:  ollama serve")
        print("  2. Set an API key:        set GEMINI_API_KEY=your_key")
        print("  3. Paste a key below:")
        print()
        try:
            entered = input("GEMINI_API_KEY: ").strip()
        except (EOFError, KeyboardInterrupt):
            return
        if not entered:
            print("[!] No key provided. Run 'hera chat --backend ollama' with Ollama running.")
            return

        agent_config = AgentConfig(backend="gemini", api_key=entered, model="gemini-2.5-flash")
        brain = HeraBrain(agent_config)
        initialized = await brain.initialize()
        if not initialized:
            print("[!] Authentication failed. Check your API key.")
            return

    print(f"[+] Backend: {brain._display}")
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
