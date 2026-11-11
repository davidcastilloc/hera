use rustfft::{num_complex::Complex, FftPlanner};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct HotCuePoint {
    pub num: u8,
    pub name: String,
    pub seconds: f32,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub bar_index: usize,
    pub role: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralCueProfile {
    pub bpm: f32,
    pub camelot_key: String,
    pub musical_key: String,
    pub energy_score: f32,
    pub punch_index: f32,
    pub groove_type: String,
    pub top_subgenre: String,
    pub first_beat_seconds: f32,
    pub phrase_in_seconds: f32,
    pub main_drop_seconds: f32,
    pub mix_out_seconds: f32,
    pub has_intro_vocals: bool,
    pub has_outro_vocals: bool,
    pub intro_vocal_ratio: f32,
    pub outro_vocal_ratio: f32,
    pub hot_cues: Vec<HotCuePoint>,
    pub embedding_64d: Vec<f32>,
}

const MAJOR_PROFILE: [f32; 12] = [
    6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
];
const MINOR_PROFILE: [f32; 12] = [
    6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
];

const PITCH_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const CAMELOT_MAJOR: [&str; 12] = [
    "8B", "3B", "10B", "5B", "12B", "7B", "2B", "9B", "4B", "11B", "6B", "1B",
];
const CAMELOT_MINOR: [&str; 12] = [
    "5A", "12A", "7A", "2A", "9A", "4A", "11A", "6A", "1A", "8A", "3A", "10A",
];

/// Analyze PCM mono samples to extract BPM, Camelot Key, 4 Acoustic Hot Cues, Vocal Activity, and 64D Style Embedding.
pub fn analyze_structure_and_style(
    mono: &[f32],
    sample_rate: u32,
    full_duration_seconds: Option<f32>,
    bpm_hint: Option<f32>,
) -> StructuralCueProfile {
    let decoded_dur = (mono.len() as f32) / (sample_rate.max(1) as f32);
    let total_dur = full_duration_seconds
        .unwrap_or(decoded_dur)
        .max(decoded_dur);

    let n_fft = 2048usize;
    let hop = 512usize;
    let n_bins = n_fft / 2 + 1;
    let frame_dt = (hop as f32) / (sample_rate.max(1) as f32);
    let bin_hz = (sample_rate as f32) / (n_fft as f32);

    if mono.len() < n_fft * 2 {
        let bpm = bpm_hint.unwrap_or(124.0);
        return fallback_structure(bpm, total_dur);
    }

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n_fft);
    let hann: Vec<f32> = (0..n_fft)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * (i as f32) / (n_fft as f32)).cos()))
        .collect();

    let n_frames = (mono.len() - n_fft) / hop + 1;
    let mut sub_env = Vec::with_capacity(n_frames);
    let mut mid_env = Vec::with_capacity(n_frames);
    let mut high_env = Vec::with_capacity(n_frames);
    let mut sub_flux = Vec::with_capacity(n_frames);
    let mut mid_flux = Vec::with_capacity(n_frames);
    let mut chroma_accum = [0.0_f64; 12];
    let mut band_energy_32 = [0.0_f64; 32];
    let mut band_sq_32 = [0.0_f64; 32];

    let mut prev_sub = 0.0_f32;
    let mut prev_mid = 0.0_f32;
    let mut buf = vec![
        Complex {
            re: 0.0_f32,
            im: 0.0_f32
        };
        n_fft
    ];

    for frame_idx in 0..n_frames {
        let start = frame_idx * hop;
        for i in 0..n_fft {
            buf[i] = Complex {
                re: mono[start + i] * hann[i],
                im: 0.0,
            };
        }
        fft.process(&mut buf);

        let mut s_e = 0.0_f32;
        let mut m_e = 0.0_f32;
        let mut h_e = 0.0_f32;

        for (b, c) in buf.iter().enumerate().take(n_bins).skip(1) {
            let f = (b as f32) * bin_hz;
            let mag = (c.re * c.re + c.im * c.im).sqrt();

            if (20.0..=135.0).contains(&f) {
                s_e += mag;
            } else if (300.0..=3400.0).contains(&f) {
                m_e += mag;
            } else if (4000.0..=12000.0).contains(&f) {
                h_e += mag;
            }

            // 32 pseudo-Mel bands for 64D Style Embedding
            let band_idx = (((f / (sample_rate as f32 * 0.5)).sqrt() * 32.0) as usize).min(31);
            let mag64 = mag as f64;
            band_energy_32[band_idx] += mag64;
            band_sq_32[band_idx] += mag64 * mag64;
        }

        // Parabolic Peak-Interpolated Harmonic Chroma Accumulation (165 Hz - 2200 Hz)
        // Eliminates coarse FFT bin quantization bias (which previously skewed toward 3A/12A)
        for b in 2..(n_bins.saturating_sub(2)) {
            let f_center = (b as f32) * bin_hz;
            if !(165.0..=2200.0).contains(&f_center) {
                continue;
            }
            let m_prev = (buf[b - 1].re * buf[b - 1].re + buf[b - 1].im * buf[b - 1].im).sqrt();
            let m_curr = (buf[b].re * buf[b].re + buf[b].im * buf[b].im).sqrt();
            let m_next = (buf[b + 1].re * buf[b + 1].re + buf[b + 1].im * buf[b + 1].im).sqrt();

            // Only accumulate local tonal spectral peaks above neighborhood floor
            if m_curr > m_prev && m_curr >= m_next && m_curr > 1e-4 {
                let denom = m_prev - 2.0 * m_curr + m_next;
                let delta = if denom.abs() > 1e-9 {
                    (0.5 * (m_prev - m_next) / denom).clamp(-0.5, 0.5)
                } else {
                    0.0
                };
                let f_exact = ((b as f32) + delta) * bin_hz;
                let midi = 69.0 + 12.0 * (f_exact / 440.0).log2();
                let pc_float = midi.rem_euclid(12.0);
                let pc_nearest = (pc_float.round() as isize).rem_euclid(12) as usize;
                let cents_dist = (pc_float - (pc_float.round())).abs();
                // Gaussian tuning window around semitone center + log-compressed harmonic weight
                let tuning_weight = (-4.5 * cents_dist * cents_dist).exp();
                let harm_weight = ((1.0 + m_curr * 15.0).ln() * tuning_weight) as f64;
                chroma_accum[pc_nearest] += harm_weight;
            }
        }

        sub_flux.push((s_e - prev_sub).max(0.0));
        mid_flux.push((m_e - prev_mid).max(0.0));
        prev_sub = s_e;
        prev_mid = m_e;

        sub_env.push(s_e);
        mid_env.push(m_e);
        high_env.push(h_e);
    }

    // 1. BPM Estimation via 5-Layer Consensus BPM Engine (or explicit hint)
    let bpm = match bpm_hint {
        Some(b) if b > 40.0 => normalize_dj_bpm(b),
        _ => {
            let report = crate::bpm_engine::analyze_bpm_consensus(mono, sample_rate, None);
            report.bpm
        }
    };
    let beat_sec = 60.0 / bpm.max(60.0);
    let bar_sec = 4.0 * beat_sec;

    // 2. Camelot Key Detection via Krumhansl-Schmuckler Profile Correlation
    let (camelot_key, musical_key) = detect_camelot_key(&chroma_accum);

    // 3. Groove & Transient Punch Classification
    let (punch_index, groove_type, energy_score) =
        classify_groove_and_punch(&sub_env, &mid_env, &high_env, &sub_flux, bpm);

    // 4. Cue 1: Real First Downbeat (`1.1.1`)
    let search_limit_sec = 15.0_f32.min(decoded_dur * 0.35);
    let max_search_frame = ((search_limit_sec / frame_dt) as usize)
        .max(1)
        .min(sub_flux.len().saturating_sub(1));
    let local_max_flux = sub_flux[..max_search_frame]
        .iter()
        .copied()
        .fold(1e-9_f32, f32::max);
    let kick_thresh = local_max_flux * 0.32;

    let mut first_kick_frame = 0usize;
    for idx in 0..max_search_frame {
        if sub_flux[idx] >= kick_thresh {
            let mut best = idx;
            let end_peak = (idx + 3).min(max_search_frame);
            for k in idx..end_peak {
                if sub_flux[k] > sub_flux[best] {
                    best = k;
                }
            }
            first_kick_frame = best;
            break;
        }
    }
    let first_beat_sec = ((first_kick_frame as f32) * frame_dt
        + (n_fft as f32 * 0.25) / (sample_rate as f32))
        .min(decoded_dur * 0.35);

    // 5. Bar-level Sub, Mid & Total Energy Profiles locked to `first_beat_sec`
    let n_bars_decoded = (((decoded_dur - first_beat_sec) / bar_sec) as usize).max(4);
    let n_bars_total = (((total_dur - first_beat_sec) / bar_sec) as usize).max(n_bars_decoded);

    let mut bar_sub = vec![0.0_f32; n_bars_decoded];
    let mut bar_mid = vec![0.0_f32; n_bars_decoded];
    let mut bar_tot = vec![0.0_f32; n_bars_decoded];

    for b in 0..n_bars_decoded {
        let t0 = first_beat_sec + (b as f32) * bar_sec;
        let t1 = t0 + bar_sec;
        let f0 = ((t0 / frame_dt) as usize).min(n_frames.saturating_sub(1));
        let f1 = ((t1 / frame_dt) as usize).max(f0 + 1).min(n_frames);
        let len = (f1.saturating_sub(f0).max(1)) as f32;
        let s_mean: f32 = sub_env[f0..f1].iter().sum::<f32>() / len;
        let m_mean: f32 = mid_env[f0..f1].iter().sum::<f32>() / len;
        let h_mean: f32 = high_env[f0..f1].iter().sum::<f32>() / len;
        bar_sub[b] = s_mean;
        bar_mid[b] = m_mean;
        bar_tot[b] = s_mean + m_mean + 0.5 * h_mean;
    }

    let max_sub = bar_sub.iter().copied().fold(1e-9_f32, f32::max);
    let max_mid = bar_mid.iter().copied().fold(1e-9_f32, f32::max);
    let max_tot = bar_tot.iter().copied().fold(1e-9_f32, f32::max);
    for b in 0..n_bars_decoded {
        bar_sub[b] /= max_sub;
        bar_mid[b] /= max_mid;
        bar_tot[b] /= max_tot;
    }

    // 6. Cue 3 (Main Drop) & Cue 2 (Phrase / Vocal Entry)
    let mut best_drop_bar = 16usize.min(n_bars_decoded.saturating_sub(1));
    let mut best_drop_score = -1e9_f32;
    let drop_limit = n_bars_decoded.saturating_sub(2).min(48);

    let mut candidate_b = 8usize;
    while candidate_b <= drop_limit {
        let pre_start = candidate_b.saturating_sub(4);
        let pre_sub = mean_slice(&bar_sub[pre_start..candidate_b]);
        let post_end = (candidate_b + 4).min(n_bars_decoded);
        let post_sub = mean_slice(&bar_sub[candidate_b..post_end]);
        let post_tot = mean_slice(&bar_tot[candidate_b..post_end]);
        let contrast = (post_sub - pre_sub) * 1.45 + post_tot * 0.55;
        if contrast > best_drop_score {
            best_drop_score = contrast;
            best_drop_bar = candidate_b;
        }
        candidate_b += 4;
    }

    let mut phrase_bar = 8usize.min(best_drop_bar.saturating_sub(4)).max(4);
    let mut best_phrase_score = -1e9_f32;
    let max_phrase_cand = best_drop_bar
        .saturating_sub(4)
        .min(n_bars_decoded.saturating_sub(2));
    let mut p_b = 4usize;
    while p_b <= max_phrase_cand {
        let pre_mid = mean_slice(&bar_mid[..p_b]);
        let post_end = (p_b + 4).min(n_bars_decoded);
        let post_mid = mean_slice(&bar_mid[p_b..post_end]);
        let post_tot = mean_slice(&bar_tot[p_b..post_end]);
        let delta = (post_mid - pre_mid) + 0.35 * post_tot;
        if delta > best_phrase_score {
            best_phrase_score = delta;
            phrase_bar = p_b;
        }
        p_b += 4;
    }
    if best_drop_bar <= phrase_bar {
        best_drop_bar = (phrase_bar + 8).min(n_bars_total.saturating_sub(4));
    }

    // 7. Cue 4 (Mix-Out Point)
    let mix_out_bar = if total_dur <= decoded_dur * 1.05 && n_bars_decoded >= 24 {
        let start_out = ((n_bars_decoded as f32 * 0.65) as usize / 4).max(1) * 4;
        let mut best_out = (n_bars_decoded.saturating_sub(8) / 4) * 4;
        let mut best_out_score = -1e9_f32;
        let mut ob = start_out;
        while ob < n_bars_decoded.saturating_sub(2) {
            let pre_s = ob.saturating_sub(4);
            let pre_t = mean_slice(&bar_tot[pre_s..ob]);
            let post_e = (ob + 4).min(n_bars_decoded);
            let post_t = mean_slice(&bar_tot[ob..post_e]);
            let post_m = mean_slice(&bar_mid[ob..post_e]);
            let drop_score = (pre_t - post_t) + (1.0 - post_m) * 0.4;
            if drop_score > best_out_score {
                best_out_score = drop_score;
                best_out = ob;
            }
            ob += 4;
        }
        best_out.max(best_drop_bar + 8)
    } else {
        let target_out_bars = ((n_bars_total.saturating_sub(8)) / 4) * 4;
        target_out_bars.max(best_drop_bar + 8)
    };

    let phrase_in_sec = (first_beat_sec + (phrase_bar as f32) * bar_sec).min(total_dur * 0.55);
    let main_drop_sec = (first_beat_sec + (best_drop_bar as f32) * bar_sec).min(total_dur * 0.85);
    let mix_lo = (main_drop_sec + bar_sec * 2.0).min(total_dur * 0.85);
    let mix_hi = (total_dur - bar_sec * 2.0).max(mix_lo);
    let mix_out_sec = (first_beat_sec + (mix_out_bar as f32) * bar_sec)
        .max(mix_lo)
        .min(mix_hi);

    // 8. Intro / Outro Vocal Presence Detection
    // Requires both high mid-band ratio relative to the track's peak AND vocal-mid dominance over sub/high
    let intro_end = 8usize.min(n_bars_decoded);
    let intro_vocal_ratio = mean_slice(&bar_mid[..intro_end]);
    let intro_sub_ratio = mean_slice(&bar_sub[..intro_end]);
    let outro_start = n_bars_decoded.saturating_sub(8);
    let outro_vocal_ratio = mean_slice(&bar_mid[outro_start..]);
    let outro_sub_ratio = mean_slice(&bar_sub[outro_start..]);
    let has_intro_vocals = intro_vocal_ratio >= 0.68 && intro_vocal_ratio > intro_sub_ratio * 1.05;
    let has_outro_vocals = outro_vocal_ratio >= 0.72 && outro_vocal_ratio > outro_sub_ratio * 1.05;

    // 9. Build 64D L2-Normalized Style Embedding
    let inv_f = 1.0 / (n_frames.max(1) as f64);
    let mut emb = Vec::with_capacity(64);
    for b in 0..32 {
        let mean = band_energy_32[b] * inv_f;
        let var = (band_sq_32[b] * inv_f - mean * mean).max(0.0);
        emb.push(mean as f32);
        emb.push(var.sqrt() as f32);
    }
    let norm = emb.iter().map(|&x| x * x).sum::<f32>().sqrt().max(1e-9);
    for x in &mut emb {
        *x = ((*x / norm) * 10000.0).round() / 10000.0;
    }

    let top_subgenre = infer_subgenre(bpm, &groove_type, punch_index);

    let hot_cues = vec![
        HotCuePoint {
            num: 1,
            name: "1.1.1 Downbeat".to_string(),
            seconds: round3(first_beat_sec),
            bar_index: 0,
            role: "downbeat".to_string(),
            confidence: 0.96,
        },
        HotCuePoint {
            num: 2,
            name: format!("Phrase In ({phrase_bar}b)"),
            seconds: round3(phrase_in_sec),
            bar_index: phrase_bar,
            role: "phrase_in".to_string(),
            confidence: 0.90,
        },
        HotCuePoint {
            num: 3,
            name: format!("Main Drop ({best_drop_bar}b)"),
            seconds: round3(main_drop_sec),
            bar_index: best_drop_bar,
            role: "drop".to_string(),
            confidence: 0.94,
        },
        HotCuePoint {
            num: 4,
            name: format!("Mix Out ({mix_out_bar}b)"),
            seconds: round3(mix_out_sec),
            bar_index: mix_out_bar,
            role: "mix_out".to_string(),
            confidence: 0.91,
        },
    ];

    StructuralCueProfile {
        bpm: round2(bpm),
        camelot_key,
        musical_key,
        energy_score: round3(energy_score),
        punch_index: round3(punch_index),
        groove_type,
        top_subgenre,
        first_beat_seconds: round3(first_beat_sec),
        phrase_in_seconds: round3(phrase_in_sec),
        main_drop_seconds: round3(main_drop_sec),
        mix_out_seconds: round3(mix_out_sec),
        has_intro_vocals,
        has_outro_vocals,
        intro_vocal_ratio: round3(intro_vocal_ratio),
        outro_vocal_ratio: round3(outro_vocal_ratio),
        hot_cues,
        embedding_64d: emb,
    }
}

