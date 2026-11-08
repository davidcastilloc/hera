use hera_node_rs::p2p::P2PMeshState;
use hera_node_rs::router::build_hera_router;
use std::env;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let node_id = env::var("HERA_NODE_ID").unwrap_or_else(|_| "hera-vps-node-01".to_string());
    let state = P2PMeshState::new(&node_id);
    let app = build_hera_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!(
        "⚡ Hera Rust Universal Node (`{}`) listening on http://{}",
        node_id, addr
    );
    println!("   ├─ GET  /health                    (Node telemetry & runtime target)");
    println!("   ├─ GET  /api/v1/node/manifest      (Capabilities & Cloud referral links)");
    println!("   ├─ POST /api/v1/audio/analyze-file (Pure-Rust Symphonia + rustfft Forensics & 4 Hot Cues)");
    println!("   ├─ POST /api/v1/crates/build       (4-Layer Synergy + Global 2-Opt TSP Crate Optimizer)");
    println!("   ├─ GET  /api/v1/p2p/search         (Zero-Trust Verified P2P Mesh Search)");
    println!("   └─ POST /api/v1/p2p/announce       (Quarantine-First P2P Track Gatekeeper)");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app)
        .await
        .expect("Hera Rust HTTP server failed");
}
