//! Zero-IO single-pass acoustic metrics: RGB waveform, K-weighted LUFS, 64D i8 & sub-phase.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct AudioLoudnessMetrics {
    pub loudness_lufs: f32,
    pub true_peak_dbfs: f32,
    pub replay_gain_db: f32,
    pub first_audible_sec: f64,
}

impl Default for AudioLoudnessMetrics {
    fn default() -> Self {
        Self {
            loudness_lufs: -70.0,
            true_peak_dbfs: -120.0,
            replay_gain_db: 0.0,
            first_audible_sec: 0.0,
        }
    }
}

/// Splits mono PCM into `[sub_u8, mid_u8, air_u8]` (`180 Hz` / `3.2 kHz` 1-pole IIR)
/// normalized against each band's 98th-percentile column RMS.
pub fn build_rgb_waveform_blob(samples: &[f32], sample_rate: u32, cols: usize) -> Vec<u8> {
    if cols == 0 {
        return Vec::new();
    }
    if samples.is_empty() || sample_rate == 0 {
        return vec![0_u8; cols * 3];
    }

    let sr = (sample_rate.max(8_000)) as f64;
    let alpha_sub = (1.0 - (-2.0 * PI * 180.0 / sr).exp()) as f32;
    let alpha_mid = (1.0 - (-2.0 * PI * 3_200.0 / sr).exp()) as f32;

    let mut sub_energy = vec![0.0_f32; cols];
    let mut mid_energy = vec![0.0_f32; cols];
    let mut air_energy = vec![0.0_f32; cols];

    let mut lp_sub = 0.0_f32;
    let mut lp_mid = 0.0_f32;
    let n = samples.len();

    for c in 0..cols {
        let start = c * n / cols;
        let end = (((c + 1) * n) / cols).max(start + 1).min(n);
        let mut s_acc = 0.0_f32;
        let mut m_acc = 0.0_f32;
        let mut a_acc = 0.0_f32;
        for &x in &samples[start..end] {
            lp_sub += alpha_sub * (x - lp_sub);
            lp_mid += alpha_mid * (x - lp_mid);
            let sub = lp_sub;
            let mid = lp_mid - lp_sub;
            let air = x - lp_mid;
            s_acc += sub * sub;
            m_acc += mid * mid;
            a_acc += air * air;
        }
        let inv = 1.0 / ((end - start) as f32);
        sub_energy[c] = (s_acc * inv).sqrt();
        mid_energy[c] = (m_acc * inv).sqrt();
        air_energy[c] = (a_acc * inv).sqrt();
    }

    let p98 = |vals: &[f32]| -> f32 {
        let mut sorted = vals.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = (((sorted.len() - 1) as f64 * 0.98).round() as usize).min(sorted.len() - 1);
        sorted[idx]
    };

    let ref_sub = p98(&sub_energy);
    let ref_mid = p98(&mid_energy);
    let ref_air = p98(&air_energy);

    let mut out = Vec::with_capacity(cols * 3);
    for c in 0..cols {
        let q = |v: f32, r: f32| -> u8 {
            if r < 1e-5 || v < 1e-5 {
                0
            } else {
                ((v / r).clamp(0.0, 1.0) * 255.0).round() as u8
            }
        };
        out.push(q(sub_energy[c], ref_sub));
        out.push(q(mid_energy[c], ref_mid));
        out.push(q(air_energy[c], ref_air));
    }
    out
}

