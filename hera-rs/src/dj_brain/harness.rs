use super::dna::DjStyleDnaProfile;
#[cfg(not(target_arch = "wasm32"))]
use super::http_client::http_post_chat_completions;
#[cfg(not(target_arch = "wasm32"))]
use super::prompts::{build_agent_harness_system_prompt, build_openai_tools_catalog};
#[cfg(not(target_arch = "wasm32"))]
use super::CF_WORKERS_AI_FREE_DAILY_NEURONS;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentToolExecutionStep {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub step: usize,
    pub tool_name: String,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub arguments: serde_json::Value,
    pub result_preview: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CloudflareFreeAiTurnRequest {
    pub dj_handle: String,
    pub message: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub session_id: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub style_dna: Option<DjStyleDnaProfile>,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<_>"))]
    pub daily_neurons_used_so_far: f64,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<_>"))]
    pub cache_hit: bool,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub backend: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub base_url: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub model: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub api_key: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable, as = "Option<u32>"))]
    pub max_tool_iterations: Option<usize>,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub commander_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CloudflareFreeAiTurnResponse {
    pub backend_used: String,
    pub model_used: String,
    pub cf_aig_cache_status: String,
    pub neurons_consumed_this_turn: f64,
    pub daily_neurons_remaining: f64,
    pub estimated_cost_usd: f64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub d1_synced_bytes: usize,
    pub assistant_reply: String,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<_>"))]
    pub tool_calls_trace: Vec<AgentToolExecutionStep>,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<u32>"))]
    pub iterations_executed: usize,
}

