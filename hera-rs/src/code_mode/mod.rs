//! Cloudflare "Code Mode" Engine for Hera MCP (`https://blog.cloudflare.com/code-mode/`).
//!
//! Instead of forcing an LLM to make 10 sequential tool round-trips through its context window,
//! Code Mode exposes:
//! 1. Strongly-typed TypeScript definitions (`GET /mcp/code-mode.d.ts`) for the `hera` SDK namespace.
//! 2. An in-memory sandboxed Code Mode runner (`hera_code_mode` MCP tool & `POST /mcp/code-mode`)
//!    that executes multi-step TypeScript/JS `hera.*` programs and declarative pipelines in < 1 ms
//!    directly inside Rust (both on Cloudflare Workers WASM and Native VPS).

mod dispatch_core;
mod dispatch_swarm;
mod parser;
mod sdk_spec;

pub use sdk_spec::{get_openapi_3_1_spec, get_typescript_sdk_definitions};

use crate::p2p::P2PMeshState;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Request payload for `POST /mcp/code-mode` and the `hera_code_mode` MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeModeRequest {
    /// TypeScript / JavaScript code using the `hera.*` SDK (e.g. `const tracks = await hera.searchMusic({...}); return hera.buildDjCrate({...});`)
    #[serde(default)]
    pub code: Option<String>,
    /// Optional declarative pipeline steps for deterministic multi-operation chaining.
    #[serde(default)]
    pub pipeline: Option<Vec<CodeModeStep>>,
    /// Optional initial context variables accessible inside the script/pipeline.
    #[serde(default)]
    pub context: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeModeStep {
    /// Variable name to bind the output of this step into the execution scope.
    pub bind: String,
    /// SDK method name: `hera.searchMusic`, `hera.filterTracks`, `hera.evaluateTransition`, `hera.buildDjCrate`, `hera.auditPcm`, `hera.auditRepertoire`.
    pub call: String,
    /// Arguments object (supports `"$varName"` references to previous step bindings).
    #[serde(default)]
    pub args: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeModeExecutionResult {
    pub mode: String,
    pub operations_executed: usize,
    pub execution_log: Vec<String>,
    pub output: Value,
}

/// Execute a Code Mode request (either TypeScript `hera.*` script or declarative step pipeline) in memory.
pub fn execute_code_mode(
    req: CodeModeRequest,
    state: &P2PMeshState,
) -> Result<CodeModeExecutionResult, String> {
    let mut scope: Map<String, Value> = req.context;
    let mut log: Vec<String> = Vec::new();
    let mut ops_count = 0usize;

    let steps = if let Some(ref ts_code) = req.code {
        if !ts_code.trim().is_empty() {
            parser::parse_ts_code_to_steps(ts_code, &mut scope, &mut log)?
        } else {
            req.pipeline.unwrap_or_default()
        }
    } else {
        req.pipeline.unwrap_or_default()
    };

    if steps.is_empty() && req.code.is_none() {
        return Err(
            "Provide either `code` (TypeScript/JS using `hera.*`) or `pipeline` steps.".to_string(),
        );
    }

    let mut last_value = Value::Null;

    for step in steps {
        ops_count += 1;
        let resolved_args = parser::resolve_variables(&step.args, &scope);
        let result = execute_single_sdk_call(&step.call, &resolved_args, state, &mut log)?;
        last_value = result.clone();
        scope.insert(step.bind.clone(), result);
    }

    if let Some(ret_expr) = scope.get("__return_expr__").cloned() {
        last_value = parser::resolve_variables(&ret_expr, &scope);
    }

    Ok(CodeModeExecutionResult {
        mode: "cloudflare_code_mode_rust_sandbox".to_string(),
        operations_executed: ops_count,
        execution_log: log,
        output: last_value,
    })
}

fn execute_single_sdk_call(
    call: &str,
    args: &Value,
    state: &P2PMeshState,
    log: &mut Vec<String>,
) -> Result<Value, String> {
    let normalized = call.trim().trim_start_matches("await ").trim();
    if let Some(res) = dispatch_core::execute_core_sdk_call(normalized, args, state, log) {
        return res;
    }
    dispatch_swarm::execute_swarm_sdk_call(call, normalized, args, state, log)
}
