    use super::*;

    /// Envolventes multibanda auto-contenidas con su `onset_fps` exacto (`sample_rate / hop_size`).
    /// Garantiza que pistas a `44,100 Hz`, `48,000 Hz` o `96,000 Hz` produzcan exactamente el mismo BPM.
    #[derive(Debug, Clone)]
    pub struct MultibandNovelty {
        pub sub_kick: Vec<f64>,
        pub low_body: Vec<f64>,
        pub perc_clave: Vec<f64>,
        pub hi_guiro: Vec<f64>,
        pub composite: Vec<f64>,
        pub num_frames: usize,
        pub sample_rate: u32,
        pub hop_size: usize,
        pub onset_fps: f64,
        pub trim_offset_sec: f64,
        pub band_ratios: [f64; 4],
    }

    pub fn trim_leading_trailing_silence<'a>(
        samples: &'a [f32],
        sample_rate: u32,
    ) -> (&'a [f32], f64, f64) {
        let sr = sample_rate.max(8_000) as usize;
        let win = (sr / 50).max(64); // 20 ms exactos en cualquier sample_rate
        let thresh = 0.001_f32;
        let n_wins = samples.len() / win;
        if n_wins < 8 {
            return (samples, 0.0, 0.0);
        }

        let mut start_w = 0;
        let mut run = 0;
        for w in 0..n_wins {
            let sl = &samples[w * win..(w + 1) * win];
            let rms = (sl.iter().map(|&x| x * x).sum::<f32>() / (win as f32)).sqrt();
            if rms >= thresh {
                run += 1;
                if run >= 3 {
                    start_w = w.saturating_sub(2);
                    break;
                }
            } else {
                run = 0;
            }
        }

        let mut end_w = n_wins;
        run = 0;
        for w in (start_w..n_wins).rev() {
            let sl = &samples[w * win..(w + 1) * win];
            let rms = (sl.iter().map(|&x| x * x).sum::<f32>() / (win as f32)).sqrt();
            if rms >= thresh {
                run += 1;
                if run >= 3 {
                    end_w = (w + 3).min(n_wins);
                    break;
                }
            } else {
                run = 0;
            }
        }

        let s_idx = start_w * win;
        let e_idx = (end_w * win).max(s_idx).min(samples.len());
        let lead_sec = (s_idx as f64) / (sr as f64);
        let trail_sec = ((samples.len() - e_idx) as f64) / (sr as f64);
        (&samples[s_idx..e_idx], lead_sec, trail_sec)
    }

    /// Extrae las 4 bandas de HERA calculando dinámicamente los bines en Hz según `sample_rate`:
    /// - `sub_kick`: `25–130 Hz` (IIR Butterworth calibrado a `sample_rate`)
    /// - `low_body`: `130–450 Hz`
    /// - `perc_clave`: `650–3800 Hz`
    /// - `hi_guiro`: `4200–11000 Hz`
    pub fn extract_multiband_novelty(
        trimmed_pcm: &[f32],
        sample_rate: u32,
        trim_offset_sec: f64,
        fft_planner: &mut FftPlanner<f32>,
    ) -> MultibandNovelty {
        let sr = sample_rate.max(8_000);
        // Escala ventana y hop si el archivo es Hi-Res (88.2 / 96 kHz) para mantener ~170–188 FPS
        let scale_mult = if sr >= 80_000 { 2 } else { 1 };
        let fft_size = BASE_ONSET_FFT_SIZE * scale_mult;
        let hop_size = BASE_ONSET_HOP_SIZE * scale_mult;
        let onset_fps = (sr as f64) / (hop_size as f64);

        if trimmed_pcm.len() < fft_size {
            return MultibandNovelty {
                sub_kick: Vec::new(),
                low_body: Vec::new(),
                perc_clave: Vec::new(),
                hi_guiro: Vec::new(),
                composite: Vec::new(),
                num_frames: 0,
                sample_rate: sr,
                hop_size,
                onset_fps,
                trim_offset_sec,
                band_ratios: [0.25; 4],
            };
        }

        let num_frames = (trimmed_pcm.len() - fft_size) / hop_size + 1;

        // 1. Banda `sub_kick` (25–130 Hz) vía filtro IIR parametrizado con `sr` real
        let sub_kick_pcm = isolate_sub_kick_band_iir(trimmed_pcm, sr as f64);
        let mut sub_env = vec![0.0_f64; num_frames];
        for f in 0..num_frames {
            let start = f * hop_size;
            let slice = &sub_kick_pcm[start..start + fft_size];
            let energy =
                slice.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>() / (fft_size as f64);
            sub_env[f] = (1.0 + 4000.0 * energy.sqrt()).ln();
        }

        let mut raw_sub = vec![0.0_f64; num_frames];
        for f in 1..num_frames {
            raw_sub[f] = (sub_env[f] - sub_env[f - 1]).max(0.0);
        }

        // 2. Bines exactos calculados en función de `sr / fft_size` (idénticos en Hz para 44.1k y 48k)
        let bin_hz = (sr as f64) / (fft_size as f64);
        let n_bins = fft_size / 2 + 1;
        let body_lo = ((130.0 / bin_hz).round() as usize).max(2);
        let body_hi = ((450.0 / bin_hz).round() as usize).min(n_bins - 1);
        let clave_lo = ((650.0 / bin_hz).round() as usize).max(body_hi + 1);
        let clave_hi = ((3800.0 / bin_hz).round() as usize).min(n_bins - 1);
        let guiro_lo = ((4200.0 / bin_hz).round() as usize).max(clave_hi + 1);
        let guiro_hi = ((11000.0 / bin_hz).round() as usize).min(n_bins - 1);

        let fft = fft_planner.plan_fft_forward(fft_size);
        let hann: Vec<f32> = (0..fft_size)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * (i as f32) / (fft_size as f32)).cos()))
            .collect();

        let mut fft_buf = vec![Complex { re: 0.0_f32, im: 0.0_f32 }; fft_size];
        let mut prev_log_mag = vec![0.0_f32; n_bins];

        let mut raw_body = vec![0.0_f64; num_frames];
        let mut raw_clave = vec![0.0_f64; num_frames];
        let mut raw_guiro = vec![0.0_f64; num_frames];

        for f in 0..num_frames {
            let offset = f * hop_size;
            for i in 0..fft_size {
                fft_buf[i] = Complex {
                    re: trimmed_pcm[offset + i] * hann[i],
                    im: 0.0,
                };
            }
            fft.process(&mut fft_buf);

            let mut flux_body = 0.0_f32;
            let mut flux_clave = 0.0_f32;
            let mut flux_guiro = 0.0_f32;

            for k in body_lo..=guiro_hi {
                let log_m = (1.0 + 80.0 * fft_buf[k].norm()).ln();
                let diff = (log_m - prev_log_mag[k]).max(0.0);
                prev_log_mag[k] = log_m;

                if f > 0 {
                    if k <= body_hi {
                        flux_body += diff;
                    } else if k >= clave_lo && k <= clave_hi {
                        flux_clave += diff;
                    } else if k >= guiro_lo {
                        flux_guiro += diff;
                    }
                }
            }
            raw_body[f] = flux_body as f64;
            raw_clave[f] = flux_clave as f64;
            raw_guiro[f] = flux_guiro as f64;
        }

        let n_body = (body_hi.saturating_sub(body_lo) + 1).max(1) as f64;
        let n_clave = (clave_hi.saturating_sub(clave_lo) + 1).max(1) as f64;
        let n_guiro = (guiro_hi.saturating_sub(guiro_lo) + 1).max(1) as f64;

        let sum_s = raw_sub.iter().sum::<f64>();
        let sum_b = raw_body.iter().sum::<f64>() / n_body;
        let sum_c = raw_clave.iter().sum::<f64>() / n_clave;
        let sum_g = raw_guiro.iter().sum::<f64>() / n_guiro;
        let total_e = (sum_s + sum_b + sum_c + sum_g).max(1e-9);
        let band_ratios = [
            sum_s / total_e,
            sum_b / total_e,
            sum_c / total_e,
            sum_g / total_e,
        ];

        let half_win = ((0.18 * onset_fps).round() as usize).max(12);
        let sub_kick = normalize_novelty_curve(&raw_sub, half_win);
        let low_body = normalize_novelty_curve(&raw_body, half_win);
        let perc_clave = normalize_novelty_curve(&raw_clave, half_win);
        let hi_guiro = normalize_novelty_curve(&raw_guiro, half_win);

        let mut composite = Vec::with_capacity(num_frames);
        for i in 0..num_frames {
            composite.push(
                0.40 * sub_kick[i]
                    + 0.24 * low_body[i]
                    + 0.24 * perc_clave[i]
                    + 0.12 * hi_guiro[i],
            );
        }

        MultibandNovelty {
            sub_kick,
            low_body,
            perc_clave,
            hi_guiro,
            composite,
            num_frames,
            sample_rate: sr,
            hop_size,
            onset_fps,
            trim_offset_sec,
            band_ratios,
        }
    }

    fn isolate_sub_kick_band_iir(input: &[f32], sr: f64) -> Vec<f32> {
        let mut out = Vec::with_capacity(input.len());
        let rc_hp = 1.0 / (2.0 * std::f64::consts::PI * 25.0);
        let dt = 1.0 / sr;
        let alpha_hp = rc_hp / (rc_hp + dt);

        let omega = 2.0 * std::f64::consts::PI * 130.0 / sr;
        let sn = omega.sin();
        let cs = omega.cos();
        let alpha_lp = sn / std::f64::consts::SQRT_2;
        let a0 = 1.0 + alpha_lp;
        let b0 = ((1.0 - cs) * 0.5) / a0;
        let b1 = (1.0 - cs) / a0;
        let b2 = b0;
        let a1 = (-2.0 * cs) / a0;
        let a2 = (1.0 - alpha_lp) / a0;

        let mut prev_in = 0.0_f64;
        let mut prev_hp = 0.0_f64;
        let (mut x1, mut x2, mut y1, mut y2) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);

        for &s in input {
            let x = s as f64;
            let hp = alpha_hp * (prev_hp + x - prev_in);
            prev_in = x;
            prev_hp = hp;

            let lp = b0 * hp + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
            x2 = x1;
            x1 = hp;
            y2 = y1;
            y1 = lp;
            out.push(lp as f32);
        }
        out
    }

    fn normalize_novelty_curve(raw: &[f64], half_win: usize) -> Vec<f64> {
        let n = raw.len();
        if n == 0 {
            return Vec::new();
        }
        let mut prefix = vec![0.0_f64; n + 1];
        for i in 0..n {
            prefix[i + 1] = prefix[i] + raw[i];
        }

        let mut local = vec![0.0_f64; n];
        for i in 0..n {
            let l = i.saturating_sub(half_win);
            let r = (i + half_win + 1).min(n);
            let mean = (prefix[r] - prefix[l]) / ((r - l) as f64);
            local[i] = (raw[i] - mean).max(0.0);
        }

        let variance = local.iter().map(|v| v * v).sum::<f64>() / (n as f64);
        let std = variance.sqrt().max(1e-6);
        for v in &mut local {
            *v /= std;
        }
        local
    }