//! Unit tests for `scanner::audio_metrics` (`768 B` RGB, `-11 LUFS` anti-clip, `64D` i8 & sub-phase).

use super::*;
use std::f32::consts::TAU;

#[test]
fn test_rgb_waveform_blob_768_bytes_and_band_separation() {
    let sr = 44_100_u32;
    let seg = sr as usize;
    let mut samples = Vec::with_capacity(seg * 3);
    for freq in [60.0_f32, 1_000.0, 6_000.0] {
        for i in 0..seg {
            samples.push(0.6 * (TAU * freq * (i as f32) / (sr as f32)).sin());
        }
    }
    let rgb_blob = build_rgb_waveform_blob(&samples, sr, 256);
    assert_eq!(rgb_blob.len(), 768);
    assert_eq!(build_rgb_waveform_blob(&[], sr, 256), vec![0_u8; 768]);
    let (cs, cm, ca) = (42 * 3, 128 * 3, 212 * 3);
    assert!(rgb_blob[cs] > rgb_blob[cs + 1] && rgb_blob[cs] > rgb_blob[cs + 2]);
    assert!(rgb_blob[cm + 1] > rgb_blob[cm] && rgb_blob[cm + 1] > rgb_blob[cm + 2]);
    assert!(rgb_blob[ca + 2] > rgb_blob[ca] && rgb_blob[ca + 2] > rgb_blob[ca + 1]);
}

#[test]
fn test_loudness_minus_11_lufs_anti_clip_and_lead_in_silence() {
    let sr = 48_000_u32;
    let mut samples = vec![0.0_f32; (sr as usize) * 2];
    for i in 0..(sr as usize) * 4 {
        let s = if i == sr as usize {
            0.99
        } else {
            0.10 * (TAU * 1_000.0 * (i as f32) / (sr as f32)).sin()
        };
        samples.push(s);
    }
    let m = compute_loudness_and_lead_in(&samples, sr);
    assert!(m.first_audible_sec >= 2.0 && m.first_audible_sec < 2.08);
    let gain_lin = 10.0_f32.powf(m.replay_gain_db / 20.0);
    let peak_lin = 10.0_f32.powf(m.true_peak_dbfs / 20.0);
    assert!(gain_lin * peak_lin <= 0.967);
    assert_eq!(
        compute_loudness_and_lead_in(&vec![0.0_f32; 4_096], sr).replay_gain_db,
        0.0
    );
}

#[test]
fn test_quantize_embedding_64d_and_cosine_similarity_i8() {
    let mut a = vec![0.0_f32; 64];
    let mut b = vec![0.0_f32; 64];
    for i in 0..32 {
        a[i] = ((i + 1) as f32) * 0.1;
        b[i + 32] = ((i + 1) as f32) * 0.1;
    }
    let qa = quantize_embedding_64d(&a);
    let qa_scaled = quantize_embedding_64d(&a.iter().map(|v| v * 3.5).collect::<Vec<_>>());
    let qb = quantize_embedding_64d(&b);
    assert!(cosine_similarity_i8(&qa, &qa_scaled) > 0.98);
    assert!(cosine_similarity_i8(&qa, &qb).abs() < 0.1);
}

#[test]
fn test_check_stereo_sub_mono_phase_in_phase_vs_inverted() {
    let sr = 44_100_u32;
    let mut in_phase = Vec::with_capacity((sr as usize) * 2);
    let mut inverted = Vec::with_capacity((sr as usize) * 2);
    for i in 0..(sr as usize) {
        let sub = 0.7 * (TAU * 60.0 * (i as f32) / (sr as f32)).sin();
        in_phase.extend_from_slice(&[sub, sub]);
        inverted.extend_from_slice(&[sub, -sub]);
    }
    assert!(check_stereo_sub_mono_phase(&in_phase, sr) > 0.9);
    assert!(check_stereo_sub_mono_phase(&inverted, sr) < -0.8);
}
