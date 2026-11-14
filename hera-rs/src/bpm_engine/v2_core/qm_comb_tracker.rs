    use super::multiband_onset::MultibandNovelty;
    use super::*;

    #[derive(Debug, Clone)]
    pub struct QmTempoAnalysis {
        pub top_comb_peaks: Vec<(f64, f64)>,
        pub viterbi_bpm: f64,
        pub sub_kick_comb_by_lag: Vec<f64>,
        pub composite_comb_by_lag: Vec<f64>,
        pub min_lag: usize,
        pub max_lag: usize,
    }

    pub fn analyze_qm_comb_tempogram(nov: &MultibandNovelty) -> QmTempoAnalysis {
        let fps = nov.onset_fps;
        let min_lag = ((60.0 * fps) / 215.0).floor().max(12.0) as usize;
        let max_lag = ((60.0 * fps) / 55.0).ceil() as usize;
        let num_lags = max_lag + 1;

        // Ventana de ~6 segundos escalada dinámicamente a `nov.onset_fps`
        let target_win = ((6.0 * fps).round() as usize).max(max_lag * 2 + 16);
        let win_len = target_win.min(nov.num_frames);
        let win_hop = (win_len / 4).max(64);

        if win_len <= max_lag + 4 {
            return QmTempoAnalysis {
                top_comb_peaks: vec![(120.0, 0.1)],
                viterbi_bpm: 120.0,
                sub_kick_comb_by_lag: vec![0.0; num_lags],
                composite_comb_by_lag: vec![0.0; num_lags],
                min_lag,
                max_lag,
            };
        }

        let num_wins = ((nov.num_frames - win_len) / win_hop) + 1;
        let beta = (60.0 * fps) / 120.0;

        let mut tempogram = vec![vec![0.0_f64; num_lags]; num_wins];
        let mut global_comp_comb = vec![0.0_f64; num_lags];
        let mut global_sub_comb = vec![0.0_f64; num_lags];

        let max_acf_lag = (max_lag * 4).min(win_len - 1);
        let mut acf_comp = vec![0.0_f64; max_acf_lag + 1];
        let mut acf_sub = vec![0.0_f64; max_acf_lag + 1];

        for w in 0..num_wins {
            let start = w * win_hop;
            let comp_slice = &nov.composite[start..start + win_len];
            let sub_slice = &nov.sub_kick[start..start + win_len];

            let comp_norm = comp_slice.iter().map(|x| x * x).sum::<f64>().max(1e-9);
            let sub_norm = sub_slice.iter().map(|x| x * x).sum::<f64>().max(1e-9);

            if comp_norm < 1e-4 {
                continue;
            }

            for l in min_lag..=max_acf_lag {
                let mut sc = 0.0_f64;
                let mut ss = 0.0_f64;
                for i in l..win_len {
                    sc += comp_slice[i] * comp_slice[i - l];
                    ss += sub_slice[i] * sub_slice[i - l];
                }
                acf_comp[l] = (sc / comp_norm).max(0.0);
                acf_sub[l] = (ss / sub_norm).max(0.0);
            }

            for lag in min_lag..=max_lag {
                let mut comb_c = 0.0_f64;
                let mut comb_s = 0.0_f64;
                for m in 1..=4_usize {
                    let center = lag * m;
                    if center > max_acf_lag {
                        break;
                    }
                    let weight_m = 1.0 / (m as f64);
                    let width = (m - 1).min(2);
                    let mut local_c = 0.0_f64;
                    let mut local_s = 0.0_f64;
                    let mut w_norm = 0.0_f64;
                    for d in -(width as isize)..=(width as isize) {
                        let idx = (center as isize + d) as usize;
                        if idx >= min_lag && idx <= max_acf_lag {
                            let tri = 1.0 - (d.unsigned_abs() as f64) / ((width + 1) as f64);
                            local_c += acf_comp[idx] * tri;
                            local_s += acf_sub[idx] * tri;
                            w_norm += tri;
                        }
                    }
                    if w_norm > 0.0 {
                        comb_c += weight_m * (local_c / w_norm);
                        comb_s += weight_m * (local_s / w_norm);
                    }
                }

                let l_f = lag as f64;
                let rayleigh = (l_f / (beta * beta)) * (-0.5 * (l_f / beta).powi(2)).exp();
                let mild_prior = 0.65 + 0.35 * (rayleigh / (0.6065 / beta));

                let weighted_c = comb_c * mild_prior;
                tempogram[w][lag] = weighted_c;
                global_comp_comb[lag] += weighted_c;
                global_sub_comb[lag] += comb_s;
            }
        }

        let viterbi_lag_f = decode_viterbi_tempo_path(&tempogram, min_lag, max_lag);
        let viterbi_bpm = (60.0 * fps) / viterbi_lag_f;

        let mut peaks: Vec<(f64, f64)> = Vec::new();
        let max_g = global_comp_comb[min_lag..=max_lag]
            .iter()
            .copied()
            .fold(1e-9_f64, f64::max);

        for lag in (min_lag + 1)..max_lag {
            let y0 = global_comp_comb[lag - 1];
            let y1 = global_comp_comb[lag];
            let y2 = global_comp_comb[lag + 1];
            if y1 > y0 && y1 >= y2 && y1 > max_g * 0.22 {
                let denom = y0 - 2.0 * y1 + y2;
                let delta = if denom.abs() > 1e-12 {
                    (0.5 * (y0 - y2) / denom).clamp(-0.5, 0.5)
                } else {
                    0.0
                };
                let exact_lag = (lag as f64) + delta;
                let bpm = (60.0 * fps) / exact_lag;
                peaks.push((bpm, y1 / max_g));
            }
        }
        peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        peaks.truncate(6);
        if peaks.is_empty() {
            peaks.push((viterbi_bpm, 1.0));
        }

        QmTempoAnalysis {
            top_comb_peaks: peaks,
            viterbi_bpm,
            sub_kick_comb_by_lag: global_sub_comb,
            composite_comb_by_lag: global_comp_comb,
            min_lag,
            max_lag,
        }
    }

    fn decode_viterbi_tempo_path(tempogram: &[Vec<f64>], min_lag: usize, max_lag: usize) -> f64 {
        let n_wins = tempogram.len();
        let num_lags = max_lag + 1;
        let mut dp = vec![0.0_f64; num_lags];
        let mut back = vec![vec![min_lag; num_lags]; n_wins];

        for lag in min_lag..=max_lag {
            dp[lag] = tempogram[0][lag];
        }

        let sigma_step = 2.2_f64;
        for w in 1..n_wins {
            let mut next_dp = vec![-1e18_f64; num_lags];
            for curr in min_lag..=max_lag {
                let lo = curr.saturating_sub(8).max(min_lag);
                let hi = (curr + 8).min(max_lag);
                let mut best_prev = curr;
                let mut best_val = -1e18_f64;
                for prev in lo..=hi {
                    let diff = (curr as f64) - (prev as f64);
                    let trans_pen = -0.08 * (diff / sigma_step).powi(2);
                    let cand = dp[prev] + trans_pen;
                    if cand > best_val {
                        best_val = cand;
                        best_prev = prev;
                    }
                }
                next_dp[curr] = best_val + tempogram[w][curr];
                back[w][curr] = best_prev;
            }
            dp = next_dp;
        }

        let mut curr = (min_lag..=max_lag)
            .max_by(|&a, &b| dp[a].partial_cmp(&dp[b]).unwrap_or(Ordering::Equal))
            .unwrap_or((min_lag + max_lag) / 2);

        // ¡PARCHE v2.1!: Interpolación parabólica por ventana antes de la mediana (elimina saltos de 1.5 BPM)
        let mut path_lags = Vec::with_capacity(n_wins);
        for w in (0..n_wins).rev() {
            let refined = if curr > min_lag && curr < max_lag {
                let y0 = tempogram[w][curr - 1];
                let y1 = tempogram[w][curr];
                let y2 = tempogram[w][curr + 1];
                let denom = y0 - 2.0 * y1 + y2;
                let delta = if denom.abs() > 1e-12 {
                    (0.5 * (y0 - y2) / denom).clamp(-0.5, 0.5)
                } else {
                    0.0
                };
                (curr as f64) + delta
            } else {
                curr as f64
            };
            path_lags.push(refined);
            curr = back[w][curr];
        }
        super::median_f64(&mut path_lags)
    }

}

// ============================================================================
// 4B. SUBMÓDULO `phase_lock` — BLOQUEO FINO DE FASE (28 s, coh²) + SNAP MIXXX
// ============================================================================
