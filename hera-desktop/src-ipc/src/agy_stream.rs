//! Pure NDJSON stream parser and prompt context formatter for `agy --output-format stream-json`.

use crate::agent_types::AgentAttachment;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedAgyStreamEvent {
    Init {
        conversation_id: String,
    },
    TextDelta {
        delta: String,
    },
    NativeToolStep {
        step_index: usize,
        tool_name: String,
        state: String,
    },
    Result {
        conversation_id: Option<String>,
        response: String,
        status: String,
    },
    Ignored,
}

/// Parse a single NDJSON line emitted by `agy --output-format stream-json`.
pub fn parse_agy_stream_json_line(line: &str) -> ParsedAgyStreamEvent {
    let trimmed = line.trim();
    if trimmed.is_empty() || !trimmed.starts_with('{') {
        return ParsedAgyStreamEvent::Ignored;
    }
    let Ok(val) = serde_json::from_str::<Value>(trimmed) else {
        return ParsedAgyStreamEvent::Ignored;
    };
    let event = val.get("event").and_then(|v| v.as_str()).unwrap_or("");
    match event {
        "init" => {
            if let Some(cid) = val.get("conversation_id").and_then(|v| v.as_str()) {
                ParsedAgyStreamEvent::Init {
                    conversation_id: cid.to_string(),
                }
            } else {
                ParsedAgyStreamEvent::Ignored
            }
        }
        "step_update" => {
            let Some(su) = val.get("step_update") else {
                return ParsedAgyStreamEvent::Ignored;
            };
            let step_type = su.get("step_type").and_then(|v| v.as_str()).unwrap_or("");
            if step_type == "agent_response" {
                if let Some(delta) = su.get("text_delta").and_then(|v| v.as_str()) {
                    if !delta.is_empty() {
                        return ParsedAgyStreamEvent::TextDelta {
                            delta: delta.to_string(),
                        };
                    }
                }
            } else if let Some(tname) = su
                .get("tool_name")
                .or_else(|| su.get("name"))
                .and_then(|v| v.as_str())
            {
                if tname != "call_mcp_tool" {
                    let idx = su.get("step_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let state = su
                        .get("state")
                        .and_then(|v| v.as_str())
                        .unwrap_or("ACTIVE")
                        .to_string();
                    return ParsedAgyStreamEvent::NativeToolStep {
                        step_index: idx,
                        tool_name: tname.to_string(),
                        state,
                    };
                }
            }
            ParsedAgyStreamEvent::Ignored
        }
        "result" => {
            let Some(res) = val.get("result") else {
                return ParsedAgyStreamEvent::Ignored;
            };
            let cid = res
                .get("conversation_id")
                .and_then(|v| v.as_str())
                .map(String::from);
            let response = res
                .get("response")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let status = res
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("SUCCESS")
                .to_string();
            ParsedAgyStreamEvent::Result {
                conversation_id: cid,
                response,
                status,
            }
        }
        _ => ParsedAgyStreamEvent::Ignored,
    }
}

/// Format a user prompt with structured HERA Desktop attachments when present.
pub fn format_user_prompt_with_context(message: &str, attachments: &[AgentAttachment]) -> String {
    if attachments.is_empty() {
        return message.to_string();
    }
    let mut ctx_lines = Vec::new();
    for att in attachments {
        ctx_lines.push(format!(
            "- Adjunto [{}]: label=\"{}\", id=\"{}\", path={:?}, meta={:?}",
            att.kind, att.label, att.id, att.path, att.meta
        ));
    }
    format!(
        "Contexto adjunto por el usuario en HERA Desktop:\n{}\n\nSolicitud del usuario:\n{}",
        ctx_lines.join("\n"),
        message
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let tool_line = r#"{"event":"step_update","step_update":{"step_index":3,"state":"DONE","step_type":"tool_call","tool_name":"view_file"}}"#;
        assert_eq!(
            parse_agy_stream_json_line(tool_line),
            ParsedAgyStreamEvent::NativeToolStep {
                step_index: 3,
                tool_name: "view_file".to_string(),
                state: "DONE".to_string()
            }
        );

        let mcp_wrapper = r#"{"event":"step_update","step_update":{"step_index":4,"state":"ACTIVE","step_type":"tool_call","tool_name":"call_mcp_tool"}}"#;
        assert_eq!(
            parse_agy_stream_json_line(mcp_wrapper),
            ParsedAgyStreamEvent::Ignored
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

        assert_eq!(
            parse_agy_stream_json_line("not json"),
            ParsedAgyStreamEvent::Ignored
        );
    }
}
