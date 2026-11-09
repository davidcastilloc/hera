use super::backends::{scan_local_ai_servers, DetectedLocalServer};
#[cfg(not(target_arch = "wasm32"))]
use super::cli_runner::check_binary_in_path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DetectedCliAgent {
    pub id: String,
    pub binary_name: String,
    pub display_name: String,
    pub installed: bool,
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub version_hint: Option<String>,
    pub command_preview: String,
    pub zero_api_key_needed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct McpHostIntegrationGuide {
    pub stdio_command: String,
    pub http_endpoint: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub tools_count: usize,
    pub sample_config_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SwarmProtectionStatus {
    pub quarantine_first_enforced: bool,
    pub p2p_seeding_active: bool,
    pub deterministic_hot_cues_enabled: bool,
    pub turbo_credit_multiplier: f32,
    pub commander_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AiConnectivityOnboardingReport {
    pub recommended_mode: String,
    pub recommended_backend_id: String,
    pub summary_badge: String,
    pub cli_agents: Vec<DetectedCliAgent>,
    pub local_servers: Vec<DetectedLocalServer>,
    pub cloud_free_available: bool,
    pub mcp_integration: McpHostIntegrationGuide,
    pub swarm_protection: SwarmProtectionStatus,
}

pub fn scan_system_cli_agents() -> Vec<DetectedCliAgent> {
    const CANDIDATES: &[(&str, &str, &str, &str)] = &[
        (
            "cli_agy",
            "agy",
            "Antigravity CLI (agy)",
            "agy -p --disable-slash-commands <prompt>",
        ),
        (
            "cli_claude",
            "claude",
            "Claude Code (claude)",
            "claude -p <prompt>",
        ),
        (
            "cli_codex",
            "codex",
            "OpenAI Codex CLI (codex)",
            "codex exec <prompt>",
        ),
        (
            "cli_gemini",
            "gemini",
            "Google Gemini CLI (gemini)",
            "gemini --prompt <prompt>",
        ),
        (
            "cli_ollama",
            "ollama",
            "Ollama CLI (ollama)",
            "ollama run llama3.3 <prompt>",
        ),
    ];

    #[cfg(target_arch = "wasm32")]
    {
        CANDIDATES
            .iter()
            .map(|&(id, bin, display_name, cmd_preview)| DetectedCliAgent {
                id: id.to_string(),
                binary_name: bin.to_string(),
                display_name: display_name.to_string(),
                installed: false,
                version_hint: None,
                command_preview: cmd_preview.to_string(),
                zero_api_key_needed: true,
            })
            .collect()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::scope(|s| {
            let handles: Vec<_> = CANDIDATES
                .iter()
                .map(|&(id, bin, display_name, cmd_preview)| {
                    s.spawn(move || {
                        let (installed, path_hint) = check_binary_in_path(bin);
                        DetectedCliAgent {
                            id: id.to_string(),
                            binary_name: bin.to_string(),
                            display_name: display_name.to_string(),
                            installed,
                            version_hint: path_hint,
                            command_preview: cmd_preview.to_string(),
                            zero_api_key_needed: true,
                        }
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        })
    }
}

pub fn scan_ai_connectivity_onboarding(
    commander_mode: Option<&str>,
) -> AiConnectivityOnboardingReport {
    #[cfg(not(target_arch = "wasm32"))]
    let (cli_agents, local_servers) = std::thread::scope(|s| {
        let cli_job = s.spawn(scan_system_cli_agents);
        let local_job = s.spawn(scan_local_ai_servers);
        (
            cli_job.join().unwrap_or_default(),
            local_job.join().unwrap_or_default(),
        )
    });
    #[cfg(target_arch = "wasm32")]
    let (cli_agents, local_servers) = (scan_system_cli_agents(), scan_local_ai_servers());

    let active_local = local_servers.iter().find(|s| s.reachable);
    let active_cli = cli_agents.iter().find(|c| c.installed);

    let (recommended_mode, recommended_backend_id, summary_badge) = if let Some(local) =
        active_local
    {
        (
            "local_server".to_string(),
            local.id.clone(),
            format!(
                "🖥️ Local Detectado: {} (100% Offline & $0.00)",
                local.display_name
            ),
        )
    } else if let Some(cli) = active_cli {
        (
            "cloud_free".to_string(),
            "cloudflare_free".to_string(),
            format!(
                "⚡ Cloud Free Activo ($0.00 instantáneo) · {} disponible en selector Pro",
                cli.display_name
            ),
        )
    } else {
        (
            "cloud_free".to_string(),
            "cloudflare_free".to_string(),
            "⚡ Cloud Free Activo: Cloudflare Workers AI (10,000 Neurons/día a $0.00)".to_string(),
        )
    };

    let sample_config_json = serde_json::json!({
        "mcpServers": {
            "hera": {
                "command": "hera-node",
                "args": ["--mcp"],
                "env": {
                    "HERA_P2P_SHARING": "true"
                }
            }
        }
    })
    .to_string();

    let mcp_integration = McpHostIntegrationGuide {
        stdio_command: "hera-node --mcp".to_string(),
        http_endpoint: "http://localhost:8787/mcp".to_string(),
        tools_count: 21,
        sample_config_json,
    };

    let mode_str = commander_mode.unwrap_or("executive_boss");

    AiConnectivityOnboardingReport {
        recommended_mode,
        recommended_backend_id,
        summary_badge,
        cli_agents,
        local_servers,
        cloud_free_available: true,
        mcp_integration,
        swarm_protection: SwarmProtectionStatus {
            quarantine_first_enforced: true,
            p2p_seeding_active: true,
            deterministic_hot_cues_enabled: true,
            turbo_credit_multiplier: 1.5,
            commander_mode: mode_str.to_string(),
        },
    }
}