#[allow(dead_code)]
fn estimate_bpm_from_flux(sub_flux: &[f32], mid_flux: &[f32], frame_dt: f32) -> f32 {
    let n_all = sub_flux.len().min(mid_flux.len());
    if n_all < 128 {
        return 124.0;
    }

    // Focus on the rhythmic body of the track (skip beatless first 8% if track is long)
    let start_idx = if n_all > 1500 { n_all / 12 } else { 0 };
    let sub_slice = &sub_flux[start_idx..n_all];
    let mid_slice = &mid_flux[start_idx..n_all];
    let n = sub_slice.len();

    // Local moving-average onset novelty whitening (window = 16 frames ~ 170ms)
    let mut novelty = vec![0.0_f32; n];
    let mut sub_novelty = vec![0.0_f32; n];
    let win = 16usize;
    for i in 0..n {
        let lo = i.saturating_sub(win);
        let hi = (i + win + 1).min(n);
        let len = (hi - lo) as f32;
        let s_mean = sub_slice[lo..hi].iter().sum::<f32>() / len;
        let m_mean = mid_slice[lo..hi].iter().sum::<f32>() / len;
        let s_nov = (sub_slice[i] - s_mean).max(0.0);
        let m_nov = (mid_slice[i] - m_mean).max(0.0);
        sub_novelty[i] = s_nov;
        novelty[i] = s_nov * 1.45 + m_nov * 0.55;
    }

    let max_lag = (((60.0 / 35.0) / frame_dt).ceil() as usize).min(n / 2);
    let min_lag = (((60.0 / 170.0) / frame_dt).floor() as usize).max(2);
    if max_lag <= min_lag {
        return 124.0;
    }

    let mut acf = vec![0.0_f32; max_lag + 1];
    let mut acf_sub = vec![0.0_f32; max_lag + 1];
    let norm0: f64 = novelty
        .iter()
        .map(|&x| (x as f64) * (x as f64))
        .sum::<f64>()
        .max(1e-9);
    let norm0_sub: f64 = sub_novelty
        .iter()
        .map(|&x| (x as f64) * (x as f64))
        .sum::<f64>()
        .max(1e-9);

    for lag in min_lag..=max_lag {
        let mut sum = 0.0_f64;
        let mut sum_s = 0.0_f64;
        let count = n - lag;
        for i in 0..count {
            sum += (novelty[i] as f64) * (novelty[i + lag] as f64);
            sum_s += (sub_novelty[i] as f64) * (sub_novelty[i + lag] as f64);
        }
        let scale = (n as f64) / (count.max(1) as f64);
        acf[lag] = ((sum * scale) / norm0) as f32;
        acf_sub[lag] = ((sum_s * scale) / norm0_sub) as f32;
    }

    let interp_acf = |arr: &[f32], lag_f: f32| -> f32 {
        let idx = lag_f.floor() as usize;
        if idx < min_lag || idx + 1 >= arr.len() {
            return 0.0;
        }
        let frac = lag_f - (idx as f32);
        arr[idx] * (1.0 - frac) + arr[idx + 1] * frac
    };

    // Evaluate candidate BPMs in [75.0 .. 148.0] using a 4-Harmonic Pulse-Train Comb Filter
    let mut best_bpm = 124.0_f32;
    let mut best_score = -1e9_f32;

    let mut cand_bpm = 75.0_f32;
    while cand_bpm <= 148.0 {
        let lag1 = (60.0 / cand_bpm) / frame_dt;
        let c1 = interp_acf(&acf, lag1);
        let c2 = interp_acf(&acf, lag1 * 2.0);
        let c3 = interp_acf(&acf, lag1 * 3.0);
        let c4 = interp_acf(&acf, lag1 * 4.0);
        let sub1 = interp_acf(&acf_sub, lag1);
        let sub2 = interp_acf(&acf_sub, lag1 * 2.0);

        let mut comb = c1 * 1.00 + c2 * 0.60 + c3 * 0.30 + c4 * 0.25 + (sub1 + sub2) * 0.35;

        // Natural DJ genre tempo prior: Urban/Reggaeton/Afrobeat (78..105) & Club/House/Guaracha (116..132)
        let prior = if (78.0..=105.0).contains(&cand_bpm) || (116.0..=132.0).contains(&cand_bpm) {
            1.08
        } else if (105.0..116.0).contains(&cand_bpm) {
            0.94
        } else {
            0.88
        };
        comb *= prior;

        if comb > best_score {
            best_score = comb;
            best_bpm = cand_bpm;
        }
        cand_bpm += 0.25;
    }

    // Dembow / Afrobeat / Tresillo 4:3 Dotted-Eighth De-aliasing:
    // In Reggaeton/Afrobeat (78..96 BPM), the tresillo snare/rimshot at 3/4 beat creates a peak at 4/3 * BPM (104..128 BPM).
    let dembow_root = best_bpm * 0.75;
    if (76.0..=97.0).contains(&dembow_root) {
        let lag_root = (60.0 / dembow_root) / frame_dt;
        let lag_cur = (60.0 / best_bpm) / frame_dt;
        let root_sub = interp_acf(&acf_sub, lag_root) + interp_acf(&acf_sub, lag_root * 2.0);
        let cur_sub = interp_acf(&acf_sub, lag_cur);
        let thresh = if best_bpm < 117.6 { 0.82 } else { 1.18 };
        if root_sub >= cur_sub * thresh {
            best_bpm = dembow_root;
        }
    }

    // Conversely, in Afro-House / Club (118..130 BPM), a 4/5 polyrhythmic bassline (~94..104 BPM)
    // can appear; check if 5/4 * best_bpm has stronger 4-on-the-floor kick periodicity.
    let house_five_four = best_bpm * 1.25;
    if (92.0..=104.5).contains(&best_bpm) && (117.0..=130.0).contains(&house_five_four) {
        let lag_h = (60.0 / house_five_four) / frame_dt;
        let h_sub = interp_acf(&acf_sub, lag_h) + 0.7 * interp_acf(&acf_sub, lag_h * 2.0);
        let cur_sub = interp_acf(&acf_sub, (60.0 / best_bpm) / frame_dt);
        if h_sub >= cur_sub * 1.05 {
            best_bpm = house_five_four;
        }
    }

    // Snap to integer BPM if very close (studio masters are almost always integer BPMs)
    let nearest_int = best_bpm.round();
    if (best_bpm - nearest_int).abs() <= 0.40 {
        nearest_int
    } else {
        (best_bpm * 10.0).round() / 10.0
    }
}

