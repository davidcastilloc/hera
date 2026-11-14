    use super::*;
    use std::fs;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum HintSource {
        VirtualDjDatabase,
        EmbeddedTag,
        FilenamePattern,
        UserProvided,
    }

    impl HintSource {
        pub fn prior_weight(self) -> f64 {
            match self {
                Self::UserProvided => 0.22,
                Self::VirtualDjDatabase => 0.16,
                Self::EmbeddedTag => 0.14,
                Self::FilenamePattern => 0.12,
            }
        }
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct BpmHint {
        pub bpm: f64,
        pub source: HintSource,
    }

    /// En VirtualDJ `database.xml`, `<Scan Bpm="0.476190"/>` almacena segundos/beat (`60 / BPM`).
    pub fn parse_virtualdj_bpm_value(raw: &str) -> Option<f64> {
        let cleaned = raw.trim().replace(',', ".");
        let val: f64 = cleaned.parse().ok()?;
        if !val.is_finite() || val <= 0.0 {
            return None;
        }
        let bpm = if (0.20..=1.50).contains(&val) {
            60.0 / val
        } else {
            val
        };
        if (45.0..=230.0).contains(&bpm) {
            Some(bpm)
        } else {
            None
        }
    }

    /// Extrae BPM desde nombres como `"[126 BPM]"`, `"[82 BPM - 8B]"`, `"(125bpm)"`, `"_95_bpm"`.
    pub fn extract_bpm_from_filename(path: &Path) -> Option<f64> {
        let stem = path.file_stem()?.to_str()?.to_ascii_lowercase();
        let bytes = stem.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            if bytes[i].is_ascii_digit() {
                let start = i;
                let mut has_dot = false;
                while i < len && (bytes[i].is_ascii_digit() || (!has_dot && bytes[i] == b'.')) {
                    if bytes[i] == b'.' {
                        has_dot = true;
                    }
                    i += 1;
                }
                let num_str = &stem[start..i];
                let mut j = i;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'_' || bytes[j] == b'-') {
                    j += 1;
                }
                if stem[j..].starts_with("bpm") {
                    let after_bpm = j + 3;
                    let valid_boundary =
                        after_bpm >= len || !bytes[after_bpm].is_ascii_alphabetic();
                    let prev_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
                    if valid_boundary && prev_ok {
                        if let Ok(v) = num_str.parse::<f64>() {
                            if (50.0..=220.0).contains(&v) {
                                return Some(v);
                            }
                        }
                    }
                }
            } else {
                i += 1;
            }
        }
        // Soporte adicional para prefijos de DJ Pool al inicio: "[90] SALGO PA LA CALLE"
        let trimmed = stem.trim_start();
        if trimmed.starts_with('[') {
            if let Some(close_idx) = trimmed.find(']') {
                let inside = trimmed[1..close_idx].trim();
                if !inside.is_empty() && inside.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
                    if let Ok(v) = inside.parse::<f64>() {
                        if (60.0..=195.0).contains(&v) {
                            return Some(v);
                        }
                    }
                }
            }
        }
        None
    }

    #[derive(Debug, Default, Clone)]
    pub struct VirtualDjDbIndex {
        by_normalized_path: HashMap<String, (Option<f64>, Option<String>)>,
        by_filename_lower: HashMap<String, (Option<f64>, Option<String>)>,
    }

    impl VirtualDjDbIndex {
        pub fn from_xml_file(xml_path: &Path) -> Result<Self, EngineError> {
            let content =
                fs::read_to_string(xml_path).map_err(|e| EngineError::Io(e.to_string()))?;
            Ok(Self::from_xml_str(&content))
        }

        pub fn from_xml_str(xml: &str) -> Self {
            let mut index = Self::default();
            let mut cursor = 0;

            while let Some(song_rel) = xml[cursor..].find("<Song ") {
                let song_start = cursor + song_rel;
                let song_end = match xml[song_start..].find("</Song>") {
                    Some(end_rel) => song_start + end_rel + "</Song>".len(),
                    None => break,
                };
                let block = &xml[song_start..song_end];
                cursor = song_end;

                let file_path = extract_xml_attr(block, "FilePath");
                let scan_bpm = block
                    .find("<Scan ")
                    .and_then(|pos| extract_xml_attr(&block[pos..], "Bpm"))
                    .and_then(|s| parse_virtualdj_bpm_value(&s));
                let tags_bpm = block
                    .find("<Tags ")
                    .and_then(|pos| extract_xml_attr(&block[pos..], "Bpm"))
                    .and_then(|s| parse_virtualdj_bpm_value(&s));
                let scan_key = block
                    .find("<Scan ")
                    .and_then(|pos| extract_xml_attr(&block[pos..], "Key"));

                let best_bpm = scan_bpm.or(tags_bpm);
                if best_bpm.is_none() && scan_key.is_none() {
                    continue;
                }

                if let Some(fp) = file_path {
                    let norm = fp.replace('\\', "/").to_ascii_lowercase();
                    if let Some(fname) = Path::new(&fp).file_name().and_then(|n| n.to_str()) {
                        index.by_filename_lower.insert(
                            fname.to_ascii_lowercase(),
                            (best_bpm, scan_key.clone()),
                        );
                    }
                    index.by_normalized_path.insert(norm, (best_bpm, scan_key));
                }
            }
            index
        }

        pub fn lookup_bpm(&self, track_path: &Path) -> Option<f64> {
            let norm = track_path.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
            if let Some((Some(bpm), _)) = self.by_normalized_path.get(&norm) {
                return Some(*bpm);
            }
            let fname = track_path.file_name()?.to_str()?.to_ascii_lowercase();
            self.by_filename_lower.get(&fname).and_then(|(b, _)| *b)
        }
    }

    fn extract_xml_attr(fragment: &str, attr_name: &str) -> Option<String> {
        let tag_end = fragment.find('>')?;
        let header = &fragment[..tag_end];
        let pattern = format!("{}=\"", attr_name);
        let start = header.find(&pattern)? + pattern.len();
        let end = header[start..].find('"')? + start;
        Some(
            header[start..end]
                .replace("&amp;", "&")
                .replace("&quot;", "\"")
                .replace("&apos;", "'"),
        )
    }

    pub fn resolve_best_hint(
        explicit_hint: Option<f64>,
        track_path: Option<&Path>,
        embedded_tag_bpm: Option<f64>,
        vdj_db: Option<&VirtualDjDbIndex>,
    ) -> Option<BpmHint> {
        if let Some(b) = explicit_hint.filter(|&x| (45.0..=230.0).contains(&x)) {
            return Some(BpmHint {
                bpm: b,
                source: HintSource::UserProvided,
            });
        }
        if let (Some(db), Some(p)) = (vdj_db, track_path) {
            if let Some(b) = db.lookup_bpm(p) {
                return Some(BpmHint {
                    bpm: b,
                    source: HintSource::VirtualDjDatabase,
                });
            }
        }
        if let Some(b) = embedded_tag_bpm.filter(|&x| (45.0..=230.0).contains(&x)) {
            return Some(BpmHint {
                bpm: b,
                source: HintSource::EmbeddedTag,
            });
        }
        if let Some(p) = track_path {
            if let Some(b) = extract_bpm_from_filename(p) {
                return Some(BpmHint {
                    bpm: b,
                    source: HintSource::FilenamePattern,
                });
            }
        }
        None
    }
