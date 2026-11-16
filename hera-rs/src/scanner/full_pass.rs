use crate::bpm_engine::v2_core::{
    consensus_ai::analyze_bpm_consensus_v2,
    hints::BpmHint,
    key_engine::detect_song_key,
    multiband_onset::{extract_multiband_novelty, MultibandNovelty},
};
use crate::cue_engine::{merge_and_place_8_hot_cues, CueCandidate, CueOrigin};
use crate::forensics_glitch::{scan_pcm_for_cd_skips, GlitchScanReport};
use crate::scanner::audio_metrics::{
    build_rgb_waveform_blob, compute_loudness_and_lead_in, quantize_embedding_64d,
    AudioLoudnessMetrics,
};
use crate::scanner::fast_seek::is_transition_candidate;
use rustfft::FftPlanner;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullHotCue {
    pub num: u8,
    pub name: String,
    pub seconds: f64,
    #[serde(default = "default_cue_role")]
    pub role: String,
    #[serde(default = "default_cue_origin")]
    pub origin: CueOrigin,
}

fn default_cue_role() -> String {
    "Structural".to_string()
}

fn default_cue_origin() -> CueOrigin {
    CueOrigin::HeraAuto
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullPassResult {
    pub bpm: f64,
    pub camelot_key: String,
    pub first_beat_seconds: f64,
    pub hot_cues: Vec<FullHotCue>,
    pub waveform_peaks_blob: Vec<u8>,
    pub entry_bpm: Option<f64>,
    pub exit_bpm: Option<f64>,
    pub is_transition_candidate: bool,
    pub has_intro_vocals: bool,
    pub has_outro_vocals: bool,
    pub embedding_64d_q8: Vec<i8>,
    pub rgb_waveform_blob: Vec<u8>,
    pub loudness: AudioLoudnessMetrics,
    pub glitch_report: GlitchScanReport,
}

pub fn build_waveform_peaks_blob(samples: &[f32], bins: usize) -> Vec<u8> {
    if samples.is_empty() || bins == 0 {
        return vec![0; bins * 2];
    }
    let mut out = Vec::with_capacity(bins * 2);
    let step = (samples.len() as f64) / (bins as f64);
    for b in 0..bins {
        let start = (b as f64 * step) as usize;
        let end = (((b + 1) as f64 * step) as usize)
            .min(samples.len())
            .max(start + 1);
        let mut min_v = 1.0_f32;
        let mut max_v = -1.0_f32;
        for &s in &samples[start..end] {
            if s < min_v {
                min_v = s;
            }
            if s > max_v {
                max_v = s;
            }
        }
        let min_q = (min_v.clamp(-1.0, 1.0) * 127.0).round() as i8;
        let max_q = (max_v.clamp(-1.0, 1.0) * 127.0).round() as i8;
        out.push(min_q as u8);
        out.push(max_q as u8);
    }
    out
}

fn slice_novelty(nov: &MultibandNovelty, start: usize, end: usize) -> MultibandNovelty {
    let s = start.min(nov.num_frames);
    let e = end.min(nov.num_frames).max(s);
    MultibandNovelty {
        sub_kick: nov.sub_kick[s..e].to_vec(),
        low_body: nov.low_body[s..e].to_vec(),
        perc_clave: nov.perc_clave[s..e].to_vec(),
        hi_guiro: nov.hi_guiro[s..e].to_vec(),
        composite: nov.composite[s..e].to_vec(),
        num_frames: e - s,
        sample_rate: nov.sample_rate,
        hop_size: nov.hop_size,
        onset_fps: nov.onset_fps,
        trim_offset_sec: nov.trim_offset_sec + (s as f64) / nov.onset_fps.max(1.0),
        band_ratios: nov.band_ratios,
    }
}

pub fn run_full_pass_from_pcm(
    samples: &[f32],
    sample_rate: u32,
    hint: Option<BpmHint>,
) -> FullPassResult {
    run_full_pass_from_pcm_with_manual_cues(samples, sample_rate, hint, &[])
}

pub fn run_full_pass_from_pcm_with_manual_cues(
    samples: &[f32],
    sample_rate: u32,
    hint: Option<BpmHint>,
    manual_cues: &[CueCandidate],
) -> FullPassResult {
    let mut planner = FftPlanner::<f32>::new();
    let nov = extract_multiband_novelty(samples, sample_rate, 0.0, &mut planner);
    let bpm_res = analyze_bpm_consensus_v2(&nov, hint);
    let key_res = detect_song_key(samples, sample_rate, &mut planner);

    let dur_sec = (samples.len() as f64) / (sample_rate.max(8_000) as f64);
    let first_beat = bpm_res.first_beat_sec.clamp(0.0, dur_sec.max(0.0));
    let loudness = compute_loudness_and_lead_in(samples, sample_rate);
    let rgb_waveform_blob = build_rgb_waveform_blob(samples, sample_rate, 256);
    let glitch_report = scan_pcm_for_cd_skips(samples, sample_rate, 0);

    let (entry_bpm, exit_bpm, is_trans) = if dur_sec >= 75.0 && nov.num_frames >= 256 {
        let q = nov.num_frames / 4;
        let nov_in = slice_novelty(&nov, 0, q);
        let nov_out = slice_novelty(&nov, nov.num_frames - q, nov.num_frames);
        let bpm_in = analyze_bpm_consensus_v2(&nov_in, hint).bpm;
        let bpm_out = analyze_bpm_consensus_v2(&nov_out, hint).bpm;
        if is_transition_candidate(bpm_in, bpm_out) {
            (Some(bpm_in), Some(bpm_out), true)
        } else {
            (None, None, false)
        }
    } else {
        (None, None, false)
    };

    let structure = crate::structure::analyze_structure_and_style(
        samples,
        sample_rate,
        Some(dur_sec as f32),
        Some(bpm_res.bpm as f32),
    );
    let base_4_cues: Vec<CueCandidate> = structure
        .hot_cues
        .into_iter()
        .take(4)
        .map(|c| CueCandidate {
            num: c.num,
            name: c.name,
            seconds: c.seconds as f64,
            role: c.role,
            origin: CueOrigin::HeraAuto,
        })
        .collect();

    let merged_cues = merge_and_place_8_hot_cues(
        samples,
        sample_rate,
        bpm_res.bpm,
        first_beat,
        loudness.first_audible_sec,
        &base_4_cues,
        manual_cues,
        None,
    );
    let hot_cues = merged_cues
        .into_iter()
        .map(|c| FullHotCue {
            num: c.num,
            name: c.name,
            seconds: c.seconds,
            role: c.role,
            origin: c.origin,
        })
        .collect();

    FullPassResult {
        bpm: bpm_res.bpm,
        camelot_key: key_res.camelot,
        first_beat_seconds: first_beat,
        hot_cues,
        waveform_peaks_blob: build_waveform_peaks_blob(samples, 256),
        entry_bpm,
        exit_bpm,
        is_transition_candidate: is_trans,
        has_intro_vocals: structure.has_intro_vocals,
        has_outro_vocals: structure.has_outro_vocals,
        embedding_64d_q8: quantize_embedding_64d(&structure.embedding_64d).to_vec(),
        rgb_waveform_blob,
        loudness,
        glitch_report,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_full_pass_on_file(
    path: &std::path::Path,
    hint: Option<BpmHint>,
    limiter: Option<&crate::bpm_engine::v2_core::audio_ingest::ProcessLimiter>,
) -> Result<FullPassResult, crate::bpm_engine::v2_core::types::EngineError> {
    run_full_pass_on_file_with_manual_cues(path, hint, &[], limiter)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_full_pass_on_file_with_manual_cues(
    path: &std::path::Path,
    hint: Option<BpmHint>,
    manual_cues: &[CueCandidate],
    limiter: Option<&crate::bpm_engine::v2_core::audio_ingest::ProcessLimiter>,
) -> Result<FullPassResult, crate::bpm_engine::v2_core::types::EngineError> {
    let dec = crate::bpm_engine::v2_core::audio_ingest::load_audio_mono_native_sr(path, limiter)?;
    Ok(run_full_pass_from_pcm_with_manual_cues(
        &dec.samples,
        dec.sample_rate,
        hint,
        manual_cues,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_pass_single_decode_produces_4_cues_and_512b_peaks() {
        let sr = 44_100_u32;
        let samples = vec![0.25_f32; (sr as usize) * 10];
        let res = run_full_pass_from_pcm(&samples, sr, None);
        assert_eq!(res.hot_cues.len(), 4);
        assert_eq!(res.waveform_peaks_blob.len(), 512);
        assert!(res.bpm >= 70.0 && res.bpm <= 190.0);
        assert!(!res.camelot_key.is_empty());
    }

    #[test]
    fn test_full_pass_zero_io_harvest_flags_cd_skips_and_8_cues() {
        let sr = 22_050_u32;
        let n = (sr as usize) * 36;
        let mut samples = Vec::with_capacity(n);
        for i in 0..n {
            let t = (i as f32) / (sr as f32);
            samples.push(0.55 * (2.0 * std::f32::consts::PI * 220.0 * t).sin());
        }
        let drop_start = (sr as usize) * 10;
        let drop_len = ((sr as f32) * 0.080) as usize;
        samples[drop_start - 1] = 0.75;
        samples[drop_start..drop_start + drop_len].fill(0.0);
        samples[drop_start + drop_len] = -0.75;

        let stutter_len = ((sr as f32) * 0.060) as usize;
        let stutter_pos = stutter_len * 80;
        for k in 0..stutter_len {
            let v = 0.65 * ((k as f32 * 0.37).sin() + 0.5 * (k as f32 * 0.11).cos());
            samples[stutter_pos + k] = v;
            samples[stutter_pos + stutter_len + k] = v;
        }

        let res = run_full_pass_from_pcm(&samples, sr, None);
        assert!(res.glitch_report.has_audio_skips);
        assert!(res.glitch_report.micro_dropout_count >= 1);
        assert!(res.glitch_report.buffer_repeat_stutter_count >= 1);
        assert_eq!(res.rgb_waveform_blob.len(), 768);
        assert_eq!(res.embedding_64d_q8.len(), 64);
        assert_eq!(res.hot_cues.len(), 8);
    }
}
