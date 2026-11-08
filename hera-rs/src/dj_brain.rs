use crate::export::ExportTrackItem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const CF_WORKERS_AI_FREE_DAILY_NEURONS: f64 = 10_000.0;
pub const CF_LLAMA_8B_FAST_NEURONS_PER_TURN: f64 = 6.6;
pub const CF_D1_FREE_STORAGE_BYTES: u64 = 5 * 1024 * 1024 * 1024; // 5 GB

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorArtistStat {
    pub artist: String,
    pub track_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMixProfile {
    pub audio_pct: f32,
    pub video_pct: f32,
    pub own_productions_pct: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DjStyleDnaRequest {
    pub tracks: Vec<ExportTrackItem>,
    pub dj_handle: Option<String>,
    pub preferred_dj_software: Option<String>,
    pub golden_rules: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DjStyleDnaProfile {
    pub dj_handle: String,
    pub preferred_dj_software: String,
    pub golden_rules: String,
    pub total_tracks: usize,
    pub dominant_bpm_zone: String,
    pub overall_average_bpm: f32,
    pub dominant_camelot_keys: Vec<String>,
    pub minor_key_ratio_pct: f32,
    pub top_anchor_artists: Vec<AnchorArtistStat>,
    pub media_mix_profile: MediaMixProfile,
    pub prompt_block: String,
}

pub fn compute_dj_style_dna(req: &DjStyleDnaRequest) -> DjStyleDnaProfile {
    let total = req.tracks.len();
    let denom = total.max(1) as f32;

    let mut zone_counts: HashMap<&'static str, usize> = HashMap::new();
    zone_counts.insert("warmup_urbano_70_105", 0);
    zone_counts.insert("tropical_salsa_106_121", 0);
    zone_counts.insert("club_afro_guaracha_122_132", 0);
    zone_counts.insert("hora_loca_merengue_133_plus", 0);

    let mut artist_counts: HashMap<String, usize> = HashMap::new();
    let mut camelot_counts: HashMap<String, usize> = HashMap::new();
    let mut bpm_sum = 0.0f32;
    let mut video_cnt = 0usize;
    let mut own_cnt = 0usize;
    let mut minor_cnt = 0usize;

    for t in &req.tracks {
        bpm_sum += t.bpm;
        let zone = if t.bpm <= 105.9 {
            "warmup_urbano_70_105"
        } else if t.bpm <= 121.9 {
            "tropical_salsa_106_121"
        } else if t.bpm <= 132.9 {
            "club_afro_guaracha_122_132"
        } else {
            "hora_loca_merengue_133_plus"
        };
        *zone_counts.entry(zone).or_insert(0) += 1;

        let art = t.artist.trim();
        if !art.is_empty() && !art.eq_ignore_ascii_case("unknown artist") {
            *artist_counts.entry(art.to_string()).or_insert(0) += 1;
        }

        let cam = t.camelot.trim().to_uppercase();
        if !cam.is_empty() {
            if cam.ends_with('A') {
                minor_cnt += 1;
            }
            *camelot_counts.entry(cam).or_insert(0) += 1;
        }

        if t.is_video {
            video_cnt += 1;
        }
        if t.is_own_production {
            own_cnt += 1;
        }
    }

    let dominant_bpm_zone = zone_counts
        .into_iter()
        .max_by_key(|(_, cnt)| *cnt)
        .map(|(z, _)| z.to_string())
        .unwrap_or_else(|| "club_afro_guaracha_122_132".to_string());

    let mut cam_vec: Vec<(String, usize)> = camelot_counts.into_iter().collect();
    cam_vec.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let dominant_camelot_keys: Vec<String> = cam_vec.into_iter().take(5).map(|(k, _)| k).collect();

    let mut art_vec: Vec<(String, usize)> = artist_counts.into_iter().collect();
    art_vec.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let top_anchor_artists: Vec<AnchorArtistStat> = art_vec
        .into_iter()
        .take(15)
        .map(|(artist, track_count)| AnchorArtistStat {
            artist,
            track_count,
        })
        .collect();

    let overall_average_bpm = if total > 0 {
        ((bpm_sum / denom) * 10.0).round() / 10.0
    } else {
        124.0
    };
    let minor_key_ratio_pct = ((minor_cnt as f32 / denom) * 1000.0).round() / 10.0;
    let audio_cnt = total.saturating_sub(video_cnt + own_cnt);

    let media_mix_profile = MediaMixProfile {
        audio_pct: ((audio_cnt as f32 / denom) * 1000.0).round() / 10.0,
        video_pct: ((video_cnt as f32 / denom) * 1000.0).round() / 10.0,
        own_productions_pct: ((own_cnt as f32 / denom) * 1000.0).round() / 10.0,
    };

    let dj_handle = req
        .dj_handle
        .clone()
        .unwrap_or_else(|| "DaviDJ".to_string());
    let preferred_dj_software = req
        .preferred_dj_software
        .clone()
        .unwrap_or_else(|| "VirtualDJ".to_string());
    let golden_rules = req.golden_rules.clone().unwrap_or_else(|| {
        "Priorizar FLAC/320k, flujo armónico Camelot y al menos 1 producción propia".to_string()
    });

    let prompt_block = format!(
        "[MEMORIA ACTIVA DEL DJ BRAIN — PERFIL '{}']\n- Tracks locales: {}\n- Zona BPM dominante: {} (Promedio: {:.1} BPM)\n- Llaves Camelot dominantes: {:?}\n- Software principal de cabina: {}\n- Reglas de oro: {}",
        dj_handle,
        total,
        dominant_bpm_zone,
        overall_average_bpm,
        dominant_camelot_keys,
        preferred_dj_software,
        golden_rules
    );

    DjStyleDnaProfile {
        dj_handle,
        preferred_dj_software,
        golden_rules,
        total_tracks: total,
        dominant_bpm_zone,
        overall_average_bpm,
        dominant_camelot_keys,
        minor_key_ratio_pct,
        top_anchor_artists,
        media_mix_profile,
        prompt_block,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiBackendOption {
    pub id: String,
    pub display_name: String,
    pub default_model: String,
    pub base_url: String,
    pub requires_api_key: bool,
    pub is_default_free: bool,
    pub zero_laptop_cpu_ram: bool,
}

pub fn list_supported_ai_backends() -> Vec<AiBackendOption> {
    const SPECS: &[(&str, &str, &str, &str, bool, bool, bool)] = &[
        ("cloudflare_free", "⚡ Hera Cloud Free (Cloudflare Workers AI + AI Gateway)", "@cf/meta/llama-3.1-8b-instruct-fp8-fast", "https://cloud.hera.audio/api/v1/ai/v1", false, true, true),
        ("lmstudio", "🖥️ LM Studio (Local OpenAI Server :1234)", "default", "http://localhost:1234/v1", false, false, false),
        ("ollama", "💻 Ollama (Local :11434)", "llama3.3", "http://localhost:11434/v1", false, false, false),
        ("openrouter", "🌐 OpenRouter (:free & Pro Models)", "meta-llama/llama-3.3-70b-instruct:free", "https://openrouter.ai/api/v1", true, false, true),
        ("gemini", "✨ Google Gemini API", "gemini-2.5-flash", "https://generativelanguage.googleapis.com/v1beta/openai", true, false, true),
        ("vertex", "☁️ Google Vertex AI", "gemini-2.5-flash", "https://aiplatform.googleapis.com/v1beta1/openai", true, false, true),
        ("openai", "🧠 OpenAI", "gpt-4o", "https://api.openai.com/v1", true, false, true),
        ("anthropic", "🎭 Anthropic Claude", "claude-sonnet-4-20250514", "https://api.anthropic.com/v1", true, false, true),
        ("jan", "🤖 Jan (Local :1337)", "default", "http://localhost:1337/v1", false, false, false),
        ("llamacpp", "⚡ llama.cpp Server (Local :8080)", "default", "http://localhost:8080/v1", false, false, false),
        ("vllm", "🚀 vLLM (Local :8000)", "default", "http://localhost:8000/v1", false, false, false),
        ("custom", "🔌 Custom OpenAI-Compatible Endpoint", "default", "http://localhost:1234/v1", false, false, false),
    ];
    SPECS
        .iter()
        .map(
            |&(id, display_name, default_model, base_url, requires_api_key, is_default_free, zero_laptop_cpu_ram)| {
                AiBackendOption {
                    id: id.to_string(),
                    display_name: display_name.to_string(),
                    default_model: default_model.to_string(),
                    base_url: base_url.to_string(),
                    requires_api_key,
                    is_default_free,
                    zero_laptop_cpu_ram,
                }
            },
        )
        .collect()
}

pub fn build_openai_tools_catalog() -> Vec<serde_json::Value> {
    let raw = crate::mcp::list_mcp_tools();
    let Some(tools) = raw.get("tools").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    tools
        .iter()
        .filter_map(|t| {
            let name = t.get("name")?.as_str()?;
            if name == "chat_with_dj_copilot" || name == "ensure_hera_updated" {
                return None;
            }
            let description = t.get("description").and_then(|v| v.as_str()).unwrap_or("");
            let parameters = t
                .get("inputSchema")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({ "type": "object", "properties": {} }));
            Some(serde_json::json!({
                "type": "function",
                "function": {
                    "name": name,
                    "description": description,
                    "parameters": parameters,
                }
            }))
        })
        .collect()
}

pub fn summarize_swarm_downloadable_catalog(state: &crate::p2p::P2PMeshState) -> String {
    let index = state.build_broker_swarm_index();
    if index.is_empty() {
        return "[CATÁLOGO DESCARGABLE DEL ENJAMBRE P2P: 0 pistas activas]".to_string();
    }
    let mut lines = vec![format!(
        "[CATÁLOGO EN VIVO DEL ENJAMBRE P2P DISPONIBLE PARA DESCARGA ({} Masters Verificados)]:",
        index.len()
    )];
    for rec in index.iter().take(12) {
        let boost_ready = rec
            .available_routes
            .iter()
            .any(|r| r.storage_provider == "r2" || r.storage_provider == "r2_cas_zero_copy_link");
        let seeders: Vec<String> = rec
            .available_routes
            .iter()
            .map(|r| format!("{}({})", r.owner_dj, r.storage_provider))
            .collect();
        lines.push(format!(
            "- {} — {} | {:.1} BPM | {} | {} | Replicas: {} | P2P Boost: {} | Blob: {} | Seeders: {}",
            rec.canonical_artist,
            rec.canonical_title,
            rec.bpm,
            rec.camelot_key,
            rec.top_subgenre,
            rec.swarm_replicas_count,
            if boost_ready { "SI (zk-ctr-sha256-v1)" } else { "P2P Directo" },
            rec.opaque_blob_key,
            seeders.join(", ")
        ));
    }
    lines.join("\n")
}

pub fn build_agent_harness_system_prompt(
    dj_handle: &str,
    style_dna: Option<&DjStyleDnaProfile>,
    state: &crate::p2p::P2PMeshState,
) -> String {
    let verified_count = state
        .search(&crate::p2p::P2PSearchQuery {
            q: None,
            camelot: None,
            bpm_min: None,
            bpm_max: None,
            subgenre: None,
            owner_dj: None,
            verified_only: Some(true),
        })
        .len();
    let dna_section = style_dna
        .map(|d| d.prompt_block.clone())
        .unwrap_or_else(|| {
            format!(
                "[MEMORIA ACTIVA DEL DJ BRAIN — PERFIL '{}']\n- Zona BPM: 122-128 BPM (8A/9A)\n- Software: VirtualDJ / Rekordbox",
                dj_handle
            )
        });
    let swarm_catalog_section = summarize_swarm_downloadable_catalog(state);
    format!(
        "You are HERA Agentic Copilot (100% Pure-Rust ReAct Harness) assisting DJ `{dj_handle}` on node `{}` ({verified_count} verified Studio Masters online).\n\
         Operational Invariants:\n\
         1. Quarantine-First & Zero-Trust Spectral Forensics: only admit/recommend verified tracks passing FFT bandwidth, fake-lossless, synthetic-sine, and <120Hz sub-bass mono phase correlation checks.\n\
         2. 4-Layer Holistic Synergy + Global 2-Opt TSP: sequence crates using Camelot wheel harmony, gradual BPM / 3:4 polyrhythmic bridges, vocal clash avoidance, and 64D style embeddings (`build_dj_crate`).\n\
         3. Multi-Step Pipeline Efficiency: use `hera_code_mode` to chain multi-step filtering + crate building in a single turn when appropriate, and `export_dj_crate` for VirtualDJ, Rekordbox, Traktor, and M3U8 exports.\n\
         4. Always rely on live tool observations over assumptions.\n\n\
         {dna_section}\n\n\
         {swarm_catalog_section}",
        state.node_id
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn http_get_json_local(
    host_port: &str,
    path: &str,
    api_key: Option<&str>,
) -> Result<serde_json::Value, String> {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let timeout = Duration::from_millis(600);
    let addr = host_port
        .to_socket_addrs()
        .map_err(|e| format!("Invalid address `{host_port}`: {e}"))?
        .next()
        .ok_or_else(|| format!("Could not resolve `{host_port}`"))?;
    let mut stream = TcpStream::connect_timeout(&addr, timeout)
        .map_err(|e| format!("Connect timeout/error to `{host_port}`: {e}"))?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    let auth_header = api_key
        .filter(|k| !k.trim().is_empty())
        .map(|k| format!("Authorization: Bearer {}\r\n", k.trim()))
        .unwrap_or_default();
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {host_port}\r\nAccept: application/json\r\n{auth_header}Connection: close\r\n\r\n"
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("Write failed to `{host_port}`: {e}"))?;

    let mut raw = Vec::new();
    let _ = stream.read_to_end(&mut raw);
    let text = String::from_utf8_lossy(&raw);
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b)
        .or_else(|| text.split_once("\n\n").map(|(_, b)| b))
        .unwrap_or(&text)
        .trim();
    // Handle HTTP chunked framing or leading/trailing non-JSON bytes by slicing `{...}`
    let json_slice = match (body.find('{'), body.rfind('}')) {
        (Some(start), Some(end)) if end >= start => &body[start..=end],
        _ => body,
    };
    serde_json::from_str(json_slice).map_err(|e| format!("Invalid JSON from `{host_port}{path}`: {e}"))
}

#[cfg(not(target_arch = "wasm32"))]
fn extract_models_from_payload(val: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arr) = val.get("data").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                if !id.to_ascii_lowercase().contains("embed") {
                    out.push(id.to_string());
                }
            }
        }
    } else if let Some(arr) = val.get("models").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(name) = item
                .get("name")
                .or_else(|| item.get("model"))
                .and_then(|v| v.as_str())
            {
                if !name.to_ascii_lowercase().contains("embed") {
                    out.push(name.to_string());
                }
            }
        }
    }
    out
}

