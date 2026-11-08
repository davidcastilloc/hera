# Pure-Rust Agentic Harness & Universal Local/Cloud LLM Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Equip `hera-rs` and `hera-desktop` with a 100% Pure-Rust multi-turn Agentic Harness (`HeraAgentHarness`) that connects to LM Studio (`:1234`), Ollama (`:11434`), or any OpenAI-compatible backend, discovers loaded models, exposes all 20 operational HERA MCP system tools, executes a self-healing ReAct loop against live `P2PMeshState`, and surfaces tool execution traces in the UI.

**Architecture:** Extends `hera-rs/src/mcp.rs` to export `pub fn dispatch_tool_call` and transforms `hera-rs/src/dj_brain.rs` into a multi-turn ReAct agent orchestrator speaking the standard OpenAI `/v1/chat/completions` and `/v1/models` protocol over zero-dependency Rust networking (`TcpStream` for local/LAN `http://` + native `curl` fallback for `https://`), with deterministic fallback when offline.

**Tech Stack:** Rust 2021 (`hera-rs` + `hera-cloud`, `serde`, `serde_json`, `axum`), Tauri v2 (`hera-desktop/src-tauri`), TypeScript + React 19 + StyleX (`hera-desktop/src`).

**Spec:** `docs/superpowers/specs/2026-11-08-rust-agent-harness-and-local-llm-design.md`

## Global Constraints

