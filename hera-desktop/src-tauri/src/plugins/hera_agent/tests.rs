use super::*;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn make_test_db() -> DB {
    let opts = SqliteConnectOptions::new()
        .filename(":memory:")
        .create_if_missing(true);
    let mut connection = SqliteConnection::connect_with(&opts).await.unwrap();
    sqlx::migrate!("src/migrations")
        .run_direct(&mut connection)
        .await
        .unwrap();
    DB { connection }
}

async fn post_loopback_http(port: u16, auth: Option<&str>, body: &str) -> (u16, String) {
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect to loopback MCP");
    let auth_header = auth
        .map(|token| format!("Authorization: {token}\r\n"))
        .unwrap_or_default();
    let req = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\n{auth_header}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut resp_bytes = Vec::new();
    stream.read_to_end(&mut resp_bytes).await.unwrap();
    let resp_str = String::from_utf8_lossy(&resp_bytes).to_string();
    let (head, payload) = resp_str.split_once("\r\n\r\n").unwrap_or((&resp_str, ""));
    let status_code = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(500);
    (status_code, payload.to_string())
}

#[test]
fn test_parse_agy_stream_json_events() {
    let init_line = r#"{"event":"init","conversation_id":"conv-abc-99","init":{"permission_mode":"always-proceed"}}"#;
    assert_eq!(
        parse_agy_stream_json_line(init_line),
        ParsedAgyStreamEvent::Init {
            conversation_id: "conv-abc-99".to_string()
        }
    );

    let delta_line = r#"{"event":"step_update","step_update":{"conversation_id":"conv-abc-99","step_index":1,"state":"ACTIVE","step_type":"agent_response","text_delta":"Playlist lista"}}"#;
    assert_eq!(
        parse_agy_stream_json_line(delta_line),
        ParsedAgyStreamEvent::TextDelta {
            delta: "Playlist lista".to_string()
        }
    );

    let res_line = r#"{"event":"result","result":{"conversation_id":"conv-abc-99","status":"SUCCESS","response":"Playlist lista\n"}}"#;
    assert_eq!(
        parse_agy_stream_json_line(res_line),
        ParsedAgyStreamEvent::Result {
            conversation_id: Some("conv-abc-99".to_string()),
            response: "Playlist lista\n".to_string(),
            status: "SUCCESS".to_string()
        }
    );
}

