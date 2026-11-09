mod analyze;
mod dispatch_core;
mod dispatch_swarm;
mod schemas_core;
mod schemas_swarm;

use crate::p2p::P2PMeshState;
use crate::router::current_runtime_target;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: Option<String>,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn ok(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

/// Returns the complete list of MCP tool definitions exposed by `hera-node-rs`.
pub fn list_mcp_tools() -> Value {
    let mut tools = schemas_core::core_tool_schemas();
    tools.extend(schemas_swarm::swarm_tool_schemas());
    json!({ "tools": tools })
}

/// Handle a single MCP JSON-RPC 2.0 request (works identically over `stdio` and HTTP `/mcp`).
pub fn handle_mcp_request(req: JsonRpcRequest, state: &P2PMeshState) -> Option<JsonRpcResponse> {
    let method = req.method.as_str();

    // Notifications (no `id`) do not expect a response frame in MCP stdio
    if method.starts_with("notifications/") && req.id.is_none() {
        return None;
    }

    match method {
        "initialize" => Some(JsonRpcResponse::ok(
            req.id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "hera-node-rs",
                    "version": env!("CARGO_PKG_VERSION"),
                    "runtime": current_runtime_target()
                }
            }),
        )),
        "ping" => Some(JsonRpcResponse::ok(req.id, json!({}))),
        "tools/list" => Some(JsonRpcResponse::ok(req.id, list_mcp_tools())),
        "tools/call" => {
            let tool_name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let args = req.params.get("arguments").cloned().unwrap_or(json!({}));

            match dispatch_tool_call(tool_name, args, state) {
                Ok(payload) => Some(JsonRpcResponse::ok(
                    req.id,
                    json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string())
                            }
                        ],
                        "structuredContent": payload,
                        "isError": false
                    }),
                )),
                Err(err_msg) => Some(JsonRpcResponse::ok(
                    req.id,
                    json!({
                        "content": [
                            {
                                "type": "text",
                                "text": format!("ERROR: {err_msg}")
                            }
                        ],
                        "isError": true
                    }),
                )),
            }
        }
        _ => Some(JsonRpcResponse::err(
            req.id,
            -32601,
            format!("Method not found: {method}"),
        )),
    }
}

pub fn dispatch_tool_call(name: &str, args: Value, state: &P2PMeshState) -> Result<Value, String> {
    if let Some(res) = dispatch_core::dispatch_core_tool(name, &args, state) {
        return res;
    }
    dispatch_swarm::dispatch_swarm_tool(name, args, state)
}

/// Run the MCP JSON-RPC 2.0 Server over `stdin`/`stdout` (`hera-node --mcp`).
#[cfg(not(target_arch = "wasm32"))]
pub fn run_mcp_stdio_loop(state: P2PMeshState) {
    use std::io::{self, BufRead, Write};
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let Ok(raw) = line else { break };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let resp = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => handle_mcp_request(req, &state),
            Err(e) => Some(JsonRpcResponse::err(
                None,
                -32700,
                format!("Parse error: {e}"),
            )),
        };
        if let Some(r) = resp {
            if let Ok(out) = serde_json::to_string(&r) {
                let _ = writeln!(stdout, "{out}");
                let _ = stdout.flush();
            }
        }
    }
}