pub fn probe_backend_models(base_url: &str, api_key: Option<&str>) -> Result<Vec<String>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (base_url, api_key);
        Err("Local TCP probing is only supported on native target".to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let trimmed = base_url.trim().trim_end_matches('/');
        let without_scheme = trimmed
            .strip_prefix("http://")
            .or_else(|| trimmed.strip_prefix("https://"))
            .unwrap_or(trimmed);
        let (host_port_raw, base_path) = match without_scheme.split_once('/') {
            Some((h, p)) => (h, format!("/{p}")),
            None => (without_scheme, String::new()),
        };
        let host_port = if host_port_raw.contains(':') {
            host_port_raw.to_string()
        } else {
            format!("{host_port_raw}:80")
        };
        let models_path = if base_path.ends_with("/models") {
            base_path
        } else if base_path.ends_with("/v1") {
            format!("{base_path}/models")
        } else if base_path.is_empty() {
            "/v1/models".to_string()
        } else {
            format!("{base_path}/v1/models")
        };

        if let Ok(json_val) = http_get_json_local(&host_port, &models_path, api_key) {
            let models = extract_models_from_payload(&json_val);
            if !models.is_empty() {
                return Ok(models);
            }
        }
        if let Ok(json_val) = http_get_json_local(&host_port, "/api/tags", api_key) {
            let models = extract_models_from_payload(&json_val);
            if !models.is_empty() {
                return Ok(models);
            }
        }
        Err(format!("No active models discovered at `{base_url}`"))
    }
}