fn normalize_dj_bpm(mut bpm: f32) -> f32 {
    while bpm < 75.0 {
        bpm *= 2.0;
    }
    while bpm > 155.0 {
        bpm *= 0.5;
    }
    bpm
}

fn detect_camelot_key(chroma: &[f64; 12]) -> (String, String) {
    let sum: f64 = chroma.iter().sum();
    if sum <= 1e-12 {
        return ("8A".to_string(), "Am".to_string());
    }

    let c_mean = (sum / 12.0) as f32;
    let mut c_centered = [0.0_f32; 12];
    let mut c_var = 0.0_f32;
    for i in 0..12 {
        let d = (chroma[i] as f32) - c_mean;
        c_centered[i] = d;
        c_var += d * d;
    }
    let c_std = c_var.sqrt().max(1e-9);

    let maj_mean: f32 = MAJOR_PROFILE.iter().sum::<f32>() / 12.0;
    let min_mean: f32 = MINOR_PROFILE.iter().sum::<f32>() / 12.0;
    let maj_std: f32 = MAJOR_PROFILE
        .iter()
        .map(|&x| (x - maj_mean) * (x - maj_mean))
        .sum::<f32>()
        .sqrt()
        .max(1e-9);
    let min_std: f32 = MINOR_PROFILE
        .iter()
        .map(|&x| (x - min_mean) * (x - min_mean))
        .sum::<f32>()
        .sqrt()
        .max(1e-9);

    let mut best_score = -1e9_f32;
    let mut best_camelot = "8A";
    let mut best_name = "Am".to_string();

    for root in 0..12 {
        let mut maj_cov = 0.0_f32;
        let mut min_cov = 0.0_f32;
        for i in 0..12 {
            let c_val = c_centered[(root + i) % 12];
            maj_cov += c_val * (MAJOR_PROFILE[i] - maj_mean);
            min_cov += c_val * (MINOR_PROFILE[i] - min_mean);
        }
        let maj_r = maj_cov / (c_std * maj_std);
        let min_r = min_cov / (c_std * min_std);

        if maj_r > best_score {
            best_score = maj_r;
            best_camelot = CAMELOT_MAJOR[root];
            best_name = PITCH_NAMES[root].to_string();
        }
        if min_r > best_score {
            best_score = min_r;
            best_camelot = CAMELOT_MINOR[root];
            best_name = format!("{}m", PITCH_NAMES[root]);
        }
    }

    (best_camelot.to_string(), best_name)
}

