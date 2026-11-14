use crate::bpm_engine::v2_core::{
    consensus_ai::analyze_bpm_consensus_v2,
    hints::BpmHint,
    key_engine::detect_song_key,
    multiband_onset::extract_multiband_novelty,
};
use rustfft::FftPlanner;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastWindowPlan {
    pub use_full_file: bool,
    pub win_a_start_sec: f64,
    pub win_a_dur_sec: f64,
    pub win_b_start_sec: f64,
    pub win_b_dur_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastScanResult {
    pub bpm: f64,
    pub camelot_key: String,
    pub is_transition_candidate: bool,
    pub is_fake_320kbps: bool,
    pub bpm_window_a: f64,
    pub bpm_window_b: f64,
}

pub fn plan_fast_windows(total_duration_sec: f64) -> FastWindowPlan {
    if total_duration_sec <= 75.0 {
        return FastWindowPlan {
            use_full_file: true,
            win_a_start_sec: 0.0,
            win_a_dur_sec: total_duration_sec.max(0.0),
            win_b_start_sec: 0.0,
            win_b_dur_sec: 0.0,
        };
    }
    let start_a = (0.35 * total_duration_sec - 12.0).max(0.0);
    let start_b = (0.65 * total_duration_sec - 8.0).max(start_a + 24.0);
    FastWindowPlan {
        use_full_file: false,
        win_a_start_sec: start_a,
        win_a_dur_sec: 24.0,
        win_b_start_sec: start_b,
        win_b_dur_sec: 16.0,
    }
}

#[inline]
pub fn is_transition_candidate(bpm_a: f64, bpm_b: f64) -> bool {
    if (bpm_a - bpm_b).abs() <= 1.5 {
        return false;
    }
    let not_double_a = (bpm_a - bpm_b * 2.0).abs() > 2.0;
    let not_double_b = (bpm_b - bpm_a * 2.0).abs() > 2.0;
    not_double_a && not_double_b
}

#[inline]
pub fn is_fake_320kbps(effective_bandwidth_hz: f32, claimed_bitrate_kbps: u32) -> bool {
    claimed_bitrate_kbps >= 320 && effective_bandwidth_hz < 16_200.0
}

pub fn analyze_fast_windows(
    samples_a: &[f32],
    samples_b: &[f32],
    sample_rate: u32,
    hint: Option<BpmHint>,
    effective_bandwidth_hz: f32,
    claimed_bitrate_kbps: u32,
) -> FastScanResult {
    let mut planner = FftPlanner::<f32>::new();
    let nov_a = extract_multiband_novelty(samples_a, sample_rate, 0.0, &mut planner);
    let res_a = analyze_bpm_consensus_v2(&nov_a, hint);

    let (bpm_b, final_bpm, key_samples) = if samples_b.is_empty() {
        (res_a.bpm, res_a.bpm, samples_a.to_vec())
    } else {
        let nov_b = extract_multiband_novelty(samples_b, sample_rate, 0.0, &mut planner);
        let res_b = analyze_bpm_consensus_v2(&nov_b, hint);
        let mut merged = Vec::with_capacity(samples_a.len() + samples_b.len());
        merged.extend_from_slice(samples_a);
        merged.extend_from_slice(samples_b);
        let nov_m = extract_multiband_novelty(&merged, sample_rate, 0.0, &mut planner);
        let res_m = analyze_bpm_consensus_v2(&nov_m, hint);
        (res_b.bpm, res_m.bpm, merged)
    };

    let key = detect_song_key(&key_samples, sample_rate, &mut planner);
    FastScanResult {
        bpm: final_bpm,
        camelot_key: key.camelot,
        is_transition_candidate: !samples_b.is_empty()
            && is_transition_candidate(res_a.bpm, bpm_b),
        is_fake_320kbps: is_fake_320kbps(effective_bandwidth_hz, claimed_bitrate_kbps),
        bpm_window_a: res_a.bpm,
        bpm_window_b: bpm_b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_window_plan_and_forensic_flags() {
        let short = plan_fast_windows(60.0);
        assert!(short.use_full_file);

        let long = plan_fast_windows(200.0);
        assert!(!long.use_full_file);
        assert!((long.win_a_start_sec - 58.0).abs() < 1e-6);
        assert!((long.win_a_dur_sec - 24.0).abs() < 1e-6);
        assert!((long.win_b_start_sec - 122.0).abs() < 1e-6);
        assert!((long.win_b_dur_sec - 16.0).abs() < 1e-6);

        assert!(is_transition_candidate(100.0, 126.0));
        assert!(!is_transition_candidate(126.0, 126.5));
        assert!(!is_transition_candidate(85.0, 170.0));

        assert!(is_fake_320kbps(15_800.0, 320));
        assert!(!is_fake_320kbps(19_500.0, 320));
    }
}
