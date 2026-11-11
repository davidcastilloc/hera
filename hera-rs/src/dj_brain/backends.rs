#[cfg(not(target_arch = "wasm32"))]
use super::http_client::probe_backend_models;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AiBackendOption {
    pub id: String,
    pub display_name: String,
    pub default_model: String,
    pub base_url: String,
    pub requires_api_key: bool,
    pub is_default_free: bool,
    pub zero_laptop_cpu_ram: bool,
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DetectedLocalServer {
    pub id: String,
    pub display_name: String,
    pub base_url: String,
    pub reachable: bool,
    pub active_models: Vec<String>,
    #[cfg_attr(feature = "ts", ts(optional = nullable, as = "Option<u32>"))]
    pub latency_ms: Option<u64>,
}

type BackendSpec = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    bool,
    bool,
    bool,
    &'static str,
);

pub fn list_supported_ai_backends() -> Vec<AiBackendOption> {
    const SPECS: &[BackendSpec] = &[
        (
            "cloudflare_free",
            "⚡ Hera Cloud Free (Cloudflare Workers AI + AI Gateway)",
            "@cf/meta/llama-3.1-8b-instruct-fp8-fast",
            "https://cloud.hera.audio/api/v1/ai/v1",
            false,
            true,
            true,
            "cloud_free",
        ),
        (
            "cli_agy",
            "⌨️ Antigravity CLI (agy)",
            "inherit",
            "local://agy",
            false,
            true,
            false,
            "cli_agent",
        ),
        (
            "cli_claude",
            "⌨️ Claude Code CLI (claude)",
            "inherit",
            "local://claude",
            false,
            true,
            false,
            "cli_agent",
        ),
        (
            "cli_codex",
            "⌨️ OpenAI Codex CLI (codex)",
            "inherit",
            "local://codex",
            false,
            true,
            false,
            "cli_agent",
        ),
        (
            "cli_gemini",
            "⌨️ Google Gemini CLI (gemini)",
            "inherit",
            "local://gemini",
            false,
            true,
            false,
            "cli_agent",
        ),
        (
            "lmstudio",
            "🖥️ LM Studio (Local OpenAI Server :1234)",
            "default",
            "http://localhost:1234/v1",
            false,
            false,
            false,
            "local_server",
        ),
        (
            "ollama",
            "💻 Ollama (Local :11434)",
            "llama3.3",
            "http://localhost:11434/v1",
            false,
            false,
            false,
            "local_server",
        ),
        (
            "openrouter",
            "🌐 OpenRouter (:free & Pro Models)",
            "meta-llama/llama-3.3-70b-instruct:free",
            "https://openrouter.ai/api/v1",
            true,
            false,
            true,
            "byok_cloud",
        ),
        (
            "gemini",
            "✨ Google Gemini API",
            "gemini-2.5-flash",
            "https://generativelanguage.googleapis.com/v1beta/openai",
            true,
            false,
            true,
            "byok_cloud",
        ),
        (
            "vertex",
            "☁️ Google Vertex AI",
            "gemini-2.5-flash",
            "https://aiplatform.googleapis.com/v1beta1/openai",
            true,
            false,
            true,
            "byok_cloud",
        ),
        (
            "openai",
            "🧠 OpenAI",
            "gpt-4o",
            "https://api.openai.com/v1",
            true,
            false,
            true,
            "byok_cloud",
        ),
        (
            "anthropic",
            "🎭 Anthropic Claude",
            "claude-sonnet-4-20250514",
            "https://api.anthropic.com/v1",
            true,
            false,
            true,
            "byok_cloud",
        ),
        (
            "jan",
            "🤖 Jan (Local :1337)",
            "default",
            "http://localhost:1337/v1",
            false,
            false,
            false,
            "local_server",
        ),
        (
            "llamacpp",
            "⚡ llama.cpp Server (Local :8080)",
            "default",
            "http://localhost:8080/v1",
            false,
            false,
            false,
            "local_server",
        ),
        (
            "vllm",
            "🚀 vLLM (Local :8000)",
            "default",
            "http://localhost:8000/v1",
            false,
            false,
            false,
            "local_server",
        ),
        (
            "custom",
            "🔌 Custom OpenAI-Compatible Endpoint",
            "default",
            "http://localhost:1234/v1",
            false,
            false,
            false,
            "byok_cloud",
        ),
    ];
    SPECS
        .iter()
        .map(
            |&(
                id,
                display_name,
                default_model,
                base_url,
                requires_api_key,
                is_default_free,
                zero_laptop_cpu_ram,
                category,
            )| AiBackendOption {
                id: id.to_string(),
                display_name: display_name.to_string(),
                default_model: default_model.to_string(),
                base_url: base_url.to_string(),
                requires_api_key,
                is_default_free,
                zero_laptop_cpu_ram,
                category: Some(category.to_string()),
            },
        )
        .collect()
}

pub fn scan_local_ai_servers() -> Vec<DetectedLocalServer> {
    let candidates = [
        ("lmstudio", "LM Studio (:1234)", "http://localhost:1234/v1"),
        ("ollama", "Ollama (:11434)", "http://localhost:11434/v1"),
        ("jan", "Jan (:1337)", "http://localhost:1337/v1"),
        ("llamacpp", "llama.cpp (:8080)", "http://localhost:8080/v1"),
        ("vllm", "vLLM (:8000)", "http://localhost:8000/v1"),
    ];

    #[cfg(target_arch = "wasm32")]
    {
        candidates
            .into_iter()
            .map(|(id, display_name, url)| DetectedLocalServer {
                id: id.to_string(),
                display_name: display_name.to_string(),
                base_url: url.to_string(),
                reachable: false,
                active_models: Vec::new(),
                latency_ms: None,
            })
            .collect()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::scope(|s| {
            let handles: Vec<_> = candidates
                .into_iter()
                .map(|(id, display_name, url)| {
                    s.spawn(move || {
                        let start = std::time::Instant::now();
                        let models = probe_backend_models(url, None).unwrap_or_default();
                        let reachable = !models.is_empty();
                        let latency_ms = if reachable {
                            Some(start.elapsed().as_millis() as u64)
                        } else {
                            None
                        };
                        DetectedLocalServer {
                            id: id.to_string(),
                            display_name: display_name.to_string(),
                            base_url: url.to_string(),
                            reachable,
                            active_models: models,
                            latency_ms,
                        }
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        })
    }
}

pub fn auto_detect_local_backend() -> Option<(String, String, String)> {
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let candidates = [
            ("lmstudio", "http://127.0.0.1:1234/v1"),
            ("ollama", "http://127.0.0.1:11434/v1"),
            ("jan", "http://127.0.0.1:1337/v1"),
            ("llamacpp", "http://127.0.0.1:8080/v1"),
            ("vllm", "http://127.0.0.1:8000/v1"),
        ];
        for (backend_id, url) in candidates {
            if let Ok(models) = probe_backend_models(url, None) {
                if let Some(first) = models.into_iter().next() {
                    return Some((backend_id.to_string(), url.to_string(), first));
                }
            }
        }
        None
    }
}
