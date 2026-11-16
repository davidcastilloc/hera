//! 14 Booth Genre Families (`Genre × Era`) & Curated Latin / Open-Format Lexicon.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum BoothGenreFamily {
    Unknown = 0,
    Tecnomerengue = 1,
    MerenguetonModerno = 2,
    MerengueClasico = 3,
    SalsaBaulRomantica = 4,
    SalsaBravaChoque = 5,
    ReggaetonOldSchool = 6,
    ReggaetonActualPerreo = 7,
    DembowReparto = 8,
    AfroHouse = 9,
    GuarachaAleteoTech = 10,
    Bachata = 11,
    CumbiaVallenato = 12,
    HoraLocaGaitas = 13,
    LlaneraDespecho = 14,
}

const NAMES: [&str; 15] = [
    "Unknown",
    "Tecnomerengue",
    "MerenguetonModerno",
    "MerengueClasico",
    "SalsaBaulRomantica",
    "SalsaBravaChoque",
    "ReggaetonOldSchool",
    "ReggaetonActualPerreo",
    "DembowReparto",
    "AfroHouse",
    "GuarachaAleteoTech",
    "Bachata",
    "CumbiaVallenato",
    "HoraLocaGaitas",
    "LlaneraDespecho",
];

const LABELS_ES: [&str; 15] = [
    "Sin Clasificar",
    "Tecnomerengue / Euro-Merengue",
    "Merenguetón Moderno",
    "Merengue Clásico",
    "Salsa Baúl / Romántica",
    "Salsa Brava / Choque",
    "Reggaetón Old School",
    "Reggaetón Actual / Perreo",
    "Dembow / Reparto",
    "Afro House / Melodic",
    "Guaracha / Aleteo / Tech",
    "Bachata",
    "Cumbia / Vallenato",
    "Hora Loca / Gaitas",
    "Llanera / Despecho",
];

const ERA_WINDOWS: [(u16, u16); 15] = [
    (2015, 2025),
    (1990, 1999),
    (2022, 2026),
    (1986, 2002),
    (1988, 2002),
    (1974, 2010),
    (2001, 2012),
    (2018, 2026),
    (2018, 2026),
    (2018, 2026),
    (2017, 2026),
    (2004, 2024),
    (1992, 2022),
    (1985, 2015),
    (2005, 2025),
];

impl BoothGenreFamily {
    pub const fn code(self) -> u8 {
        self as u8
    }

    pub const fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Tecnomerengue,
            2 => Self::MerenguetonModerno,
            3 => Self::MerengueClasico,
            4 => Self::SalsaBaulRomantica,
            5 => Self::SalsaBravaChoque,
            6 => Self::ReggaetonOldSchool,
            7 => Self::ReggaetonActualPerreo,
            8 => Self::DembowReparto,
            9 => Self::AfroHouse,
            10 => Self::GuarachaAleteoTech,
            11 => Self::Bachata,
            12 => Self::CumbiaVallenato,
            13 => Self::HoraLocaGaitas,
            14 => Self::LlaneraDespecho,
            _ => Self::Unknown,
        }
    }

    pub const fn as_str(self) -> &'static str {
        NAMES[self as usize]
    }
    pub const fn display_label_es(self) -> &'static str {
        LABELS_ES[self as usize]
    }
    pub const fn default_era_window(self) -> (u16, u16) {
        ERA_WINDOWS[self as usize]
    }

    pub const fn default_era_midpoint(self) -> u16 {
        let (a, b) = self.default_era_window();
        (a + b) / 2
    }

    pub const fn is_salsa(self) -> bool {
        matches!(self, Self::SalsaBaulRomantica | Self::SalsaBravaChoque)
    }

    pub const fn is_perreo_urbano(self) -> bool {
        matches!(
            self,
            Self::ReggaetonOldSchool | Self::ReggaetonActualPerreo | Self::DembowReparto
        )
    }

    pub fn from_str_name(raw: &str) -> Option<Self> {
        let norm = normalize_ascii_lower(raw);
        if norm.is_empty() {
            return None;
        }
        if let Ok(code) = norm.parse::<u8>() {
            let fam = Self::from_code(code);
            return (fam != Self::Unknown).then_some(fam);
        }
        for c in 1..=14_u8 {
            let fam = Self::from_code(c);
            if normalize_ascii_lower(fam.as_str()) == norm
                || norm.replace(' ', "") == fam.as_str().to_ascii_lowercase()
            {
                return Some(fam);
            }
        }
        classify_by_tags_or_folder(&[raw.to_string()], "", &norm).map(|(f, _)| f)
    }
}