#[tokio::test]
async fn test_local_mcp_auth_and_sqlite_playlist_and_os_tools() {
    let mut db = make_test_db().await;
    db.insert_tracks(vec![
        Track {
            id: "t1".to_string(),
            path: "library/opus.wav".to_string(),
            title: "Opus".to_string(),
            album: "Cabina".to_string(),
            album_artist: "Eric Prydz".to_string(),
            artists: vec!["Eric Prydz".to_string()],
            genres: vec!["Progressive".to_string()],
            year: Some(2024),
            duration: 240,
            track_no: None,
            track_of: None,
            disk_no: None,
            disk_of: None,
            is_compilation: false,
            bpm: Some(126.0),
            camelot: Some("8A".to_string()),
            energy: Some(0.85),
            hera_seal: None,
            hot_cues: Some(vec![]),
        },
        Track {
            id: "t2".to_string(),
            path: "library/cesonia.wav".to_string(),
            title: "Cesonia".to_string(),
            album: "Cabina".to_string(),
            album_artist: "Keinemusik".to_string(),
            artists: vec!["Keinemusik".to_string()],
            genres: vec!["Afro House".to_string()],
            year: Some(2024),
            duration: 210,
            track_no: None,
            track_of: None,
            disk_no: None,
            disk_of: None,
            is_compilation: false,
            bpm: Some(125.0),
            camelot: Some("9A".to_string()),
            energy: Some(0.82),
            hera_seal: None,
            hot_cues: Some(vec![]),
        },
    ])
    .await
    .unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let ctx: AgentRuntimeContext<tauri::Wry> = AgentRuntimeContext {
        host: AgentHostBridge::Standalone(Arc::new(Mutex::new(db))),
        mcp_port: port,
        mcp_bearer_token: "secret-test-token".to_string(),
        active_turns: Arc::new(Mutex::new(HashMap::new())),
        active_mcp_session: Arc::new(Mutex::new(None)),
        pending_approvals: Arc::new(Mutex::new(HashMap::new())),
    };

    let router = build_local_mcp_router(ctx.clone());
    let srv_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    // 1. Unauthorized request must return 401
    let (unauth_status, _) = post_loopback_http(
        port,
        None,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
    )
    .await;
    assert_eq!(unauth_status, 401);

    // 2. Authorized tools/list returns unified 42+ tools
    let (list_status, list_body) = post_loopback_http(
        port,
        Some("Bearer secret-test-token"),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
    )
    .await;
    assert_eq!(list_status, 200);
    let list_json: Value = serde_json::from_str(&list_body).unwrap();
    let tools_arr = list_json["result"]["tools"].as_array().unwrap();
    assert!(
        tools_arr.len() >= 42,
        "expected >= 42 unified tools, got {}",
        tools_arr.len()
    );

    // 3. Execute smart intent plan: create playlist + push VirtualDJ live crate + carpeta temporal
    assert_eq!(human_tool_title("create_playlist"), "Crear playlist");
    assert_eq!(
        human_tool_title("run_powershell_command"),
        "Ejecutar en PowerShell"
    );

    let reply = execute_smart_intent_plan(
        &ctx,
        "session-test",
        "Arma una playlist «Warmup Test», mándala a VirtualDJ y revisa mi carpeta temporal",
        &[],
    )
    .await
    .unwrap();
    assert!(reply.contains("Warmup Test"));
    assert!(reply.contains("VirtualDJ"));
    assert!(reply.contains("carpeta temporal"));

    let diag_reply = execute_smart_intent_plan(
        &ctx,
        "session-test",
        "Ejecuta un diagnóstico rápido del sistema en PowerShell",
        &[],
    )
    .await
    .unwrap();
    assert!(diag_reply.contains("Diagnóstico de unidades"));

    let playlists_res = dispatch_unified_agent_tool(&ctx, None, "list_playlists", json!({}))
        .await
        .unwrap();
    assert_eq!(playlists_res["total"], 1);
    assert_eq!(playlists_res["playlists"][0]["name"], "Warmup Test");
    assert_eq!(playlists_res["playlists"][0]["track_count"], 2);

    // 4. Execute OS directory tool on temp_dir
    let tmp_str = std::env::temp_dir().to_string_lossy().to_string();
    let dir_res = dispatch_unified_agent_tool(
        &ctx,
        None,
        "list_directory_entries",
        json!({ "path": tmp_str, "max_entries": 5 }),
    )
    .await
    .unwrap();
    assert!(dir_res.get("entries").and_then(|v| v.as_array()).is_some());

    // 5. Execute curatorial tools: golden pairs, excluded tracks, and DJ feedback
    let gp_res = dispatch_unified_agent_tool(
        &ctx,
        None,
        "record_golden_pair",
        json!({
            "from_track_id": "trk-01",
            "to_track_id": "trk-02",
            "custom_bonus": 0.42,
            "note": "Transición épica en 125 BPM"
        }),
    )
    .await
    .unwrap();
    assert_eq!(gp_res["success"], true);
    assert_eq!(gp_res["bonus"], 0.42);

    let ex_res = dispatch_unified_agent_tool(
        &ctx,
        None,
        "exclude_track_curation",
        json!({
            "title_pattern": "Clavelito",
            "reason": "Pista desgastada"
        }),
    )
    .await
    .unwrap();
    assert_eq!(ex_res["success"], true);
    assert_eq!(ex_res["title_pattern"], "Clavelito");

    let fb_res = dispatch_unified_agent_tool(
        &ctx,
        Some("session-test"),
        "record_dj_feedback",
        json!({
            "category": "energy",
            "rule_text": "Subir la energía después de la hora 2"
        }),
    )
    .await
    .unwrap();
    assert_eq!(fb_res["success"], true);
    assert_eq!(fb_res["category"], "energy");

    srv_handle.abort();
}
