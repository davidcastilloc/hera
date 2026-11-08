use hera_node_rs::mcp::run_mcp_stdio_loop;
use hera_node_rs::p2p::P2PMeshState;
use hera_node_rs::router::build_hera_router;
use std::env;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let is_mcp_stdio = args
        .iter()
        .any(|a| a == "--mcp" || a == "mcp" || a == "serve");

    let node_id = env::var("HERA_NODE_ID").unwrap_or_else(|_| "hera-vps-node-01".to_string());
    let state = P2PMeshState::new(&node_id);

    if is_mcp_stdio {
        // Pure JSON-RPC 2.0 over stdin/stdout for AI Agents (Claude, Antigravity, Cursor, Codex)
        run_mcp_stdio_loop(state);
        return;
    }

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let app = build_hera_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!(
        "⚡ Hera Rust Universal Node (`{}`) listening on http://{}",
        node_id, addr
    );
    println!("   ├─ GET  /health                    (Node telemetry & runtime target)");
    println!("   ├─ POST /mcp                       (Remote MCP JSON-RPC 2.0 Server)");
    println!("   ├─ GET  /mcp/tools                 (MCP Tool Schema Catalog)");
    println!("   ├─ GET  /api/v1/node/manifest      (Capabilities & Cloud referral links)");
    println!("   ├─ POST /api/v1/audio/analyze-file (Pure-Rust Symphonia + rustfft Forensics & 4 Hot Cues)");
    println!("   ├─ POST /api/v1/crates/build       (4-Layer Synergy + Global 2-Opt TSP Crate Optimizer)");
    println!("   ├─ GET  /api/v1/p2p/search         (Zero-Trust Verified P2P Mesh Search)");
    println!("   └─ POST /api/v1/p2p/announce       (Quarantine-First P2P Track Gatekeeper)");
    println!("   💡 Tip: Run `hera-node --mcp` for stdio MCP mode.");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app)
        .await
        .expect("Hera Rust HTTP server failed");
}
