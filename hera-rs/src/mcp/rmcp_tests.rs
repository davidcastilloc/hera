use super::rmcp_server::{build_rmcp_tools, serve_mcp_io_rmcp};
use super::{handle_mcp_request, list_mcp_tools, JsonRpcRequest};
use crate::p2p::P2PMeshState;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

async fn send_json_line<W: tokio::io::AsyncWrite + Unpin>(writer: &mut W, msg: Value) {
    let mut raw = serde_json::to_string(&msg).expect("serialize JSON-RPC message");
    raw.push('\n');
    writer
        .write_all(raw.as_bytes())
        .await
        .expect("write JSON-RPC line to duplex transport");
    writer.flush().await.expect("flush duplex transport");
}

async fn recv_json_line<R: tokio::io::AsyncRead + Unpin>(
    lines: &mut tokio::io::Lines<BufReader<R>>,
) -> Value {
    let line = tokio::time::timeout(std::time::Duration::from_secs(5), lines.next_line())
        .await
        .expect("timed out waiting for rmcp response line")
        .expect("read line from duplex transport")
        .expect("expected JSON-RPC line before EOF");
    serde_json::from_str(&line).expect("parse JSON-RPC response line")
}

#[test]
fn shadow_rmcp_tools_match_legacy_catalog_1_to_1() {
    let rmcp_tools = build_rmcp_tools();
    assert_eq!(
        rmcp_tools.len(),
        22,
        "rmcp catalog must expose all 22 HERA tools (Step 0 + 21 domain tools)"
    );

    let rmcp_serialized =
        serde_json::to_value(&rmcp_tools).expect("serialize Vec<rmcp::model::Tool>");
    let legacy_catalog = list_mcp_tools();

    assert_eq!(
        rmcp_serialized, legacy_catalog["tools"],
        "build_rmcp_tools() must match list_mcp_tools()[\"tools\"] 1:1 across name, description, and inputSchema"
    );
}

#[tokio::test]
async fn duplex_rmcp_session_serves_initialize_ping_list_and_tool_calls() {
    let state = P2PMeshState::new("hera-rmcp-duplex-node");
    let shadow_state = state.clone();

    let (server_stream, client_stream) = tokio::io::duplex(64 * 1024);
    let server_task =
        tokio::spawn(async move { serve_mcp_io_rmcp(state, tokio::io::split(server_stream)).await });

    let (client_read, mut client_write) = tokio::io::split(client_stream);
    let mut client_lines = BufReader::new(client_read).lines();

    // 1. Handshake: `initialize` + `notifications/initialized`
    send_json_line(
        &mut client_write,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "hera-shadow-test-client",
                    "version": "0.6.0"
                }
            }
        }),
    )
    .await;

    let init_resp = recv_json_line(&mut client_lines).await;
    assert_eq!(init_resp["id"], 1);
    assert_eq!(init_resp["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(init_resp["result"]["serverInfo"]["name"], "hera-node-rs");
    assert_eq!(
        init_resp["result"]["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(
        init_resp["result"]["capabilities"]["tools"]["listChanged"],
        false
    );

    send_json_line(
        &mut client_write,
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }),
    )
    .await;

    // 2. `ping`
    send_json_line(
        &mut client_write,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "ping"
        }),
    )
    .await;
    let ping_resp = recv_json_line(&mut client_lines).await;
    assert_eq!(ping_resp["id"], 2);
    assert_eq!(ping_resp["result"], json!({}));

    // 3. `tools/list` parity over wire transport
    send_json_line(
        &mut client_write,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list"
        }),
    )
    .await;
    let list_resp = recv_json_line(&mut client_lines).await;
    assert_eq!(list_resp["id"], 3);
    let wire_tools = list_resp["result"]["tools"]
        .as_array()
        .expect("tools/list must return tools array");
    assert_eq!(wire_tools.len(), 22);
    assert_eq!(list_resp["result"]["tools"], list_mcp_tools()["tools"]);

    // 4. `tools/call` (`ensure_hera_updated` with `pull_remote: false` + `get_swarm_strength`)
    send_json_line(
        &mut client_write,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "ensure_hera_updated",
                "arguments": { "pull_remote": false }
            }
        }),
    )
    .await;
    let updated_resp = recv_json_line(&mut client_lines).await;
    let shadow_http = handle_mcp_request(
        JsonRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(json!(4)),
            method: "tools/call".to_string(),
            params: json!({
                "name": "ensure_hera_updated",
                "arguments": { "pull_remote": false }
            }),
        },
        &shadow_state,
    )
    .expect("HTTP MCP response")
    .result
    .expect("HTTP MCP result");

    assert_eq!(updated_resp["id"], 4);
    assert_eq!(updated_resp["result"]["isError"], false);
    assert_eq!(
        updated_resp["result"]["structuredContent"],
        shadow_http["structuredContent"]
    );
    assert_eq!(updated_resp["result"]["content"], shadow_http["content"]);

    send_json_line(
        &mut client_write,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "get_swarm_strength",
                "arguments": {}
            }
        }),
    )
    .await;
    let swarm_resp = recv_json_line(&mut client_lines).await;
    assert_eq!(swarm_resp["id"], 5);
    assert_eq!(swarm_resp["result"]["isError"], false);
    assert!(swarm_resp["result"]["structuredContent"].is_object());

    // 5. Error parity on unknown tool name (`isError: true`, `"ERROR: ..."`)
    send_json_line(
        &mut client_write,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "nonexistent_hera_tool",
                "arguments": {}
            }
        }),
    )
    .await;
    let err_resp = recv_json_line(&mut client_lines).await;
    assert_eq!(err_resp["id"], 6);
    assert_eq!(err_resp["result"]["isError"], true);
    let err_text = err_resp["result"]["content"][0]["text"]
        .as_str()
        .expect("error content text");
    assert!(err_text.starts_with("ERROR: "));

    // 6. Clean EOF shutdown
    drop(client_write);
    drop(client_lines);
    let exit = tokio::time::timeout(std::time::Duration::from_secs(5), server_task)
        .await
        .expect("server task shutdown timeout")
        .expect("server task join");
    assert_eq!(exit, Ok(()));
}
