    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[repr(u8)]
    pub enum PitchClass {
        C = 0,
        Db = 1,
        D = 2,
        Eb = 3,
        E = 4,
        F = 5,
        Gb = 6,
        G = 7,
        Ab = 8,
        A = 9,
        Bb = 10,
        B = 11,
    }

    impl PitchClass {
        pub fn from_semitone(idx: usize) -> Self {
            match idx.rem_euclid(12) {
                0 => Self::C,
                1 => Self::Db,
                2 => Self::D,
                3 => Self::Eb,
                4 => Self::E,
                5 => Self::F,
                6 => Self::Gb,
                7 => Self::G,
                8 => Self::Ab,
                9 => Self::A,
                10 => Self::Bb,
                _ => Self::B,
            }
        }

        pub fn label(self) -> &'static str {
            match self {
                Self::C => "C",
                Self::Db => "Db",
                Self::D => "D",
                Self::Eb => "Eb",
                Self::E => "E",
                Self::F => "F",
                Self::Gb => "F#",
                Self::G => "G",
                Self::Ab => "Ab",
                Self::A => "A",
                Self::Bb => "Bb",
                Self::B => "B",
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum KeyScale {
        Major,
        Minor,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MusicalKeyResult {
        pub tonic: PitchClass,
        pub scale: KeyScale,
        pub standard_name: String,
        pub camelot: String,
        pub open_key: String,
        pub confidence: f32,
        pub tuning_cents: f32,
    }

    const BGATE_MAJ: [f64; 12] = [1.00, 0.00, 0.42, 0.00, 0.53, 0.37, 0.00, 0.77, 0.00, 0.38, 0.21, 0.30];
    const BGATE_MIN: [f64; 12] = [1.00, 0.00, 0.36, 0.39, 0.00, 0.38, 0.00, 0.74, 0.27, 0.00, 0.42, 0.23];
    const EDMA_MAJ: [f64; 12] = [1.00, 0.29, 0.50, 0.40, 0.60, 0.56, 0.32, 0.80, 0.31, 0.45, 0.42, 0.39];
    const EDMA_MIN: [f64; 12] = [1.00, 0.31, 0.44, 0.58, 0.33, 0.49, 0.29, 0.78, 0.43, 0.29, 0.53, 0.32];
    const TEMP_MAJ: [f64; 12] = [0.748, 0.060, 0.488, 0.082, 0.670, 0.460, 0.096, 0.715, 0.104, 0.366, 0.057, 0.400];
    const TEMP_MIN: [f64; 12] = [0.712, 0.084, 0.474, 0.618, 0.049, 0.460, 0.105, 0.747, 0.404, 0.067, 0.133, 0.330];
    const SHAATH_MAJ: [f64; 12] = [6.6, 2.0, 3.5, 2.3, 4.6, 4.0, 2.5, 5.2, 2.4, 3.7, 2.3, 3.4];
    const SHAATH_MIN: [f64; 12] = [6.5, 2.7, 3.5, 5.4, 2.6, 3.5, 2.5, 5.2, 4.0, 2.7, 4.3, 3.2];

    /// ¡PARCHE CRÍTICO v2.1!:
    /// 1. Recibe `sample_rate: u32` real (elimina el desplazamiento de `-1.47 semitonos` en archivos de 48 kHz).
    /// 2. Incorpora `bass_roots_12` (`50–185 Hz`) para anclar la tónica del bajo y desempatar `8A (Am)` vs `8B (C)`.
    pub fn detect_song_key(
        samples: &[f32],
        sample_rate: u32,
        fft_planner: &mut FftPlanner<f32>,
    ) -> MusicalKeyResult {
        let sr = sample_rate.max(8_000);
        let n_fft: usize = if sr >= 80_000 { 8192 } else { 4096 };
        let hop: usize = n_fft / 2;
        if samples.len() < n_fft {
            return super::camelot_wheel::build_key_result(PitchClass::C, KeyScale::Major, 0.0, 0.0);
        }

        let fft = fft_planner.plan_fft_forward(n_fft);
        let window: Vec<f32> = (0..n_fft)
            .map(|i| {
                let p = 2.0 * PI * (i as f32) / ((n_fft - 1) as f32);
                0.35875 - 0.48829 * p.cos() + 0.14128 * (2.0 * p).cos() - 0.01168 * (3.0 * p).cos()
            })
            .collect();

        let bin_hz = (sr as f64) / (n_fft as f64);
        // Inicia en 92 Hz para excluir el golpe fundamental del bombo urbano (50-80 Hz, ~Bb1 = 58 Hz = 3A)
        let min_bin = ((92.0 / bin_hz).ceil() as usize).max(2);
        let max_bin = ((3520.0 / bin_hz).floor() as usize).min(n_fft / 2 - 2);

        let num_frames = (samples.len() - n_fft) / hop + 1;
        let step = (num_frames / 550).max(1);

        let mut buf = vec![Complex { re: 0.0_f32, im: 0.0_f32 }; n_fft];
        let mut mag = vec![0.0_f64; n_fft / 2 + 1];
        let mut hpcp = [0.0_f64; HPCP_BINS];
        let mut peaks_12 = [0.0_f64; 12];
        let mut bass_roots_12 = [0.0_f64; 12];

        for f_idx in (0..num_frames).step_by(step) {
            let off = f_idx * hop;
            for i in 0..n_fft {
                buf[i] = Complex {
                    re: samples[off + i] * window[i],
                    im: 0.0,
                };
            }
            fft.process(&mut buf);

            let mut log_s = 0.0_f64;
            let mut lin_s = 0.0_f64;
            let cnt = (max_bin - min_bin + 1) as f64;
            for k in min_bin..=max_bin {
                let m = (buf[k].norm() as f64).max(1e-9);
                mag[k] = m;
                log_s += m.ln();
                lin_s += m;
            }
            mag[min_bin - 1] = (buf[min_bin - 1].norm() as f64).max(1e-9);
            mag[max_bin + 1] = (buf[max_bin + 1].norm() as f64).max(1e-9);

            let flatness = (log_s / cnt).exp() / (lin_s / cnt).max(1e-9);
            if flatness > 0.44 || (lin_s / cnt) < 1e-4 {
                continue;
            }
            let h_weight = (1.0 - flatness / 0.44).powi(2);
            let mean_m = lin_s / cnt;

            for k in min_bin..=max_bin {
                let (m0, m1, m2) = (mag[k - 1], mag[k], mag[k + 1]);
                if m1 > m0 && m1 > m2 && m1 > mean_m * 1.35 {
                    let denom = m0 - 2.0 * m1 + m2;
                    let delta = if denom.abs() > 1e-12 {
                        (0.5 * (m0 - m2) / denom).clamp(-0.5, 0.5)
                    } else {
                        0.0
                    };
                    let freq = ((k as f64) + delta) * bin_hz;
                    let amp = m1.powf(0.65) * h_weight;

                    let midi_p = 69.0 + 12.0 * (freq / 440.0).log2();
                    let pc_12 = (midi_p.round() as isize).rem_euclid(12) as usize;
                    peaks_12[pc_12] += amp;

                    // Ancla armónica de bajo en registro limpio (95–245 Hz, libre del sub-kick de 58 Hz)
                    if (95.0..=245.0).contains(&freq) {
                        bass_roots_12[pc_12] += amp;
                    }

                    for h in 1..=4 {
                        let f_sub = freq / (h as f64);
                        if f_sub < 45.0 {
                            break;
                        }
                        let decay = 0.62_f64.powi((h - 1) as i32);
                        let midi_c = 69.0 + 12.0 * (f_sub / 440.0).log2();
                        let bin_pos =
                            (midi_c * (HPCP_BINS_PER_SEMITONE as f64)).rem_euclid(HPCP_BINS as f64);
                        let center = bin_pos.round() as isize;
                        for b_off in -1..=1 {
                            let dist = (bin_pos - ((center + b_off) as f64)).abs();
                            if dist < 1.0 {
                                let w = (0.5 * std::f64::consts::PI * dist).cos().powi(2);
                                let idx =
                                    (center + b_off).rem_euclid(HPCP_BINS as isize) as usize;
                                hpcp[idx] += amp * decay * w;
                            }
                        }
                    }
                }
            }
        }

        let mut sub_bins = [0.0_f64; HPCP_BINS_PER_SEMITONE];
        for (i, &v) in hpcp.iter().enumerate() {
            sub_bins[i % HPCP_BINS_PER_SEMITONE] += v;
        }
        let (mut sin_a, mut cos_a) = (0.0_f64, 0.0_f64);
        for (k, &e) in sub_bins.iter().enumerate() {
            let ang = 2.0 * std::f64::consts::PI * (k as f64) / (HPCP_BINS_PER_SEMITONE as f64);
            sin_a += e * ang.sin();
            cos_a += e * ang.cos();
        }
        let detune_rad = sin_a.atan2(cos_a);
        let tuning_cents = ((detune_rad / (2.0 * std::f64::consts::PI)) * 100.0) as f32;
        let shift_bins =
            ((detune_rad / (2.0 * std::f64::consts::PI)) * (HPCP_BINS_PER_SEMITONE as f64)).round()
                as isize;

        let mut pcp_12 = [0.0_f64; 12];
        for st in 0..12 {
            let c = (st * HPCP_BINS_PER_SEMITONE) as isize + shift_bins;
            let b0 = (c - 1).rem_euclid(HPCP_BINS as isize) as usize;
            let b1 = c.rem_euclid(HPCP_BINS as isize) as usize;
            let b2 = (c + 1).rem_euclid(HPCP_BINS as isize) as usize;
            pcp_12[st] = 0.25 * hpcp[b0] + 0.50 * hpcp[b1] + 0.25 * hpcp[b2];
        }

        let max_pcp = pcp_12.iter().copied().fold(0.0_f64, f64::max).max(1e-12);
        for v in &mut pcp_12 {
            *v /= max_pcp;
            if *v < 0.12 {
                *v = 0.0;
            }
        }

        let max_bass = bass_roots_12.iter().copied().fold(0.0_f64, f64::max).max(1e-12);

        let mut scored: Vec<(PitchClass, KeyScale, f64)> = Vec::with_capacity(24);
        for root in 0..12 {
            let bass_bonus = 0.045 * (bass_roots_12[root] / max_bass);

            let maj = 0.35 * corr_12(&pcp_12, &BGATE_MAJ, root)
                + 0.25 * corr_12(&pcp_12, &EDMA_MAJ, root)
                + 0.25 * corr_12(&pcp_12, &TEMP_MAJ, root)
                + 0.15 * corr_12(&pcp_12, &SHAATH_MAJ, root)
                + bass_bonus;
            scored.push((PitchClass::from_semitone(root), KeyScale::Major, maj));

            let min = 0.35 * corr_12(&pcp_12, &BGATE_MIN, root)
                + 0.25 * corr_12(&pcp_12, &EDMA_MIN, root)
                + 0.25 * corr_12(&pcp_12, &TEMP_MIN, root)
                + 0.15 * corr_12(&pcp_12, &SHAATH_MIN, root)
                + bass_bonus;
            scored.push((PitchClass::from_semitone(root), KeyScale::Minor, min));
        }
        scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(Ordering::Equal));

        let (win_tonic, mut win_scale, mut win_score) = scored[0];
        let r_idx = win_tonic as usize;
        let maj3 = peaks_12[(r_idx + 4) % 12];
        let min3 = peaks_12[(r_idx + 3) % 12];
        let opp_scale = match win_scale {
            KeyScale::Major => KeyScale::Minor,
            KeyScale::Minor => KeyScale::Major,
        };
        if let Some(&(_, _, par_score)) = scored
            .iter()
            .find(|&&(t, s, _)| t == win_tonic && s == opp_scale)
        {
            if (win_score - par_score) < 0.09 {
                if (win_scale == KeyScale::Major && min3 > maj3 * 1.32)
                    || (win_scale == KeyScale::Minor && maj3 > min3 * 1.38)
                {
                    win_scale = opp_scale;
                    win_score = par_score;
                }
            }
        }

        let runner_up = scored.get(1).map(|x| x.2).unwrap_or(0.0);
        let conf = (((win_score - runner_up).max(0.0) / win_score.abs().max(1e-6)) * 2.2)
            .clamp(0.0, 1.0) as f32;

        super::camelot_wheel::build_key_result(win_tonic, win_scale, conf, tuning_cents)
    }

    fn corr_12(pcp: &[f64; 12], prof: &[f64; 12], shift: usize) -> f64 {
        let mx = pcp.iter().sum::<f64>() / 12.0;
        let my = prof.iter().sum::<f64>() / 12.0;
        let (mut num, mut dx2, mut dy2) = (0.0_f64, 0.0_f64, 0.0_f64);
        for i in 0..12 {
            let py = prof[(i + 12 - (shift % 12)) % 12];
            let dx = pcp[i] - mx;
            let dy = py - my;
            num += dx * dy;
            dx2 += dx * dx;
            dy2 += dy * dy;
        }
        let den = (dx2 * dy2).sqrt();
        if den > 1e-12 {
            num / den
        } else {
            0.0
        }
    }