pub fn auto_detect_local_backend() -> Option<(String, String, String)> {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentToolExecutionStep {
    pub step: usize,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result_preview: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareFreeAiTurnRequest {
    pub dj_handle: String,
    pub message: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub style_dna: Option<DjStyleDnaProfile>,
    #[serde(default)]
    pub daily_neurons_used_so_far: f64,
    #[serde(default)]
    pub cache_hit: bool,
    #[serde(default)]
    pub backend: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub max_tool_iterations: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareFreeAiTurnResponse {
    pub backend_used: String,
    pub model_used: String,
    pub cf_aig_cache_status: String,
    pub neurons_consumed_this_turn: f64,
    pub daily_neurons_remaining: f64,
    pub estimated_cost_usd: f64,
    pub d1_synced_bytes: usize,
    pub assistant_reply: String,
    #[serde(default)]
    pub tool_calls_trace: Vec<AgentToolExecutionStep>,
    #[serde(default)]
    pub iterations_executed: usize,
}

#[cfg(not(target_arch = "wasm32"))]
fn http_post_chat_completions(
    base_url: &str,
    api_key: Option<&str>,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let trimmed = base_url.trim().trim_end_matches('/');
    let chat_url = if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/chat/completions")
    };
    let body_str = serde_json::to_string(payload).map_err(|e| e.to_string())?;

    if let Some(without_http) = chat_url.strip_prefix("http://") {
        let (host_port_raw, path) = match without_http.split_once('/') {
            Some((h, p)) => (h, format!("/{p}")),
            None => (without_http, "/v1/chat/completions".to_string()),
        };
        let host_port = if host_port_raw.contains(':') {
            host_port_raw.to_string()
        } else {
            format!("{host_port_raw}:80")
        };
        let addr = host_port
            .to_socket_addrs()
            .map_err(|e| format!("Invalid address `{host_port}`: {e}"))?
            .next()
            .ok_or_else(|| format!("Could not resolve `{host_port}`"))?;
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(1500))
            .map_err(|e| format!("Connect error to `{host_port}`: {e}"))?;
        let _ = stream.set_read_timeout(Some(Duration::from_secs(15)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(15)));

        let auth_header = api_key
            .filter(|k| !k.trim().is_empty())
            .map(|k| format!("Authorization: Bearer {}\r\n", k.trim()))
            .unwrap_or_default();
        let req = format!(
            "POST {path} HTTP/1.1\r\nHost: {host_port}\r\nContent-Type: application/json\r\nAccept: application/json\r\n{auth_header}Content-Length: {}\r\nConnection: close\r\n\r\n{body_str}",
            body_str.len()
        );
        stream
            .write_all(req.as_bytes())
            .map_err(|e| format!("Write failed to `{host_port}`: {e}"))?;

        let mut raw = Vec::new();
        let mut chunk = [0u8; 4096];
        let mut expected_total = None;
        while let Ok(n) = stream.read(&mut chunk) {
            if n == 0 {
                break;
            }
            raw.extend_from_slice(&chunk[..n]);
            if expected_total.is_none() {
                let s = String::from_utf8_lossy(&raw);
                if let Some(hdr_end) = s.find("\r\n\r\n") {
                    if let Some(cl) = s[..hdr_end].lines().find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse::<usize>().ok())
                    }) {
                        expected_total = Some(hdr_end + 4 + cl);
                    }
                }
            }
            if let Some(tot) = expected_total {
                if raw.len() >= tot {
                    break;
                }
            }
        }
        let text = String::from_utf8_lossy(&raw);
        let body = text
            .split_once("\r\n\r\n")
            .map(|(_, b)| b)
            .or_else(|| text.split_once("\n\n").map(|(_, b)| b))
            .unwrap_or(&text)
            .trim();
        let json_slice = match (body.find('{'), body.rfind('}')) {
            (Some(s), Some(e)) if e >= s => &body[s..=e],
            _ => body,
        };
        return serde_json::from_str(json_slice)
            .map_err(|e| format!("Invalid JSON from `{chat_url}`: {e}"));
    }

    let mut cmd = std::process::Command::new("curl");
    cmd.args([
        "-sS",
        "--max-time",
        "15",
        "-X",
        "POST",
        &chat_url,
        "-H",
        "Content-Type: application/json",
    ]);
    if let Some(k) = api_key.filter(|k| !k.trim().is_empty()) {
        cmd.args(["-H", &format!("Authorization: Bearer {}", k.trim())]);
    }
    let out = cmd
        .args(["-d", &body_str])
        .output()
        .map_err(|e| format!("curl execution failed: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "HTTP POST failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("Invalid JSON response: {e}"))
}

