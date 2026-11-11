use hera_node_rs::mcp::run_mcp_stdio_loop;
use hera_node_rs::p2p::{P2PMeshState, ScanLocalDirRequest};
use hera_node_rs::router::build_hera_router;
use std::env;
use std::net::SocketAddr;

fn extract_cli_flag(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let is_mcp_stdio = args
        .iter()
        .any(|a| a == "--mcp" || a == "mcp" || a == "serve");

    let node_id = extract_cli_flag(&args, "--node-id")
        .or_else(|| env::var("HERA_NODE_ID").ok())
        .unwrap_or_else(|| "hera-vps-node-01".to_string());
    let dj_handle = extract_cli_flag(&args, "--dj")
        .or_else(|| env::var("HERA_DJ_HANDLE").ok())
        .unwrap_or_else(|| "DaviDJ".to_string());
    let music_dir =
        extract_cli_flag(&args, "--music-dir").or_else(|| env::var("HERA_MUSIC_DIR").ok());
    let public_url =
        extract_cli_flag(&args, "--public-url").or_else(|| env::var("HERA_PUBLIC_URL").ok());
    let federate_url =
        extract_cli_flag(&args, "--federate-url").or_else(|| env::var("HERA_FEDERATE_URL").ok());

    let state = P2PMeshState::new(&node_id);

    // If the user launched `hera-node --dj DJFoo --music-dir /path/to/music` on their laptop or server,
    // immediately scan their local disk, audit every track with Zero-Trust DSP, and enable P2P seeding!
    if let Some(ref dir) = music_dir {
        match state.scan_local_directory(ScanLocalDirRequest {
            dj_handle: dj_handle.clone(),
            directory_path: dir.clone(),
            public_node_url: public_url.clone(),
            bandwidth_mbps: Some(800),
        }) {
            Ok(scan_res) => {
                eprintln!(
                    "💿 Local P2P Seeder Active (`{}`): {} verified masters admitted, {} quarantined from `{}`",
                    dj_handle,
                    scan_res.admitted_tracks.len(),
                    scan_res.rejected_quarantine_tracks.len(),
                    dir
                );
            }
            Err(e) => {
                eprintln!("⚠️ Could not scan `--music-dir {}`: {}", dir, e);
            }
        }
    }

    // If `--federate-url` (`HERA_FEDERATE_URL`) is provided, automatically push this node's
    // sealed catalog (`hera_seal`) to `hera-cloud` or another peer node on startup!
    if let Some(ref remote_hub) = federate_url {
        match state.push_federation_over_http(remote_hub, &dj_handle, public_url.clone(), 800) {
            Ok(fed_res) => {
                eprintln!(
                    "🌐 Auto-Federated with `{}` (`{}`): {} tracks accepted, {} total swarm tracks synced.",
                    remote_hub,
                    fed_res.responder_node_id,
                    fed_res.tracks_accepted,
                    fed_res.federated_swarm_catalog.len()
                );
            }
            Err(e) => {
                eprintln!("⚠️ Auto-Federation to `{}` failed: {}", remote_hub, e);
            }
        }
    }

    if is_mcp_stdio {
        // Pure JSON-RPC 2.0 over stdin/stdout for AI Agents (Claude, Antigravity, Cursor, Codex)
        run_mcp_stdio_loop(state);
        return;
    }

    let bind_host = extract_cli_flag(&args, "--bind")
        .or_else(|| env::var("HERA_BIND_ADDR").ok())
        .unwrap_or_else(|| "127.0.0.1".to_string());

    let port: u16 = extract_cli_flag(&args, "--port")
        .or_else(|| env::var("PORT").ok())
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let app = build_hera_router(state);

    let addr: SocketAddr = format!("{bind_host}:{port}")
        .parse()
        .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], port)));
    println!(
        "⚡ Hera Rust Universal Node (`{}`) listening on http://{}",
        node_id, addr
    );
    println!("   ├─ GET  /health                         (Node telemetry & runtime target)");
    println!("   ├─ POST /mcp                            (Remote MCP JSON-RPC 2.0 Server)");
    println!("   ├─ POST /mcp/code-mode                  (Cloudflare Code Mode TypeScript Runner)");
    println!("   ├─ POST /api/v1/node/scan-local-dir     (Local Laptop/VPS Direct Disk Storage & P2P Seeder)");
    println!("   ├─ POST /api/v1/dj/connect-drive        (Zero-OAuth Google Drive / Dropbox / OneDrive Onboarding)");
    println!("   ├─ POST /api/v1/p2p/federate-sync       (Federated Node Handshake & Sealed Catalog Sync)");
    println!("   ├─ POST /api/v1/audio/analyze-file      (Pure-Rust Symphonia + rustfft Forensics & 4 Hot Cues)");
    println!("   ├─ POST /api/v1/crates/build            (4-Layer Synergy + Global 2-Opt TSP Crate Optimizer)");
    println!("   ├─ GET  /api/v1/p2p/search              (Zero-Trust Verified P2P Mesh Search)");
    println!("   └─ GET  /api/v1/p2p/download/:track_id  (Direct Local P2P Binary Audio Stream)");
    println!("   💡 Tip: Run `hera-node --dj DJFoo --music-dir ./masters --federate-url http://hub:8080` to seed & auto-federate.");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("Hera Rust HTTP server failed");
}
