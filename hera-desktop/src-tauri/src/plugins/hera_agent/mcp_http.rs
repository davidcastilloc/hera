use super::*;

/// Build the Axum router for the Bearer-protected loopback MCP server (`POST /mcp`).
pub fn build_local_mcp_router<R: Runtime>(ctx: AgentRuntimeContext<R>) -> Router {
    Router::new()
        .route("/mcp", post(mcp_http_handler::<R>))
        .with_state(ctx)
}

async fn mcp_http_handler<R: Runtime>(
    AxumState(ctx): AxumState<AgentRuntimeContext<R>>,
    headers: HeaderMap,
    Json(req): Json<Value>,
) -> Response {
    let expected = format!("Bearer {}", ctx.mcp_bearer_token);
    let provided = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Unauthorized MCP client" })),
        )
            .into_response();
    }

    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));

    // Notifications (no `id` or `notifications/*`) do not expect a response frame (JSON-RPC 2.0 §4.1)
    if id.is_none() || method.starts_with("notifications/") {
        return StatusCode::ACCEPTED.into_response();
    }

    match method {
        "initialize" => (
            StatusCode::OK,
            Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "hera-desktop-mcp", "version": env!("CARGO_PKG_VERSION") }
                }
            })),
        )
            .into_response(),
        "ping" => (
            StatusCode::OK,
            Json(json!({ "jsonrpc": "2.0", "id": id, "result": {} })),
        )
            .into_response(),
        "tools/list" => (
            StatusCode::OK,
            Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": build_unified_mcp_tools_list() }
            })),
        )
            .into_response(),
        "tools/call" => {
            let Some(name) = params.get("name").and_then(|v| v.as_str()) else {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32602, "message": "Missing tool name" }
                    })),
                )
                    .into_response();
            };
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let active_sid = ctx.active_mcp_session.lock().await.clone();
            match dispatch_unified_agent_tool(&ctx, active_sid.as_deref(), name, args).await {
                Ok(val) => {
                    let pretty =
                        serde_json::to_string_pretty(&val).unwrap_or_else(|_| val.to_string());
                    (
                        StatusCode::OK,
                        Json(json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{ "type": "text", "text": pretty }],
                                "structuredContent": val,
                                "isError": false
                            }
                        })),
                    )
                        .into_response()
                }
                Err(err_msg) => (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{ "type": "text", "text": format!("Error: {err_msg}") }],
                            "isError": true
                        }
                    })),
                )
                    .into_response(),
            }
        }
        _ => (
            StatusCode::OK,
            Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Unknown method '{method}'") }
            })),
        )
            .into_response(),
    }
}
