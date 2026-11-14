    use super::key_engine::{KeyScale, MusicalKeyResult, PitchClass};

    pub fn build_key_result(
        tonic: PitchClass,
        scale: KeyScale,
        confidence: f32,
        tuning_cents: f32,
    ) -> MusicalKeyResult {
        let (num, cam_let, ok_let) = match (tonic, scale) {
            (PitchClass::Ab, KeyScale::Minor) => (1, 'A', 'm'),
            (PitchClass::Eb, KeyScale::Minor) => (2, 'A', 'm'),
            (PitchClass::Bb, KeyScale::Minor) => (3, 'A', 'm'),
            (PitchClass::F, KeyScale::Minor) => (4, 'A', 'm'),
            (PitchClass::C, KeyScale::Minor) => (5, 'A', 'm'),
            (PitchClass::G, KeyScale::Minor) => (6, 'A', 'm'),
            (PitchClass::D, KeyScale::Minor) => (7, 'A', 'm'),
            (PitchClass::A, KeyScale::Minor) => (8, 'A', 'm'),
            (PitchClass::E, KeyScale::Minor) => (9, 'A', 'm'),
            (PitchClass::B, KeyScale::Minor) => (10, 'A', 'm'),
            (PitchClass::Gb, KeyScale::Minor) => (11, 'A', 'm'),
            (PitchClass::Db, KeyScale::Minor) => (12, 'A', 'm'),

            (PitchClass::B, KeyScale::Major) => (1, 'B', 'd'),
            (PitchClass::Gb, KeyScale::Major) => (2, 'B', 'd'),
            (PitchClass::Db, KeyScale::Major) => (3, 'B', 'd'),
            (PitchClass::Ab, KeyScale::Major) => (4, 'B', 'd'),
            (PitchClass::Eb, KeyScale::Major) => (5, 'B', 'd'),
            (PitchClass::Bb, KeyScale::Major) => (6, 'B', 'd'),
            (PitchClass::F, KeyScale::Major) => (7, 'B', 'd'),
            (PitchClass::C, KeyScale::Major) => (8, 'B', 'd'),
            (PitchClass::G, KeyScale::Major) => (9, 'B', 'd'),
            (PitchClass::D, KeyScale::Major) => (10, 'B', 'd'),
            (PitchClass::A, KeyScale::Major) => (11, 'B', 'd'),
            (PitchClass::E, KeyScale::Major) => (12, 'B', 'd'),
        };
        let ok_num = ((num + 4) % 12) + 1;
        let sc_str = match scale {
            KeyScale::Major => "major",
            KeyScale::Minor => "minor",
        };
        MusicalKeyResult {
            tonic,
            scale,
            standard_name: format!("{} {}", tonic.label(), sc_str),
            camelot: format!("{}{}", num, cam_let),
            open_key: format!("{}{}", ok_num, ok_let),
            confidence,
            tuning_cents,
        }
    }