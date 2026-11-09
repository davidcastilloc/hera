use super::cli_runner::run_command_with_timeout;
use super::*;

#[test]
fn test_12_backend_registry_and_openai_tools_catalog() {
    let backends = list_supported_ai_backends();
    assert_eq!(backends.len(), 16);
    let lmstudio = backends.iter().find(|b| b.id == "lmstudio").unwrap();
    assert_eq!(lmstudio.base_url, "http://localhost:1234/v1");
    let ollama = backends.iter().find(|b| b.id == "ollama").unwrap();
    assert_eq!(ollama.base_url, "http://localhost:11434/v1");

    let agy = backends.iter().find(|b| b.id == "cli_agy").unwrap();
    assert_eq!(agy.category.as_deref(), Some("cli_agent"));

    let tools = build_openai_tools_catalog();
    assert_eq!(tools.len(), 20);
    for t in &tools {
        assert_eq!(t["type"], "function");
        let fn_name = t["function"]["name"].as_str().unwrap();
        assert_ne!(fn_name, "chat_with_dj_copilot");
        assert!(t["function"]["parameters"].is_object());
    }

    let state = crate::p2p::P2PMeshState::new("hera-test-node");
    let prompt = build_agent_harness_system_prompt("DaviDJ", None, &state, None);
    assert!(prompt.contains("Quarantine-First"));
    assert!(prompt.contains("hera_code_mode"));
    assert!(prompt.contains("hera-test-node"));
    assert!(prompt.contains("PROTOCOLO COMANDANTE DE CABINA"));
}

#[test]
fn test_probe_backend_models_mock_lmstudio_ephemeral_tcp() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let body = r#"{"data":[{"id":"qwen2.5-coder-14b-instruct-gguf"},{"id":"text-embedding-nomic"}]}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });

    let base_url = format!("http://{addr}/v1");
    let models = probe_backend_models(&base_url, None).unwrap();
    assert_eq!(models, vec!["qwen2.5-coder-14b-instruct-gguf".to_string()]);
    let _ = handle.join();
}

#[test]
fn test_ai_connectivity_onboarding_scan_and_5_modes() {
    let report = scan_ai_connectivity_onboarding(Some("autonomous_operator"));
    assert!(report.cloud_free_available);
    assert_eq!(report.mcp_integration.tools_count, 21);
    assert!(report
        .mcp_integration
        .sample_config_json
        .contains("hera-node"));
    assert_eq!(
        report.swarm_protection.commander_mode,
        "autonomous_operator"
    );
    assert!(report.swarm_protection.quarantine_first_enforced);
    assert!(report.swarm_protection.p2p_seeding_active);

    assert!(!report.cli_agents.is_empty());
    let agy = report
        .cli_agents
        .iter()
        .find(|c| c.id == "cli_agy")
        .unwrap();
    assert_eq!(agy.binary_name, "agy");
    assert!(agy.zero_api_key_needed);

    assert!(!report.local_servers.is_empty());
    let lm = report
        .local_servers
        .iter()
        .find(|s| s.id == "lmstudio")
        .unwrap();
    assert_eq!(lm.base_url, "http://localhost:1234/v1");

    let state = crate::p2p::P2PMeshState::new("hera-cli-test-node");
    let req = CloudflareFreeAiTurnRequest {
        dj_handle: "DaviDJ".to_string(),
        message: "Diagnostica mi sesión DJ".to_string(),
        session_id: None,
        style_dna: None,
        daily_neurons_used_so_far: 0.0,
        cache_hit: false,
        backend: Some("cli_agy".to_string()),
        base_url: None,
        model: None,
        api_key: None,
        max_tool_iterations: Some(3),
        commander_mode: Some("executive_boss".to_string()),
    };

    let resp = evaluate_agent_harness_turn(&req, &state);
    assert_eq!(resp.backend_used, "cli_agy");
    assert!(resp.assistant_reply.len() > 10);
}

#[test]
fn test_parallel_onboarding_scan_latency_and_command_timeout_guard() {
    let start = std::time::Instant::now();
    let report = scan_ai_connectivity_onboarding(Some("executive_boss"));
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_millis(2500),
        "scan_ai_connectivity_onboarding took {:?} (expected < 2.5s)",
        elapsed
    );
    assert_eq!(report.local_servers.len(), 5);

    let slow_cmd = if cfg!(windows) {
        let mut c = std::process::Command::new("powershell");
        c.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 5",
        ]);
        c
    } else {
        let mut c = std::process::Command::new("sleep");
        c.arg("5");
        c
    };
    let timeout_start = std::time::Instant::now();
    let res = run_command_with_timeout(slow_cmd, std::time::Duration::from_millis(180));
    assert!(res.is_err());
    assert!(
        timeout_start.elapsed() < std::time::Duration::from_millis(2000),
        "run_command_with_timeout did not terminate slow child quickly enough"
    );
}

#[test]
fn test_build_cli_agent_args_places_print_flag_immediately_before_prompt() {
    let prompt = "Sistema\n\nUsuario: Arma una playlist 8A";
    let args = build_cli_agent_args("agy", prompt, "Arma una playlist 8A");
    assert!(args.len() >= 2);
    assert_eq!(args[args.len() - 2], "-p");
    assert_eq!(args[args.len() - 1], prompt);
    assert!(args.contains(&"--disable-slash-commands".to_string()));
}
