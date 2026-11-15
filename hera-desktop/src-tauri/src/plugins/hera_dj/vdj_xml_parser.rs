use std::collections::HashMap;
use std::io::BufRead;

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyCue {
    pub num: u8,
    pub name: String,
    pub seconds: f64,
}

#[derive(Debug, Clone, Default)]
pub struct VdjParsedEntry {
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub cues_count: usize,
    pub cues: Vec<LegacyCue>,
    pub author: Option<String>,
    pub title: Option<String>,
    pub duration_sec: Option<f64>,
    pub bitrate_kbps: Option<u32>,
}

pub fn unescape_xml(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            let mut entity = String::new();
            let mut found_semi = false;
            for ec in chars.by_ref() {
                if ec == ';' {
                    found_semi = true;
                    break;
                }
                entity.push(ec);
                if entity.len() > 10 {
                    break;
                }
            }
            if found_semi {
                match entity.as_str() {
                    "amp" => out.push('&'),
                    "apos" => out.push('\''),
                    "quot" => out.push('"'),
                    "lt" => out.push('<'),
                    "gt" => out.push('>'),
                    s if s.starts_with("#x") || s.starts_with("#X") => {
                        if let Ok(code) = u32::from_str_radix(&s[2..], 16) {
                            if let Some(ch) = char::from_u32(code) {
                                out.push(ch);
                                continue;
                            }
                        }
                        out.push('&');
                        out.push_str(&entity);
                        out.push(';');
                    }
                    s if s.starts_with('#') => {
                        if let Ok(code) = s[1..].parse::<u32>() {
                            if let Some(ch) = char::from_u32(code) {
                                out.push(ch);
                                continue;
                            }
                        }
                        out.push('&');
                        out.push_str(&entity);
                        out.push(';');
                    }
                    _ => {
                        out.push('&');
                        out.push_str(&entity);
                        out.push(';');
                    }
                }
            } else {
                out.push('&');
                out.push_str(&entity);
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn normalize_lookup_path(raw: &str) -> String {
    let unescaped = unescape_xml(raw);
    let slashed = unescaped.replace('\\', "/");
    let stripped = slashed.strip_prefix("//?/").unwrap_or(&slashed);
    stripped.to_lowercase()
}

pub fn parse_virtualdj_bpm_value(raw: &str) -> Option<f64> {
    let val = raw.trim().parse::<f64>().ok()?;
    if (0.20..=1.50).contains(&val) {
        Some(((60.0 / val) * 100.0).round() / 100.0)
    } else if (35.0..=240.0).contains(&val) {
        Some(val)
    } else {
        None
    }
}

pub fn normalize_key_to_camelot(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    let up = t.to_ascii_uppercase();
    if let Some(suffix) = up.strip_suffix('A').or_else(|| up.strip_suffix('B')) {
        if let Ok(n) = suffix.parse::<u8>() {
            if (1..=12).contains(&n) {
                let letter = up.chars().last().unwrap();
                return Some(format!("{n}{letter}"));
            }
        }
    }
    let mapped = match t.to_ascii_lowercase().as_str() {
        "abm" | "g#m" => "1A", "b" | "cb" => "1B",
        "ebm" | "d#m" => "2A", "f#" | "gb" => "2B",
        "bbm" | "a#m" => "3A", "db" | "c#" => "3B",
        "fm" => "4A", "ab" | "g#" => "4B",
        "cm" => "5A", "eb" | "d#" => "5B",
        "gm" => "6A", "bb" | "a#" => "6B",
        "dm" => "7A", "f" => "7B",
        "am" => "8A", "c" => "8B",
        "em" => "9A", "g" => "9B",
        "bm" => "10A", "d" => "10B",
        "f#m" | "gbm" => "11A", "a" => "11B",
        "c#m" | "dbm" => "12A", "e" => "12B",
        _ => return Some(t.to_string()),
    };
    Some(mapped.to_string())
}

fn extract_attr<'a>(line: &'a str, attr: &str) -> Option<&'a str> {
    let pos = line.find(attr)?;
    let rest = &line[pos + attr.len()..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

pub fn parse_vdj_database_reader<R: BufRead>(reader: R) -> HashMap<String, VdjParsedEntry> {
    let mut map = HashMap::new();
    let mut current_path: Option<String> = None;
    let mut current = VdjParsedEntry::default();

    for line_res in reader.lines() {
        let Ok(line) = line_res else { continue };
        let trimmed = line.trim();

        if trimmed.starts_with("<Song ") {
            if let Some(fp) = current_path.take() {
                map.insert(normalize_lookup_path(&fp), current);
            }
            current = VdjParsedEntry::default();
            if let Some(fp) = extract_attr(trimmed, "FilePath=\"") {
                current_path = Some(fp.to_string());
            }
        } else if trimmed.starts_with("<Scan ") {
            if let Some(bpm_s) = extract_attr(trimmed, "Bpm=\"") {
                if let Some(b) = parse_virtualdj_bpm_value(bpm_s) {
                    current.bpm = Some(b);
                }
            }
            if let Some(key_s) = extract_attr(trimmed, "Key=\"") {
                current.key = normalize_key_to_camelot(key_s);
            }
        } else if trimmed.starts_with("<Tags ") {
            if current.bpm.is_none() {
                if let Some(bpm_s) = extract_attr(trimmed, "Bpm=\"") {
                    current.bpm = parse_virtualdj_bpm_value(bpm_s);
                }
            }
            if current.key.is_none() {
                if let Some(key_s) = extract_attr(trimmed, "Key=\"") {
                    current.key = normalize_key_to_camelot(key_s);
                }
            }
            if let Some(a) = extract_attr(trimmed, "Author=\"") {
                current.author = Some(unescape_xml(a));
            }
            if let Some(t) = extract_attr(trimmed, "Title=\"") {
                current.title = Some(unescape_xml(t));
            }
        } else if trimmed.starts_with("<Infos ") {
            current.duration_sec = extract_attr(trimmed, "SongLength=\"").and_then(|s| s.parse().ok());
            current.bitrate_kbps = extract_attr(trimmed, "Bitrate=\"").and_then(|s| s.parse().ok());
        } else if trimmed.starts_with("<Bpm ") && current.bpm.is_none() {
            if let Some(v) = extract_attr(trimmed, "Value=\"") {
                current.bpm = parse_virtualdj_bpm_value(v);
            }
        } else if trimmed.starts_with("<Key ") && current.key.is_none() {
            if let Some(v) = extract_attr(trimmed, "Value=\"") {
                current.key = normalize_key_to_camelot(v);
            }
        } else if trimmed.starts_with("<Poi ") {
            current.cues_count += 1;
            if current.cues.len() < 8 && extract_attr(trimmed, "Type=\"") == Some("cue") {
                if let Some(pos_s) = extract_attr(trimmed, "Pos=\"").and_then(|s| s.parse::<f64>().ok()) {
                    let num = extract_attr(trimmed, "Num=\"")
                        .and_then(|s| s.parse::<u8>().ok())
                        .unwrap_or((current.cues.len() + 1) as u8);
                    let name = extract_attr(trimmed, "Name=\"")
                        .map(unescape_xml)
                        .unwrap_or_else(|| format!("Cue {num}"));
                    current.cues.push(LegacyCue { num, name, seconds: pos_s });
                }
            }
        }
    }
    if let Some(fp) = current_path {
        map.insert(normalize_lookup_path(&fp), current);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_vdj_2026_scan_bpm_key_xml_entities_and_pois() {
        let xml = r#"<VirtualDJ_Database Version="8.5">
  <Song FilePath="C:\Music\Alleh &amp; Yorghaki - Único.webm" FileSize="123456">
    <Tags Author="Alleh &amp; Yorghaki" Title="Único" />
    <Infos SongLength="214.5" Bitrate="320" />
    <Scan Version="801" Bpm="0.472441" Key="F#m" />
    <Poi Name="Intro" Pos="15.495" Num="1" Type="cue" />
    <Poi Name="Drop" Pos="45.100" Num="2" Type="cue" />
    <Poi Name="Break" Pos="90.000" Num="3" Type="cue" />
    <Poi Name="Outro" Pos="180.250" Num="4" Type="cue" />
  </Song>
</VirtualDJ_Database>"#;
        let map = parse_vdj_database_reader(Cursor::new(xml));
        let lookup = normalize_lookup_path(r"\\?\C:\Music\Alleh & Yorghaki - Único.webm");
        let entry = map.get(&lookup).expect("should find entry with \\\\?\\ prefix");
        assert_eq!(entry.bpm, Some(127.0));
        assert_eq!(entry.key.as_deref(), Some("11A"));
        assert_eq!(entry.cues.len(), 4);
        assert_eq!(entry.author.as_deref(), Some("Alleh & Yorghaki"));
    }
}
