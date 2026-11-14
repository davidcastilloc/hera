use crate::bpm_engine::v2_core::{
    consensus_ai::analyze_bpm_consensus_v2,
    hints::BpmHint,
    key_engine::detect_song_key,
    multiband_onset::extract_multiband_novelty,
};
use rustfft::FftPlanner;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullHotCue {
    pub num: u8,
    pub name: String,
    pub seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullPassResult {
    pub bpm: f64,
    pub camelot_key: String,
    pub first_beat_seconds: f64,
    pub hot_cues: Vec<FullHotCue>,
    pub waveform_peaks_blob: Vec<u8>,
}

pub fn build_waveform_peaks_blob(samples: &[f32], bins: usize) -> Vec<u8> {
    if samples.is_empty() || bins == 0 {
        return vec![0; bins * 2];
    }
    let mut out = Vec::with_capacity(bins * 2);
    let step = (samples.len() as f64) / (bins as f64);
    for b in 0..bins {
        let start = (b as f64 * step) as usize;
        let end = (((b + 1) as f64 * step) as usize).min(samples.len()).max(start + 1);
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

pub fn run_full_pass_from_pcm(
    samples: &[f32],
    sample_rate: u32,
    hint: Option<BpmHint>,
) -> FullPassResult {
    let mut planner = FftPlanner::<f32>::new();
    let nov = extract_multiband_novelty(samples, sample_rate, 0.0, &mut planner);
    let bpm_res = analyze_bpm_consensus_v2(&nov, hint);
    let key_res = detect_song_key(samples, sample_rate, &mut planner);

    let dur_sec = (samples.len() as f64) / (sample_rate.max(8_000) as f64);
    let bar_sec = (60.0 / bpm_res.bpm.max(60.0)) * 4.0;
    let first_beat = bpm_res.first_beat_sec.clamp(0.0, dur_sec.max(0.0));
    let phrase_in = (first_beat + bar_sec * 8.0).min((dur_sec * 0.35).max(first_beat));
    let main_drop = (first_beat + bar_sec * 16.0).min((dur_sec * 0.55).max(phrase_in));
    let mix_out = (dur_sec - bar_sec * 8.0).max(main_drop);

    let hot_cues = vec![
        FullHotCue { num: 1, name: "1.1.1 Downbeat".into(), seconds: first_beat },
        FullHotCue { num: 2, name: "Phrase In".into(), seconds: phrase_in },
        FullHotCue { num: 3, name: "Main Drop".into(), seconds: main_drop },
        FullHotCue { num: 4, name: "Mix Out".into(), seconds: mix_out },
    ];

    FullPassResult {
        bpm: bpm_res.bpm,
        camelot_key: key_res.camelot,
        first_beat_seconds: first_beat,
        hot_cues,
        waveform_peaks_blob: build_waveform_peaks_blob(samples, 256),
    }
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
}
