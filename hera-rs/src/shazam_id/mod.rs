//! Acoustic Constellation Fingerprinting (`Wang 2003` Shazam Landmark Engine).

pub mod constellation;
pub mod matcher;

pub use constellation::{extract_constellation_landmarks, LandmarkHash};
pub use matcher::{match_landmarks_against_index, ShazamMatch};

#[cfg(test)]
mod tests {
    use super::{extract_constellation_landmarks, match_landmarks_against_index};
    use std::f32::consts::TAU;

    fn synth_musical_track(sample_rate: u32, duration_sec: f32, base_freq: f32) -> Vec<f32> {
        let total = (sample_rate as f32 * duration_sec) as usize;
        let mut out = vec![0.0_f32; total];
        let chord_progression: [(f32, f32, f32); 6] = [
            (1.0, 1.25, 1.5),
            (1.122, 1.498, 1.682),
            (1.335, 1.682, 2.0),
            (1.498, 1.888, 2.245),
            (1.26, 1.587, 1.888),
            (1.059, 1.335, 1.782),
        ];
        for (i, sample) in out.iter_mut().enumerate() {
            let t = i as f32 / sample_rate as f32;
            let step = ((t / 0.45) as usize) % chord_progression.len();
            let step_phase = (t % 0.45) / 0.45;
            let env = (-3.8 * step_phase).exp();
            let (r1, r2, r3) = chord_progression[step];
            let f1 = base_freq * r1 * (1.0 + 0.03 * ((t * 0.7).floor()));
            let f2 = base_freq * 2.1 * r2;
            let f3 = base_freq * 3.7 * r3;
            let perc = if (i / (sample_rate as usize / 8)) % 3 == 0 && (t % 0.125) < 0.025 {
                let click_env = 1.0 - (t % 0.125) / 0.025;
                0.35 * click_env * (TAU * (1850.0 + 210.0 * (step as f32)) * t).sin()
            } else {
                0.0
            };
            *sample = env
                * (0.42 * (TAU * f1 * t).sin()
                    + 0.30 * (TAU * f2 * t).sin()
                    + 0.22 * (TAU * f3 * t).sin())
                + perc;
        }
        out
    }

    #[test]
    fn test_shazam_constellation_matches_2_5s_offset_with_noise_and_rejects_unrelated() {
        let sr = 44_100_u32;
        let master = synth_musical_track(sr, 12.0, 380.0);
        let other = synth_musical_track(sr, 12.0, 610.0);

        let master_landmarks = extract_constellation_landmarks(&master, sr);
        let other_landmarks = extract_constellation_landmarks(&other, sr);
        assert!(
            master_landmarks.len() >= 40,
            "expected rich constellation, got {}",
            master_landmarks.len()
        );
        assert!(master_landmarks.iter().all(|l| l.hash_u64 != 0));

        let mut index_hits: Vec<(u64, String, i64)> = master_landmarks
            .iter()
            .map(|l| {
                (
                    l.hash_u64,
                    "trk-master-01".to_string(),
                    i64::from(l.anchor_time_ms),
                )
            })
            .collect();
        index_hits.extend(other_landmarks.iter().map(|l| {
            (
                l.hash_u64,
                "trk-other-02".to_string(),
                i64::from(l.anchor_time_ms),
            )
        }));

        // Slice master starting at +2.500s and add -30 dB deterministic LCG white noise (amplitude ~0.0316)
        let offset_samples = (2.5_f32 * sr as f32) as usize;
        let mut rng = 0x9E37_79B9_7F4A_7C15_u64;
        let noisy_slice: Vec<f32> = master[offset_samples..]
            .iter()
            .map(|&s| {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                let unit = ((rng >> 32) as u32 as f32) / (u32::MAX as f32) * 2.0 - 1.0;
                s + unit * 0.0316
            })
            .collect();

        let query_landmarks = extract_constellation_landmarks(&noisy_slice, sr);
        let matched = match_landmarks_against_index(&query_landmarks, &index_hits)
            .expect("must identify sliced + noisy copy");

        assert_eq!(matched.track_id, "trk-master-01");
        assert!(
            matched.aligned_landmarks >= 8,
            "expected >= 8 aligned landmarks, got {}",
            matched.aligned_landmarks
        );
        assert!(
            (matched.estimated_offset_ms - 2500).abs() <= 120,
            "expected offset near 2500ms (+-120ms), got {}ms",
            matched.estimated_offset_ms
        );
        assert!(matched.confidence > 0.25);

        // Unrelated signal must return None (zero false positives)
        let unrelated = synth_musical_track(sr, 10.0, 830.0);
        let unrelated_query = extract_constellation_landmarks(&unrelated, sr);
        assert!(match_landmarks_against_index(&unrelated_query, &index_hits).is_none());
    }
}
