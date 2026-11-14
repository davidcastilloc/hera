    use super::hints::BpmHint;
    use super::latin_heuristics::{evaluate_groove_at_bpm, GrooveSignature, GrooveStyle};
    use super::multiband_onset::MultibandNovelty;
    use super::phase_lock::fine_grid_phase_lock_and_beats;
    use super::qm_comb_tracker::analyze_qm_comb_tempogram;
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BpmConsensusResult {
        pub bpm: f64,
        pub first_beat_sec: f64,
        pub beats_sec: Vec<f64>,
        pub confidence: f32,
        pub detected_style: GrooveStyle,
        pub applied_ratio_note: String,
    }

    pub fn analyze_bpm_consensus_v2(
        nov: &MultibandNovelty,
        bpm_hint: Option<BpmHint>,
    ) -> BpmConsensusResult {
        let qm = analyze_qm_comb_tempogram(nov);
        let fps = nov.onset_fps;

        let mut base_seeds: Vec<f64> = qm.top_comb_peaks.iter().take(3).map(|p| p.0).collect();
        base_seeds.push(qm.viterbi_bpm);
        if let Some(h) = bpm_hint {
            base_seeds.push(h.bpm);
        }

        let ratios = [1.0_f64, 0.75, 4.0 / 3.0, 0.5, 2.0, 2.0 / 3.0, 1.5];
        let mut candidates: Vec<(f64, &'static str)> = Vec::new();

        for &seed in &base_seeds {
            for &r in &ratios {
                let c = seed * r;
                if (55.0..=210.0).contains(&c) {
                    if !candidates.iter().any(|existing| (existing.0 - c).abs() < 0.85) {
                        let tag = if (r - 1.0).abs() < 1e-3 {
                            "1.0x"
                        } else if (r - 0.75).abs() < 1e-3 {
                            "0.75x (3:4)"
                        } else if (r - 4.0 / 3.0).abs() < 1e-2 {
                            "1.33x (4:3)"
                        } else if (r - 0.5).abs() < 1e-3 {
                            "0.5x"
                        } else if (r - 2.0).abs() < 1e-3 {
                            "2.0x"
                        } else {
                            "3:2 metric"
                        };
                        candidates.push((c, tag));
                    }
                }
            }
        }

        let max_comp_comb = qm
            .composite_comb_by_lag
            .iter()
            .copied()
            .fold(1e-9_f64, f64::max);
        let max_sub_comb = qm
            .sub_kick_comb_by_lag
            .iter()
            .copied()
            .fold(1e-9_f64, f64::max);

        let evals: Vec<(f64, &'static str, GrooveSignature, f64, f64)> = candidates
            .iter()
            .map(|&(bpm, label)| {
                let sig = evaluate_groove_at_bpm(nov, bpm);
                let exact_lag = ((60.0 * fps) / bpm)
                    .clamp(qm.min_lag as f64, qm.max_lag as f64);
                // Interpolación lineal en el espectro peine (evita escalones discretos)
                let l0 = exact_lag.floor() as usize;
                let l1 = (l0 + 1).min(qm.max_lag);
                let frac = exact_lag - (l0 as f64);
                let comp_e = ((1.0 - frac) * qm.composite_comb_by_lag[l0]
                    + frac * qm.composite_comb_by_lag[l1])
                    / max_comp_comb;
                let sub_e = ((1.0 - frac) * qm.sub_kick_comb_by_lag[l0]
                    + frac * qm.sub_kick_comb_by_lag[l1])
                    / max_sub_comb;
                (bpm, label, sig, comp_e, sub_e)
            })
            .collect();

        let mut best_idx = 0_usize;
        let mut best_total_score = -1e9_f64;

        for (idx, (bpm, _label, sig, comp_e, sub_e)) in evals.iter().enumerate() {
            let mut score = 0.44 * comp_e
                + 0.24 * sub_e
                + 0.20 * sig.sub_kick_phase_score
                + 0.12 * sig.kick_quarter_hit_ratio;

            let parent_4_3 = bpm * (4.0 / 3.0);
            if let Some((_, _, parent_sig, _, parent_sub_e)) = evals
                .iter()
                .find(|e| (e.0 - parent_4_3).abs() <= 1.6)
            {
                if parent_sig.veto_three_quarter_alias && *parent_sub_e >= *sub_e * 0.90 {
                    score -= 0.55;
                }
            }

            match sig.style {
                GrooveStyle::FourOnTheFloorClub | GrooveStyle::UrbanLatinFourFourHybrid => {
                    score += 0.22;
                }
                GrooveStyle::MerengueDrive => {
                    score += 0.20;
                }
                GrooveStyle::ReggaetonDembow => {
                    score += 0.21;
                }
                GrooveStyle::SalsaTumbao => {
                    let has_club_parent = evals.iter().any(|e| {
                        (e.0 - parent_4_3).abs() <= 1.6 && e.2.veto_three_quarter_alias
                    });
                    if !has_club_parent {
                        score += 0.26;
                    }
                }
                GrooveStyle::BachataGroove => {
                    score += 0.16;
                }
                GrooveStyle::GeneralGroove => {}
            }

            if *bpm > 158.0 {
                let half_bpm = bpm * 0.5;
                if let Some((_, _, half_sig, _, half_sub)) =
                    evals.iter().find(|e| (e.0 - half_bpm).abs() <= 1.5)
                {
                    if matches!(
                        half_sig.style,
                        GrooveStyle::ReggaetonDembow | GrooveStyle::SalsaTumbao
                    ) || *half_sub >= *sub_e * 0.92
                    {
                        score -= 0.32;
                    }
                }
            }
            if *bpm < 76.0 {
                let double_bpm = bpm * 2.0;
                if let Some((_, _, dbl_sig, _, _)) =
                    evals.iter().find(|e| (e.0 - double_bpm).abs() <= 2.0)
                {
                    if dbl_sig.guiro_sixteenth_density >= 0.65
                        || dbl_sig.veto_three_quarter_alias
                    {
                        score -= 0.35;
                    }
                }
            }

            if let Some(hint) = bpm_hint {
                let rel_err = ((bpm - hint.bpm).abs()) / hint.bpm.max(1.0);
                if rel_err <= 0.025 {
                    score += hint.source.prior_weight();
                } else if ((bpm * 2.0 - hint.bpm).abs() / hint.bpm.max(1.0)) <= 0.025
                    || ((bpm * 0.5 - hint.bpm).abs() / hint.bpm.max(1.0)) <= 0.025
                {
                    score -= hint.source.prior_weight() * 0.45;
                }
            }

            if score > best_total_score {
                best_total_score = score;
                best_idx = idx;
            }
        }

        let (winning_coarse_bpm, winning_label, winning_sig, _, _) = &evals[best_idx];
        let prefer_kick = winning_sig.veto_three_quarter_alias
            || matches!(
                winning_sig.style,
                GrooveStyle::ReggaetonDembow | GrooveStyle::MerengueDrive
            );

        let (locked_bpm, first_beat_sec, beats_sec, coherence) =
            fine_grid_phase_lock_and_beats(*winning_coarse_bpm, nov, prefer_kick);

        let confidence = (0.55 * coherence + 0.45 * best_total_score.clamp(0.0, 1.0))
            .clamp(0.0, 1.0) as f32;

        BpmConsensusResult {
            bpm: locked_bpm,
            first_beat_sec,
            beats_sec,
            confidence,
            detected_style: winning_sig.style,
            applied_ratio_note: (*winning_label).to_string(),
        }
    }