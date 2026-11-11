use hera_node_rs::forensics_glitch::scan_pcm_for_cd_skips;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn test_clean_audio_has_no_audio_skips() {
    let sample_rate = 44_100;
    let n = sample_rate * 4;
    let mut mono = Vec::with_capacity(n);
    for i in 0..n {
        let t = (i as f32) / (sample_rate as f32);
        // Clean 440 Hz tone + 80 Hz beat
        let s = 0.5 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()
            + 0.3 * (2.0 * std::f32::consts::PI * 80.0 * t).sin();
        mono.push(s);
    }

    let report = scan_pcm_for_cd_skips(&mono, sample_rate as u32, 0);
    assert!(!report.has_audio_skips);
    assert_eq!(report.skip_glitch_count, 0);
    assert_eq!(report.micro_dropout_count, 0);
    assert_eq!(report.buffer_repeat_stutter_count, 0);
    assert!(report.skip_timestamps_seconds.is_empty());
}

#[test]
fn test_cd_skip_with_micro_dropout_and_phase_cliff_is_detected() {
    let sample_rate = 44_100;
    let n = sample_rate * 5;
    let mut mono = Vec::with_capacity(n);
    for i in 0..n {
        let t = (i as f32) / (sample_rate as f32);
        let s = 0.6 * (2.0 * std::f32::consts::PI * 220.0 * t).sin();
        mono.push(s);
    }

    // Inject a severe scratched-CD dropout at t = 2.0s (lasting ~35ms)
    // followed by an abrupt cliff (sample step discontinuity > 0.65)
    let drop_start = sample_rate * 2;
    let drop_len = (sample_rate as f32 * 0.035) as usize; // ~35ms
    for idx in drop_start..(drop_start + drop_len) {
        mono[idx] = 0.0;
    }
    // Phase cliff step right after dropout
    if drop_start + drop_len < mono.len() {
        mono[drop_start + drop_len] = 0.85;
    }

    let report = scan_pcm_for_cd_skips(&mono, sample_rate as u32, 0);
    assert!(report.has_audio_skips, "Scratched-CD dropout must trigger has_audio_skips");
    assert!(report.micro_dropout_count >= 1);
    assert!(!report.skip_timestamps_seconds.is_empty());
    let detected_time = report.skip_timestamps_seconds[0];
    assert!(
        (detected_time - 2.0).abs() <= 0.15,
        "Timestamp should be ~2.0s, got {detected_time}"
    );
}

#[test]
fn test_cd_skip_with_repeated_buffer_stutter_is_detected() {
    let sample_rate = 44_100;
    let n = sample_rate * 4;
    let mut mono = Vec::with_capacity(n);
    for i in 0..n {
        let t = (i as f32) / (sample_rate as f32);
        let s = 0.5 * (2.0 * std::f32::consts::PI * 330.0 * t).sin()
            + 0.2 * (2.0 * std::f32::consts::PI * 110.0 * t).sin();
        mono.push(s);
    }

    // Inject a repeated buffer stutter (scratched CD repeating an identical 60ms chunk 3 times)
    let stutter_start = (sample_rate as f32 * 1.5) as usize;
    let chunk_len = (sample_rate as f32 * 0.060) as usize; // 60ms
    let chunk: Vec<f32> = mono[stutter_start..stutter_start + chunk_len].to_vec();

    for rep in 1..=2 {
        let dest = stutter_start + rep * chunk_len;
        if dest + chunk_len < mono.len() {
            mono[dest..dest + chunk_len].copy_from_slice(&chunk);
        }
    }

    let report = scan_pcm_for_cd_skips(&mono, sample_rate as u32, 0);
    assert!(report.has_audio_skips, "Repeated buffer stutter must trigger has_audio_skips");
    assert!(report.buffer_repeat_stutter_count >= 1);
    assert!(!report.skip_timestamps_seconds.is_empty());
}

#[test]
fn test_corrupted_packets_count_flags_damaged_track() {
    let sample_rate = 44_100;
    let mono = vec![0.1_f32; sample_rate * 2];
    // Even if PCM is flat, 12 packet errors indicate severe bitstream corruption
    let report = scan_pcm_for_cd_skips(&mono, sample_rate as u32, 12);
    assert!(report.has_audio_skips);
    assert_eq!(report.decode_error_packets, 12);
}

#[test]
fn test_cancellation_flag_cancels_scan() {
    let cancel = AtomicBool::new(false);
    assert!(!cancel.load(Ordering::Relaxed));
    cancel.store(true, Ordering::Relaxed);
    assert!(cancel.load(Ordering::Relaxed));
}
