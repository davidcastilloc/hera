//! Spectral peak constellation extractor (`Wang 2003` 5-band logarithmic STFT landmarks).

use rustfft::{num_complex::Complex, FftPlanner};
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

const CANONICAL_SR: u32 = 11_025;
const MAX_DURATION_SEC: usize = 60;
const FFT_SIZE: usize = 1024;
const HOP_SIZE: usize = 352; // ~31.93 ms per frame at 11,025 Hz
const TIME_RADIUS_FRAMES: usize = 2;
const MIN_DT_FRAMES: usize = 1;
const MAX_DT_FRAMES: usize = 63;
const FAN_OUT_TARGETS: usize = 6;

/// Five logarithmic frequency bands spanning `~300 Hz .. 5,000 Hz` at `11,025 Hz` (`1024` FFT).
const LOG_SUB_BANDS: [(usize, usize); 5] = [
    (28, 49),   // 301 Hz .. 527 Hz
    (49, 86),   // 527 Hz .. 926 Hz
    (86, 151),  // 926 Hz .. 1,626 Hz
    (151, 265), // 1,626 Hz .. 2,853 Hz
    (265, 465), // 2,853 Hz .. 5,006 Hz
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandmarkHash {
    pub hash_u64: u64,
    pub anchor_time_ms: u32,
}

#[derive(Debug, Clone, Copy)]
struct SpectralPeak {
    frame: usize,
    bin: u16,
    mag_db: f32,
}

fn resample_to_canonical_11k(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    if samples.is_empty() || sample_rate == 0 {
        return Vec::new();
    }
    let max_in = (sample_rate as usize).saturating_mul(MAX_DURATION_SEC);
    let capped = &samples[..samples.len().min(max_in)];
    if sample_rate == CANONICAL_SR {
        return capped.to_vec();
    }

    let ratio = (sample_rate as f64) / (CANONICAL_SR as f64);
    let out_len = ((capped.len() as f64) / ratio).floor() as usize;
    if out_len == 0 {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(out_len);
    if ratio >= 1.0 {
        // Anti-aliasing boxcar decimation
        for m in 0..out_len {
            let s0 = ((m as f64) * ratio) as usize;
            let s1 = (((m + 1) as f64) * ratio).ceil() as usize;
            let end = s1.min(capped.len()).max(s0 + 1);
            let sum: f32 = capped[s0..end].iter().copied().sum();
            out.push(sum / ((end - s0) as f32));
        }
    } else {
        // Linear interpolation when upsampling low-rate test clips
        let last = capped.len() - 1;
        for m in 0..out_len {
            let pos = (m as f64) * ratio;
            let idx = (pos.floor() as usize).min(last);
            let frac = (pos - (idx as f64)) as f32;
            let a = capped[idx];
            let b = capped[(idx + 1).min(last)];
            out.push(a + (b - a) * frac);
        }
    }
    out
}

#[inline]
fn pack_landmark_hash(f1_bin: u16, f2_bin: u16, dt_frames: usize) -> u64 {
    let f1 = u64::from(f1_bin) & 0x3FF;
    let f2 = u64::from(f2_bin) & 0x3FF;
    let dt = (dt_frames as u64).clamp(1, 0xFFF) & 0xFFF;
    (f1 << 22) | (f2 << 12) | dt
}

/// Extracts deterministic Shazam constellation landmarks from up to `60s` of mono PCM audio.
pub fn extract_constellation_landmarks(samples: &[f32], sample_rate: u32) -> Vec<LandmarkHash> {
    let canonical = resample_to_canonical_11k(samples, sample_rate);
    if canonical.len() < FFT_SIZE {
        return Vec::new();
    }

    let num_frames = (canonical.len() - FFT_SIZE) / HOP_SIZE + 1;
    if num_frames < 4 {
        return Vec::new();
    }

    let hann: Vec<f32> = (0..FFT_SIZE)
        .map(|i| 0.5 * (1.0 - (TAU * (i as f32) / (FFT_SIZE as f32)).cos()))
        .collect();

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let mut buf = vec![
        Complex {
            re: 0.0_f32,
            im: 0.0_f32
        };
        FFT_SIZE
    ];

    // Per frame & per band: store `(dominant_bin, max_mag_db)`
    let mut band_peaks = vec![[(0_u16, -120.0_f32); 5]; num_frames];
    let mut frame_energy_db = vec![-120.0_f32; num_frames];

    for (t, slot) in band_peaks.iter_mut().enumerate() {
        let offset = t * HOP_SIZE;
        for i in 0..FFT_SIZE {
            buf[i] = Complex {
                re: canonical[offset + i] * hann[i],
                im: 0.0,
            };
        }
        fft.process(&mut buf);

        let mut band_sum = 0.0_f32;
        for (b_idx, &(b_start, b_end)) in LOG_SUB_BANDS.iter().enumerate() {
            let mut best_bin = b_start;
            let mut best_mag_sq = 0.0_f32;
            for (bin, c) in buf.iter().enumerate().take(b_end).skip(b_start) {
                let mag_sq = c.re * c.re + c.im * c.im;
                if mag_sq > best_mag_sq {
                    best_mag_sq = mag_sq;
                    best_bin = bin;
                }
            }
            let db = 10.0 * (best_mag_sq.max(1e-12)).log10();
            slot[b_idx] = (best_bin as u16, db);
            band_sum += db;
        }
        frame_energy_db[t] = band_sum * 0.2;
    }

    // Compute adaptive noise floor from median frame energy
    let mut sorted_db = frame_energy_db.clone();
    sorted_db.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_db = sorted_db[sorted_db.len() / 2];
    let min_peak_db = (median_db - 6.0).max(-55.0);

    // 2D local peak picking across time radius (+-2 frames) within each logarithmic sub-band
    let mut peaks = Vec::new();
    for t in 0..num_frames {
        let t0 = t.saturating_sub(TIME_RADIUS_FRAMES);
        let t1 = (t + TIME_RADIUS_FRAMES).min(num_frames - 1);
        let local_mean: f32 = frame_energy_db[t0..=t1].iter().sum::<f32>() / ((t1 - t0 + 1) as f32);

        for b_idx in 0..LOG_SUB_BANDS.len() {
            let (cand_bin, cand_db) = band_peaks[t][b_idx];
            if cand_db < min_peak_db || cand_db < local_mean + 1.5 {
                continue;
            }
            let mut is_local_max = true;
            for (nt, nslot) in band_peaks.iter().enumerate().take(t1 + 1).skip(t0) {
                if nt == t {
                    continue;
                }
                let neighbor_db = nslot[b_idx].1;
                if neighbor_db > cand_db || (neighbor_db == cand_db && nt < t) {
                    is_local_max = false;
                    break;
                }
            }
            if is_local_max {
                peaks.push(SpectralPeak {
                    frame: t,
                    bin: cand_bin,
                    mag_db: cand_db,
                });
            }
        }
    }

    // Sort chronologically by `(frame, bin)` for deterministic target-zone pairing
    peaks.sort_by(|a, b| a.frame.cmp(&b.frame).then_with(|| a.bin.cmp(&b.bin)));

    let mut landmarks = Vec::with_capacity(peaks.len() * FAN_OUT_TARGETS);
    let mut target_buf: Vec<(usize, u16, f32)> = Vec::with_capacity(32);

    for (idx, anchor) in peaks.iter().enumerate() {
        target_buf.clear();
        for target in &peaks[idx + 1..] {
            let dt = target.frame.saturating_sub(anchor.frame);
            if dt < MIN_DT_FRAMES {
                continue;
            }
            if dt > MAX_DT_FRAMES {
                break;
            }
            target_buf.push((dt, target.bin, target.mag_db));
        }

        // Select up to `FAN_OUT_TARGETS` strongest peaks in the target zone so `-30 dB` noise
        // spurious peaks never displace true musical harmonics
        target_buf.sort_by(|a, b| {
            b.2.partial_cmp(&a.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
                .then_with(|| a.1.cmp(&b.1))
        });

        let anchor_time_ms =
            (((anchor.frame as u64) * (HOP_SIZE as u64) * 1000) / u64::from(CANONICAL_SR)) as u32;

        for &(dt, f2_bin, _) in target_buf.iter().take(FAN_OUT_TARGETS) {
            landmarks.push(LandmarkHash {
                hash_u64: pack_landmark_hash(anchor.bin, f2_bin, dt),
                anchor_time_ms,
            });
        }
    }

    landmarks
}
