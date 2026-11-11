mod analyze;
mod dispatch_core;
mod dispatch_swarm;
#[cfg(feature = "native")]
pub mod rmcp_server;
#[cfg(all(test, feature = "native"))]
mod rmcp_tests;
mod schemas_core;
mod schemas_swarm;

#[cfg(feature = "native")]
pub use rmcp_server::{build_rmcp_tools, serve_mcp_io_rmcp, serve_mcp_stdio_rmcp, HeraRmcpServer};

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

/// Handle a single stateless MCP JSON-RPC 2.0 request (used by HTTP `POST /mcp` on `native` & `cloudflare` WASM).
pub fn handle_mcp_request(req: JsonRpcRequest, state: &P2PMeshState) -> Option<JsonRpcResponse> {
    let method = req.method.as_str();

    // Notifications (no `id` or `notifications/*` method) do not expect a response frame (JSON-RPC 2.0 §4.1)
    if req.id.is_none() || method.starts_with("notifications/") {
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

/// Run the official `rmcp` MCP Server over `stdin`/`stdout` (`hera-node --mcp`).
#[cfg(feature = "native")]
pub fn run_mcp_stdio_loop(state: P2PMeshState) {
    let outcome = match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(|| handle.block_on(serve_mcp_stdio_rmcp(state)))
        }
        Ok(_) => std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("failed to build stdio tokio runtime: {e}"))?
                .block_on(serve_mcp_stdio_rmcp(state))
        })
        .join()
        .unwrap_or_else(|_| Err("MCP stdio runtime thread panicked".to_string())),
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("failed to build stdio tokio runtime: {e}"))
            .and_then(|rt| rt.block_on(serve_mcp_stdio_rmcp(state))),
    };

    if let Err(err) = outcome {
        eprintln!("⚠️ MCP stdio server error: {err}");
    }
}
