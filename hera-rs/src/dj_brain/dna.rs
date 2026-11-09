use crate::export::ExportTrackItem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AnchorArtistStat {
    pub artist: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub track_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MediaMixProfile {
    pub audio_pct: f32,
    pub video_pct: f32,
    pub own_productions_pct: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DjStyleDnaRequest {
    pub tracks: Vec<ExportTrackItem>,
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub dj_handle: Option<String>,
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub preferred_dj_software: Option<String>,
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub golden_rules: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DjStyleDnaProfile {
    pub dj_handle: String,
    pub preferred_dj_software: String,
    pub golden_rules: String,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
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
