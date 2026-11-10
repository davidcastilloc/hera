use super::*;

pub(super) fn spawn_agent_turn_worker<R: Runtime>(
    bg_ctx: AgentRuntimeContext<R>,
    bg_session: AgentSessionSummary,
    bg_assistant_id: String,
    user_text: String,
    bg_attachments: Vec<AgentAttachment>,
    bg_created_at: i64,
) {
    tauri::async_runtime::spawn(async move {
        bg_ctx.emit_stream(AgentStreamEvent {
            session_id: bg_session.id.clone(),
            message_id: bg_assistant_id.clone(),
            kind: "turn_started".to_string(),
            text_delta: None,
            full_content: None,
            step: None,
            session: Some(bg_session.clone()),
            actions: None,
        });

        let turn_result = if bg_session.backend_id == "cli_agy" {
            match run_agy_streaming_turn(
                &bg_ctx,
                &bg_session,
                &bg_assistant_id,
                &user_text,
                &bg_attachments,
            )
            .await
            {
                Ok(reply) => Ok(reply),
                Err(err) if err.contains("detenido por el usuario") => Err(err),
                Err(_) => {
                    execute_smart_intent_plan(&bg_ctx, &bg_session.id, &user_text, &bg_attachments)
                        .await
                }
            }
        } else {
            execute_smart_intent_plan(&bg_ctx, &bg_session.id, &user_text, &bg_attachments).await
        };

        let (steps, actions) = {
            let mut turns = bg_ctx.active_turns.lock().await;
            if let Some(done_tracker) = turns.remove(&bg_session.id) {
                (done_tracker.steps, done_tracker.actions)
            } else {
                (Vec::new(), Vec::new())
            }
        };

        match turn_result {
            Ok(final_reply) => {
                let final_msg = AgentStoredMessage {
                    id: bg_assistant_id.clone(),
                    session_id: bg_session.id.clone(),
                    role: "assistant".to_string(),
                    content: final_reply.clone(),
                    attachments_json: "[]".to_string(),
                    tool_steps_json: encode_steps_and_actions(&steps, &actions),
                    status: "done".to_string(),
                    created_at: bg_created_at,
                };
                let _ = bg_ctx
                    .with_db(|db| {
                        Box::pin(async move {
                            db.upsert_agent_message(&final_msg)
                                .await
                                .map_err(|e| e.to_string())
                        })
                    })
                    .await;

                bg_ctx.emit_stream(AgentStreamEvent {
                    session_id: bg_session.id.clone(),
                    message_id: bg_assistant_id,
                    kind: "turn_completed".to_string(),
                    text_delta: None,
                    full_content: Some(final_reply),
                    step: None,
                    session: Some(bg_session),
                    actions: Some(actions),
                });
            }
            Err(err_msg) => {
                let err_stored = AgentStoredMessage {
                    id: bg_assistant_id.clone(),
                    session_id: bg_session.id.clone(),
                    role: "assistant".to_string(),
                    content: err_msg.clone(),
                    attachments_json: "[]".to_string(),
                    tool_steps_json: encode_steps_and_actions(&steps, &actions),
                    status: "error".to_string(),
                    created_at: bg_created_at,
                };
                let _ = bg_ctx
                    .with_db(|db| {
                        Box::pin(async move {
                            db.upsert_agent_message(&err_stored)
                                .await
                                .map_err(|e| e.to_string())
                        })
                    })
                    .await;

                bg_ctx.emit_stream(AgentStreamEvent {
                    session_id: bg_session.id,
                    message_id: bg_assistant_id,
                    kind: "turn_error".to_string(),
                    text_delta: None,
                    full_content: Some(err_msg),
                    step: None,
                    session: None,
                    actions: None,
                });
            }
        }
    });
}
