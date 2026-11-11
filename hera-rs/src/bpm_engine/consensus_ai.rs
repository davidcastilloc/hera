//! Consensus arbiter cross-referencing acoustic evidence with external hints (VDJ / ID3).

use super::ExternalBpmHint;

pub fn arbitrate_bpm_consensus(
    acoustic_bpm: f32,
    external_hint: Option<&ExternalBpmHint>,
    rhythm_family: &str,
) -> f32 {
    let Some(hint) = external_hint else {
        return acoustic_bpm;
    };

    let hint_bpm = hint.bpm;
    if !(30.0..300.0).contains(&hint_bpm) {
        return acoustic_bpm;
    }

    // Genre-specific priors
    let genre_lower = hint
        .genre
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();

    let is_salsa = rhythm_family.eq_ignore_ascii_case("salsa")
        || genre_lower.contains("salsa");

    let is_tecnomerengue = rhythm_family.eq_ignore_ascii_case("tecnomerengue")
        || genre_lower.contains("tecnomerengue")
        || genre_lower.contains("merengue");

    if is_salsa {
        // Salsa canonical scale is strictly 82..108 BPM.
        // Check for 2x octave error in tag (e.g. 184 BPM -> 92 BPM)
        let folded_half = hint_bpm * 0.5;
        if (folded_half - acoustic_bpm).abs() < 2.0 && (80.0..=110.0).contains(&folded_half) {
            return folded_half;
        }

        // Check for 4/3x cross-rhythm error in tag (e.g. 125.3 BPM -> 94.0 BPM)
        let folded_three_fourths = hint_bpm * 0.75;
        if (folded_three_fourths - acoustic_bpm).abs() < 2.5
            && (80.0..=110.0).contains(&folded_three_fourths)
        {
            return acoustic_bpm;
        }

        // If tag is way out of Salsa range (e.g. 129.0) but acoustic is in 82..108, trust acoustic
        if !(80.0..=115.0).contains(&hint_bpm) && (80.0..=110.0).contains(&acoustic_bpm) {
            return acoustic_bpm;
        }
    }

    if is_tecnomerengue {
        // Tecnomerengue canonical scale is strictly 130..165 BPM.
        // If tag is half-tempo (e.g. 74..80 BPM), unfold to double
        let unfolded_double = hint_bpm * 2.0;
        if (unfolded_double - acoustic_bpm).abs() < 2.0
            && (130.0..=165.0).contains(&unfolded_double)
        {
            return unfolded_double;
        }
    }

    // If tag and acoustic match closely (< 1.5 BPM difference), trust acoustic with fine precision
    if (hint_bpm - acoustic_bpm).abs() <= 1.5 {
        return acoustic_bpm;
    }

    // Otherwise default to acoustic analysis
    acoustic_bpm
}
