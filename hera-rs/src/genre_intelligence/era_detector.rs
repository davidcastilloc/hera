//! Era extraction and `Genre × Era` booth compatibility scoring.

use super::taxonomy::{normalize_ascii_lower, BoothGenreFamily};
use super::GenreEraProfile;

pub fn extract_era_year(
    title: &str,
    folder: &str,
    year_hint: Option<u16>,
    family: BoothGenreFamily,
    lexicon_year: Option<u16>,
) -> u16 {
    if let Some(y) = year_hint {
        if (1955..=2030).contains(&y) {
            return y;
        }
    }
    let text = format!(
        "{} {}",
        normalize_ascii_lower(title),
        normalize_ascii_lower(folder)
    );
    for tok in text.split_whitespace() {
        if tok.len() == 4 && tok.bytes().all(|b| b.is_ascii_digit()) {
            if let Ok(y) = tok.parse::<u16>() {
                if (1960..=2030).contains(&y) {
                    return y;
                }
            }
        }
    }
    if text.contains("90s") || text.contains("noventa") {
        return 1995;
    }
    if text.contains("80s") || text.contains("ochenta") {
        return 1985;
    }
    if text.contains("2000s") || text.contains("old school") {
        return 2006;
    }
    lexicon_year.unwrap_or_else(|| family.default_era_midpoint())
}

pub fn era_decade_label(year: u16) -> String {
    if year == 0 {
        return "Unknown".to_string();
    }
    let dec = (year / 10) * 10;
    format!("{dec}s")
}

/// Computes `Genre × Era` compatibility (`0.0..=1.0`), isolating Salsa vs Perreo and
/// 90s Euro-Merengue vs 2026 Merenguetón unless `allow_crossover` or `is_transition_candidate`.
pub fn compute_genre_era_compatibility(
    a: &GenreEraProfile,
    b: &GenreEraProfile,
    allow_crossover: bool,
) -> f32 {
    if a.family == BoothGenreFamily::Unknown || b.family == BoothGenreFamily::Unknown {
        return 0.90;
    }
    if a.is_transition_candidate || b.is_transition_candidate {
        return 0.88;
    }

    let era_diff = a.inferred_era_year.abs_diff(b.inferred_era_year);
    if a.family == b.family {
        return if era_diff <= 12 {
            1.00
        } else if era_diff <= 18 {
            0.90
        } else if allow_crossover {
            0.80
        } else {
            0.62
        };
    }

    let sibling_salsa = a.family.is_salsa() && b.family.is_salsa();
    let sibling_urbano = a.family.is_perreo_urbano() && b.family.is_perreo_urbano();
    let sibling_merengue_classic = matches!(
        (a.family, b.family),
        (
            BoothGenreFamily::Tecnomerengue,
            BoothGenreFamily::MerengueClasico
        ) | (
            BoothGenreFamily::MerengueClasico,
            BoothGenreFamily::Tecnomerengue
        )
    );
    let sibling_club = matches!(
        (a.family, b.family),
        (
            BoothGenreFamily::AfroHouse,
            BoothGenreFamily::GuarachaAleteoTech
        ) | (
            BoothGenreFamily::GuarachaAleteoTech,
            BoothGenreFamily::AfroHouse
        )
    );

    if allow_crossover {
        return if sibling_salsa || sibling_urbano || sibling_merengue_classic || sibling_club {
            0.84
        } else {
            0.68
        };
    }

    // Hard booth isolation 1: Salsa vs Reggaeton/Perreo/Dembow (numeric 82-102 BPM collision)
    if (a.family.is_salsa() && b.family.is_perreo_urbano())
        || (b.family.is_salsa() && a.family.is_perreo_urbano())
    {
        return 0.10;
    }

    // Hard booth isolation 2: 90s Euro-Merengue / Clasico vs 2026 Merenguetón (~126 BPM collision)
    let a_retro_mer = matches!(
        a.family,
        BoothGenreFamily::Tecnomerengue | BoothGenreFamily::MerengueClasico
    );
    let b_retro_mer = matches!(
        b.family,
        BoothGenreFamily::Tecnomerengue | BoothGenreFamily::MerengueClasico
    );
    if (a_retro_mer && b.family == BoothGenreFamily::MerenguetonModerno)
        || (b_retro_mer && a.family == BoothGenreFamily::MerenguetonModerno)
    {
        return 0.12;
    }

    if sibling_salsa {
        return if era_diff <= 15 { 0.78 } else { 0.48 };
    }
    if sibling_club {
        return if era_diff <= 15 { 0.70 } else { 0.45 };
    }
    if sibling_urbano {
        return if era_diff <= 15 { 0.65 } else { 0.24 };
    }
    if sibling_merengue_classic {
        return if era_diff <= 15 { 0.72 } else { 0.40 };
    }

    if era_diff > 15 {
        0.18
    } else {
        0.28
    }
}