fn classify_groove_and_punch(
    sub_env: &[f32],
    mid_env: &[f32],
    high_env: &[f32],
    sub_flux: &[f32],
    bpm: f32,
) -> (f32, String, f32) {
    let n = sub_env.len().max(1) as f32;
    let mean_sub = sub_env.iter().sum::<f32>() / n;
    let max_sub = sub_env.iter().copied().fold(1e-9_f32, f32::max);
    let mean_flux = sub_flux.iter().sum::<f32>() / n;
    let max_flux = sub_flux.iter().copied().fold(1e-9_f32, f32::max);

    let punch_index = ((max_flux / (mean_flux * 4.5 + 1e-6)).clamp(0.15, 0.98) * 0.65
        + (mean_sub / max_sub).clamp(0.1, 0.95) * 0.35)
        .clamp(0.1, 0.99);

    let mean_mid = mid_env.iter().sum::<f32>() / n;
    let mean_high = high_env.iter().sum::<f32>() / n;
    let energy_score = ((mean_sub * 0.5 + mean_mid * 0.35 + mean_high * 0.15)
        / (max_sub * 0.65 + 1e-6))
        .clamp(0.25, 0.98);

    let groove_type = if (88.0..=114.0).contains(&bpm) {
        "dembow_tresillo".to_string()
    } else if (118.0..=134.0).contains(&bpm) && punch_index >= 0.48 {
        "four_on_floor".to_string()
    } else if bpm > 134.0 {
        "breakbeat".to_string()
    } else {
        "syncopated".to_string()
    };

    (punch_index, groove_type, energy_score)
}

