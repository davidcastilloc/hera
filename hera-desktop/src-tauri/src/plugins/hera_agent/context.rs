use super::*;

pub struct ActiveTurnTracker {
    pub message_id: String,
    pub execution_policy: String,
    pub cancel_flag: Arc<AtomicBool>,
    pub steps: Vec<AgentToolStepView>,
    pub actions: Vec<AgentFollowupAction>,
}

#[allow(dead_code)]
pub enum AgentHostBridge<R: Runtime> {
    Tauri(AppHandle<R>),
    Standalone(Arc<Mutex<DB>>),
}

impl<R: Runtime> Clone for AgentHostBridge<R> {
    fn clone(&self) -> Self {
        match self {
            Self::Tauri(app) => Self::Tauri(app.clone()),
            Self::Standalone(db) => Self::Standalone(db.clone()),
        }
    }
}

pub struct AgentRuntimeContext<R: Runtime> {
    pub host: AgentHostBridge<R>,
    pub mcp_port: u16,
    pub mcp_bearer_token: String,
    pub active_turns: Arc<Mutex<HashMap<String, ActiveTurnTracker>>>,
    pub active_mcp_session: Arc<Mutex<Option<String>>>,
    pub pending_approvals: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
}

impl<R: Runtime> Clone for AgentRuntimeContext<R> {
    fn clone(&self) -> Self {
        Self {
            host: self.host.clone(),
            mcp_port: self.mcp_port,
            mcp_bearer_token: self.mcp_bearer_token.clone(),
            active_turns: self.active_turns.clone(),
            active_mcp_session: self.active_mcp_session.clone(),
            pending_approvals: self.pending_approvals.clone(),
        }
    }
}

impl<R: Runtime> AgentRuntimeContext<R> {
    pub(super) async fn with_db<F, T>(&self, f: F) -> Result<T, String>
    where
        F: for<'a> FnOnce(
            &'a mut DB,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<T, String>> + Send + 'a>,
        >,
    {
        match &self.host {
            AgentHostBridge::Tauri(app) => {
                let Some(db_state) = app.try_state::<DBState>() else {
                    return Err("Base de datos aún inicializándose".to_string());
                };
                let mut guard = db_state.get_lock().await;
                f(&mut guard).await
            }
            AgentHostBridge::Standalone(db_arc) => {
                let mut guard = db_arc.lock().await;
                f(&mut guard).await
            }
        }
    }

    pub(super) fn emit_stream(&self, evt: AgentStreamEvent) {
        if let AgentHostBridge::Tauri(app) = &self.host {
            let _ = app.emit("hera-agent:stream", evt);
        }
    }

    pub(super) fn emit_ui_sync(&self, evt: AgentUiSyncEvent) {
        if let AgentHostBridge::Tauri(app) = &self.host {
            let _ = app.emit("hera-agent:ui-sync", evt);
        }
    }
}

pub(super) fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(super) fn stored_to_view(msg: AgentStoredMessage) -> AgentMessageView {
    let attachments: Vec<AgentAttachment> =
        serde_json::from_str(&msg.attachments_json).unwrap_or_default();
    let (tool_steps, actions): (Vec<AgentToolStepView>, Vec<AgentFollowupAction>) =
        if let Ok(obj) = serde_json::from_str::<Value>(&msg.tool_steps_json) {
            if obj.is_array() {
                (serde_json::from_value(obj).unwrap_or_default(), Vec::new())
            } else {
                let steps = obj
                    .get("steps")
                    .cloned()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                let acts = obj
                    .get("actions")
                    .cloned()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                (steps, acts)
            }
        } else {
            (Vec::new(), Vec::new())
        };
    AgentMessageView {
        id: msg.id,
        session_id: msg.session_id,
        role: msg.role,
        content: msg.content,
        attachments,
        tool_steps,
        actions,
        status: msg.status,
        created_at: msg.created_at,
    }
}

pub(super) fn encode_steps_and_actions(
    steps: &[AgentToolStepView],
    actions: &[AgentFollowupAction],
) -> String {
    json!({
        "steps": steps,
        "actions": actions,
    })
    .to_string()
}

