    use super::consensus_ai::analyze_bpm_consensus_v2;
    use super::hints::{extract_bpm_from_filename, parse_virtualdj_bpm_value, VirtualDjDbIndex};
    use super::key_engine::detect_song_key;
    use super::multiband_onset::extract_multiband_novelty;
    use super::*;

    /// Caso exacto de regresión v2.1: Afro/Tech House a **48,000 Hz** (`Fisher 125 BPM / 7B F major`).
    /// Antes daba `114.85 BPM / 4A` por asumir `44,100 Hz`; ahora debe clavar `125.00 BPM / 7B`.
    #[test]
    fn test_afro_tech_house_48khz_no_sample_rate_bias() {
        let sr = 48_000_u32;
        let duration_sec = 20.0_f64;
        let n = (duration_sec * (sr as f64)) as usize;
        let mut pcm = vec![0.0_f32; n];
        let beat_sec = 60.0 / 125.0;

        // F major: F2 bajo (MIDI 41) + acorde F4-A4-C5 (MIDI 65, 69, 72)
        for (i, sample) in pcm.iter_mut().enumerate() {
            let t = (i as f64) / (sr as f64);
            let beat_phase = t.rem_euclid(beat_sec);
            let kick = if beat_phase < 0.08 {
                0.72 * (-beat_phase * 42.0).exp()
                    * (2.0 * std::f64::consts::PI * 58.0 * beat_phase).sin()
            } else {
                0.0
            };

            let mut harm = 0.0_f64;
            for &(m, w) in &[(41.0_f64, 0.9), (65.0, 0.8), (69.0, 0.65), (72.0, 0.7)] {
                let f = 440.0 * 2.0_f64.powf((m - 69.0) / 12.0);
                harm += w * (2.0 * std::f64::consts::PI * f * t).sin();
            }
            *sample = (kick + 0.18 * harm).clamp(-0.98, 0.98) as f32;
        }

        let mut planner = FftPlanner::<f32>::new();
        let nov = extract_multiband_novelty(&pcm, sr, 0.0, &mut planner);
        let res = analyze_bpm_consensus_v2(&nov, None);
        let key = detect_song_key(&pcm, sr, &mut planner);

        assert!(
            (res.bpm - 125.0).abs() <= 0.05,
            "Afro/Tech House @ 48 kHz debe dar 125.00 BPM (no 114.85), dio {:.2}",
            res.bpm
        );
        assert_eq!(key.camelot, "7B");
    }

    /// Caso exacto de regresión v2.1: Pista de `126.00 BPM` (`MUJERON` / `Calle Ciega`) con un
    /// corte de medio compás a mitad del tema que antes partía el pico global en `125.00` y `127.00`.
    #[test]
    fn test_merengueton_126_with_phase_flip_cut_locks_exact_126() {
        let sr = 44_100_u32;
        let duration_sec = 28.0_f64;
        let n = (duration_sec * (sr as f64)) as usize;
        let mut pcm = vec![0.0_f32; n];
        let beat_sec = 60.0 / 126.0;

        for (i, sample) in pcm.iter_mut().enumerate() {
            let t = (i as f64) / (sr as f64);
            // En t = 14.0s introduce un salto de medio beat (180° inversión de fase global)
            let shifted_t = if t < 14.0 { t } else { t + beat_sec * 0.5 };
            let bar_sec = beat_sec * 4.0;
            let bar_pos = shifted_t.rem_euclid(bar_sec);
            let step_sec = bar_sec / 16.0;
            let step = (bar_pos / step_sec).floor() as usize;
            let sp = bar_pos - (step as f64) * step_sec;

            let kick = if step % 4 == 0 && sp < 0.085 {
                0.70 * (-sp * 40.0).exp() * (2.0 * std::f64::consts::PI * 60.0 * sp).sin()
            } else {
                0.0
            };
            let tri_phase = bar_pos.rem_euclid(bar_sec / 3.0);
            let mambo = if tri_phase < 0.055 {
                0.50 * (-tri_phase * 55.0).exp()
                    * (2.0 * std::f64::consts::PI * 1500.0 * tri_phase).sin()
            } else {
                0.0
            };
            *sample = (kick + mambo).clamp(-0.98, 0.98) as f32;
        }

        let mut planner = FftPlanner::<f32>::new();
        let nov = extract_multiband_novelty(&pcm, sr, 0.0, &mut planner);
        let res = analyze_bpm_consensus_v2(&nov, None);

        assert!(
            (res.bpm - 126.0).abs() <= 0.05,
            "Con inversión de fase a mitad de canción debe clavar 126.00 (no 125/127), dio {:.2}",
            res.bpm
        );
    }

    #[test]
    fn test_virtualdj_xml_and_filename_hints() {
        let vdj_xml = r#"
        <VirtualDJ_Database Version="2026">
          <Song FilePath="/Music/Latin/Reggi El Autentico - MUJERON.mp4" FileSize="14200123">
            <Scan Version="801" Bpm="0.476190" Key="F" />
          </Song>
        </VirtualDJ_Database>
        "#;
        let db = VirtualDjDbIndex::from_xml_str(vdj_xml);
        let looked = db
            .lookup_bpm(Path::new("Reggi El Autentico - MUJERON.mp4"))
            .unwrap();
        assert!((looked - 126.0).abs() < 0.05);
        assert_eq!(parse_virtualdj_bpm_value("0.480000"), Some(125.0));
        assert_eq!(
            extract_bpm_from_filename(Path::new("03 - ella es - tony vega [82 BPM - 8B].mp3")),
            Some(82.0)
        );
    }
