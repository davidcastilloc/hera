use super::consensus_ai::analyze_bpm_consensus_v2;
use super::hints::{
    extract_bpm_from_filename, parse_virtualdj_bpm_value, BpmHint, HintSource, VirtualDjDbIndex,
};
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

        let kick = if step.is_multiple_of(4) && sp < 0.085 {
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

#[test]
fn test_wiener_khinchin_acf_parity_under_1e10() {
    let win_len = 1024_usize;
    let min_lag = 40_usize;
    let max_acf_lag = 700_usize;
    let slice: Vec<f64> = (0..win_len)
        .map(|i| ((i as f64) * 0.13).sin().abs() + 0.3 * ((i as f64) * 0.05).cos())
        .collect();
    let norm = slice.iter().map(|x| x * x).sum::<f64>().max(1e-9);

    let n_fft = (win_len * 2).next_power_of_two();
    let mut planner = FftPlanner::<f64>::new();
    let fft_fwd = planner.plan_fft_forward(n_fft);
    let fft_inv = planner.plan_fft_inverse(n_fft);
    let mut fft_buf = vec![
        Complex {
            re: 0.0_f64,
            im: 0.0_f64
        };
        n_fft
    ];
    let mut acf_fft = vec![0.0_f64; max_acf_lag + 1];

    super::qm_comb_tracker::linear_acf_wiener_khinchin(
        &slice,
        norm,
        min_lag,
        max_acf_lag,
        n_fft,
        fft_fwd.as_ref(),
        fft_inv.as_ref(),
        &mut fft_buf,
        &mut acf_fft,
    );

    for l in min_lag..=max_acf_lag {
        let mut sc = 0.0_f64;
        for i in l..win_len {
            sc += slice[i] * slice[i - l];
        }
        let acf_loop = (sc / norm).max(0.0);
        assert!(
            (acf_loop - acf_fft[l]).abs() < 1e-10,
            "lag {l}: loop={acf_loop} fft={}",
            acf_fft[l]
        );
    }
}

#[test]
fn symphonia_decodes_48khz_wav_without_ffmpeg_fallback() {
    let dir = std::env::temp_dir();
    let wav_path = dir.join("hera_test_48khz_h1.wav");
    let sr: u32 = 48_000;
    let num_samples = sr * 4;
    let data_bytes = num_samples * 2;
    let mut buf = Vec::with_capacity(44 + data_bytes as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16_u32.to_le_bytes());
    buf.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1_u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sr.to_le_bytes());
    buf.extend_from_slice(&(sr * 2).to_le_bytes());
    buf.extend_from_slice(&2_u16.to_le_bytes());
    buf.extend_from_slice(&16_u16.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_bytes.to_le_bytes());
    for i in 0..num_samples {
        let t = (i as f32) / (sr as f32);
        let s = ((2.0 * std::f32::consts::PI * 440.0 * t).sin() * 16000.0) as i16;
        buf.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(&wav_path, &buf).expect("write temp wav");
    let decoded =
        super::audio_ingest::load_audio_mono_native_sr(&wav_path, None).expect("decode 48kHz wav");
    let _ = std::fs::remove_file(&wav_path);
    assert!(!decoded.used_ffmpeg_fallback);
    assert_eq!(decoded.sample_rate, 48_000);
}

#[test]
fn test_salsa_4_3x_vdj_hint_cross_audit_vs_tecnomerengue_protection() {
    let sr = 44_100_u32;
    let dur = 24.0_f64;
    let n = (dur * sr as f64) as usize;
    let beat_salsa = 60.0 / 96.0;
    let bar_salsa = beat_salsa * 4.0;
    let mut salsa_pcm = vec![0.0_f32; n];
    for (i, out) in salsa_pcm.iter_mut().enumerate() {
        let t = i as f64 / sr as f64;
        let beat_num = (t / bar_salsa).fract() * 4.0;
        let d25 = (beat_num - 2.5).abs();
        let d40 = (beat_num - 3.95).abs().min((beat_num + 0.05).abs());
        let env_b = if d25 < 0.16 {
            (1.0 - d25 / 0.16).powi(2)
        } else if d40 < 0.16 {
            (1.0 - d40 / 0.16).powi(2)
        } else {
            0.0
        };
        let bass = (0.65 * (2.0 * std::f64::consts::PI * 145.0 * t).sin()
            + 0.35 * (2.0 * std::f64::consts::PI * 220.0 * t).sin())
            * env_b
            * 0.65;
        let subpos = beat_num.fract();
        let clave_pos = (beat_num * 4.0 / 3.0).fract();
        let campana = if subpos < 0.14 {
            (1.0 - subpos / 0.14).powi(3) * (2.0 * std::f64::consts::PI * 1150.0 * t).sin() * 0.26
        } else {
            0.0
        };
        let tresillo = if clave_pos < 0.14 {
            (1.0 - clave_pos / 0.14).powi(3)
                * (2.0 * std::f64::consts::PI * 2350.0 * t).sin()
                * 0.44
        } else {
            0.0
        };
        *out = (bass + campana + tresillo).clamp(-0.98, 0.98) as f32;
    }

    let mut planner = FftPlanner::<f32>::new();
    let nov_salsa = extract_multiband_novelty(&salsa_pcm, sr, 0.0, &mut planner);
    let misfired_vdj_hint = BpmHint {
        bpm: 128.0,
        source: HintSource::VirtualDjDatabase,
    };
    let res_salsa = analyze_bpm_consensus_v2(&nov_salsa, Some(misfired_vdj_hint));
    assert!(
        (res_salsa.bpm - 96.0).abs() <= 1.0,
        "Salsa 96 BPM with 128 BPM VDJ 4/3x hint must resolve to ~96 BPM, got {:.2}",
        res_salsa.bpm
    );

    // Tecnomerengue at 142 BPM with dense 16th-note guira must stay ~142 BPM (never fold to 71)
    let beat_tec = 60.0 / 142.0;
    let mut tec_pcm = vec![0.0_f32; n];
    let mut rng: u64 = 0x7EC0_1420_CAFE_u64;
    for (i, out) in tec_pcm.iter_mut().enumerate() {
        let t = i as f64 / sr as f64;
        let bf = (t / beat_tec).fract();
        let sf = (t / beat_tec * 4.0).fract();
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let white = (((rng >> 32) as u32 as f64) / (u32::MAX as f64)) * 2.0 - 1.0;
        let guira = white
            * (2.0 * std::f64::consts::PI * 6500.0 * t).sin()
            * if sf < 0.40 { 1.0 - sf / 0.40 } else { 0.08 }
            * 0.45;
        let tambora = if bf < 0.16 {
            (1.0 - bf / 0.16) * (2.0 * std::f64::consts::PI * 75.0 * t).sin() * 0.60
        } else {
            0.0
        };
        *out = (guira + tambora).clamp(-0.98, 0.98) as f32;
    }
    let nov_tec = extract_multiband_novelty(&tec_pcm, sr, 0.0, &mut planner);
    let half_tag_hint = BpmHint {
        bpm: 71.0,
        source: HintSource::EmbeddedTag,
    };
    let res_tec = analyze_bpm_consensus_v2(&nov_tec, Some(half_tag_hint));
    assert!(
        (res_tec.bpm - 142.0).abs() <= 1.2,
        "Tecnomerengue 142 BPM must stay ~142 BPM even with 71 BPM half-time tag, got {:.2}",
        res_tec.bpm
    );
}
