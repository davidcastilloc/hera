# Spec: Pure-Rust Agentic Harness (`HeraAgentHarness`) & Universal Local/Cloud LLM Backend Integration (`LM Studio` / `Ollama` / `Cloudflare AI`)

**Date:** 2026-11-08  
**Author:** David Castillo (`@davidcastilloc`, `vikruzdavid@gmail.com`)  
**Repositories:** `davidcastilloc/hera` & `davidcastilloc/hera-cloud`  
**Status:** Approved

---

## 1. Intent & Problem Statement

Following the `Zero-Python` migration (`c08677f`), the legacy Python `google-antigravity` SDK agent (`src/hera/agent/brain.py` and `src/hera/agent/backends.py`) was removed so `HERA Desktop` and `hera-node` ship as a self-contained `~8 MB` 100% Rust binary. However, `hera-rs/src/dj_brain.rs` was left with a static 5-item backend list (missing `lmstudio` on port `1234` and other local engines) and a deterministic template stub in `evaluate_cloudflare_free_ai_turn` that does not perform real OpenAI-compatible HTTP calls or expose HERA's 21 native MCP tools to local/cloud LLMs.

To solve real DJ engineering and repertoire curation problems autonomously—just like a professional coding agent harness—HERA's Rust backend must provide a **Pure-Rust Multi-Turn Agentic Harness (`HeraAgentHarness`)** capable of connecting to **LM Studio** (`http://localhost:1234/v1` or LAN IP), **Ollama** (`http://localhost:11434/v1`), or any OpenAI-compatible cloud/local endpoint, discovering loaded models, injecting the 20 operational system tools from `hera-rs/src/mcp.rs` (including `hera_code_mode`), executing a multi-turn `ReAct` (`Reason -> Act -> Observe -> Self-Heal`) loop against the live `P2PMeshState`, and returning both the synthesized answer and a structured `tool_calls_trace`.

---

## 2. Goals, Non-Goals & Constraints

### Goals
1. **12-Backend Universal Registry & Local Auto-Discovery (`hera-rs/src/dj_brain.rs`):**
   - Support all 12 backends from the original HERA architecture: `cloudflare_free`, `openrouter`, `gemini`, `vertex`, `openai`, `anthropic`, `lmstudio` (`http://localhost:1234/v1`), `ollama` (`http://localhost:11434/v1`), `jan` (`http://localhost:1337/v1`), `llamacpp` (`http://localhost:8080/v1`), `vllm` (`http://localhost:8000/v1`), and `custom`.
   - Provide `probe_backend_models(base_url)` (`GET /v1/models` and Ollama `/api/tags` fallback) and `auto_detect_local_backend()` (`lmstudio` -> `ollama` -> `jan` -> `llamacpp` -> `vllm`).
2. **System Context, DJ Style DNA & Full MCP Tool Catalog Injection:**
   - Transform `mcp::list_mcp_tools()` into OpenAI-compatible function tool definitions (`build_openai_tools_catalog()`), exposing all 20 operational HERA tools (`search_music`, `get_track_candidates`, `analyze_track`, `analyze_batch`, `build_dj_crate`, `audit_repertoire`, `announce_p2p_track`, `get_node_manifest`, `hera_code_mode`, `get_code_mode_sdk`, `connect_dj_drive`, `get_swarm_strength`, `swarm_heartbeat`, `swarm_route_ticket`, `swarm_custody_audit`, `swarm_parallel_manifest`, `swarm_crdt_sync`, `swarm_subscribe`, `export_dj_crate`, `compute_dj_style_dna`) while excluding `chat_with_dj_copilot` to prevent recursive self-calls.
   - Build a rich operational system prompt (`build_agent_harness_system_prompt`) combining HERA invariants (`Quarantine-First`, Zero-Trust FFT forensics, 4-Layer Holistic Synergy, `hera_code_mode`), live `P2PMeshState` catalog metrics, and the DJ's `DjStyleDnaProfile`.
3. **Multi-Turn ReAct Loop, Dual Tool Call Protocol & Self-Healing (`evaluate_agent_harness_turn`):**
   - Execute up to `max_tool_iterations` (default `6`, bounded `1..=10`) turns against any OpenAI-compatible `/v1/chat/completions` endpoint.
   - Support both standard OpenAI `message.tool_calls` and fallback `<tool_call>{...}</tool_call>` / fenced JSON blocks emitted by quantized local GGUF models in LM Studio and Ollama.
   - Dispatch tool invocations in-memory via `mcp::dispatch_tool_call(name, args, state)`. If a tool returns an error or malformed JSON arguments, return a structured error observation (`role: "tool"`) with a self-healing hint so the model can correct its call in the next iteration.
   - Record every tool execution in `tool_calls_trace: Vec<AgentToolExecutionStep>` (`step`, `tool_name`, `arguments`, `result_preview`, `is_error`).
4. **Desktop UI Selector & Live Tool Trace (`hera-desktop`):**
   - Allow selecting `backend` (`cloudflare_free`, `lmstudio`, `ollama`, `openrouter`, `gemini`, `openai`, `custom`), custom `base_url` (e.g. `http://localhost:1234/v1` or LAN `http://10.0.0.x:1234/v1`), and `model` in `hera-desktop/src/routes/bridge.tsx`, and display the executed `tool_calls_trace` badges in the Copilot panel.

### Non-Goals
- Re-introducing Python or `google-antigravity` Python packages into `hera`.
- Adding heavy third-party Rust crates that break `wasm32-unknown-unknown` compilation for Cloudflare Workers.

---

## 3. Approaches Considered

1. **Restore Legacy Python `google-antigravity` Agent:** Rejected by user; violates the `100% Pure Rust (~8 MB, Zero Python)` architecture established in `c08677f` and `ADR-006`.
2. **Add Heavy External Rust Agent Crates (`rig-core` / `async-openai`):** Rejected under Ponytail Ultra / YAGNI; pulls in `reqwest`/`openssl`/`rustls` dependency trees that conflict with `wasm32` Cloudflare Worker builds and bloat compile times.
3. **Pure-Rust Universal OpenAI-Compatible ReAct Harness in `hera-rs/src/dj_brain.rs` Wired Directly to `hera-rs/src/mcp.rs` (Selected):** Uses zero-dependency `std::net::TcpStream` HTTP/1.1 for local/LAN endpoints (`http://localhost:1234/v1`, etc.) with native `curl` fallback for `https://` endpoints on non-WASM targets, compiles cleanly on both `native` and `wasm32`, and reuses `mcp::list_mcp_tools()` and `mcp::dispatch_tool_call()` with zero duplication.
