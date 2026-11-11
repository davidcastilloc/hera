//! 4-Band Percussive Onset Novelty Decomposition:
//! - SubKick (25-130 Hz): Electronic kick, sub-bass 808, tambora low hit.
//! - LowBody (130-450 Hz): Salsa baby-bass tumbao, conga open tone.
//! - PercClave (650-3800 Hz): Salsa campana, bongo martillo, clave, conga slap.
//! - HiGuiro (4200-11000 Hz): Merengue/Tecnomerengue güira 16th-notes, hi-hats.

use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

#[derive(Debug, Clone)]
pub struct MultibandNovelty {
    pub sub_kick: Vec<f32>,
    pub low_body: Vec<f32>,
    pub perc_clave: Vec<f32>,
    pub hi_guiro: Vec<f32>,
    pub frame_dt: f32,
}

pub fn decompose_multiband_onsets(mono: &[f32], sample_rate: u32) -> MultibandNovelty {
    let n_fft = 2048usize;
    let hop = 512usize;
    let sr = sample_rate.max(1) as f32;
    let frame_dt = (hop as f32) / sr;
    let bin_hz = sr / (n_fft as f32);

    if mono.len() < n_fft {
        return MultibandNovelty {
            sub_kick: Vec::new(),
            low_body: Vec::new(),
            perc_clave: Vec::new(),
            hi_guiro: Vec::new(),
            frame_dt,
        };
    }

    let bin_sub_lo = (25.0 / bin_hz).floor() as usize;
    let bin_sub_hi = ((130.0 / bin_hz).ceil() as usize).min(n_fft / 2);

    let bin_low_lo = (130.0 / bin_hz).floor() as usize;
    let bin_low_hi = ((450.0 / bin_hz).ceil() as usize).min(n_fft / 2);

    let bin_clave_lo = (650.0 / bin_hz).floor() as usize;
    let bin_clave_hi = ((3800.0 / bin_hz).ceil() as usize).min(n_fft / 2);

    let bin_guiro_lo = (4200.0 / bin_hz).floor() as usize;
    let bin_guiro_hi = ((11000.0 / bin_hz).ceil() as usize).min(n_fft / 2);

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n_fft);
    let hann: Vec<f32> = (0..n_fft)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * (i as f32) / (n_fft as f32)).cos()))
        .collect();

    let mut raw_sub = Vec::new();
    let mut raw_low = Vec::new();
    let mut raw_clave = Vec::new();
    let mut raw_guiro = Vec::new();

    let mut buffer = vec![Complex::new(0.0_f32, 0.0_f32); n_fft];
    let mut prev_sub = 0.0_f32;
    let mut prev_low = 0.0_f32;
    let mut prev_clave = 0.0_f32;
    let mut prev_guiro = 0.0_f32;

    let total_frames = (mono.len() - n_fft) / hop;
    for f in 0..=total_frames {
        let offset = f * hop;
        for i in 0..n_fft {
            buffer[i] = Complex::new(mono[offset + i] * hann[i], 0.0);
        }
        fft.process(&mut buffer);

        let sum_band = |lo: usize, hi: usize| -> f32 {
            if lo >= hi {
                return 0.0;
            }
            buffer[lo..hi].iter().map(|c| c.norm()).sum()
        };

        let curr_sub = sum_band(bin_sub_lo, bin_sub_hi);
        let curr_low = sum_band(bin_low_lo, bin_low_hi);
        let curr_clave = sum_band(bin_clave_lo, bin_clave_hi);
        let curr_guiro = sum_band(bin_guiro_lo, bin_guiro_hi);

        raw_sub.push((curr_sub - prev_sub).max(0.0));
        raw_low.push((curr_low - prev_low).max(0.0));
        raw_clave.push((curr_clave - prev_clave).max(0.0));
        raw_guiro.push((curr_guiro - prev_guiro).max(0.0));

        prev_sub = curr_sub;
        prev_low = curr_low;
        prev_clave = curr_clave;
        prev_guiro = curr_guiro;
    }

    let whiten = |raw: &[f32]| -> Vec<f32> {
        let n = raw.len();
        let mut out = vec![0.0_f32; n];
        let win = 16usize;
        for i in 0..n {
            let lo = i.saturating_sub(win);
            let hi = (i + win + 1).min(n);
            let mean = raw[lo..hi].iter().sum::<f32>() / ((hi - lo) as f32);
            out[i] = (raw[i] - mean).max(0.0);
        }
        out
    };

    MultibandNovelty {
        sub_kick: whiten(&raw_sub),
        low_body: whiten(&raw_low),
        perc_clave: whiten(&raw_clave),
        hi_guiro: whiten(&raw_guiro),
        frame_dt,
    }
}