pub fn normalize_ascii_lower(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_space = true;
    for ch in input.chars() {
        let mapped = match ch {
            'á' | 'à' | 'ä' | 'â' | 'Á' | 'À' | 'Ä' | 'Â' => 'a',
            'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'Ó' | 'Ò' | 'Ö' | 'Ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => 'u',
            'ñ' | 'Ñ' => 'n',
            '_' | '-' | '/' | '\\' | '.' | ',' | '(' | ')' | '[' | ']' => ' ',
            c => c.to_ascii_lowercase(),
        };
        if mapped.is_ascii_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(mapped);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

pub fn classify_by_curated_lexicon(
    artist_norm: &str,
    title_norm: &str,
) -> Option<(BoothGenreFamily, u16, f32)> {
    let joined = format!(" {artist_norm} {title_norm} ");
    let rules: &[(&str, BoothGenreFamily, u16, f32)] = &[
        ("alleh|yorghaki|merengueton", BoothGenreFamily::MerenguetonModerno, 2025, 0.95),
        ("sandy papo|sandy & papo|proyecto uno|ilegales|fulanito|diveana|miguel moly|roberto antonio|fantasmas del caribe|zona rica|natusha|karolina", BoothGenreFamily::Tecnomerengue, 1996, 0.94),
        ("wilfrido vargas|sergio vargas|eddy herrera|elvis crespo|olga tanon|milly quezada|tono rosario|hermanos rosario|rubby perez", BoothGenreFamily::MerengueClasico, 1994, 0.92),
        ("tony vega|grupo niche|los titanes|frankie ruiz|eddie santiago|jerry rivera|gilberto santa rosa|maelo ruiz|willie gonzalez|luis enrique|rey ruiz|tito rojas|lalo rodriguez|paquito guzman|guayacan|adolescentes|salserin|marc anthony", BoothGenreFamily::SalsaBaulRomantica, 1994, 0.94),
        ("hector lavoe|willie colon|fania|oscar d leon|oscar d'leon|dimension latina|gran combo|joe arroyo|celia cruz|sonora poncena|richie ray|ruben blades|chiquito team", BoothGenreFamily::SalsaBravaChoque, 1984, 0.92),
        ("daddy yankee|don omar|wisin|yandel|tego calderon|ivy queen|plan b|zion lennox|hector el father|alexis fido|trebol clan|rkm", BoothGenreFamily::ReggaetonOldSchool, 2006, 0.93),
        ("feid|bad bunny|karol g|rauw alejandro|myke towers|ryan castro|blessd|quevedo|dei v|floyymenor|cris mj|ozuna|j balvin|maluma", BoothGenreFamily::ReggaetonActualPerreo, 2023, 0.93),
        ("el alfa|rochy rd|chimbala|tokischa|bebeshito|bulin 47|kiko el crazy|yailin", BoothGenreFamily::DembowReparto, 2023, 0.92),
        ("keinemusik|black coffee|adam port|rampa|hugel|caiiro|da capo|enoo napa|&me", BoothGenreFamily::AfroHouse, 2023, 0.93),
        ("fumaratto|victor cardenas|daniel parranda|marcela reyes|deorro|gordo|davidj", BoothGenreFamily::GuarachaAleteoTech, 2023, 0.91),
        ("romeo santos|aventura|prince royce|zacarias ferreira|frank reyes|anthony santos|raulin rodriguez", BoothGenreFamily::Bachata, 2012, 0.92),
        ("carlos vives|diomedes diaz|binomio de oro|silvestre dangond|angeles azules|sonora dinamita|pastor lopez|rodolfo aicardi", BoothGenreFamily::CumbiaVallenato, 2004, 0.91),
        ("guaco|maracaibo 15|cardenales del exito|gran coquivacoa|billo", BoothGenreFamily::HoraLocaGaitas, 1998, 0.91),
        ("reynaldo armas|luis silva|jorge guerrero|vitico castillo|yeison jimenez|jessi uribe|paola jara|dario gomez|carin leon|christian nodal|grupo frontera", BoothGenreFamily::LlaneraDespecho, 2015, 0.91),
    ];
    for (pat, fam, year, conf) in rules {
        if pat
            .split('|')
            .any(|tok| joined.contains(&format!(" {tok} ")) || artist_norm.contains(tok))
        {
            return Some((*fam, *year, *conf));
        }
    }
    None
}

pub fn classify_by_tags_or_folder(
    tags_genre: &[String],
    folder: &str,
    title_norm: &str,
) -> Option<(BoothGenreFamily, f32)> {
    let mut hay = String::new();
    for t in tags_genre {
        hay.push(' ');
        hay.push_str(&normalize_ascii_lower(t));
    }
    hay.push(' ');
    hay.push_str(&normalize_ascii_lower(folder));
    hay.push(' ');
    hay.push_str(title_norm);
    let kw_rules: &[(&str, BoothGenreFamily, f32)] = &[
        ("merengueton", BoothGenreFamily::MerenguetonModerno, 0.88),
        (
            "tecnomerengue|euro merengue|merengue house",
            BoothGenreFamily::Tecnomerengue,
            0.88,
        ),
        ("merengue", BoothGenreFamily::MerengueClasico, 0.82),
        (
            "salsa baul|salsa romantica",
            BoothGenreFamily::SalsaBaulRomantica,
            0.88,
        ),
        (
            "salsa brava|salsa choque|salsa",
            BoothGenreFamily::SalsaBravaChoque,
            0.80,
        ),
        (
            "reggaeton old school|old school perreo|clasico reggaeton",
            BoothGenreFamily::ReggaetonOldSchool,
            0.88,
        ),
        (
            "perreo|reggaeton|urbano",
            BoothGenreFamily::ReggaetonActualPerreo,
            0.84,
        ),
        ("dembow|reparto", BoothGenreFamily::DembowReparto, 0.85),
        (
            "afro house|melodic afro|afrohouse",
            BoothGenreFamily::AfroHouse,
            0.86,
        ),
        (
            "guaracha|aleteo|latin tech|tech house|tribal",
            BoothGenreFamily::GuarachaAleteoTech,
            0.84,
        ),
        ("bachata", BoothGenreFamily::Bachata, 0.86),
        ("cumbia|vallenato", BoothGenreFamily::CumbiaVallenato, 0.85),
        (
            "hora loca|gaita|tambor|calipso|carnaval",
            BoothGenreFamily::HoraLocaGaitas,
            0.85,
        ),
        (
            "llanera|despecho|popular|joropo|ranchera",
            BoothGenreFamily::LlaneraDespecho,
            0.85,
        ),
    ];
    for (pat, fam, conf) in kw_rules {
        if pat.split('|').any(|kw| hay.contains(kw)) {
            return Some((*fam, *conf));
        }
    }
    None
}
