use hera_node_rs::dj_brain::AiBackendOption;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct AgentAttachment {
    #[ts(type = "\"track\" | \"playlist\" | \"folder\" | \"file\"")]
    pub kind: String,
    pub id: String,
    pub label: String,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub path: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub meta: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct AgentFollowupAction {
    #[ts(type = "\"play_track\" | \"open_playlist\" | \"open_library\" | \"mini_mode\"")]
    pub kind: String,
    pub label: String,
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct AgentToolStepView {
    pub id: String,
    pub tool_name: String,
    pub title: String,
    pub summary: String,
    #[ts(type = "\"running\" | \"awaiting_approval\" | \"done\" | \"error\" | \"cancelled\"")]
    pub status: String,
    pub arguments_preview: String,
    pub receipt_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct AgentMessageView {
    pub id: String,
    pub session_id: String,
    #[ts(type = "\"user\" | \"assistant\"")]
    pub role: String,
    pub content: String,
    pub attachments: Vec<AgentAttachment>,
    pub tool_steps: Vec<AgentToolStepView>,
    #[serde(default)]
    pub actions: Vec<AgentFollowupAction>,
    #[ts(type = "\"streaming\" | \"done\" | \"error\"")]
    pub status: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct AgentSessionSummary {
    pub id: String,
    pub title: String,
    pub backend_id: String,
    #[ts(type = "\"auto\" | \"confirm\"")]
    pub execution_policy: String,
    pub agy_conversation_id: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[serde(default)]
    #[ts(type = "number")]
    pub message_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentStreamEvent {
    pub session_id: String,
    pub message_id: String,
    #[ts(type = "\"turn_started\" | \"step_upsert\" | \"text_delta\" | \"turn_completed\" | \"turn_error\"")]
    pub kind: String,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub text_delta: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub full_content: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub step: Option<AgentToolStepView>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub session: Option<AgentSessionSummary>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub actions: Option<Vec<AgentFollowupAction>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentUiSyncEvent {
    #[ts(type = "\"invalidate_library\" | \"player_control\" | \"navigate\"")]
    pub kind: String,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub action: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub track_id: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub seek_seconds: Option<f64>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub route: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AgentRuntimeStatus {
    pub mcp_port: u16,
    #[ts(type = "number")]
    pub tools_count: usize,
    pub agy_installed: bool,
    pub agy_path: Option<String>,
    pub supported_backends: Vec<AiBackendOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateAgentSessionRequest {
    #[serde(default)]
    #[ts(optional = nullable)]
    pub title: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub backend_id: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub execution_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpdateAgentSessionRequest {
    pub session_id: String,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub title: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub backend_id: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub execution_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct StartAgentTurnRequest {
    #[serde(default)]
    #[ts(optional = nullable)]
    pub session_id: Option<String>,
    pub message: String,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub attachments: Vec<AgentAttachment>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub backend_id: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub execution_policy: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub base_url: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub model: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct StartAgentTurnResponse {
    pub session: AgentSessionSummary,
    pub user_message: AgentMessageView,
    pub assistant_message: AgentMessageView,
}
