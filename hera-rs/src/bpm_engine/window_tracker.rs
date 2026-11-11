//! Sliding-Window Tempo Curve & Variable BPM Tracking using Viterbi continuity.

use super::TempoAnchorPoint;

#[derive(Debug, Clone)]
pub struct WindowTrackingResult {
    pub median_bpm: f32,
    pub entry_bpm: f32,
    pub exit_bpm: f32,
    pub min_bpm: f32,
    pub max_bpm: f32,
    pub drift_bpm: f32,
    pub drift_pct: f32,
    pub is_variable: bool,
    pub anchors: Vec<TempoAnchorPoint>,
}

pub fn track_windows_continuity(
    combined_novelty: &[f32],
    frame_dt: f32,
) -> WindowTrackingResult {
    let n = combined_novelty.len();
    if n < 64 || frame_dt <= 0.0 {
        return WindowTrackingResult {
            median_bpm: 124.0,
            entry_bpm: 124.0,
            exit_bpm: 124.0,
            min_bpm: 124.0,
            max_bpm: 124.0,
            drift_bpm: 0.0,
            drift_pct: 0.0,
            is_variable: false,
            anchors: Vec::new(),
        };
    }

    let win_len = ((12.0 / frame_dt).round() as usize).max(32);
    let hop_len = ((4.0 / frame_dt).round() as usize).max(8);

    // Compute window energies to filter out silent/rubato intros
    let mut window_energies = Vec::new();
    let mut start = 0;
    while start + win_len <= n {
        let energy: f32 = combined_novelty[start..start + win_len].iter().sum();
        window_energies.push(energy);
        start += hop_len;
    }

    let mut sorted_energies = window_energies.clone();
    sorted_energies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_energy = if !sorted_energies.is_empty() {
        sorted_energies[sorted_energies.len() / 2]
    } else {
        1.0
    };
    let min_energy_threshold = median_energy * 0.22;

    let min_lag = (((60.0 / 225.0) / frame_dt).floor() as usize).max(2);
    let max_lag = (((60.0 / 68.0) / frame_dt).ceil() as usize).min(win_len / 2);

    let mut raw_anchors: Vec<TempoAnchorPoint> = Vec::new();
    let mut start_idx = 0;
    let mut w_idx = 0;

    while start_idx + win_len <= n {
        let energy = window_energies.get(w_idx).copied().unwrap_or(0.0);
        let center_sec = (start_idx + win_len / 2) as f32 * frame_dt;
        w_idx += 1;

        if energy < min_energy_threshold {
            start_idx += hop_len;
            continue;
        }

        let slice = &combined_novelty[start_idx..start_idx + win_len];
        if let Some((bpm, conf)) = estimate_window_bpm(slice, frame_dt, min_lag, max_lag) {
            raw_anchors.push(TempoAnchorPoint {
                seconds: center_sec,
                local_bpm: bpm,
                confidence: conf,
            });
        }
        start_idx += hop_len;
    }

    if raw_anchors.is_empty() {
        return WindowTrackingResult {
            median_bpm: 124.0,
            entry_bpm: 124.0,
            exit_bpm: 124.0,
            min_bpm: 124.0,
            max_bpm: 124.0,
            drift_bpm: 0.0,
            drift_pct: 0.0,
            is_variable: false,
            anchors: Vec::new(),
        };
    }

    // Viterbi-like continuity smoothing across adjacent windows
    let mut smoothed_anchors = raw_anchors.clone();
    for i in 1..smoothed_anchors.len() {
        let prev = smoothed_anchors[i - 1].local_bpm;
        let curr = smoothed_anchors[i].local_bpm;
        // Check for octave jump (2x or 0.5x)
        if (curr * 0.5 - prev).abs() < (curr - prev).abs() {
            smoothed_anchors[i].local_bpm = curr * 0.5;
        } else if (curr * 2.0 - prev).abs() < (curr - prev).abs() {
            smoothed_anchors[i].local_bpm = curr * 2.0;
        }
    }

    let mut bpms: Vec<f32> = smoothed_anchors.iter().map(|a| a.local_bpm).collect();
    bpms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_bpm = bpms[bpms.len() / 2];

    let entry_slice_len = 3.min(smoothed_anchors.len());
    let mut entry_bpms: Vec<f32> = smoothed_anchors[..entry_slice_len]
        .iter()
        .map(|a| a.local_bpm)
        .collect();
    entry_bpms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let entry_bpm = entry_bpms[entry_bpms.len() / 2];

    let exit_slice_start = smoothed_anchors.len().saturating_sub(3);
    let mut exit_bpms: Vec<f32> = smoothed_anchors[exit_slice_start..]
        .iter()
        .map(|a| a.local_bpm)
        .collect();
    exit_bpms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let exit_bpm = exit_bpms[exit_bpms.len() / 2];

    let min_bpm = bpms[0];
    let max_bpm = bpms[bpms.len() - 1];
    let drift_bpm = (max_bpm - min_bpm).max(0.0);
    let drift_pct = if median_bpm > 0.0 {
        (drift_bpm / median_bpm) * 100.0
    } else {
        0.0
    };
    let is_variable = drift_bpm >= 2.0 || drift_pct >= 2.2;

    WindowTrackingResult {
        median_bpm,
        entry_bpm,
        exit_bpm,
        min_bpm,
        max_bpm,
        drift_bpm,
        drift_pct,
        is_variable,
        anchors: smoothed_anchors,
    }
}

fn estimate_window_bpm(
    slice: &[f32],
    frame_dt: f32,
    min_lag: usize,
    max_lag: usize,
) -> Option<(f32, f32)> {
    let n = slice.len();
    if max_lag <= min_lag || max_lag >= n {
        return None;
    }

    let norm: f64 = slice.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>().max(1e-9);
    let mut best_lag = min_lag;
    let mut best_val = -1.0_f32;
    let mut acf = vec![0.0_f32; max_lag + 2];

    for lag in min_lag..=max_lag {
        let mut sum = 0.0_f64;
        let count = n - lag;
        for i in 0..count {
            sum += (slice[i] as f64) * (slice[i + lag] as f64);
        }
        let val = ((sum * (n as f64 / count as f64)) / norm) as f32;
        acf[lag] = val;
        if val > best_val {
            best_val = val;
            best_lag = lag;
        }
    }

    if best_val <= 0.05 || best_lag <= min_lag || best_lag >= max_lag {
        return None;
    }

    // Sub-frame parabolic interpolation for continuous fractional precision
    let y0 = acf[best_lag - 1];
    let y1 = acf[best_lag];
    let y2 = acf[best_lag + 1];
    let denom = 2.0 * (2.0 * y1 - y0 - y2);
    let delta = if denom.abs() > 1e-6 {
        (y0 - y2) / denom
    } else {
        0.0
    };
    let precise_lag = (best_lag as f32) + delta.clamp(-0.5, 0.5);
    let bpm = 60.0 / (precise_lag * frame_dt);

    Some((bpm, best_val.clamp(0.0, 1.0)))
}
