use super::*;

#[tauri::command]
pub async fn get_agent_runtime_status<R: Runtime>(
    app: AppHandle<R>,
) -> Result<AgentRuntimeStatus, String> {
    let ctx_port = app.state::<AgentRuntimeContext<R>>().mcp_port;
    tauri::async_runtime::spawn_blocking(move || {
        let (agy_installed, agy_path) = check_binary_in_path("agy");
        AgentRuntimeStatus {
            mcp_port: ctx_port,
            tools_count: build_unified_mcp_tools_list().len(),
            agy_installed,
            agy_path,
            supported_backends: list_supported_ai_backends(),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_agent_sessions<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<AgentSessionSummary>, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    state
        .with_db(|db| {
            Box::pin(async move { db.list_agent_sessions().await.map_err(|e| e.to_string()) })
        })
        .await
}

#[tauri::command]
pub async fn create_agent_session<R: Runtime>(
    app: AppHandle<R>,
    request: CreateAgentSessionRequest,
) -> Result<AgentSessionSummary, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    let title = request
        .title
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Nueva sesión".to_string());
    let backend_id = request
        .backend_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "cli_agy".to_string());
    let policy = request
        .execution_policy
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "auto".to_string());

    state
        .with_db(|db| {
            Box::pin(async move {
                db.create_agent_session(title, backend_id, policy)
                    .await
                    .map_err(|e| e.to_string())
            })
        })
        .await
}

#[tauri::command]
pub async fn update_agent_session<R: Runtime>(
    app: AppHandle<R>,
    request: UpdateAgentSessionRequest,
) -> Result<Option<AgentSessionSummary>, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    state
        .with_db(|db| {
            Box::pin(async move {
                db.update_agent_session_meta(
                    &request.session_id,
                    request.title.as_deref(),
                    None,
                    request.backend_id.as_deref(),
                    request.execution_policy.as_deref(),
                )
                .await
                .map_err(|e| e.to_string())
            })
        })
        .await
}

#[tauri::command]
pub async fn delete_agent_session<R: Runtime>(
    app: AppHandle<R>,
    session_id: String,
) -> Result<bool, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    state
        .with_db(|db| {
            Box::pin(async move {
                db.delete_agent_session(&session_id)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(true)
            })
        })
        .await
}

#[tauri::command]
pub async fn get_agent_session_messages<R: Runtime>(
    app: AppHandle<R>,
    session_id: String,
) -> Result<Vec<AgentMessageView>, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    let raw = state
        .with_db(|db| {
            Box::pin(async move {
                db.get_agent_messages(&session_id)
                    .await
                    .map_err(|e| e.to_string())
            })
        })
        .await?;
    Ok(raw.into_iter().map(stored_to_view).collect())
}

#[tauri::command]
pub async fn start_agent_turn<R: Runtime>(
    app: AppHandle<R>,
    request: StartAgentTurnRequest,
) -> Result<StartAgentTurnResponse, String> {
    let ctx = app.state::<AgentRuntimeContext<R>>().inner().clone();
    let backend_id = request
        .backend_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "cli_agy".to_string());
    let policy = request
        .execution_policy
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "auto".to_string());
    let user_text = request.message.trim().to_string();
    if user_text.is_empty() && request.attachments.is_empty() {
        return Err("Escribe un mensaje o adjunta un elemento".to_string());
    }

    let derived_title = derive_session_title(&user_text);
    let req_sid = request.session_id.clone();
    let b_cl = backend_id.clone();
    let p_cl = policy.clone();

    let session = ctx
        .with_db(|db| {
            Box::pin(async move {
                if let Some(sid) = req_sid
                    && let Some(existing) = db
                        .get_agent_session(&sid)
                        .await
                        .map_err(|e| e.to_string())?
                {
                    let existing_msgs = db
                        .get_agent_messages(&sid)
                        .await
                        .map_err(|e| e.to_string())?;
                    let new_title = if existing_msgs.is_empty() && existing.title == "Nueva sesión"
                    {
                        Some(derived_title.as_str())
                    } else {
                        None
                    };
                    let updated = db
                        .update_agent_session_meta(&sid, new_title, None, Some(&b_cl), Some(&p_cl))
                        .await
                        .map_err(|e| e.to_string())?;
                    return Ok(updated.unwrap_or(existing));
                }
                db.create_agent_session(derived_title, b_cl, p_cl)
                    .await
                    .map_err(|e| e.to_string())
            })
        })
        .await?;

    let now_ms = now_epoch_ms();
    let user_stored = AgentStoredMessage {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session.id.clone(),
        role: "user".to_string(),
        content: user_text.clone(),
        attachments_json: serde_json::to_string(&request.attachments)
            .unwrap_or_else(|_| "[]".to_string()),
        tool_steps_json: "[]".to_string(),
        status: "done".to_string(),
        created_at: now_ms,
    };
    let assistant_stored = AgentStoredMessage {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session.id.clone(),
        role: "assistant".to_string(),
        content: String::new(),
        attachments_json: "[]".to_string(),
        tool_steps_json: "[]".to_string(),
        status: "streaming".to_string(),
        created_at: now_ms + 1,
    };

    let u_cl = user_stored.clone();
    let a_cl = assistant_stored.clone();
    ctx.with_db(|db| {
        Box::pin(async move {
            db.upsert_agent_message(&u_cl)
                .await
                .map_err(|e| e.to_string())?;
            db.upsert_agent_message(&a_cl)
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        })
    })
    .await?;

    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut turns = ctx.active_turns.lock().await;
        turns.insert(
            session.id.clone(),
            ActiveTurnTracker {
                message_id: assistant_stored.id.clone(),
                execution_policy: session.execution_policy.clone(),
                cancel_flag,
                steps: Vec::new(),
                actions: Vec::new(),
            },
        );
    }

    spawn_agent_turn_worker(
        ctx.clone(),
        session.clone(),
        assistant_stored.id.clone(),
        user_text,
        request.attachments.clone(),
        assistant_stored.created_at,
    );

    Ok(StartAgentTurnResponse {
        session,
        user_message: stored_to_view(user_stored),
        assistant_message: stored_to_view(assistant_stored),
    })
}

#[tauri::command]
pub async fn cancel_agent_turn<R: Runtime>(
    app: AppHandle<R>,
    session_id: String,
) -> Result<bool, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    let turns = state.active_turns.lock().await;
    if let Some(tracker) = turns.get(&session_id) {
        tracker.cancel_flag.store(true, Ordering::SeqCst);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn resolve_agent_approval<R: Runtime>(
    app: AppHandle<R>,
    step_id: String,
    approved: bool,
) -> Result<bool, String> {
    let state = app.state::<AgentRuntimeContext<R>>();
    let mut approvals = state.pending_approvals.lock().await;
    if let Some(tx) = approvals.remove(&step_id) {
        let _ = tx.send(approved);
        Ok(true)
    } else {
        Ok(false)
    }
}