- Zero Python dependencies and zero new heavy crates in `hera-rs/Cargo.toml`; must compile cleanly on both `native` (`tokio`) and `cloudflare` (`wasm32`) targets.
- Never commit OS-specific absolute paths (`C:\`, `/Users/`, `/home/`).
- Every task must be verified with real terminal receipts (`cargo check`, `cargo test`) and committed atomically under `David Castillo <vikruzdavid@gmail.com>`.

## Review Focus

- Quantized local models in LM Studio / Ollama sometimes emit tool calls inside `message.content` (`<tool_call>{"name":...}</tool_call>` or fenced JSON) instead of `message.tool_calls`: the parser must handle both natively.
- Recursive tool loop prevention: `build_openai_tools_catalog()` must exclude `chat_with_dj_copilot` and `evaluate_agent_harness_turn` must cap iterations at `max_tool_iterations` (`1..=10`, default `6`).
- Self-healing tool errors: when `dispatch_tool_call` fails or receives malformed JSON arguments, the harness must return a structured error observation (`role: "tool"`) rather than aborting the turn.
- HTTP chunked/Content-Length parsing on local servers: LM Studio (`http://localhost:1234/v1`) sends HTTP/1.1 responses that may include `Connection: close` or `Content-Length`; the client must parse the JSON body reliably and enforce connection/read timeouts so unreachable ports never hang the UI.

---

### Task 1: Universal 12-Backend Registry, Model Auto-Discovery & OpenAI Tool Catalog (`hera-rs/src/mcp.rs` & `hera-rs/src/dj_brain.rs`)

**Files:**
- Modify: `hera-rs/src/mcp.rs`
- Modify: `hera-rs/src/dj_brain.rs`

**Interfaces:**
- Produces:
  - `pub fn dispatch_tool_call(name: &str, args: serde_json::Value, state: &P2PMeshState) -> Result<serde_json::Value, String>` in `hera-rs/src/mcp.rs`
  - `pub fn list_supported_ai_backends() -> Vec<AiBackendOption>` (expanded to 12 backends including `lmstudio` on `http://localhost:1234/v1`, `ollama` on `http://localhost:11434/v1`, `jan`, `llamacpp`, `vllm`, `cloudflare_free`, `openrouter`, `gemini`, `vertex`, `openai`, `anthropic`, `custom`)
  - `pub fn build_openai_tools_catalog() -> Vec<serde_json::Value>`
  - `pub fn build_agent_harness_system_prompt(dj_handle: &str, style_dna: Option<&DjStyleDnaProfile>, state: &P2PMeshState) -> String`
  - `pub fn probe_backend_models(base_url: &str, api_key: Option<&str>) -> Result<Vec<String>, String>`
  - `pub fn auto_detect_local_backend() -> Option<(String, String, String)>`

- [ ] **Step 1: Expose `pub fn dispatch_tool_call` in `hera-rs/src/mcp.rs` and add unit tests in `hera-rs/src/dj_brain.rs` for the 12-backend registry, OpenAI tool schema conversion, and model probing**
- [ ] **Step 2: Implement `list_supported_ai_backends`, `build_openai_tools_catalog`, `build_agent_harness_system_prompt`, `probe_backend_models`, and `auto_detect_local_backend` in `hera-rs/src/dj_brain.rs`**
- [ ] **Step 3: Run `cargo check --manifest-path hera-rs/Cargo.toml` and `cargo test --manifest-path hera-rs/Cargo.toml` and capture `exit code 0` receipt**
- [ ] **Step 4: Mark `- [x]` in plan and commit atomically**

---

### Task 2: Multi-Turn ReAct Agent Harness Loop, Dual Tool Call Parser, Self-Healing & HTTP/MCP Wiring (`hera-rs/src/dj_brain.rs`, `hera-rs/src/router.rs`, `hera-rs/src/mcp.rs`)

**Files:**
- Modify: `hera-rs/src/dj_brain.rs`
- Modify: `hera-rs/src/router.rs`
- Modify: `hera-rs/src/mcp.rs`

**Interfaces:**
- Consumes: `mcp::dispatch_tool_call`, `build_openai_tools_catalog`, `build_agent_harness_system_prompt`, `probe_backend_models`
- Produces:
  - `pub struct AgentToolExecutionStep { pub step: usize, pub tool_name: String, pub arguments: serde_json::Value, pub result_preview: String, pub is_error: bool }`
  - Extended `CloudflareFreeAiTurnRequest` (`backend: Option<String>`, `base_url: Option<String>`, `model: Option<String>`, `api_key: Option<String>`, `max_tool_iterations: Option<usize>`)
  - Extended `CloudflareFreeAiTurnResponse` (`tool_calls_trace: Vec<AgentToolExecutionStep>`, `iterations_executed: usize`)
  - `pub fn evaluate_agent_harness_turn(req: &CloudflareFreeAiTurnRequest, state: &P2PMeshState) -> CloudflareFreeAiTurnResponse`
  - Updated `/api/v1/ai/chat` handler in `hera-rs/src/router.rs` and `chat_with_dj_copilot` in `hera-rs/src/mcp.rs` passing `&state` to `evaluate_agent_harness_turn`
  - New `GET /api/v1/ai/probe` endpoint in `hera-rs/src/router.rs` to probe local/remote LLM endpoints (`LM Studio`, `Ollama`, custom URL) and return discovered models

- [ ] **Step 1: Write unit & integration tests in `hera-rs/src/dj_brain.rs` using a local ephemeral TCP mock LM Studio server testing: (a) multi-turn ReAct tool calling (`search_music` -> `build_dj_crate` -> final synthesis), (b) self-healing recovery on invalid tool arguments, and (c) fallback `<tool_call>` tag parsing for quantized local models**
- [ ] **Step 2: Implement `evaluate_agent_harness_turn`, dual tool-call parser, HTTP/1.1 `/v1/chat/completions` client, and wire `router.rs` + `mcp.rs`**
- [ ] **Step 3: Run `cargo check --manifest-path hera-rs/Cargo.toml` and `cargo test --manifest-path hera-rs/Cargo.toml` and capture `exit code 0` receipt**
- [ ] **Step 4: Mark `- [x]` in plan and commit atomically**

---

### Task 3: `HERA Desktop` Local LLM / LM Studio UI Selector, Tool Trace View & `hera-cloud` Parity (`hera-desktop/` & `hera-cloud/`)

**Files:**
- Modify: `hera-desktop/src-tauri/src/plugins/hera_dj.rs`
- Modify: `hera-desktop/src-tauri/build.rs`
- Modify: `hera-desktop/src-tauri/capabilities/main.json`
- Modify: `hera-desktop/src/lib/bridge-hera.ts`
- Modify: `hera-desktop/src/routes/bridge.tsx`
- Modify: `hera-cloud/src/mcp.rs`
- Modify: `hera-cloud/src/dj_brain.rs`
- Modify: `hera-cloud/src/router.rs`
- Modify: `docs/superpowers/DECISIONS.md`

**Interfaces:**
- Consumes: `evaluate_agent_harness_turn`, `list_supported_ai_backends`, `probe_backend_models`, `auto_detect_local_backend`
- Produces:
  - Updated `chat_copilot` and new `probe_ai_backend` Tauri commands in `hera-desktop/src-tauri/src/plugins/hera_dj.rs` (registered in `main.rs`, `build.rs`, and `capabilities/main.json` per `ADR-007`)
  - Backend selector (`cloudflare_free`, `lmstudio`, `ollama`, `openrouter`, `gemini`, `openai`, `custom`), `baseUrl` / `model` inputs, live model probe button, and `tool_calls_trace` visualization in `hera-desktop/src/routes/bridge.tsx`
  - Parity sync in `hera-cloud/src/{mcp.rs,dj_brain.rs,router.rs}` and `ADR-008` in `docs/superpowers/DECISIONS.md`

- [ ] **Step 1: Update `hera-desktop/src-tauri/src/plugins/hera_dj.rs`, `build.rs`, `capabilities/main.json`, `bridge-hera.ts`, and `bridge.tsx`**
- [ ] **Step 2: Synchronize `hera-cloud/src/{mcp.rs,dj_brain.rs,router.rs}` and record `ADR-008` in `docs/superpowers/DECISIONS.md`**
- [ ] **Step 3: Verify `cargo check` and `cargo test` in `hera-rs`, `hera-desktop/src-tauri`, and `hera-cloud` with real `exit code 0` receipts**
- [ ] **Step 4: Mark `- [x]` in plan and commit atomically**
