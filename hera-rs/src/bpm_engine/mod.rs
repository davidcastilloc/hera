pub mod consensus_ai;
pub mod latin_heuristics;
pub mod multiband_onset;
pub mod v2_core;
pub mod window_tracker;

use serde::{Deserialize, Serialize};

pub use consensus_ai::arbitrate_bpm_consensus;
pub use latin_heuristics::evaluate_latin_heuristics;
pub use multiband_onset::{decompose_multiband_onsets, MultibandNovelty};
pub use window_tracker::{track_windows_continuity, WindowTrackingResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TempoAnchorPoint {
    pub seconds: f32,
    pub local_bpm: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ExternalBpmHint {
    pub bpm: f32,
    pub genre: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BpmAnalysisReport {
    pub bpm: f32,
    pub entry_bpm: f32,
    pub exit_bpm: f32,
    pub bpm_min_detected: f32,
    pub bpm_max_detected: f32,
    pub tempo_drift_bpm: f32,
    pub tempo_drift_pct: f32,
    pub window_iqr_bpm: f32,
    pub is_variable_bpm: bool,
    pub bpm_confidence: f32,
    pub bpm_needs_review: bool,
    pub detected_rhythm_family: String,
    pub tempo_stability_label: String,
    pub tempo_map_points: Vec<TempoAnchorPoint>,
}

/// 5-Layer Multi-Strategy BPM Consensus Analyzer (Salsa, Tecnomerengue, Variable BPM & Polyrhythms).
pub fn analyze_bpm_consensus(
    mono: &[f32],
    sample_rate: u32,
    external_hint: Option<ExternalBpmHint>,
) -> BpmAnalysisReport {
    if mono.len() < 2048 {
        let fallback_bpm = external_hint.map(|h| h.bpm).unwrap_or(124.0);
        return BpmAnalysisReport {
            bpm: fallback_bpm,
            entry_bpm: fallback_bpm,
            exit_bpm: fallback_bpm,
            bpm_min_detected: fallback_bpm,
            bpm_max_detected: fallback_bpm,
            tempo_drift_bpm: 0.0,
            tempo_drift_pct: 0.0,
            window_iqr_bpm: 0.0,
            is_variable_bpm: false,
            bpm_confidence: 0.3,
            bpm_needs_review: true,
            detected_rhythm_family: "ShortSample".to_string(),
            tempo_stability_label: "Unknown".to_string(),
            tempo_map_points: Vec::new(),
        };
    }

    // Layer 1: 4-Band Onset Decomposition
    let novelty = decompose_multiband_onsets(mono, sample_rate);
    let n = novelty.sub_kick.len();

    let sub_sum: f32 = novelty.sub_kick.iter().sum();
    let low_sum: f32 = novelty.low_body.iter().sum();
    let clave_sum: f32 = novelty.perc_clave.iter().sum();
    let guiro_sum: f32 = novelty.hi_guiro.iter().sum();
    let total = (sub_sum + low_sum + clave_sum + guiro_sum).max(1e-6);

    let is_latin_signature = (clave_sum + guiro_sum + low_sum) / total > 0.65;
    let (w_sub, w_low, w_clave, w_guiro) = if is_latin_signature {
        (0.25_f32, 0.85_f32, 1.55_f32, 0.65_f32)
    } else {
        (1.20_f32, 0.60_f32, 0.50_f32, 0.35_f32)
    };

    let mut combined_novelty = vec![0.0_f32; n];
    for (i, val) in combined_novelty.iter_mut().enumerate() {
        *val = novelty.sub_kick[i] * w_sub
            + novelty.low_body[i] * w_low
            + novelty.perc_clave[i] * w_clave
            + novelty.hi_guiro[i] * w_guiro;
    }

    // Layer 2: Sliding-Window Tempo Curve & Variable BPM Tracking
    let window_res = track_windows_continuity(&combined_novelty, novelty.frame_dt);

    // Layer 3: Latin Rhythm Heuristics (Salsa & Tecnomerengue de-aliasing)
    let latin_res = evaluate_latin_heuristics(&novelty, window_res.median_bpm);

    // Scale entry & exit BPM if Latin heuristic resolved an octave / polyrhythmic alias
    let scale_ratio = if window_res.median_bpm > 0.0 {
        latin_res.candidate_bpm / window_res.median_bpm
    } else {
        1.0
    };
    let scaled_entry = window_res.entry_bpm * scale_ratio;
    let scaled_exit = window_res.exit_bpm * scale_ratio;
    let scaled_min = window_res.min_bpm * scale_ratio;
    let scaled_max = window_res.max_bpm * scale_ratio;
    let scaled_drift = (scaled_max - scaled_min).max(0.0);
    let scaled_drift_pct = if latin_res.candidate_bpm > 0.0 {
        (scaled_drift / latin_res.candidate_bpm) * 100.0
    } else {
        0.0
    };

    // Layer 4: Consensus Arbiter (Mixxx QM-DSP + Beatgrid Phase Lock + Latin Matrix v2)
    let nov_v2 = v2_core::multiband_onset::extract_multiband_novelty(mono, sample_rate);
    let hint_v2 = external_hint.as_ref().map(|h| v2_core::hints::BpmHint {
        bpm: h.bpm as f64,
        source: v2_core::hints::HintSource::UserProvided,
        weight: h.confidence.clamp(0.0, 1.0) as f64,
    });
    let res_v2 = v2_core::consensus_ai::analyze_bpm_consensus_v2(&nov_v2, hint_v2);
    let final_bpm = res_v2.final_bpm as f32;
    let family_label = format!("{:?}", res_v2.groove_family);
    let v2_confidence = res_v2.confidence as f32;

    let is_variable = window_res.is_variable || scaled_drift >= 2.0 || scaled_drift_pct >= 2.2;
    let stability_label = if is_variable {
        "VariableTempo".to_string()
    } else {
        "ConstantGrid".to_string()
    };

    let scaled_anchors: Vec<TempoAnchorPoint> = window_res
        .anchors
        .into_iter()
        .map(|mut a| {
            a.local_bpm *= scale_ratio;
            a
        })
        .collect();

    BpmAnalysisReport {
        bpm: final_bpm,
        entry_bpm: scaled_entry,
        exit_bpm: scaled_exit,
        bpm_min_detected: scaled_min,
        bpm_max_detected: scaled_max,
        tempo_drift_bpm: scaled_drift,
        tempo_drift_pct: scaled_drift_pct,
        window_iqr_bpm: 0.5,
        is_variable_bpm: is_variable,
        bpm_confidence: v2_confidence,
        bpm_needs_review: is_variable && v2_confidence < 0.85,
        detected_rhythm_family: family_label,
        tempo_stability_label: stability_label,
        tempo_map_points: scaled_anchors,
    }
}