pub fn parse_assistant_tool_calls(message: &serde_json::Value) -> Vec<(String, String, String)> {
    let mut calls = Vec::new();
    if let Some(arr) = message.get("tool_calls").and_then(|v| v.as_array()) {
        for (idx, item) in arr.iter().enumerate() {
            let call_id = item
                .get("id")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_else(|| format!("call_{}", idx + 1));
            if let Some(func) = item.get("function") {
                if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                    let raw_args = match func.get("arguments") {
                        Some(serde_json::Value::String(s)) => s.clone(),
                        Some(v) => v.to_string(),
                        None => "{}".to_string(),
                    };
                    calls.push((call_id, name.to_string(), raw_args));
                }
            }
        }
    }
    if !calls.is_empty() {
        return calls;
    }

    let Some(content) = message.get("content").and_then(|v| v.as_str()) else {
        return calls;
    };
    let mut rest = content;
    let mut idx = 1usize;
    while let Some(start) = rest.find("<tool_call>") {
        let after_start = &rest[start + "<tool_call>".len()..];
        let Some(end) = after_start.find("</tool_call>") else {
            break;
        };
        let inner = after_start[..end].trim();
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(inner) {
            if let Some(name) = val
                .get("name")
                .or_else(|| val.get("tool"))
                .and_then(|v| v.as_str())
            {
                let raw_args = match val.get("arguments").or_else(|| val.get("parameters")) {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(v) => v.to_string(),
                    None => "{}".to_string(),
                };
                calls.push((format!("call_tag_{idx}"), name.to_string(), raw_args));
                idx += 1;
            }
        }
        rest = &after_start[end + "</tool_call>".len()..];
    }
    calls
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub(super) fn execute_live_http_react_loop(
    req: &CloudflareFreeAiTurnRequest,
    state: &crate::p2p::P2PMeshState,
    resolved_backend_id: &str,
    base_url: &str,
    model: &str,
    is_local_backend: bool,
    tool_calls_trace: &mut Vec<AgentToolExecutionStep>,
    iterations_executed: &mut usize,
) -> Option<CloudflareFreeAiTurnResponse> {
    let max_iters = req.max_tool_iterations.unwrap_or(6).clamp(1, 10);
    let sys_prompt = build_agent_harness_system_prompt(
        &req.dj_handle,
        req.style_dna.as_ref(),
        state,
        req.commander_mode.as_deref(),
    );
    let tools = build_openai_tools_catalog();
    let mut messages = vec![
        serde_json::json!({ "role": "system", "content": sys_prompt }),
        serde_json::json!({ "role": "user", "content": req.message }),
    ];

    for step_idx in 1..=max_iters {
        *iterations_executed = step_idx;
        let payload = serde_json::json!({
            "model": model,
            "messages": messages,
            "tools": tools,
            "temperature": 0.2
        });

        let Ok(resp_json) = http_post_chat_completions(base_url, req.api_key.as_deref(), &payload)
        else {
            break;
        };
        let Some(message) = resp_json
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|choice| choice.get("message"))
        else {
            break;
        };

        let parsed_calls = parse_assistant_tool_calls(message);
        if !parsed_calls.is_empty() {
            messages.push(message.clone());
            for (call_id, fn_name, raw_args) in parsed_calls {
                dispatch_react_tool_step(
                    step_idx,
                    &call_id,
                    &fn_name,
                    raw_args,
                    state,
                    tool_calls_trace,
                    &mut messages,
                );
            }
            continue;
        }

        if let Some(content) = message
            .get("content")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let d1_bytes = req.message.len() + content.len() + 64;
            return Some(CloudflareFreeAiTurnResponse {
                backend_used: resolved_backend_id.to_string(),
                model_used: model.to_string(),
                cf_aig_cache_status: if is_local_backend {
                    "LOCAL_BYPASS".to_string()
                } else if req.cache_hit {
                    "HIT".to_string()
                } else {
                    "MISS".to_string()
                },
                neurons_consumed_this_turn: 0.0,
                daily_neurons_remaining: (CF_WORKERS_AI_FREE_DAILY_NEURONS
                    - req.daily_neurons_used_so_far)
                    .max(0.0),
                estimated_cost_usd: 0.0,
                d1_synced_bytes: d1_bytes,
                assistant_reply: content.to_string(),
                tool_calls_trace: tool_calls_trace.clone(),
                iterations_executed: step_idx,
            });
        }
        break;
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn dispatch_react_tool_step(
    step_idx: usize,
    call_id: &str,
    fn_name: &str,
    raw_args: String,
    state: &crate::p2p::P2PMeshState,
    tool_calls_trace: &mut Vec<AgentToolExecutionStep>,
    messages: &mut Vec<serde_json::Value>,
) {
    match serde_json::from_str::<serde_json::Value>(&raw_args) {
        Err(e) => {
            let err_msg = format!("Invalid JSON arguments: {e}");
            tool_calls_trace.push(AgentToolExecutionStep {
                step: step_idx,
                tool_name: fn_name.to_string(),
                arguments: serde_json::Value::String(raw_args),
                result_preview: err_msg.clone(),
                is_error: true,
            });
            messages.push(serde_json::json!({
                "role": "tool",
                "tool_call_id": call_id,
                "name": fn_name,
                "content": serde_json::json!({
                    "error": err_msg,
                    "self_heal_hint": "Ensure arguments is valid JSON matching the tool inputSchema and retry."
                }).to_string()
            }));
        }
        Ok(args_val) => match crate::mcp::dispatch_tool_call(fn_name, args_val.clone(), state) {
            Ok(res_val) => {
                let serialized = res_val.to_string();
                let preview = if serialized.chars().count() > 240 {
                    format!("{}...", serialized.chars().take(240).collect::<String>())
                } else {
                    serialized.clone()
                };
                tool_calls_trace.push(AgentToolExecutionStep {
                    step: step_idx,
                    tool_name: fn_name.to_string(),
                    arguments: args_val,
                    result_preview: preview,
                    is_error: false,
                });
                messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "name": fn_name,
                    "content": serialized
                }));
            }
            Err(err_str) => {
                tool_calls_trace.push(AgentToolExecutionStep {
                    step: step_idx,
                    tool_name: fn_name.to_string(),
                    arguments: args_val,
                    result_preview: err_str.clone(),
                    is_error: true,
                });
                messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "name": fn_name,
                    "content": serde_json::json!({
                        "error": err_str,
                        "self_heal_hint": "Check tool parameters against inputSchema and retry with valid arguments."
                    }).to_string()
                }));
            }
        },
    }
}
