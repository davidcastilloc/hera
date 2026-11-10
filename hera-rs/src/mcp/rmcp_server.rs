use super::{dispatch_tool_call, schemas_core, schemas_swarm};
use crate::p2p::P2PMeshState;
use crate::router::current_runtime_target;
use rmcp::{
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorData,
        Implementation, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig,
        Tool,
    },
    service::{RequestContext, RoleServer, ServerInitializeError},
    ServerHandler, ServiceExt,
};
use serde_json::{json, Value};
use std::{borrow::Cow, future::Future, sync::Arc};

/// Convert HERA's canonical MCP schema catalog (`core` + `swarm`, 22 tools total)
/// into official `rmcp::model::Tool` descriptors with 1:1 `name`, `description`, and `inputSchema`.
pub fn build_rmcp_tools() -> Vec<Tool> {
    let mut raw_schemas = schemas_core::core_tool_schemas();
    raw_schemas.extend(schemas_swarm::swarm_tool_schemas());

    raw_schemas
        .into_iter()
        .map(|entry| {
            let name = entry
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let description = entry
                .get("description")
                .and_then(Value::as_str)
                .map(|desc| Cow::Owned(desc.to_string()));
            let input_schema = entry
                .get("inputSchema")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();

            Tool::new_with_raw(name, description, Arc::new(input_schema))
        })
        .collect()
}

/// Official `rmcp` ServerHandler backed by `P2PMeshState` and HERA's unified tool dispatcher.
#[derive(Clone)]
pub struct HeraRmcpServer {
    state: P2PMeshState,
    tools: Arc<Vec<Tool>>,
}

impl HeraRmcpServer {
    pub fn new(state: P2PMeshState) -> Self {
        Self {
            state,
            tools: Arc::new(build_rmcp_tools()),
        }
    }
}

impl ServerHandler for HeraRmcpServer {
    fn get_info(&self) -> ServerConfig {
        let mut capabilities = ServerCapabilities::builder().enable_tools().build();
        if let Some(ref mut tools_cap) = capabilities.tools {
            tools_cap.list_changed = Some(false);
        }

        ServerConfig::new(capabilities).with_server_info(
            Implementation::new("hera-node-rs", env!("CARGO_PKG_VERSION")).with_description(
                format!(
                    "Hera Universal Audio DSP, 2-Opt TSP Crate Curator & P2P Node ({})",
                    current_runtime_target()
                ),
            ),
        )
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.tools
            .iter()
            .find(|tool| tool.name.as_ref() == name)
            .cloned()
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        std::future::ready(Ok(ListToolsResult::with_all_items((*self.tools).clone())))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResponse, ErrorData>> + Send + '_ {
        let tool_name = request.name;
        let args = request
            .arguments
            .map(Value::Object)
            .unwrap_or_else(|| json!({}));

        let result = match dispatch_tool_call(tool_name.as_ref(), args, &self.state) {
            Ok(payload) => {
                let pretty =
                    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string());
                let mut ok_res = CallToolResult::structured(payload);
                ok_res.content = vec![ContentBlock::text(pretty)];
                ok_res
            }
            Err(err_msg) => {
                CallToolResult::error(vec![ContentBlock::text(format!("ERROR: {err_msg}"))])
            }
        };

        std::future::ready(Ok(CallToolResponse::from(result)))
    }
}

/// Serve HERA's official `rmcp` MCP server over any async `(AsyncRead, AsyncWrite)` byte stream
/// (used by both `stdio` and deterministic `tokio::io::duplex` tests).
pub async fn serve_mcp_io_rmcp<I, O>(state: P2PMeshState, transport: (I, O)) -> Result<(), String>
where
    I: tokio::io::AsyncRead + Send + Unpin + 'static,
    O: tokio::io::AsyncWrite + Send + Unpin + 'static,
{
    let server = HeraRmcpServer::new(state);
    let running = match server.serve(transport).await {
        Ok(running) => running,
        Err(
            ServerInitializeError::ConnectionClosed(_)
            | ServerInitializeError::ExpectedInitializeRequest(None),
        ) => return Ok(()),
        Err(err) => return Err(format!("rmcp initialization error: {err}")),
    };

    running
        .waiting()
        .await
        .map(|_| ())
        .map_err(|err| format!("rmcp runtime join error: {err}"))
}

/// Serve HERA's official `rmcp` MCP server over process `stdin`/`stdout` (`hera-node --mcp`).
pub async fn serve_mcp_stdio_rmcp(state: P2PMeshState) -> Result<(), String> {
    serve_mcp_io_rmcp(state, rmcp::transport::io::stdio()).await
}
