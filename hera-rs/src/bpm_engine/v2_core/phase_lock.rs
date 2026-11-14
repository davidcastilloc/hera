    use super::multiband_onset::MultibandNovelty;
    use super::*;

    /// **PARCHE v2.2 (`block_incoherent_phase_lock`):**
    /// Evalúa coherencia de fase en bloques locales de **12 segundos** (avance de 6 s), permitiendo
    /// que cada bloque tenga su propia fase independiente tras *cortes* de reggaetón, mambos de
    /// merengue o ediciones de VideoDJ (`.mp4`/`.webm`).
    /// Esto erradica la cancelación destructiva que desplazaba `126 → 125/127` y `129 → 128 BPM`.
    pub fn fine_grid_phase_lock_and_beats(
        approx_bpm: f64,
        nov: &MultibandNovelty,
        prefer_kick_anchor: bool,
    ) -> (f64, f64, Vec<f64>, f64) {
        let fps = nov.onset_fps;
        let duration_sec = (nov.num_frames as f64) / fps;
        if nov.num_frames < 64 || approx_bpm <= 30.0 {
            return (approx_bpm, nov.trim_offset_sec, Vec::new(), 0.0);
        }

        let mut sparse_peaks: Vec<(f64, f64)> = Vec::new();
        for i in 1..(nov.num_frames - 1) {
            let val = if prefer_kick_anchor {
                0.62 * nov.sub_kick[i] + 0.22 * nov.low_body[i] + 0.16 * nov.perc_clave[i]
            } else {
                nov.composite[i]
            };
            if val > 0.85 {
                let blend = |k: usize| {
                    if prefer_kick_anchor {
                        0.62 * nov.sub_kick[k] + 0.22 * nov.low_body[k] + 0.16 * nov.perc_clave[k]
                    } else {
                        nov.composite[k]
                    }
                };
                let (prev, next) = (blend(i - 1), blend(i + 1));
                if val >= prev && val >= next {
                    let denom = prev - 2.0 * val + next;
                    let delta = if denom.abs() > 1e-12 {
                        (0.5 * (prev - next) / denom).clamp(-0.5, 0.5)
                    } else {
                        0.0
                    };
                    let t_sec = ((i as f64) + delta) / fps;
                    sparse_peaks.push((t_sec, val));
                }
            }
        }

        if sparse_peaks.len() < 8 {
            return (approx_bpm, nov.trim_offset_sec, Vec::new(), 0.0);
        }

        // Bloques de 28.0 s (avance 14.0 s): 2.3x más nitidez espectral e inmune a cortes vía peso coh^2
        let block_sec = 20.0_f64.min((duration_sec * 0.38).max(8.0));
        let step_sec = (block_sec * 0.5).max(4.0);
        let mut blocks: Vec<(Vec<(f64, f64)>, f64)> = Vec::new();
        let mut t_start = 0.0_f64;
        while t_start < duration_sec {
            let t_end = t_start + block_sec;
            let sub: Vec<(f64, f64)> = sparse_peaks
                .iter()
                .copied()
                .filter(|&(t, _)| t >= t_start && t < t_end)
                .collect();
            if sub.len() >= 6 {
                let w_sum: f64 = sub.iter().map(|x| x.1).sum();
                blocks.push((sub, w_sum));
            }
            t_start += step_sec;
        }
        if blocks.is_empty() {
            let w_sum: f64 = sparse_peaks.iter().map(|x| x.1).sum::<f64>().max(1e-9);
            blocks.push((sparse_peaks.clone(), w_sum));
        }

        let total_block_weight: f64 = blocks.iter().map(|b| b.1).sum::<f64>().max(1e-9);

        // Barrido amplio ±3.2 BPM (paso 0.08) + barrido fino ±0.25 BPM (paso 0.01)
        let mut best_bpm = approx_bpm;
        let mut best_coherence = -1.0_f64;

        let lo_coarse = (approx_bpm - 3.2).max(45.0);
        let hi_coarse = (approx_bpm + 3.2).min(225.0);
        let mut b = lo_coarse;
        while b <= hi_coarse {
            let coh = windowed_block_coherence(&blocks, b, total_block_weight);
            if coh > best_coherence {
                best_coherence = coh;
                best_bpm = b;
            }
            b += 0.08;
        }

        let lo_fine = (best_bpm - 0.24).max(45.0);
        let hi_fine = (best_bpm + 0.24).min(225.0);
        let mut bf = (lo_fine * 100.0).round() / 100.0;
        while bf <= hi_fine {
            let coh = windowed_block_coherence(&blocks, bf, total_block_weight);
            if coh > best_coherence {
                best_coherence = coh;
                best_bpm = bf;
            }
            bf += 0.01;
        }

        // Regla de anclaje entero por bloque estable (24 beats = 12 s) estilo Mixxx
        let nearest_int = best_bpm.round();
        let int_coh = windowed_block_coherence(&blocks, nearest_int, total_block_weight);
        let beats_per_block = (block_sec * best_bpm / 60.0).max(1.0);
        let block_drift_sec =
            beats_per_block * ((60.0 / best_bpm) - (60.0 / nearest_int)).abs();

        if block_drift_sec <= MIXXX_BLOCK_MAX_PHASE_ERR_SEC && int_coh >= best_coherence * 0.975 {
            best_bpm = nearest_int;
            best_coherence = int_coh;
        } else {
            best_bpm = (best_bpm * 100.0).round() / 100.0;
        }

        // Ancla la fase inicial sobre el primer bloque energético estable de la canción
        let anchor_block = blocks
            .iter()
            .take(4)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
            .unwrap_or(&blocks[0]);
        let (_, best_phase_rad) =
            single_block_phase_coherence(&anchor_block.0, best_bpm, anchor_block.1);

        let period_sec = 60.0 / best_bpm;
        let phase_offset_local =
            (best_phase_rad.rem_euclid(2.0 * std::f64::consts::PI) / (2.0 * std::f64::consts::PI))
                * period_sec;

        let mut beats_abs = Vec::new();
        let mut t = phase_offset_local;
        while t < duration_sec {
            beats_abs.push(nov.trim_offset_sec + t);
            t += period_sec;
        }
        let first_beat_sec = beats_abs.first().copied().unwrap_or(nov.trim_offset_sec);

        (best_bpm, first_beat_sec, beats_abs, best_coherence)
    }

    #[inline]
    fn windowed_block_coherence(
        blocks: &[(Vec<(f64, f64)>, f64)],
        bpm: f64,
        total_weight: f64,
    ) -> f64 {
        let mut weighted_sum = 0.0_f64;
        for (peaks, w_sum) in blocks {
            let (coh, _) = single_block_phase_coherence(peaks, bpm, *w_sum);
            let c2 = coh * coh;
            weighted_sum += (c2 * c2) * (*w_sum);
        }
        (weighted_sum / total_weight).powf(0.25)
    }

    #[inline]
    fn single_block_phase_coherence(
        peaks: &[(f64, f64)],
        bpm: f64,
        weight_sum: f64,
    ) -> (f64, f64) {
        let two_pi_f = 2.0 * std::f64::consts::PI * (bpm / 60.0);
        let (mut re1, mut im1, mut re2, mut im2) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);

        for &(t, w) in peaks {
            let theta = two_pi_f * t;
            let (s1, c1) = theta.sin_cos();
            re1 += w * c1;
            im1 += w * s1;
            let (s2, c2) = (2.0 * theta).sin_cos();
            re2 += w * c2;
            im2 += w * s2;
        }

        let denom = weight_sum.max(1e-9);
        let r1 = (re1 * re1 + im1 * im1).sqrt() / denom;
        let r2 = (re2 * re2 + im2 * im2).sqrt() / denom;
        (0.78 * r1 + 0.22 * r2, im1.atan2(re1))
    }
}

// ============================================================================
// 5. SUBMÓDULO `latin_heuristics` — MATRIZ 16-PASOS + VETO 4/4 + SALSA VS DEMBOW
// ============================================================================
