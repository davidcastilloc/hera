pub mod backends;
pub mod cli_runner;
pub mod dna;
pub mod harness;
pub mod http_client;
pub mod onboarding;
pub mod prompts;
pub mod turn_eval;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_harness;

pub const CF_WORKERS_AI_FREE_DAILY_NEURONS: f64 = 10_000.0;
pub const CF_LLAMA_8B_FAST_NEURONS_PER_TURN: f64 = 6.6;
pub const CF_D1_FREE_STORAGE_BYTES: u64 = 5 * 1024 * 1024 * 1024; // 5 GB

pub use backends::{
    auto_detect_local_backend, list_supported_ai_backends, scan_local_ai_servers, AiBackendOption,
    DetectedLocalServer,
};
#[cfg(not(target_arch = "wasm32"))]
pub use cli_runner::execute_cli_agent_turn;
pub use cli_runner::{build_cli_agent_args, check_binary_in_path};
pub use dna::{
    compute_dj_style_dna, AnchorArtistStat, DjStyleDnaProfile, DjStyleDnaRequest, MediaMixProfile,
};
pub use harness::{
    parse_assistant_tool_calls, AgentToolExecutionStep, CloudflareFreeAiTurnRequest,
    CloudflareFreeAiTurnResponse,
};
pub use http_client::probe_backend_models;
pub use onboarding::{
    scan_ai_connectivity_onboarding, scan_system_cli_agents, AiConnectivityOnboardingReport,
    DetectedCliAgent, McpHostIntegrationGuide, SwarmProtectionStatus,
};
pub use prompts::{
    build_agent_harness_system_prompt, build_openai_tools_catalog,
    summarize_swarm_downloadable_catalog,
};
pub use turn_eval::{evaluate_agent_harness_turn, evaluate_cloudflare_free_ai_turn};