pub async fn dispatch_unified_agent_tool<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session_id: Option<&str>,
    tool_name: &str,
    args: Value,
) -> Result<Value, String> {
    let step_id = format!("step-{}", uuid::Uuid::new_v4());
    let args_preview = truncate_chars(&args.to_string(), 180);

    let mut policy = "auto".to_string();
    let mut message_id_opt = None;
    if let Some(sid) = session_id {
        let mut turns = ctx.active_turns.lock().await;
        if let Some(tracker) = turns.get_mut(sid) {
            if tracker.cancel_flag.load(Ordering::SeqCst) {
                return Err("Turno cancelado por el usuario".to_string());
            }
            policy = tracker.execution_policy.clone();
            message_id_opt = Some(tracker.message_id.clone());

            let initial_status = if policy == "confirm" && requires_system_confirmation(tool_name) {
                "awaiting_approval"
            } else {
                "running"
            };
            let step_title = human_tool_title(tool_name);
            let running_step = AgentToolStepView {
                id: step_id.clone(),
                tool_name: tool_name.to_string(),
                title: step_title,
                summary: if initial_status == "awaiting_approval" {
                    format!(
                        "Confirmar ejecución · {}",
                        truncate_chars(&args_preview, 54)
                    )
                } else {
                    "En progreso…".to_string()
                },
                status: initial_status.to_string(),
                arguments_preview: args_preview.clone(),
                receipt_text: format!("argumentos: {args_preview}"),
            };
            tracker.steps.push(running_step.clone());
            ctx.emit_stream(AgentStreamEvent {
                session_id: sid.to_string(),
                message_id: tracker.message_id.clone(),
                kind: "step_upsert".to_string(),
                text_delta: None,
                full_content: None,
                step: Some(running_step),
                session: None,
                actions: None,
            });
        }
    }

    if policy == "confirm" && requires_system_confirmation(tool_name) {
        let (tx, rx) = oneshot::channel::<bool>();
        {
            let mut approvals = ctx.pending_approvals.lock().await;
            approvals.insert(step_id.clone(), tx);
        }
        let approved = tokio::time::timeout(Duration::from_secs(180), rx)
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or(false);
        if !approved {
            if let (Some(sid), Some(mid)) = (session_id, message_id_opt.as_ref()) {
                finalize_tracked_step(
                    ctx,
                    sid,
                    mid,
                    &step_id,
                    tool_name,
                    &args,
                    Err("Ejecución cancelada por el usuario"),
                    Some("cancelled"),
                )
                .await;
            }
            return Err("Ejecución cancelada por el usuario".to_string());
        }
    }

    let exec_res = execute_tool_inner(ctx, session_id, tool_name, &args).await;

    if let (Some(sid), Some(mid)) = (session_id, message_id_opt.as_ref()) {
        finalize_tracked_step(
            ctx,
            sid,
            mid,
            &step_id,
            tool_name,
            &args,
            exec_res.as_ref().map_err(String::as_str),
            None,
        )
        .await;
    }

    exec_res
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn finalize_tracked_step<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session_id: &str,
    message_id: &str,
    step_id: &str,
    tool_name: &str,
    args: &Value,
    res: Result<&Value, &str>,
    status_override: Option<&str>,
) {
    let (title, summary, receipt_text) = summarize_tool_execution(tool_name, args, res);
    let status = status_override
        .unwrap_or(if res.is_ok() { "done" } else { "error" })
        .to_string();

    let mut updated_step = None;
    {
        let mut turns = ctx.active_turns.lock().await;
        if let Some(tracker) = turns.get_mut(session_id)
            && let Some(existing) = tracker.steps.iter_mut().find(|s| s.id == step_id)
        {
            existing.title = title;
            existing.summary = summary;
            existing.status = status;
            existing.receipt_text = receipt_text;
            updated_step = Some(existing.clone());
        }
    }

    if let Some(step) = updated_step {
        ctx.emit_stream(AgentStreamEvent {
            session_id: session_id.to_string(),
            message_id: message_id.to_string(),
            kind: "step_upsert".to_string(),
            text_delta: None,
            full_content: None,
            step: Some(step),
            session: None,
            actions: None,
        });
    }
}

pub(super) async fn record_followup_action<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session_id: Option<&str>,
    action: AgentFollowupAction,
) {
    let Some(sid) = session_id else { return };
    let mut turns = ctx.active_turns.lock().await;
    if let Some(tracker) = turns.get_mut(sid)
        && !tracker
            .actions
            .iter()
            .any(|a| a.kind == action.kind && a.target_id == action.target_id)
    {
        tracker.actions.push(action);
    }
}
