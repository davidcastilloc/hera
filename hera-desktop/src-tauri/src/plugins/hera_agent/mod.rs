mod commands;
mod context;
mod mcp_http;
mod smart_intent;
#[cfg(test)]
mod tests;
mod tools_curation;
mod tools_deck_swarm;
mod tools_host;
mod tools_library;
mod tools_playlist_vdj;
mod turn_agy;
mod turn_runner;

use std::collections::HashMap;
use std::net::TcpListener as StdTcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::State as AxumState;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use hera_node_rs::dj_brain::{check_binary_in_path, list_supported_ai_backends};
use hera_node_rs::error::constant_time_eq;
use hera_node_rs::export::{ExportCrateRequest, ExportTrackItem, export_crate_bundle};
use hera_node_rs::mcp::dispatch_tool_call as dispatch_core_mcp_tool;
use hera_node_rs::structure::HotCuePoint;
use serde_json::{Value, json};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{Mutex, oneshot};

use crate::libs::database::{AgentStoredMessage, DB, SUPPORTED_TRACKS_EXTENSIONS};
use crate::libs::track::{Track, get_track_from_file};
use crate::libs::utils::scan_dirs;
use crate::plugins::db::DBState;
use crate::plugins::hera_dj::{
    DownloadP2PTrackRequest, PushVirtualDjLiveCrateRequest, SetMiniOverlayModeRequest,
    download_p2p_track, push_virtualdj_live_crate, read_virtualdj_now_playing,
    set_mini_overlay_mode, shared_desktop_p2p_state,
};

#[allow(unused_imports)]
pub use commands::*;
#[allow(unused_imports)]
pub use context::*;
#[allow(unused_imports)]
pub use hera_ipc::agent_catalog_schemas::*;
#[allow(unused_imports)]
pub use hera_ipc::agent_catalog_summary::*;
pub use hera_ipc::agent_types::*;
#[allow(unused_imports)]
pub use hera_ipc::agy_stream::*;
#[allow(unused_imports)]
pub use mcp_http::*;
#[allow(unused_imports)]
pub use smart_intent::*;
#[allow(unused_imports)]
pub use tools_curation::*;
#[allow(unused_imports)]
pub use tools_deck_swarm::*;
#[allow(unused_imports)]
pub use tools_host::*;
#[allow(unused_imports)]
pub use tools_library::*;
#[allow(unused_imports)]
pub use tools_playlist_vdj::*;
#[allow(unused_imports)]
pub use turn_agy::*;
#[allow(unused_imports)]
pub use turn_runner::*;

pub(super) const MCP_LOOPBACK_HOST: &str = "127.0.0.1";

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    let std_listener = StdTcpListener::bind(format!("{MCP_LOOPBACK_HOST}:0"))
        .expect("Failed to bind loopback MCP listener");
    std_listener
        .set_nonblocking(true)
        .expect("Failed to configure non-blocking MCP listener");
    let mcp_port = std_listener.local_addr().map(|a| a.port()).unwrap_or(0);
    let mcp_bearer_token = format!("hera-mcp-{}", uuid::Uuid::new_v4());

    Builder::<R>::new("hera-agent")
        .invoke_handler(tauri::generate_handler![
            commands::get_agent_runtime_status,
            commands::list_agent_sessions,
            commands::create_agent_session,
            commands::update_agent_session,
            commands::delete_agent_session,
            commands::get_agent_session_messages,
            commands::start_agent_turn,
            commands::cancel_agent_turn,
            commands::resolve_agent_approval,
        ])
        .setup(move |app_handle, _api| {
            let ctx = AgentRuntimeContext {
                host: AgentHostBridge::Tauri(app_handle.clone()),
                mcp_port,
                mcp_bearer_token,
                active_turns: Arc::new(Mutex::new(HashMap::new())),
                active_mcp_session: Arc::new(Mutex::new(None)),
                pending_approvals: Arc::new(Mutex::new(HashMap::new())),
            };
            app_handle.manage(ctx.clone());

            tauri::async_runtime::spawn(async move {
                if let Ok(listener) = tokio::net::TcpListener::from_std(std_listener) {
                    let router = build_local_mcp_router(ctx);
                    let _ = axum::serve(listener, router).await;
                }
            });

            Ok(())
        })
        .build()
}
