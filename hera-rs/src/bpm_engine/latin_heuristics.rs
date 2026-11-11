//! Latin rhythm heuristics: Salsa clave/tumbao de-aliasing & Tecnomerengue fast güira retention.

use super::multiband_onset::MultibandNovelty;

#[derive(Debug, Clone)]
pub struct LatinClassification {
    pub rhythm_family: String,
    pub candidate_bpm: f32,
    pub confidence: f32,
}

pub fn evaluate_latin_heuristics(
    novelty: &MultibandNovelty,
    raw_bpm: f32,
) -> LatinClassification {
    let sub_sum: f32 = novelty.sub_kick.iter().sum();
    let low_sum: f32 = novelty.low_body.iter().sum();
    let clave_sum: f32 = novelty.perc_clave.iter().sum();
    let guiro_sum: f32 = novelty.hi_guiro.iter().sum();

    let total = (sub_sum + low_sum + clave_sum + guiro_sum).max(1e-6);
    let sub_ratio = sub_sum / total;
    let low_ratio = low_sum / total;
    let clave_ratio = clave_sum / total;
    let guiro_ratio = guiro_sum / total;

    // Tecnomerengue signature: prominent 16th-note güira (>= 40% of total novelty, or >= 28% with fast tempo)
    // and binary tambora hits.
    let is_tecnomerengue_profile = guiro_ratio >= 0.40
        || (guiro_ratio >= 0.28 && (clave_ratio + sub_ratio) >= 0.30);

    if is_tecnomerengue_profile {
        let mut tecno_bpm = raw_bpm;
        // 2/3x polyrhythmic cross-tempo resolution: e.g. ~98-112 BPM -> * 1.5 -> ~147-168 BPM
        if (96.0..=114.0).contains(&tecno_bpm) {
            tecno_bpm *= 1.5;
        }
        // 1/2x half-tempo alias resolution: e.g. 65..82 BPM -> * 2.0 -> 130..164 BPM
        else if (60.0..96.0).contains(&tecno_bpm) {
            tecno_bpm *= 2.0;
        }

        if (130.0..=165.0).contains(&tecno_bpm) {
            return LatinClassification {
                rhythm_family: "Tecnomerengue".to_string(),
                candidate_bpm: tecno_bpm,
                confidence: 0.94,
            };
        }
    }

    // Salsa signature: prominent Clave/Campana (>= 22%), subdued kick on beat 1 (<= 25%),
    // and syncopated baby-bass tumbao / congas.
    let is_salsa_profile = (clave_ratio >= 0.22 && sub_ratio <= 0.25)
        || (clave_ratio >= 0.18 && low_ratio >= 0.05 && sub_ratio <= 0.28);

    if is_salsa_profile {
        let mut salsa_bpm = raw_bpm;
        // 4/3x alias resolution: e.g. ~124-144 BPM -> * 0.75 -> 93-108 BPM
        if (118.0..=145.0).contains(&salsa_bpm) {
            salsa_bpm *= 0.75;
        }
        // 2x double-time alias resolution: e.g. 164-216 BPM -> * 0.5 -> 82-108 BPM
        while salsa_bpm > 109.0 {
            salsa_bpm *= 0.5;
        }
        while salsa_bpm < 80.0 {
            salsa_bpm *= 2.0;
        }

        if (82.0..=108.0).contains(&salsa_bpm) {
            return LatinClassification {
                rhythm_family: "Salsa".to_string(),
                candidate_bpm: salsa_bpm,
                confidence: 0.95,
            };
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