/// Computes ITU-R BS.1770 K-weighted integrated loudness (`-70 LUFS` abs + `-10 LU` rel gates),
/// 2x inter-sample true peak, `-11.0 LUFS` anti-clip replay gain, and `> -42 dBFS` lead-in.
pub fn compute_loudness_and_lead_in(samples: &[f32], sample_rate: u32) -> AudioLoudnessMetrics {
    if samples.is_empty() || sample_rate == 0 {
        return AudioLoudnessMetrics::default();
    }
    let sr = sample_rate.max(8_000) as usize;
    let sr_f = sr as f64;

    // 1. Lead-in silence detection: first 20ms window exceeding -42.0 dBFS (0.00794328)
    let win_20ms = (sr / 50).max(16);
    let lead_thresh = 0.007_943_28_f32;
    let mut first_audible_sec = 0.0_f64;
    let mut found_audible = false;
    for (w_idx, chunk) in samples.chunks(win_20ms).enumerate() {
        let rms = (chunk.iter().map(|&s| s * s).sum::<f32>() / (chunk.len() as f32)).sqrt();
        if rms > lead_thresh {
            first_audible_sec = (w_idx * win_20ms) as f64 / sr_f;
            found_audible = true;
            break;
        }
    }

    // 2. True peak with 4-tap cubic / 2x midpoint inter-sample peak check
    let mut peak_abs = 0.0_f32;
    for i in 0..samples.len() {
        let a = samples[i].abs();
        if a > peak_abs {
            peak_abs = a;
        }
        if i + 1 < samples.len() {
            let mid_lin = (0.5 * (samples[i] + samples[i + 1])).abs();
            if mid_lin > peak_abs {
                peak_abs = mid_lin;
            }
            if i >= 1 && i + 2 < samples.len() {
                let catmull =
                    (-0.0625 * samples[i - 1] + 0.5625 * samples[i] + 0.5625 * samples[i + 1]
                        - 0.0625 * samples[i + 2])
                        .abs();
                if catmull > peak_abs {
                    peak_abs = catmull;
                }
            }
        }
    }
    if !found_audible && peak_abs <= 1e-5 {
        return AudioLoudnessMetrics {
            loudness_lufs: -70.0,
            true_peak_dbfs: 20.0 * peak_abs.max(1e-6).log10(),
            replay_gain_db: 0.0,
            first_audible_sec: 0.0,
        };
    }

    // 3. K-weighting: ~38 Hz RLB high-pass + ~1.5 kHz +4 dB high-shelf (`10^(4/20) - 1 = 0.58489`)
    let rc_hp = 1.0 / (2.0 * PI * 38.0);
    let dt = 1.0 / sr_f;
    let alpha_hp = (rc_hp / (rc_hp + dt)) as f32;
    let alpha_hs = (1.0 - (-2.0 * PI * 1_500.0 / sr_f).exp()) as f32;
    let shelf_gain = 0.584_893_2_f32;

    let block_len = ((sr * 4) / 10).max(64); // 400 ms
    let hop_len = (block_len / 4).max(16); // 100 ms (75% overlap per BS.1770)
    let mut k_sq = Vec::with_capacity(samples.len());
    let mut prev_x = 0.0_f32;
    let mut prev_hp = 0.0_f32;
    let mut lp_hs = 0.0_f32;
    for &x in samples {
        let hp = alpha_hp * (prev_hp + x - prev_x);
        prev_x = x;
        prev_hp = hp;
        lp_hs += alpha_hs * (hp - lp_hs);
        let y = hp + shelf_gain * (hp - lp_hs);
        k_sq.push((y as f64) * (y as f64));
    }

    let mut block_energies = Vec::new();
    if k_sq.len() <= block_len {
        let mean_sq = k_sq.iter().sum::<f64>() / (k_sq.len() as f64);
        block_energies.push(mean_sq);
    } else {
        let mut pos = 0;
        while pos + block_len <= k_sq.len() {
            let mean_sq = k_sq[pos..pos + block_len].iter().sum::<f64>() / (block_len as f64);
            block_energies.push(mean_sq);
            pos += hop_len;
        }
    }

    // Absolute gate at -70 LUFS: -0.691 + 10*log10(z) > -70 => z > 10^((-70 + 0.691)/10)
    let abs_gate_z = 10.0_f64.powf((-70.0 + 0.691) / 10.0);
    let abs_passed: Vec<f64> = block_energies
        .into_iter()
        .filter(|&z| z > abs_gate_z)
        .collect();

    let loudness_lufs = if abs_passed.is_empty() {
        -70.0_f32
    } else {
        let mean_abs = abs_passed.iter().sum::<f64>() / (abs_passed.len() as f64);
        let rel_gate_z = mean_abs * 0.1; // -10 LU relative gate
        let rel_passed: Vec<f64> = abs_passed
            .into_iter()
            .filter(|&z| z >= rel_gate_z)
            .collect();
        let mean_z = if rel_passed.is_empty() {
            mean_abs
        } else {
            rel_passed.iter().sum::<f64>() / (rel_passed.len() as f64)
        };
        (-0.691 + 10.0 * mean_z.max(1e-12).log10()).clamp(-70.0, 0.0) as f32
    };

    let peak_lin = peak_abs.max(1e-6);
    let true_peak_dbfs = 20.0 * peak_lin.log10();
    let replay_gain_db = if loudness_lufs <= -69.5 {
        0.0
    } else {
        let desired = (-11.0 - loudness_lufs).clamp(-9.0, 6.0);
        let max_no_clip_db = -0.3 - true_peak_dbfs;
        desired.min(max_no_clip_db).clamp(-18.0, 6.0)
    };

    AudioLoudnessMetrics {
        loudness_lufs,
        true_peak_dbfs,
        replay_gain_db,
        first_audible_sec,
    }
}

