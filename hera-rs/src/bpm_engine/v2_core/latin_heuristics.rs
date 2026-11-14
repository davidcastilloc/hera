    use super::multiband_onset::MultibandNovelty;
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum GrooveStyle {
        FourOnTheFloorClub,
        UrbanLatinFourFourHybrid,
        ReggaetonDembow,
        SalsaTumbao,
        MerengueDrive,
        BachataGroove,
        GeneralGroove,
    }

    #[derive(Debug, Clone)]
    pub struct GrooveSignature {
        pub style: GrooveStyle,
        pub sub_kick_phase_score: f64,
        pub kick_quarter_hit_ratio: f64,
        pub salsa_tumbao_ratio: f64,
        pub dembow_pattern_score: f64,
        pub guiro_sixteenth_density: f64,
        pub clave_tresillo_ratio: f64,
        pub veto_three_quarter_alias: bool,
    }

    pub fn evaluate_groove_at_bpm(nov: &MultibandNovelty, cand_bpm: f64) -> GrooveSignature {
        if nov.num_frames < 64 || cand_bpm <= 35.0 {
            return GrooveSignature {
                style: GrooveStyle::GeneralGroove,
                sub_kick_phase_score: 0.0,
                kick_quarter_hit_ratio: 0.0,
                salsa_tumbao_ratio: 0.0,
                dembow_pattern_score: 0.0,
                guiro_sixteenth_density: 0.0,
                clave_tresillo_ratio: 0.0,
                veto_three_quarter_alias: false,
            };
        }

        // ¡CRÍTICO v2.1!: Usa `nov.onset_fps` dinámico (correcto para 44.1 kHz, 48 kHz y 96 kHz)
        let beat_period_frames = (60.0 * nov.onset_fps) / cand_bpm;
        let bar_frames = beat_period_frames * 4.0;
        let step_frames = bar_frames / 16.0;

        let phase_probe_steps = 24_usize;
        let mut best_beat_offset = 0.0_f64;
        let mut best_probe_e = -1.0_f64;

        for p in 0..phase_probe_steps {
            let off = (p as f64 / phase_probe_steps as f64) * beat_period_frames;
            let mut sum = 0.0_f64;
            let mut t = off;
            while (t as usize) < nov.num_frames {
                let idx = t.round() as usize;
                if idx < nov.num_frames {
                    sum += 0.55 * nov.sub_kick[idx] + 0.45 * nov.composite[idx];
                }
                t += beat_period_frames;
            }
            if sum > best_probe_e {
                best_probe_e = sum;
                best_beat_offset = off;
            }
        }

        let mut hist_sub = [0.0_f64; 16];
        let mut hist_body = [0.0_f64; 16];
        let mut hist_clave = [0.0_f64; 16];
        let mut hist_guiro = [0.0_f64; 16];
        let mut step_counts = [0.0_f64; 16];

        let mut quarter_total_beats = 0_usize;
        let mut quarter_kick_hits = 0_usize;

        for f in 0..nov.num_frames {
            let rel = (f as f64) - best_beat_offset;
            if rel < 0.0 {
                continue;
            }
            let bar_pos = rel.rem_euclid(bar_frames);
            let step_idx = ((bar_pos / step_frames).floor() as usize).min(15);

            hist_sub[step_idx] += nov.sub_kick[f];
            hist_body[step_idx] += nov.low_body[f];
            hist_clave[step_idx] += nov.perc_clave[f];
            hist_guiro[step_idx] += nov.hi_guiro[f];
            step_counts[step_idx] += 1.0;
        }

        let mut b_cursor = best_beat_offset;
        let tol_frames = (beat_period_frames * 0.14).ceil().max(2.0) as isize;
        while (b_cursor as usize) < nov.num_frames {
            let center = b_cursor.round() as isize;
            let mut local_max_kick = 0.0_f64;
            for d in -tol_frames..=tol_frames {
                let idx = center + d;
                if idx >= 0 && (idx as usize) < nov.num_frames {
                    local_max_kick = local_max_kick.max(nov.sub_kick[idx as usize]);
                }
            }
            quarter_total_beats += 1;
            if local_max_kick > 0.65 {
                quarter_kick_hits += 1;
            }
            b_cursor += beat_period_frames;
        }
        let kick_quarter_hit_ratio = if quarter_total_beats > 0 {
            (quarter_kick_hits as f64) / (quarter_total_beats as f64)
        } else {
            0.0
        };

        for s in 0..16 {
            let c = step_counts[s].max(1.0);
            hist_sub[s] /= c;
            hist_body[s] /= c;
            hist_clave[s] /= c;
            hist_guiro[s] /= c;
        }

        let best_bar_rot = [0_usize, 4, 8, 12]
            .into_iter()
            .max_by(|&r1, &r2| {
                let s1 = hist_sub[r1] + 0.5 * hist_clave[r1];
                let s2 = hist_sub[r2] + 0.5 * hist_clave[r2];
                s1.partial_cmp(&s2).unwrap_or(Ordering::Equal)
            })
            .unwrap_or(0);

        hist_sub.rotate_left(best_bar_rot);
        hist_body.rotate_left(best_bar_rot);
        hist_clave.rotate_left(best_bar_rot);
        hist_guiro.rotate_left(best_bar_rot);

        let kick_on_quarters =
            (hist_sub[0] + hist_sub[4] + hist_sub[8] + hist_sub[12]) * 0.25;
        let kick_off_quarters = (0..16)
            .filter(|s| s % 4 != 0)
            .map(|s| hist_sub[s])
            .sum::<f64>()
            / 12.0;
        let sub_kick_phase_score =
            (kick_on_quarters / (kick_on_quarters + kick_off_quarters + 1e-6)).clamp(0.0, 1.0);

        let mut salsa_tumbao_ratio = 0.0_f64;
        for rot in [0_usize, 4, 8, 12] {
            let bass_0 = 0.55 * hist_sub[rot % 16] + 0.45 * hist_body[rot % 16];
            let bass_4 = 0.55 * hist_sub[(rot + 4) % 16] + 0.45 * hist_body[(rot + 4) % 16];
            let bass_6 = 0.55 * hist_sub[(rot + 6) % 16] + 0.45 * hist_body[(rot + 6) % 16];
            let bass_12 = 0.55 * hist_sub[(rot + 12) % 16] + 0.45 * hist_body[(rot + 12) % 16];
            let ratio = (bass_6 + bass_12) / (bass_0 + bass_4 + 0.12);
            salsa_tumbao_ratio = salsa_tumbao_ratio.max(ratio);
        }

        let snare_dembow =
            (hist_clave[3] + hist_clave[6] + hist_clave[11] + hist_clave[14]) * 0.25;
        let snare_other = (0..16)
            .filter(|s| !matches!(s, 0 | 3 | 4 | 6 | 8 | 11 | 12 | 14))
            .map(|s| hist_clave[s])
            .sum::<f64>()
            / 8.0;
        let dembow_snare_contrast =
            (snare_dembow / (snare_dembow + snare_other + 1e-6)).clamp(0.0, 1.0);
        let dembow_pattern_score =
            (0.55 * sub_kick_phase_score + 0.45 * dembow_snare_contrast) * kick_quarter_hit_ratio;

        let tresillo_hits = (hist_clave[0]
            + hist_clave[3]
            + hist_clave[6]
            + hist_clave[8]
            + hist_clave[11]
            + hist_clave[14])
            / 6.0;
        let clave_mean = (hist_clave.iter().sum::<f64>() / 16.0).max(1e-6);
        let clave_tresillo_ratio = tresillo_hits / clave_mean;

        let guiro_mean = hist_guiro.iter().sum::<f64>() / 16.0;
        let guiro_active_steps = hist_guiro
            .iter()
            .filter(|&&v| v >= guiro_mean * 0.55 && v > 0.12)
            .count();
        let guiro_sixteenth_density = (guiro_active_steps as f64) / 16.0;

        let veto_three_quarter_alias =
            kick_quarter_hit_ratio >= 0.62 && sub_kick_phase_score >= 0.58;

        let style = if veto_three_quarter_alias && (112.0..=136.0).contains(&cand_bpm) {
            if clave_tresillo_ratio >= 1.22 || guiro_sixteenth_density >= 0.68 {
                GrooveStyle::UrbanLatinFourFourHybrid
            } else {
                GrooveStyle::FourOnTheFloorClub
            }
        } else if (108.0..=164.0).contains(&cand_bpm)
            && guiro_sixteenth_density >= 0.75
            && kick_quarter_hit_ratio >= 0.50
        {
            GrooveStyle::MerengueDrive
        } else if (78.0..=106.0).contains(&cand_bpm)
            && dembow_pattern_score >= 0.42
            && kick_quarter_hit_ratio >= 0.58
        {
            GrooveStyle::ReggaetonDembow
        } else if (76.0..=109.0).contains(&cand_bpm)
            && salsa_tumbao_ratio >= 1.25
            && kick_quarter_hit_ratio < 0.58
        {
            GrooveStyle::SalsaTumbao
        } else if (116.0..=144.0).contains(&cand_bpm)
            && hist_clave[12] > clave_mean * 1.35
            && hist_body[6] > hist_body[4] * 1.15
        {
            GrooveStyle::BachataGroove
        } else if veto_three_quarter_alias {
            GrooveStyle::FourOnTheFloorClub
        } else {
            GrooveStyle::GeneralGroove
        };

        GrooveSignature {
            style,
            sub_kick_phase_score,
            kick_quarter_hit_ratio,
            salsa_tumbao_ratio,
            dembow_pattern_score,
            guiro_sixteenth_density,
            clave_tresillo_ratio,
            veto_three_quarter_alias,
        }
    }