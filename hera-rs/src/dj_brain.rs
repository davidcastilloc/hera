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
    format!(
        "You are HERA Agentic Copilot (100% Pure-Rust ReAct Harness) assisting DJ `{dj_handle}` on node `{}` ({verified_count} verified Studio Masters online).\n\
         Operational Invariants:\n\
         1. Quarantine-First & Zero-Trust Spectral Forensics: only admit/recommend verified tracks passing FFT bandwidth, fake-lossless, synthetic-sine, and <120Hz sub-bass mono phase correlation checks.\n\
         2. 4-Layer Holistic Synergy + Global 2-Opt TSP: sequence crates using Camelot wheel harmony, gradual BPM / 3:4 polyrhythmic bridges, vocal clash avoidance, and 64D style embeddings (`build_dj_crate`).\n\
         3. Multi-Step Pipeline Efficiency: use `hera_code_mode` to chain multi-step filtering + crate building in a single turn when appropriate, and `export_dj_crate` for VirtualDJ, Rekordbox, Traktor, and M3U8 exports.\n\
         4. Always rely on live tool observations over assumptions.\n\n\
         {dna_section}",
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareFreeAiTurnRequest {
    pub dj_handle: String,
    pub message: String,
    pub session_id: Option<String>,
    pub style_dna: Option<DjStyleDnaProfile>,
    #[serde(default)]
    pub daily_neurons_used_so_far: f64,
    #[serde(default)]
    pub cache_hit: bool,
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
}

pub fn evaluate_cloudflare_free_ai_turn(
    req: &CloudflareFreeAiTurnRequest,
) -> CloudflareFreeAiTurnResponse {
    let cache_status = if req.cache_hit { "HIT" } else { "MISS" };
    let neurons_this_turn = if req.cache_hit {
        0.0
    } else {
        CF_LLAMA_8B_FAST_NEURONS_PER_TURN
    };
    let total_used = req.daily_neurons_used_so_far + neurons_this_turn;
    let over_daily_free = total_used > CF_WORKERS_AI_FREE_DAILY_NEURONS;

    let (backend_used, model_used) = if over_daily_free {
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

    let reply = format!(
        "¡Listo, {}! Usando tu DJ Brain ({}) preparé la acción para: \"{}\". Los archivos y los 4 Hot Cues quedan listos en tu disco local.",
        req.dj_handle, dna_ctx, req.message
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
    }
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
}