/// L2-normalizes up to 64 dimensions and quantizes against max abs component into `[-127, 127]`.
pub fn quantize_embedding_64d(vec64: &[f32]) -> [i8; 64] {
    let mut padded = [0.0_f32; 64];
    for (i, &v) in vec64.iter().take(64).enumerate() {
        if v.is_finite() {
            padded[i] = v;
        }
    }
    let norm = padded.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm <= 1e-12 {
        return [0; 64];
    }
    for v in &mut padded {
        *v /= norm;
    }
    let max_abs = padded.iter().map(|v| v.abs()).fold(0.0_f32, f32::max);
    if max_abs <= 1e-12 {
        return [0; 64];
    }
    let mut out = [0_i8; 64];
    for i in 0..64 {
        out[i] = ((padded[i] / max_abs) * 127.0).round().clamp(-127.0, 127.0) as i8;
    }
    out
}

/// Fast `< 1 us` integer dot-product cosine similarity in `[-1.0, 1.0]`.
pub fn cosine_similarity_i8(a: &[i8; 64], b: &[i8; 64]) -> f32 {
    let mut dot = 0_i32;
    let mut na = 0_i32;
    let mut nb = 0_i32;
    for i in 0..64 {
        let ai = i32::from(a[i]);
        let bi = i32::from(b[i]);
        dot += ai * bi;
        na += ai * ai;
        nb += bi * bi;
    }
    if na == 0 || nb == 0 {
        return 0.0;
    }
    ((dot as f64) / ((na as f64).sqrt() * (nb as f64).sqrt())).clamp(-1.0, 1.0) as f32
}

/// Low-passes L/R below `120 Hz` and returns Pearson phase correlation (`< 0.25` = Club Mono risk).
pub fn check_stereo_sub_mono_phase(interleaved_lr: &[f32], sample_rate: u32) -> f32 {
    if interleaved_lr.len() < 4 || sample_rate == 0 {
        return 1.0;
    }
    let sr = (sample_rate.max(8_000)) as f64;
    let alpha = 1.0 - (-2.0 * PI * 120.0 / sr).exp();
    let mut lp_l = 0.0_f64;
    let mut lp_r = 0.0_f64;
    let mut sum_ll = 0.0_f64;
    let mut sum_rr = 0.0_f64;
    let mut sum_lr = 0.0_f64;
    let mut frames = 0_usize;

    for lr in interleaved_lr.chunks_exact(2) {
        lp_l += alpha * ((lr[0] as f64) - lp_l);
        lp_r += alpha * ((lr[1] as f64) - lp_r);
        sum_ll += lp_l * lp_l;
        sum_rr += lp_r * lp_r;
        sum_lr += lp_l * lp_r;
        frames += 1;
    }
    if frames < 2 {
        return 1.0;
    }
    let denom = (sum_ll * sum_rr).sqrt();
    let mean_e = (sum_ll + sum_rr) / (2.0 * frames as f64);
    if mean_e < 1e-6 || denom <= 1e-12 {
        return 1.0;
    }
    (sum_lr / denom).clamp(-1.0, 1.0) as f32
}

#[cfg(test)]
#[path = "audio_metrics_tests.rs"]
mod tests;
