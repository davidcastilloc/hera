//! Latin rhythm heuristics: Salsa clave/tumbao de-aliasing & Tecnomerengue fast güira retention.

use super::multiband_onset::MultibandNovelty;
use super::v2_core::hints::{BpmHint, HintSource};
use super::v2_core::latin_heuristics::{GrooveSignature, GrooveStyle};

/// Cross-audits external `VirtualDjDatabase` / `EmbeddedTag` / `FilenamePattern` hints against
/// acoustic `GrooveSignature` candidates to transfer `4/3x` Salsa misfires (`120..=145 BPM`) to
/// the `0.75x` `SalsaTumbao` fundamental while protecting `130..=160 BPM` `Tecnomerengue` from
/// half-time (`65..=80 BPM`) folding.
pub fn compute_cross_audited_hint_adjustment(
    cand_bpm: f64,
    cand_sig: &GrooveSignature,
    hint: BpmHint,
    all_evals: &[(f64, &'static str, GrooveSignature, f64, f64)],
) -> f64 {
    let w = hint.source.prior_weight();
    let rel_err = (cand_bpm - hint.bpm).abs() / hint.bpm.max(1.0);

    if hint.source != HintSource::UserProvided && (120.0..=145.0).contains(&hint.bpm) {
        let salsa_fund = hint.bpm * 0.75;
        let hint_is_club_or_merengue = all_evals.iter().any(|(b, _, s, _, _)| {
            (*b - hint.bpm).abs() <= 2.5
                && (s.veto_three_quarter_alias
                    || s.style == GrooveStyle::MerengueDrive
                    || s.guiro_sixteenth_density >= 0.75)
        });
        let has_salsa_fundamental = (82.0..=109.0).contains(&salsa_fund)
            && all_evals.iter().any(|(b, _, s, _, _)| {
                (*b - salsa_fund).abs() <= 2.2 && s.style == GrooveStyle::SalsaTumbao
            });
        if has_salsa_fundamental && !hint_is_club_or_merengue {
            if (cand_bpm - salsa_fund).abs() / salsa_fund.max(1.0) <= 0.025
                && cand_sig.style == GrooveStyle::SalsaTumbao
            {
                return w + 0.14;
            }
            if rel_err <= 0.025
                || ((cand_bpm * 0.5 - salsa_fund).abs() / salsa_fund) <= 0.03
                || ((cand_bpm * 0.75 - salsa_fund).abs() / salsa_fund) <= 0.03
            {
                return -w - 0.12;
            }
            return 0.0;
        }
    }

    if hint.source != HintSource::UserProvided && (65.0..=80.0).contains(&hint.bpm) {
        let tecno_double = hint.bpm * 2.0;
        let has_tecnomerengue_double = (130.0..=160.0).contains(&tecno_double)
            && all_evals.iter().any(|(b, _, s, _, _)| {
                (*b - tecno_double).abs() <= 2.5
                    && (s.style == GrooveStyle::MerengueDrive || s.guiro_sixteenth_density >= 0.75)
            });
        if has_tecnomerengue_double {
            if (cand_bpm - tecno_double).abs() / tecno_double <= 0.025 {
                return w;
            }
            if rel_err <= 0.025 {
                return -w * 0.65;
            }
            return 0.0;
        }
    }

    if rel_err <= 0.025 {
        w
    } else if ((cand_bpm * 2.0 - hint.bpm).abs() / hint.bpm.max(1.0)) <= 0.025
        || ((cand_bpm * 0.5 - hint.bpm).abs() / hint.bpm.max(1.0)) <= 0.025
    {
        -w * 0.45
    } else {
        0.0
    }
}

#[derive(Debug, Clone)]
pub struct LatinClassification {
    pub rhythm_family: String,
    pub candidate_bpm: f32,
    pub confidence: f32,
}

pub fn evaluate_latin_heuristics(novelty: &MultibandNovelty, raw_bpm: f32) -> LatinClassification {
    let sub_sum: f32 = novelty.sub_kick.iter().sum();
    let low_sum: f32 = novelty.low_body.iter().sum();
    let clave_sum: f32 = novelty.perc_clave.iter().sum();
    let guiro_sum: f32 = novelty.hi_guiro.iter().sum();

    let total = (sub_sum + low_sum + clave_sum + guiro_sum).max(1e-6);
    let sub_ratio = sub_sum / total;
    let low_ratio = low_sum / total;
    let clave_ratio = clave_sum / total;
    let guiro_ratio = guiro_sum / total;

    // Tecnomerengue signature: dominant 16th-note güira sizzle (>= 38% and exceeding clave)
    let is_tecnomerengue_profile = guiro_ratio >= 0.38 && guiro_ratio > clave_ratio;

    if is_tecnomerengue_profile {
        let mut tecno_bpm = raw_bpm;
        // 2/3x polyrhythmic cross-tempo resolution: e.g. ~98-115 BPM -> * 1.5 -> ~147-172 BPM
        if (98.0..=115.0).contains(&tecno_bpm) {
            tecno_bpm *= 1.5;
        }
        // 1/2x half-tempo alias resolution: e.g. 60..98 BPM -> * 2.0 -> 120..196 BPM
        else if (60.0..98.0).contains(&tecno_bpm) {
            tecno_bpm *= 2.0;
        }

        if (128.0..=160.0).contains(&tecno_bpm) {
            return LatinClassification {
                rhythm_family: "Tecnomerengue".to_string(),
                candidate_bpm: tecno_bpm,
                confidence: 0.94,
            };
        } else if tecno_bpm > 160.0 {
            let folded = tecno_bpm * 0.5;
            if (80.0..=108.0).contains(&folded) {
                return LatinClassification {
                    rhythm_family: "Salsa".to_string(),
                    candidate_bpm: folded,
                    confidence: 0.95,
                };
            }
        }
    }

    // Salsa signature: prominent Clave/Campana/Bongo, low sub-kick on beat 1 (<= 28%)
    let is_salsa_profile = (clave_ratio >= 0.20 && sub_ratio <= 0.28)
        || (clave_ratio >= 0.18 && low_ratio >= 0.05 && sub_ratio <= 0.30);

    if is_salsa_profile {
        let mut salsa_bpm = raw_bpm;
        // 4/3x alias resolution: e.g. ~107-145 BPM -> * 0.75 -> 80.25-108.75 BPM
        if (107.0..=145.0).contains(&salsa_bpm) {
            salsa_bpm *= 0.75;
        }
        // 2x double-time alias resolution: e.g. 164-216 BPM -> * 0.5 -> 82-108 BPM
        while salsa_bpm > 109.0 {
            salsa_bpm *= 0.5;
        }
        while salsa_bpm < 80.0 {
            salsa_bpm *= 2.0;
        }

        if (80.0..=109.0).contains(&salsa_bpm) {
            let deals_alias = (salsa_bpm - raw_bpm).abs() >= 0.5;
            let score_raw = beatgrid_phase_score(novelty, raw_bpm);
            let score_salsa = beatgrid_phase_score(novelty, salsa_bpm);
            if !deals_alias || score_salsa > score_raw * 1.02 {
                return LatinClassification {
                    rhythm_family: "Salsa".to_string(),
                    candidate_bpm: salsa_bpm,
                    confidence: 0.95,
                };
            }
        }
    }

    // Default fallback
    LatinClassification {
        rhythm_family: if (120.0..=134.0).contains(&raw_bpm) && sub_ratio >= 0.40 {
            "House/EDM".to_string()
        } else if (86.0..=104.0).contains(&raw_bpm) && sub_ratio >= 0.35 {
            "Urbano/Reggaeton".to_string()
        } else {
            "OpenFormat".to_string()
        },
        candidate_bpm: raw_bpm,
        confidence: 0.70,
    }
}

/// Evaluates how strongly a candidate BPM aligns with real percussive onsets across the track.
/// Computes the maximum phase-aligned beatgrid energy minus off-beat energy.
fn beatgrid_phase_score(novelty: &MultibandNovelty, bpm: f32) -> f32 {
    if bpm <= 0.0 || novelty.frame_dt <= 0.0 || novelty.sub_kick.is_empty() {
        return 0.0;
    }
    let frame_rate = 1.0 / novelty.frame_dt;
    let period = (60.0 / bpm) * frame_rate;
    if period < 2.0 {
        return 0.0;
    }

    let len = novelty.sub_kick.len();
    let phases = 16usize;
    let mut best_score = f32::NEG_INFINITY;

    for p in 0..phases {
        let phi = (p as f32 / phases as f32) * period;
        let mut on_beat_sum = 0.0f32;
        let mut off_beat_sum = 0.0f32;
        let mut count = 0usize;

        let mut pos = phi;
        while (pos as usize) + 1 < len {
            let idx = pos.round() as usize;
            if idx < len {
                on_beat_sum += 0.50 * novelty.sub_kick[idx]
                    + 0.25 * novelty.low_body[idx]
                    + 0.25 * novelty.perc_clave[idx];
            }
            let off_idx = (pos + period * 0.5).round() as usize;
            if off_idx < len {
                off_beat_sum += 0.50 * novelty.sub_kick[off_idx]
                    + 0.25 * novelty.low_body[off_idx]
                    + 0.25 * novelty.perc_clave[off_idx];
            }
            count += 1;
            pos += period;
        }

        if count > 0 {
            let score = (on_beat_sum - 0.5 * off_beat_sum) / (count as f32);
            if score > best_score {
                best_score = score;
            }
        }
    }

    if best_score.is_finite() {
        best_score
    } else {
        0.0
    }
}
