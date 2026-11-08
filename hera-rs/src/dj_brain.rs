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
    vec![
        AiBackendOption {
            id: "cloudflare_free".to_string(),
            display_name: "⚡ Hera Cloud Free (Cloudflare Workers AI + AI Gateway)".to_string(),
            default_model: "@cf/meta/llama-3.1-8b-instruct-fp8-fast".to_string(),
            base_url: "https://cloud.hera.audio/api/v1/ai/v1".to_string(),
            requires_api_key: false,
            is_default_free: true,
            zero_laptop_cpu_ram: true,
        },
        AiBackendOption {
            id: "openrouter".to_string(),
            display_name: "🌐 OpenRouter (:free & Pro Models)".to_string(),
            default_model: "meta-llama/llama-3.3-70b-instruct:free".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            requires_api_key: true,
            is_default_free: false,
            zero_laptop_cpu_ram: true,
        },
        AiBackendOption {
            id: "gemini".to_string(),
            display_name: "✨ Google Gemini API".to_string(),
            default_model: "gemini-2.5-flash".to_string(),
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
            requires_api_key: true,
            is_default_free: false,
            zero_laptop_cpu_ram: true,
        },
        AiBackendOption {
            id: "openai".to_string(),
            display_name: "🧠 OpenAI".to_string(),
            default_model: "gpt-4o".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            requires_api_key: true,
            is_default_free: false,
            zero_laptop_cpu_ram: true,
        },
        AiBackendOption {
            id: "ollama".to_string(),
            display_name: "💻 Ollama (Local Optional)".to_string(),
            default_model: "llama3.3".to_string(),
            base_url: "http://localhost:11434/v1".to_string(),
            requires_api_key: false,
            is_default_free: false,
            zero_laptop_cpu_ram: false,
        },
    ]
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