fn infer_subgenre(bpm: f32, groove_type: &str, punch_index: f32) -> String {
    if (88.0..=104.0).contains(&bpm) {
        "Reggaeton / Urbano".to_string()
    } else if (104.0..=117.0).contains(&bpm) {
        "Dembow / Afrobeats".to_string()
    } else if (118.0..=123.5).contains(&bpm) {
        "Afro House / Deep Tech".to_string()
    } else if (123.5..=131.0).contains(&bpm) && groove_type == "four_on_floor" {
        if punch_index > 0.72 {
            "Tech House / Guaracha".to_string()
        } else {
            "House / Club".to_string()
        }
    } else if (132.0..=162.0).contains(&bpm) {
        "Tecnomerengue".to_string()
    } else {
        "Electronic / Club Crossover".to_string()
    }
}

fn mean_slice(s: &[f32]) -> f32 {
    if s.is_empty() {
        0.0
    } else {
        s.iter().sum::<f32>() / (s.len() as f32)
    }
}

fn round2(v: f32) -> f32 {
    (v * 100.0).round() / 100.0
}

fn round3(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}

fn fallback_structure(bpm: f32, duration: f32) -> StructuralCueProfile {
    let bar = (60.0 / bpm.max(60.0)) * 4.0;
    let c2 = (8.0 * bar).min(duration * 0.25);
    let c3 = (16.0 * bar).min(duration * 0.45);
    let c4 = (duration - 8.0 * bar).max(c3 + bar);
    StructuralCueProfile {
        bpm,
        camelot_key: "8A".to_string(),
        musical_key: "Am".to_string(),
        energy_score: 0.7,
        punch_index: 0.65,
        groove_type: "four_on_floor".to_string(),
        top_subgenre: "Electronic / Club".to_string(),
        first_beat_seconds: 0.0,
        phrase_in_seconds: round3(c2),
        main_drop_seconds: round3(c3),
        mix_out_seconds: round3(c4),
        has_intro_vocals: false,
        has_outro_vocals: false,
        intro_vocal_ratio: 0.0,
        outro_vocal_ratio: 0.0,
        hot_cues: vec![
            HotCuePoint {
                num: 1,
                name: "1.1.1 Downbeat".to_string(),
                seconds: 0.0,
                bar_index: 0,
                role: "downbeat".to_string(),
                confidence: 0.5,
            },
            HotCuePoint {
                num: 2,
                name: "Phrase In (8b)".to_string(),
                seconds: round3(c2),
                bar_index: 8,
                role: "phrase_in".to_string(),
                confidence: 0.5,
            },
            HotCuePoint {
                num: 3,
                name: "Main Drop (16b)".to_string(),
                seconds: round3(c3),
                bar_index: 16,
                role: "drop".to_string(),
                confidence: 0.5,
            },
            HotCuePoint {
                num: 4,
                name: "Mix Out".to_string(),
                seconds: round3(c4),
                bar_index: 32,
                role: "mix_out".to_string(),
                confidence: 0.5,
            },
        ],
        embedding_64d: vec![0.125; 64],
    }
}
