//! Tier-3 Acoustic Rhythm Prior fallback for untagged audio/video tracks.

use super::taxonomy::BoothGenreFamily;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AcousticRhythmSignature {
    pub bpm: f32,
    pub groove_type: String,
    pub sub_kick_ratio: f32,
    pub low_body_ratio: f32,
    pub perc_clave_ratio: f32,
    pub hi_guiro_ratio: f32,
    pub energy_score: f32,
    pub is_transition_candidate: bool,
    pub entry_bpm: Option<f32>,
    pub exit_bpm: Option<f32>,
}

/// Infers the most likely `BoothGenreFamily` solely from BPM and 4-band / groove telemetry
/// when curated artist lexicons and folder/tag hints return `Unknown`.
pub fn classify_by_acoustic_prior(sig: &AcousticRhythmSignature) -> (BoothGenreFamily, f32) {
    let bpm = sig.bpm;
    let groove = sig.groove_type.to_ascii_lowercase();

    if bpm <= 0.0 {
        return (BoothGenreFamily::Unknown, 0.0);
    }

    // Tecnomerengue: intense 16th güira sizzle in 128..=162 BPM
    if (128.0..=162.0).contains(&bpm)
        && (sig.hi_guiro_ratio >= 0.36 || groove.contains("merengue"))
        && sig.hi_guiro_ratio > sig.perc_clave_ratio
    {
        return (BoothGenreFamily::Tecnomerengue, 0.74);
    }

    // Salsa family: 80..=112 BPM with prominent clave/tumbao and low sub-kick dominance
    if (80.0..=112.0).contains(&bpm)
        && (groove.contains("salsa")
            || (sig.perc_clave_ratio >= 0.20 && sig.sub_kick_ratio <= 0.28))
    {
        let fam = if sig.energy_score >= 0.82 {
            BoothGenreFamily::SalsaBravaChoque
        } else {
            BoothGenreFamily::SalsaBaulRomantica
        };
        return (fam, 0.72);
    }

    // Reggaeton / Perreo vs Dembow
    if (84.0..=105.0).contains(&bpm) && (groove.contains("dembow") || sig.sub_kick_ratio >= 0.34) {
        return (BoothGenreFamily::ReggaetonActualPerreo, 0.68);
    }
    if (106.0..=125.0).contains(&bpm) && groove.contains("dembow") {
        return (BoothGenreFamily::DembowReparto, 0.66);
    }

    // Bachata groove
    if (116.0..=144.0).contains(&bpm) && groove.contains("bachata") {
        return (BoothGenreFamily::Bachata, 0.68);
    }

    // Merengue drive
    if (118.0..=138.0).contains(&bpm) && groove.contains("merengue") {
        let fam = if bpm <= 128.0 {
            BoothGenreFamily::MerenguetonModerno
        } else {
            BoothGenreFamily::MerengueClasico
        };
        return (fam, 0.64);
    }

    // Club: Afro House vs Guaracha / Aleteo / Tech
    if (116.0..=125.5).contains(&bpm)
        && (groove.contains("afro") || groove.contains("urbanlatinfourfour"))
    {
        return (BoothGenreFamily::AfroHouse, 0.68);
    }
    if (126.0..=136.0).contains(&bpm) && groove.contains("four_on_floor") {
        return (BoothGenreFamily::GuarachaAleteoTech, 0.65);
    }

    (BoothGenreFamily::Unknown, 0.0)
}
