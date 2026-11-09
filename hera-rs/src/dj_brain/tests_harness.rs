use super::*;

fn read_http_json_body(stream: &mut std::net::TcpStream) -> serde_json::Value {
    use std::io::Read;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut expected_total = None;
    while let Ok(n) = stream.read(&mut chunk) {
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if expected_total.is_none() {
            let s = String::from_utf8_lossy(&buf);
            if let Some(hdr_end) = s.find("\r\n\r\n") {
                let cl = s[..hdr_end]
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                expected_total = Some(hdr_end + 4 + cl);
            }
        }
        if let Some(tot) = expected_total {
            if buf.len() >= tot {
                break;
            }
        }
    }
    let s = String::from_utf8_lossy(&buf);
    let body = s.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");
    serde_json::from_str(body).unwrap_or_default()
}

fn write_http_json_response(stream: &mut std::net::TcpStream, body_val: &serde_json::Value) {
    use std::io::Write;
    let body = body_val.to_string();
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes());
}

#[test]
fn test_react_multi_turn_tool_calling_with_mock_lmstudio() {
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = std::thread::spawn(move || {
        // Turn 1: Verify 20 tools in request, return OpenAI tool_calls for `search_music`
        let (mut s1, _) = listener.accept().unwrap();
        let req1 = read_http_json_body(&mut s1);
        assert_eq!(req1["tools"].as_array().unwrap().len(), 20);
        write_http_json_response(
            &mut s1,
            &serde_json::json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "id": "call_search_1",
                            "type": "function",
                            "function": {
                                "name": "search_music",
                                "arguments": "{\"camelot\":\"8A\"}"
                            }
                        }]
                    }
                }]
            }),
        );

        // Turn 2: Verify tool observation in messages, return fallback `<tool_call>` tag for `get_node_manifest`
        let (mut s2, _) = listener.accept().unwrap();
        let req2 = read_http_json_body(&mut s2);
        let msgs2 = req2["messages"].as_array().unwrap();
        let last_msg2 = msgs2.last().unwrap();
        assert_eq!(last_msg2["role"], "tool");
        assert_eq!(last_msg2["name"], "search_music");
        write_http_json_response(
            &mut s2,
            &serde_json::json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "Checking node manifest next:\n<tool_call>{\"name\":\"get_node_manifest\",\"arguments\":{}}</tool_call>"
                    }
                }]
            }),
        );

        // Turn 3: Verify second tool observation and return final synthesis
        let (mut s3, _) = listener.accept().unwrap();
        let req3 = read_http_json_body(&mut s3);
        let msgs3 = req3["messages"].as_array().unwrap();
        let last_msg3 = msgs3.last().unwrap();
        assert_eq!(last_msg3["role"], "tool");
        assert_eq!(last_msg3["name"], "get_node_manifest");
        write_http_json_response(
            &mut s3,
            &serde_json::json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "Found verified 8A Studio Masters on node hera-react-test and confirmed manifest."
                    }
                }]
            }),
        );
    });

    let state = crate::p2p::P2PMeshState::new("hera-react-test");
    let req = CloudflareFreeAiTurnRequest {
        dj_handle: "DaviDJ".to_string(),
        message: "Find 8A tracks and check the node manifest".to_string(),
        session_id: None,
        style_dna: None,
        daily_neurons_used_so_far: 0.0,
        cache_hit: false,
        backend: Some("lmstudio".to_string()),
        base_url: Some(format!("http://{addr}/v1")),
        model: Some("qwen2.5-coder-14b-instruct".to_string()),
        api_key: None,
        max_tool_iterations: Some(6),
        commander_mode: None,
    };

    let resp = evaluate_agent_harness_turn(&req, &state);
    let _ = handle.join();

    assert_eq!(resp.backend_used, "lmstudio");
    assert_eq!(resp.model_used, "qwen2.5-coder-14b-instruct");
    assert_eq!(resp.iterations_executed, 3);
    assert_eq!(resp.tool_calls_trace.len(), 2);
    assert_eq!(resp.tool_calls_trace[0].tool_name, "search_music");
    assert!(!resp.tool_calls_trace[0].is_error);
    assert_eq!(resp.tool_calls_trace[1].tool_name, "get_node_manifest");
    assert!(!resp.tool_calls_trace[1].is_error);
    assert!(resp
        .assistant_reply
        .contains("Found verified 8A Studio Masters"));
}

#[test]
fn test_react_self_healing_on_tool_error_with_mock_lmstudio() {
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = std::thread::spawn(move || {
        // Turn 1: Call `export_dj_crate` with missing required fields `{}`
        let (mut s1, _) = listener.accept().unwrap();
        let _req1 = read_http_json_body(&mut s1);
        write_http_json_response(
            &mut s1,
            &serde_json::json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "tool_calls": [{
                            "id": "call_bad_export",
                            "type": "function",
                            "function": {
                                "name": "export_dj_crate",
                                "arguments": "{}"
                            }
                        }]
                    }
                }]
            }),
        );

        // Turn 2: Inspect `self_heal_hint` in tool error message and retry with valid arguments
        let (mut s2, _) = listener.accept().unwrap();
        let req2 = read_http_json_body(&mut s2);
        let msgs2 = req2["messages"].as_array().unwrap();
        let tool_err_msg = msgs2.last().unwrap()["content"].as_str().unwrap();
        assert!(tool_err_msg.contains("self_heal_hint"));
        write_http_json_response(
            &mut s2,
            &serde_json::json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "tool_calls": [{
                            "id": "call_fixed_export",
                            "type": "function",
                            "function": {
                                "name": "export_dj_crate",
                                "arguments": "{\"crate_name\":\"Fixed_Set\",\"tracks\":[]}"
                            }
                        }]
                    }
                }]
            }),
        );

        // Turn 3: Return final answer
        let (mut s3, _) = listener.accept().unwrap();
        let _req3 = read_http_json_body(&mut s3);
        write_http_json_response(
            &mut s3,
            &serde_json::json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "Self-healed and exported Fixed_Set successfully."
                    }
                }]
            }),
        );
    });

    let state = crate::p2p::P2PMeshState::new("hera-self-heal-test");
    let req = CloudflareFreeAiTurnRequest {
        dj_handle: "DaviDJ".to_string(),
        message: "Export my crate".to_string(),
        session_id: None,
        style_dna: None,
        daily_neurons_used_so_far: 0.0,
        cache_hit: false,
        backend: Some("lmstudio".to_string()),
        base_url: Some(format!("http://{addr}/v1")),
        model: Some("qwen2.5-coder-14b-instruct".to_string()),
        api_key: None,
        max_tool_iterations: Some(6),
        commander_mode: None,
    };

    let resp = evaluate_agent_harness_turn(&req, &state);
    let _ = handle.join();

    assert_eq!(resp.iterations_executed, 3);
    assert_eq!(resp.tool_calls_trace.len(), 2);
    assert!(resp.tool_calls_trace[0].is_error);
    assert!(!resp.tool_calls_trace[1].is_error);
    assert!(resp.assistant_reply.contains("Fixed_Set"));
}
