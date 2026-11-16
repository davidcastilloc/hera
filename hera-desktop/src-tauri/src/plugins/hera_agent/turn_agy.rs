use super::*;

pub(super) async fn run_agy_streaming_turn<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session: &AgentSessionSummary,
    assistant_msg_id: &str,
    user_message: &str,
    attachments: &[AgentAttachment],
) -> Result<String, String> {
    let (installed, path_hint) = check_binary_in_path("agy");
    if !installed {
        return Err("agy no instalado".to_string());
    }
    let exec_bin = path_hint.unwrap_or_else(|| "agy".to_string());

    let mcp_settings = json!({
        "mcpServers": {
            "hera_desktop": {
                "type": "http",
                "url": format!("http://{}:{}/mcp", MCP_LOOPBACK_HOST, ctx.mcp_port),
                "headers": {
                    "Authorization": format!("Bearer {}", ctx.mcp_bearer_token)
                }
            }
        }
    })
    .to_string();

    let full_prompt = if session.agy_conversation_id.is_some() {
        format_user_prompt_with_context(user_message, attachments)
    } else {
        let dj_handle = match &ctx.host {
            AgentHostBridge::Tauri(app) => app
                .try_state::<crate::plugins::config::ConfigManager>()
                .and_then(|c| c.get().ok())
                .map(|cfg| cfg.dj_handle)
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "DaviDJ".to_string()),
            AgentHostBridge::Standalone(_) => "DaviDJ".to_string(),
        };
        let (golden_hints, excluded_hints) = ctx
            .with_read_conn(|conn| {
                Box::pin(async move {
                    let g: Vec<String> = sqlx::query_scalar(
                        "SELECT COALESCE(NULLIF(note, ''), from_title || ' -> ' || to_title) FROM dj_golden_pairs ORDER BY created_at DESC LIMIT 10",
                    )
                    .fetch_all(&mut *conn)
                    .await
                    .unwrap_or_default();
                    let ex: Vec<String> = sqlx::query_scalar(
                        "SELECT COALESCE(NULLIF(title_pattern, ''), track_id, '') FROM dj_excluded_tracks ORDER BY created_at DESC LIMIT 10",
                    )
                    .fetch_all(&mut *conn)
                    .await
                    .unwrap_or_default();
                    Ok((g, ex))
                })
            })
            .await
            .unwrap_or_default();
        let mode = if session.execution_policy == "auto" {
            Some("autonomous_operator")
        } else {
            Some("executive_boss")
        };
        let system_hint = hera_node_rs::dj_brain::prompts::build_desktop_dj_curator_system_prompt(
            &dj_handle,
            "VirtualDJ",
            None,
            &golden_hints,
            &excluded_hints,
            mode,
        );
        format!(
            "{system_hint}\n\n{}",
            format_user_prompt_with_context(user_message, attachments)
        )
    };

    let mut cmd = tokio::process::Command::new(&exec_bin);
    cmd.args([
        "--disable-slash-commands",
        "--effort",
        "low",
        "--settings",
        &mcp_settings,
    ]);
    if session.execution_policy == "auto" {
        cmd.arg("--dangerously-skip-permissions");
    }
    if let Some(ref conv_id) = session.agy_conversation_id
        && !conv_id.trim().is_empty()
    {
        cmd.args(["--conversation", conv_id]);
    }
    cmd.args([
        "--print-timeout",
        "90s",
        "--output-format",
        "stream-json",
        "-p",
        &full_prompt,
    ]);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    {
        let mut mcp_sid = ctx.active_mcp_session.lock().await;
        *mcp_sid = Some(session.id.clone());
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Error iniciando agy: {e}"))?;
    let Some(stdout) = child.stdout.take() else {
        return Err("No se pudo capturar stdout de agy".to_string());
    };

    let mut reader = BufReader::new(stdout).lines();
    let mut accumulated = String::new();
    let mut final_response_fallback = String::new();

    loop {
        let is_cancelled = {
            let turns = ctx.active_turns.lock().await;
            turns
                .get(&session.id)
                .map(|t| t.cancel_flag.load(Ordering::SeqCst))
                .unwrap_or(false)
        };
        if is_cancelled {
            let _ = child.kill().await;
            return Err("Turno detenido por el usuario.".to_string());
        }

        match tokio::time::timeout(Duration::from_millis(450), reader.next_line()).await {
            Err(_) => continue,
            Ok(Err(e)) => {
                let _ = child.kill().await;
                return Err(format!("Error leyendo stream de agy: {e}"));
            }
            Ok(Ok(None)) => break,
            Ok(Ok(Some(line))) => match parse_agy_stream_json_line(&line) {
                ParsedAgyStreamEvent::Init { conversation_id } => {
                    let sid = session.id.clone();
                    let cid = conversation_id.clone();
                    let _ = ctx
                        .with_db(|db| {
                            Box::pin(async move {
                                db.update_agent_session_meta(&sid, None, Some(&cid), None, None)
                                    .await
                                    .map_err(|e| e.to_string())
                            })
                        })
                        .await;
                }
                ParsedAgyStreamEvent::TextDelta { delta } => {
                    accumulated.push_str(&delta);
                    ctx.emit_stream(AgentStreamEvent {
                        session_id: session.id.clone(),
                        message_id: assistant_msg_id.to_string(),
                        kind: "text_delta".to_string(),
                        text_delta: Some(delta),
                        full_content: Some(accumulated.clone()),
                        step: None,
                        session: None,
                        actions: None,
                    });
                }
                ParsedAgyStreamEvent::NativeToolStep {
                    step_index,
                    tool_name,
                    state,
                } => {
                    let step_id = format!("agy-native-{step_index}");
                    let status = if state == "DONE" { "done" } else { "running" }.to_string();
                    let view = AgentToolStepView {
                        id: step_id,
                        tool_name: tool_name.clone(),
                        title: tool_name.clone(),
                        summary: format!("Herramienta nativa · {tool_name}"),
                        status,
                        arguments_preview: String::new(),
                        receipt_text: format!("herramienta Antigravity: {tool_name} ({state})"),
                    };
                    {
                        let mut turns = ctx.active_turns.lock().await;
                        if let Some(tracker) = turns.get_mut(&session.id) {
                            if let Some(ex) = tracker.steps.iter_mut().find(|s| s.id == view.id) {
                                *ex = view.clone();
                            } else {
                                tracker.steps.push(view.clone());
                            }
                        }
                    }
                    ctx.emit_stream(AgentStreamEvent {
                        session_id: session.id.clone(),
                        message_id: assistant_msg_id.to_string(),
                        kind: "step_upsert".to_string(),
                        text_delta: None,
                        full_content: None,
                        step: Some(view),
                        session: None,
                        actions: None,
                    });
                }
                ParsedAgyStreamEvent::Result {
                    conversation_id,
                    response,
                    status,
                } => {
                    if let Some(cid) = conversation_id {
                        let sid = session.id.clone();
                        let _ = ctx
                            .with_db(|db| {
                                Box::pin(async move {
                                    db.update_agent_session_meta(&sid, None, Some(&cid), None, None)
                                        .await
                                        .map_err(|e| e.to_string())
                                })
                            })
                            .await;
                    }
                    if status != "SUCCESS"
                        && accumulated.trim().is_empty()
                        && response.trim().is_empty()
                    {
                        return Err(format!("agy finalizó con estado {status}"));
                    }
                    final_response_fallback = response;
                }
                ParsedAgyStreamEvent::Ignored => {}
            },
        }
    }

    let _ = child.wait().await;
    {
        let mut mcp_sid = ctx.active_mcp_session.lock().await;
        if mcp_sid.as_deref() == Some(session.id.as_str()) {
            *mcp_sid = None;
        }
    }

    let final_text = if !accumulated.trim().is_empty() {
        accumulated.trim().to_string()
    } else {
        final_response_fallback.trim().to_string()
    };

    if final_text.is_empty() {
        Err("agy no devolvió respuesta".to_string())
    } else {
        Ok(final_text)
    }
}
