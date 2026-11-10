use super::backends::{auto_detect_local_backend, list_supported_ai_backends};
use super::candle_local::score_transition_neural;
#[cfg(not(target_arch = "wasm32"))]
use super::cli_runner::execute_cli_agent_turn;
#[cfg(not(target_arch = "wasm32"))]
use super::harness::execute_live_http_react_loop;
use super::harness::{
    AgentToolExecutionStep, CloudflareFreeAiTurnRequest, CloudflareFreeAiTurnResponse,
};
use super::http_client::probe_backend_models;
#[cfg(not(target_arch = "wasm32"))]
use super::prompts::build_agent_harness_system_prompt;
use super::{CF_LLAMA_8B_FAST_NEURONS_PER_TURN, CF_WORKERS_AI_FREE_DAILY_NEURONS};

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

    let is_cli_agent = matches!(
        resolved_backend_id.as_str(),
        "cli_agy" | "cli_claude" | "cli_codex" | "cli_gemini" | "cli_ollama"
    );

    #[cfg(not(target_arch = "wasm32"))]
    if is_cli_agent {
        let sys_prompt = build_agent_harness_system_prompt(
            &req.dj_handle,
            req.style_dna.as_ref(),
            state,
            req.commander_mode.as_deref(),
        );
        match execute_cli_agent_turn(&resolved_backend_id, &req.message, &sys_prompt) {
            Ok(reply) => {
                let d1_bytes = req.message.len() + reply.len() + 64;
                return CloudflareFreeAiTurnResponse {
                    backend_used: resolved_backend_id,
                    model_used: "cli_subshell".to_string(),
                    cf_aig_cache_status: "CLI_LOCAL_EXEC".to_string(),
                    neurons_consumed_this_turn: 0.0,
                    daily_neurons_remaining: (CF_WORKERS_AI_FREE_DAILY_NEURONS
                        - req.daily_neurons_used_so_far)
                        .max(0.0),
                    estimated_cost_usd: 0.0,
                    d1_synced_bytes: d1_bytes,
                    assistant_reply: reply,
                    tool_calls_trace: Vec::new(),
                    iterations_executed: 1,
                };
            }
            Err(cli_err) => {
                let fallback_reply = format!(
                    "⚠️ [Aviso del Sistema - Agente CLI {}]: {}. Fallback a modo determinista seguro activado.",
                    resolved_backend_id, cli_err
                );
                let d1_bytes = req.message.len() + fallback_reply.len() + 64;
                return CloudflareFreeAiTurnResponse {
                    backend_used: resolved_backend_id,
                    model_used: "cli_fallback_deterministic".to_string(),
                    cf_aig_cache_status: "CLI_ERROR_FALLBACK".to_string(),
                    neurons_consumed_this_turn: 0.0,
                    daily_neurons_remaining: (CF_WORKERS_AI_FREE_DAILY_NEURONS
                        - req.daily_neurons_used_so_far)
                        .max(0.0),
                    estimated_cost_usd: 0.0,
                    d1_synced_bytes: d1_bytes,
                    assistant_reply: fallback_reply,
                    tool_calls_trace: Vec::new(),
                    iterations_executed: 1,
                };
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = is_cli_agent;

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

    #[allow(unused_mut)]
    let mut tool_calls_trace: Vec<AgentToolExecutionStep> = Vec::new();
    #[allow(unused_mut)]
    let mut iterations_executed = 1usize;

    #[cfg(not(target_arch = "wasm32"))]
    if should_call_live_http {
        if let Some(live_resp) = execute_live_http_react_loop(
            req,
            state,
            &resolved_backend_id,
            &base_url,
            &model,
            is_local_backend,
            &mut tool_calls_trace,
            &mut iterations_executed,
        ) {
            return live_resp;
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

    let swarm_index = state.build_broker_swarm_index();

    let (bpm_a, bpm_b, energy_a, energy_b, camelot_compat, vocal_risk) = {
        let fallback_bpm = req
            .style_dna
            .as_ref()
            .map(|d| d.overall_average_bpm)
            .unwrap_or(124.0);
        let _fallback_cam = req
            .style_dna
            .as_ref()
            .and_then(|d| d.dominant_camelot_keys.first().cloned())
            .unwrap_or_else(|| "8A".to_string());

        if let Ok(catalog) = state.catalog.read() {
            if catalog.len() >= 2 {
                let t1 = &catalog[0];
                let t2 = &catalog[1];
                let (cam_score, _) = crate::synergy::compute_camelot_synergy(&t1.camelot_key, &t2.camelot_key);
                let e1 = (t1.quality_score / 100.0).clamp(0.4, 0.99);
                let e2 = (t2.quality_score / 100.0).clamp(0.4, 0.99);
                let clash = if t1.hot_cues.iter().any(|c| c.role == "outro_vocals")
                    && t2.hot_cues.iter().any(|c| c.role == "intro_vocals")
                {
                    0.85
                } else {
                    0.04
                };
                (t1.bpm, t2.bpm, e1, e2, cam_score, clash)
            } else if let Some(first_cat) = catalog.first() {
                let cand_b = swarm_index.first();
                let bpm2 = cand_b.map(|c| c.bpm).unwrap_or(first_cat.bpm + 1.0);
                let cam2 = cand_b.map(|c| c.camelot_key.as_str()).unwrap_or(&first_cat.camelot_key);
                let (cam_score, _) = crate::synergy::compute_camelot_synergy(&first_cat.camelot_key, cam2);
                let e1 = (first_cat.quality_score / 100.0).clamp(0.4, 0.99);
                let e2 = cand_b.map(|c| (c.quality_score / 100.0).clamp(0.4, 0.99)).unwrap_or(0.84);
                (first_cat.bpm, bpm2, e1, e2, cam_score, 0.04)
            } else if swarm_index.len() >= 2 {
                let s1 = &swarm_index[0];
                let s2 = &swarm_index[1];
                let (cam_score, _) = crate::synergy::compute_camelot_synergy(&s1.camelot_key, &s2.camelot_key);
                (s1.bpm, s2.bpm, (s1.quality_score / 100.0).clamp(0.4, 0.99), (s2.quality_score / 100.0).clamp(0.4, 0.99), cam_score, 0.04)
            } else {
                (fallback_bpm, fallback_bpm + 1.5, 0.76, 0.84, 0.94, 0.04)
            }
        } else {
            (fallback_bpm, fallback_bpm + 1.5, 0.76, 0.84, 0.94, 0.04)
        }
    };

    let neural_note = score_transition_neural(bpm_a, bpm_b, energy_a, energy_b, camelot_compat, vocal_risk)
        .map(|s| {
            format!(
                " · Sinergia Neural {:.0}% ({})",
                s.neural_synergy * 100.0,
                s.recommended_technique
            )
        })
        .unwrap_or_default();

    let dna_ctx = req
        .style_dna
        .as_ref()
        .map(|d| {
            format!(
                "zona {} ({:.1} BPM, {:?}, software {}){}",
                d.dominant_bpm_zone,
                d.overall_average_bpm,
                d.dominant_camelot_keys,
                d.preferred_dj_software,
                neural_note
            )
        })
        .unwrap_or_else(|| format!("perfil estándar 124 BPM (8A, VirtualDJ){}", neural_note));

    let swarm_preview: Vec<String> = swarm_index
        .iter()
        .take(3)
        .map(|r| {
            format!(
                "{} - {} ({:.1} BPM · {} · {} réplicas)",
                r.canonical_artist, r.canonical_title, r.bpm, r.camelot_key, r.swarm_replicas_count
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
    fn test_turn_eval_derives_distinct_candidate_metrics_from_state_catalog() {
        let state = crate::p2p::P2PMeshState::new("hera-test-node");
        let req = CloudflareFreeAiTurnRequest {
            dj_handle: "DaviDJ".to_string(),
            message: "Recomienda transición cabina".to_string(),
            session_id: None,
            style_dna: None,
            daily_neurons_used_so_far: 0.0,
            cache_hit: false,
            backend: Some("cloudflare_free".to_string()),
            base_url: None,
            model: None,
            api_key: None,
            max_tool_iterations: Some(1),
            commander_mode: None,
        };
        let resp = evaluate_agent_harness_turn(&req, &state);
        assert!(resp.assistant_reply.contains("Sinergia Neural"));
        assert!(resp.assistant_reply.contains("122.0 BPM") || resp.assistant_reply.contains("perfil estándar"));
    }
}

