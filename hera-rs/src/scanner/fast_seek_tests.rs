//! Unit tests for `scanner::fast_seek`.

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

#[test]
fn test_fast_windows_populates_entry_and_exit_bpm_on_transition() {
    let sr = 22_050_u32;
    let len = (sr as usize) * 16;
    let make_kicks = |bpm: f64| -> Vec<f32> {
        let beat = 60.0 / bpm;
        (0..len)
            .map(|i| {
                let p = ((i as f64) / (sr as f64)).rem_euclid(beat);
                if p < 0.08 {
                    (0.75 * (-p * 40.0).exp() * (2.0 * std::f64::consts::PI * 60.0 * p).sin())
                        as f32
                } else {
                    0.0
                }
            })
            .collect()
    };
    let res = analyze_fast_windows(&make_kicks(96.0), &make_kicks(128.0), sr, None, 20e3, 320);
    assert!(res.is_transition_candidate);
    assert!(res.entry_bpm.is_some_and(|v| (v - 96.0).abs() <= 2.0));
    assert!(res.exit_bpm.is_some_and(|v| (v - 128.0).abs() <= 2.0));
}
