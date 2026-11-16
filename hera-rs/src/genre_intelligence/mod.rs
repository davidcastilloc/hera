//! Genre × Era Curation Intelligence (`14` Latin & Open-Format Booth Families).

pub mod acoustic_classifier;
pub mod era_detector;
pub mod taxonomy;

pub use acoustic_classifier::{classify_by_acoustic_prior, AcousticRhythmSignature};
pub use era_detector::{compute_genre_era_compatibility, era_decade_label, extract_era_year};
pub use taxonomy::{
    classify_by_curated_lexicon, classify_by_tags_or_folder, normalize_ascii_lower,
    BoothGenreFamily,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenreEraProfile {
    pub family: BoothGenreFamily,
    pub family_code: u8,
    pub family_name: String,
    pub display_label: String,
    pub confidence: f32,
    pub inferred_era_year: u16,
    pub era_decade_label: String,
    pub is_transition_candidate: bool,
    pub classification_tier: String,
}

impl Default for GenreEraProfile {
    fn default() -> Self {
        Self::from_family(BoothGenreFamily::Unknown, 0.0, 2020, false, "none")
    }
}

impl GenreEraProfile {
    pub fn from_family(
        family: BoothGenreFamily,
        confidence: f32,
        inferred_era_year: u16,
        is_transition_candidate: bool,
        tier: &str,
    ) -> Self {
        Self {
            family,
            family_code: family.code(),
            family_name: family.as_str().to_string(),
            display_label: family.display_label_es().to_string(),
            confidence: confidence.clamp(0.0, 1.0),
            inferred_era_year,
            era_decade_label: era_decade_label(inferred_era_year),
            is_transition_candidate,
            classification_tier: tier.to_string(),
        }
    }
}

/// Classifies a track across 3 tiers: curated artist lexicon, ID3/folder keywords, and acoustic prior.
pub fn classify_track_genre_and_era(
    artist: &str,
    title: &str,
    tags_genre: &[String],
    folder: &str,
    year_hint: Option<u16>,
    acoustic: Option<&AcousticRhythmSignature>,
) -> GenreEraProfile {
    let artist_norm = normalize_ascii_lower(artist);
    let title_norm = normalize_ascii_lower(title);
    let is_trans = acoustic.is_some_and(|a| a.is_transition_candidate);

    if let Some((fam, lex_year, conf)) = classify_by_curated_lexicon(&artist_norm, &title_norm) {
        let era = extract_era_year(title, folder, year_hint, fam, Some(lex_year));
        return GenreEraProfile::from_family(fam, conf, era, is_trans, "curated_lexicon");
    }

    if let Some((fam, conf)) = classify_by_tags_or_folder(tags_genre, folder, &title_norm) {
        let era = extract_era_year(title, folder, year_hint, fam, None);
        return GenreEraProfile::from_family(fam, conf, era, is_trans, "tags_folder");
    }

    if let Some(sig) = acoustic {
        let (fam, conf) = classify_by_acoustic_prior(sig);
        if fam != BoothGenreFamily::Unknown {
            let era = extract_era_year(title, folder, year_hint, fam, None);
            return GenreEraProfile::from_family(fam, conf, era, is_trans, "acoustic_prior");
        }
    }

    let fallback_year = year_hint
        .filter(|&y| (1955..=2030).contains(&y))
        .unwrap_or(2020);
    GenreEraProfile::from_family(
        BoothGenreFamily::Unknown,
        0.0,
        fallback_year,
        is_trans,
        "unknown",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_14_family_codes_roundtrip() {
        assert_eq!(BoothGenreFamily::Unknown.code(), 0);
        for c in 0..=14_u8 {
            assert_eq!(BoothGenreFamily::from_code(c).code(), c);
        }
        assert_eq!(BoothGenreFamily::LlaneraDespecho.code(), 14);
    }

    #[test]
    fn test_salsa_vs_perreo_and_90s_vs_2026_merengue_isolation() {
        let salsa =
            classify_track_genre_and_era("Tony Vega", "Lo Mio Es Amor", &[], "", None, None);
        let perreo = classify_track_genre_and_era("Feid", "Luna", &[], "", None, None);
        assert_eq!(salsa.family, BoothGenreFamily::SalsaBaulRomantica);
        assert_eq!(perreo.family, BoothGenreFamily::ReggaetonActualPerreo);
        assert!(compute_genre_era_compatibility(&salsa, &perreo, false) <= 0.15);
        assert!(compute_genre_era_compatibility(&salsa, &perreo, true) >= 0.65);

        let euro90 =
            classify_track_genre_and_era("Sandy & Papo", "Mueve Mueve", &[], "", Some(1996), None);
        let mer2025 =
            classify_track_genre_and_era("Alleh & Yorghaki", "Capaz", &[], "", Some(2025), None);
        assert_eq!(euro90.family, BoothGenreFamily::Tecnomerengue);
        assert_eq!(mer2025.family, BoothGenreFamily::MerenguetonModerno);
        assert!(compute_genre_era_compatibility(&euro90, &mer2025, false) <= 0.15);
        assert!(compute_genre_era_compatibility(&euro90, &mer2025, true) >= 0.65);

        let mut bridge = perreo.clone();
        bridge.is_transition_candidate = true;
        assert!(compute_genre_era_compatibility(&salsa, &bridge, false) >= 0.85);
    }

    #[test]
    fn test_acoustic_prior_fallback_for_untagged_files() {
        let sig = AcousticRhythmSignature {
            bpm: 142.0,
            groove_type: "MerengueDrive".into(),
            hi_guiro_ratio: 0.44,
            perc_clave_ratio: 0.18,
            ..Default::default()
        };
        let prof = classify_track_genre_and_era("Unknown", "Track 01", &[], "", None, Some(&sig));
        assert_eq!(prof.family, BoothGenreFamily::Tecnomerengue);
        assert_eq!(prof.classification_tier, "acoustic_prior");
    }
}