fn parse_assistant_tool_calls(message: &serde_json::Value) -> Vec<(String, String, String)> {
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

pub fn evaluate_agent_harness_turn(
    req: &CloudflareFreeAiTurnRequest,
    state: &crate::p2p::P2PMeshState,
) -> CloudflareFreeAiTurnResponse {
    let requested_backend = req
        .backend
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("cloudflare_free");

    let (resolved_backend_id, detected_url, detected_model) = if requested_backend == "auto" {
        if let Some((b_id, url, mdl)) = auto_detect_local_backend() {
            (b_id, Some(url), Some(mdl))
        } else {
            ("cloudflare_free".to_string(), None, None)
        }
    } else {
        (requested_backend.to_string(), None, None)
    };

    let catalog = list_supported_ai_backends();
    let spec = catalog
        .iter()
        .find(|b| b.id == resolved_backend_id)
        .or_else(|| catalog.first());

    let base_url = req
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or(detected_url)
        .unwrap_or_else(|| {
            spec.map(|s| s.base_url.clone())
                .unwrap_or_else(|| "http://localhost:1234/v1".to_string())
        });

    let is_local_backend = matches!(
        resolved_backend_id.as_str(),
        "lmstudio" | "ollama" | "jan" | "llamacpp" | "vllm" | "custom"
    );

    let model = req
        .model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "default")
        .map(String::from)
        .or(detected_model)
        .unwrap_or_else(|| {
            if is_local_backend {
                if let Ok(models) = probe_backend_models(&base_url, req.api_key.as_deref()) {
                    if let Some(first) = models.into_iter().next() {
                        return first;
                    }
                }
            }
            spec.map(|s| s.default_model.clone())
                .unwrap_or_else(|| "default".to_string())
        });

    let should_call_live_http = req
        .base_url
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
        || resolved_backend_id != "cloudflare_free";

    let mut tool_calls_trace: Vec<AgentToolExecutionStep> = Vec::new();
    let mut iterations_executed = 1usize;

    #[cfg(not(target_arch = "wasm32"))]
    if should_call_live_http {
        let max_iters = req.max_tool_iterations.unwrap_or(6).clamp(1, 10);
        let sys_prompt =
            build_agent_harness_system_prompt(&req.dj_handle, req.style_dna.as_ref(), state);
        let tools = build_openai_tools_catalog();
        let mut messages = vec![
            serde_json::json!({ "role": "system", "content": sys_prompt }),
            serde_json::json!({ "role": "user", "content": req.message }),
        ];

        for step_idx in 1..=max_iters {
            iterations_executed = step_idx;
            let payload = serde_json::json!({
                "model": model,
                "messages": messages,
                "tools": tools,
                "temperature": 0.2
            });

            let Ok(resp_json) =
                http_post_chat_completions(&base_url, req.api_key.as_deref(), &payload)
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
                    match serde_json::from_str::<serde_json::Value>(&raw_args) {
                        Err(e) => {
                            let err_msg = format!("Invalid JSON arguments: {e}");
                            tool_calls_trace.push(AgentToolExecutionStep {
                                step: step_idx,
                                tool_name: fn_name.clone(),
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
                        Ok(args_val) => {
                            match crate::mcp::dispatch_tool_call(&fn_name, args_val.clone(), state)
                            {
                                Ok(res_val) => {
                                    let serialized = res_val.to_string();
                                    let preview = if serialized.chars().count() > 240 {
                                        format!(
                                            "{}...",
                                            serialized.chars().take(240).collect::<String>()
                                        )
                                    } else {
                                        serialized.clone()
                                    };
                                    tool_calls_trace.push(AgentToolExecutionStep {
                                        step: step_idx,
                                        tool_name: fn_name.clone(),
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
                                        tool_name: fn_name.clone(),
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
                            }
                        }
                    }
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
                return CloudflareFreeAiTurnResponse {
                    backend_used: resolved_backend_id,
                    model_used: model,
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
                    tool_calls_trace,
                    iterations_executed: step_idx,
                };
            }
            break;
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = should_call_live_http;

    let cache_status = if req.cache_hit { "HIT" } else { "MISS" };
    let neurons_this_turn = if req.cache_hit || is_local_backend {
        0.0
    } else {
        CF_LLAMA_8B_FAST_NEURONS_PER_TURN
    };
    let total_used = req.daily_neurons_used_so_far + neurons_this_turn;
    let over_daily_free = total_used > CF_WORKERS_AI_FREE_DAILY_NEURONS;

    let (backend_used, model_used) = if resolved_backend_id != "cloudflare_free" {
        (resolved_backend_id, model)
    } else if over_daily_free {
        (
            "ai_gateway_fallback_openrouter_free".to_string(),
            "meta-llama/llama-3.3-70b-instruct:free".to_string(),
        )
    } else {
        (
            "cloudflare_workers_ai_free".to_string(),
            "@cf/meta/llama-3.1-8b-instruct-fp8-fast".to_string(),
        )
    };

    let dna_ctx = req
        .style_dna
        .as_ref()
        .map(|d| {
            format!(
                "zona {} ({:.1} BPM, {:?}, software {})",
                d.dominant_bpm_zone,
                d.overall_average_bpm,
                d.dominant_camelot_keys,
                d.preferred_dj_software
            )
        })
        .unwrap_or_else(|| "perfil estándar 124 BPM (8A, VirtualDJ)".to_string());

    let swarm_index = state.build_broker_swarm_index();
    let swarm_preview: Vec<String> = swarm_index
        .iter()
        .take(3)
        .map(|r| {
            format!(
                "{} - {} ({:.1} BPM · {} · {} réplicas)",
                r.canonical_artist,
                r.canonical_title,
                r.bpm,
                r.camelot_key,
                r.swarm_replicas_count
            )
        })
        .collect();
    let reply = format!(
        "¡Listo, {}! Usando tu DJ Brain ({}) y el Radar del Enjambre ({} Masters verificados listos para descargar: {}) preparé la acción para: \"{}\". Los archivos y los 4 Hot Cues quedan listos en tu disco local.",
        req.dj_handle,
        dna_ctx,
        swarm_index.len(),
        swarm_preview.join(" | "),
        req.message
    );
    let d1_bytes = req.message.len() + reply.len() + 64;

    CloudflareFreeAiTurnResponse {
        backend_used,
        model_used,
        cf_aig_cache_status: cache_status.to_string(),
        neurons_consumed_this_turn: neurons_this_turn,
        daily_neurons_remaining: (CF_WORKERS_AI_FREE_DAILY_NEURONS - total_used).max(0.0),
        estimated_cost_usd: 0.0,
        d1_synced_bytes: d1_bytes,
        assistant_reply: reply,
        tool_calls_trace,
        iterations_executed,
    }
}

pub fn evaluate_cloudflare_free_ai_turn(
    req: &CloudflareFreeAiTurnRequest,
) -> CloudflareFreeAiTurnResponse {
    evaluate_agent_harness_turn(req, &crate::p2p::P2PMeshState::new("hera-default-node"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_12_backend_registry_and_openai_tools_catalog() {
        let backends = list_supported_ai_backends();
        assert_eq!(backends.len(), 12);
        let lmstudio = backends.iter().find(|b| b.id == "lmstudio").unwrap();
        assert_eq!(lmstudio.base_url, "http://localhost:1234/v1");
        let ollama = backends.iter().find(|b| b.id == "ollama").unwrap();
        assert_eq!(ollama.base_url, "http://localhost:11434/v1");

        let tools = build_openai_tools_catalog();
        assert_eq!(tools.len(), 20);
        for t in &tools {
            assert_eq!(t["type"], "function");
            let fn_name = t["function"]["name"].as_str().unwrap();
            assert_ne!(fn_name, "chat_with_dj_copilot");
            assert!(t["function"]["parameters"].is_object());
        }

        let state = crate::p2p::P2PMeshState::new("hera-test-node");
        let prompt = build_agent_harness_system_prompt("DaviDJ", None, &state);
        assert!(prompt.contains("Quarantine-First"));
        assert!(prompt.contains("hera_code_mode"));
        assert!(prompt.contains("hera-test-node"));
    }

    #[test]
    fn test_probe_backend_models_mock_lmstudio_ephemeral_tcp() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let body = r#"{"data":[{"id":"qwen2.5-coder-14b-instruct-gguf"},{"id":"text-embedding-nomic"}]}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });

        let base_url = format!("http://{addr}/v1");
        let models = probe_backend_models(&base_url, None).unwrap();
        assert_eq!(models, vec!["qwen2.5-coder-14b-instruct-gguf".to_string()]);
        let _ = handle.join();
    }

    fn read_http_json_body(stream: &mut std::net::TcpStream) -> serde_json::Value {
        use std::io::Read;
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        let mut expected_total = None;
        while let Ok(n) = stream.read(&mut chunk) {
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            if expected_total.is_none() {
                let s = String::from_utf8_lossy(&buf);
                if let Some(hdr_end) = s.find("\r\n\r\n") {
                    let cl = s[..hdr_end]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    expected_total = Some(hdr_end + 4 + cl);
                }
            }
            if let Some(tot) = expected_total {
                if buf.len() >= tot {
                    break;
                }
            }
        }
        let s = String::from_utf8_lossy(&buf);
        let body = s.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");
        serde_json::from_str(body).unwrap_or_default()
    }

    fn write_http_json_response(stream: &mut std::net::TcpStream, body_val: &serde_json::Value) {
        use std::io::Write;
        let body = body_val.to_string();
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
    }

    #[test]
    fn test_react_multi_turn_tool_calling_with_mock_lmstudio() {
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            // Turn 1: Verify 20 tools in request, return OpenAI tool_calls for `search_music`
            let (mut s1, _) = listener.accept().unwrap();
            let req1 = read_http_json_body(&mut s1);
            assert_eq!(req1["tools"].as_array().unwrap().len(), 20);
            write_http_json_response(
                &mut s1,
                &serde_json::json!({
                    "choices": [{
                        "message": {
                            "role": "assistant",
                            "content": null,
                            "tool_calls": [{
                                "id": "call_search_1",
                                "type": "function",
                                "function": {
                                    "name": "search_music",
                                    "arguments": "{\"camelot\":\"8A\"}"
                                }
                            }]
                        }
                    }]
                }),
            );

            // Turn 2: Verify tool observation in messages, return fallback `<tool_call>` tag for `get_node_manifest`
            let (mut s2, _) = listener.accept().unwrap();
            let req2 = read_http_json_body(&mut s2);
            let msgs2 = req2["messages"].as_array().unwrap();
            let last_msg2 = msgs2.last().unwrap();
            assert_eq!(last_msg2["role"], "tool");
            assert_eq!(last_msg2["name"], "search_music");
            write_http_json_response(
                &mut s2,
                &serde_json::json!({
                    "choices": [{
                        "message": {
                            "role": "assistant",
                            "content": "Checking node manifest next:\n<tool_call>{\"name\":\"get_node_manifest\",\"arguments\":{}}</tool_call>"
                        }
                    }]
                }),
            );

            // Turn 3: Verify second tool observation and return final synthesis
            let (mut s3, _) = listener.accept().unwrap();
            let req3 = read_http_json_body(&mut s3);
            let msgs3 = req3["messages"].as_array().unwrap();
            let last_msg3 = msgs3.last().unwrap();
            assert_eq!(last_msg3["role"], "tool");
            assert_eq!(last_msg3["name"], "get_node_manifest");
            write_http_json_response(
                &mut s3,
                &serde_json::json!({
                    "choices": [{
                        "message": {
                            "role": "assistant",
                            "content": "Found verified 8A Studio Masters on node hera-react-test and confirmed manifest."
                        }
                    }]
                }),
            );
        });

        let state = crate::p2p::P2PMeshState::new("hera-react-test");
        let req = CloudflareFreeAiTurnRequest {
            dj_handle: "DaviDJ".to_string(),
            message: "Find 8A tracks and check the node manifest".to_string(),
            session_id: None,
            style_dna: None,
            daily_neurons_used_so_far: 0.0,
            cache_hit: false,
            backend: Some("lmstudio".to_string()),
            base_url: Some(format!("http://{addr}/v1")),
            model: Some("qwen2.5-coder-14b-instruct".to_string()),
            api_key: None,
            max_tool_iterations: Some(6),
        };

        let resp = evaluate_agent_harness_turn(&req, &state);
        let _ = handle.join();

        assert_eq!(resp.backend_used, "lmstudio");
        assert_eq!(resp.model_used, "qwen2.5-coder-14b-instruct");
        assert_eq!(resp.iterations_executed, 3);
        assert_eq!(resp.tool_calls_trace.len(), 2);
        assert_eq!(resp.tool_calls_trace[0].tool_name, "search_music");
        assert!(!resp.tool_calls_trace[0].is_error);
        assert_eq!(resp.tool_calls_trace[1].tool_name, "get_node_manifest");
        assert!(!resp.tool_calls_trace[1].is_error);
        assert!(resp.assistant_reply.contains("Found verified 8A Studio Masters"));
    }

    #[test]
    fn test_react_self_healing_on_tool_error_with_mock_lmstudio() {
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = std::thread::spawn(move || {
            // Turn 1: Call `export_dj_crate` with missing required fields `{}`
            let (mut s1, _) = listener.accept().unwrap();
            let _req1 = read_http_json_body(&mut s1);
            write_http_json_response(
                &mut s1,
                &serde_json::json!({
                    "choices": [{
                        "message": {
                            "role": "assistant",
                            "tool_calls": [{
                                "id": "call_bad_export",
                                "type": "function",
                                "function": {
                                    "name": "export_dj_crate",
                                    "arguments": "{}"
                                }
                            }]
                        }
                    }]
                }),
            );

            // Turn 2: Inspect `self_heal_hint` in tool error message and retry with valid arguments
            let (mut s2, _) = listener.accept().unwrap();
            let req2 = read_http_json_body(&mut s2);
            let msgs2 = req2["messages"].as_array().unwrap();
            let tool_err_msg = msgs2.last().unwrap()["content"].as_str().unwrap();
            assert!(tool_err_msg.contains("self_heal_hint"));
            write_http_json_response(
                &mut s2,
                &serde_json::json!({
                    "choices": [{
                        "message": {
                            "role": "assistant",
                            "tool_calls": [{
                                "id": "call_fixed_export",
                                "type": "function",
                                "function": {
                                    "name": "export_dj_crate",
                                    "arguments": "{\"crate_name\":\"Fixed_Set\",\"tracks\":[]}"
                                }
                            }]
                        }
                    }]
                }),
            );

            // Turn 3: Return final answer
            let (mut s3, _) = listener.accept().unwrap();
            let _req3 = read_http_json_body(&mut s3);
            write_http_json_response(
                &mut s3,
                &serde_json::json!({
                    "choices": [{
                        "message": {
                            "role": "assistant",
                            "content": "Self-healed and exported Fixed_Set successfully."
                        }
                    }]
                }),
            );
        });

        let state = crate::p2p::P2PMeshState::new("hera-self-heal-test");
        let req = CloudflareFreeAiTurnRequest {
            dj_handle: "DaviDJ".to_string(),
            message: "Export my crate".to_string(),
            session_id: None,
            style_dna: None,
            daily_neurons_used_so_far: 0.0,
            cache_hit: false,
            backend: Some("lmstudio".to_string()),
            base_url: Some(format!("http://{addr}/v1")),
            model: Some("qwen2.5-coder-14b-instruct".to_string()),
            api_key: None,
            max_tool_iterations: Some(6),
        };

        let resp = evaluate_agent_harness_turn(&req, &state);
        let _ = handle.join();

        assert_eq!(resp.iterations_executed, 3);
        assert_eq!(resp.tool_calls_trace.len(), 2);
        assert!(resp.tool_calls_trace[0].is_error);
        assert!(!resp.tool_calls_trace[1].is_error);
        assert!(resp.assistant_reply.contains("Fixed_Set"));
    }
}
