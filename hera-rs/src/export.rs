use crate::structure::HotCuePoint;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportTrackItem {
    pub track_id: String,
    pub artist: String,
    pub title: String,
    pub version: Option<String>,
    pub file_path: String,
    pub bpm: f32,
    pub camelot: String,
    pub duration_seconds: f32,
    #[serde(default = "default_bitrate")]
    pub bitrate_kbps: u32,
    #[serde(default = "default_energy")]
    pub energy: f32,
    #[serde(default)]
    pub file_size_bytes: u64,
    #[serde(default)]
    pub is_video: bool,
    #[serde(default)]
    pub is_own_production: bool,
    #[serde(default)]
    pub hot_cues: Vec<HotCuePoint>,
}

fn default_bitrate() -> u32 {
    1411
}
fn default_energy() -> f32 {
    0.82
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCrateRequest {
    pub crate_name: String,
    #[serde(default)]
    pub brief: String,
    pub tracks: Vec<ExportTrackItem>,
    #[serde(default = "default_export_formats")]
    pub formats: Vec<String>,
    pub output_dir: Option<String>,
}

fn default_export_formats() -> Vec<String> {
    vec![
        "m3u8".to_string(),
        "rekordbox_xml".to_string(),
        "virtualdj".to_string(),
        "traktor_nml".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCrateResponse {
    pub crate_name: String,
    pub track_count: usize,
    pub m3u8_content: Option<String>,
    pub rekordbox_xml_content: Option<String>,
    pub virtualdj_folder_xml: Option<String>,
    pub virtualdj_database_xml: Option<String>,
    pub traktor_nml_content: Option<String>,
    pub written_files: Vec<String>,
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '_' || c == '-' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn resolve_four_hot_cues(track: &ExportTrackItem) -> Vec<HotCuePoint> {
    if track.hot_cues.len() >= 4 {
        return track.hot_cues[..4].to_vec();
    }
    let bpm = track.bpm.max(70.0);
    let dur = track.duration_seconds.max(60.0);
    let bar_sec = (60.0 / bpm) * 4.0;
    vec![
        HotCuePoint {
            num: 1,
            name: "1. Intro Beat".to_string(),
            seconds: 0.25,
            bar_index: 0,
            role: "intro_beat".to_string(),
            confidence: 0.95,
        },
        HotCuePoint {
            num: 2,
            name: "2. Phrase In".to_string(),
            seconds: (bar_sec * 8.0).min(dur * 0.25),
            bar_index: 8,
            role: "phrase_in".to_string(),
            confidence: 0.92,
        },
        HotCuePoint {
            num: 3,
            name: "3. Main Drop".to_string(),
            seconds: (bar_sec * 16.0).min(dur * 0.48),
            bar_index: 16,
            role: "main_drop".to_string(),
            confidence: 0.94,
        },
        HotCuePoint {
            num: 4,
            name: "4. Mix Out".to_string(),
            seconds: (dur - bar_sec * 8.0).max(dur * 0.78),
            bar_index: 48,
            role: "mix_out".to_string(),
            confidence: 0.91,
        },
    ]
}

pub fn generate_m3u8(tracks: &[ExportTrackItem]) -> String {
    let mut out = String::from("#EXTM3U\n");
    for t in tracks {
        let dur_int = t.duration_seconds.round() as i64;
        let full_title = match &t.version {
            Some(v) if !v.is_empty() => format!("{} - {} ({})", t.artist, t.title, v),
            _ => format!("{} - {}", t.artist, t.title),
        };
        out.push_str(&format!(
            "#EXTINF:{},{}\n{}\n",
            dur_int, full_title, t.file_path
        ));
    }
    out
}

pub fn generate_rekordbox_xml(crate_name: &str, tracks: &[ExportTrackItem]) -> String {
    let rb_colors = [
        ("40", "226", "20"),  // Cue 1: Green
        ("0", "210", "255"),  // Cue 2: Cyan
        ("255", "45", "45"),  // Cue 3: Red
        ("255", "190", "0"),  // Cue 4: Amber
    ];
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<DJ_PLAYLISTS Version=\"1.0.0\">\n");
    xml.push_str("  <PRODUCT Name=\"Hera\" Version=\"0.2.0\" Company=\"Hera DJ\"/>\n");
    xml.push_str(&format!(
        "  <COLLECTION Entries=\"{}\">\n",
        tracks.len()
    ));

    for (idx, t) in tracks.iter().enumerate() {
        let track_num = idx + 1;
        let full_title = match &t.version {
            Some(v) if !v.is_empty() => format!("{} [{}]", t.title, v),
            _ => t.title.clone(),
        };
        let norm_path = t.file_path.replace('\\', "/");
        let loc_url = format!("file://localhost/{}", norm_path.trim_start_matches('/'));
        let cues = resolve_four_hot_cues(t);
        let cue1_sec = cues.first().map(|c| c.seconds).unwrap_or(0.25);

        xml.push_str(&format!(
            "    <TRACK TrackID=\"{}\" Name=\"{}\" Artist=\"{}\" TotalTime=\"{}\" AverageBpm=\"{:.2}\" Tonality=\"{}\" BitRate=\"{}\" Location=\"{}\" Comments=\"Energy:{:.2}\">\n",
            track_num,
            xml_escape(&full_title),
            xml_escape(&t.artist),
            t.duration_seconds.round() as u32,
            t.bpm,
            xml_escape(&t.camelot),
            t.bitrate_kbps,
            xml_escape(&loc_url),
            t.energy
        ));
        xml.push_str(&format!(
            "      <TEMPO Inizio=\"{:.3}\" Bpm=\"{:.2}\" Metro=\"4/4\" Battito=\"1\"/>\n",
            cue1_sec, t.bpm
        ));
        for (c_idx, cue) in cues.iter().take(4).enumerate() {
            let (r, g, b) = rb_colors[c_idx];
            xml.push_str(&format!(
                "      <POSITION_MARK Name=\"{}\" Type=\"0\" Start=\"{:.3}\" Num=\"{}\" Red=\"{}\" Green=\"{}\" Blue=\"{}\"/>\n",
                xml_escape(&cue.name),
                cue.seconds,
                c_idx,
                r,
                g,
                b
            ));
        }
        xml.push_str("    </TRACK>\n");
    }

    xml.push_str("  </COLLECTION>\n");
    xml.push_str("  <PLAYLISTS>\n");
    xml.push_str("    <NODE Type=\"0\" Name=\"ROOT\">\n");
    xml.push_str(&format!(
        "      <NODE Type=\"1\" Name=\"{}\" KeyType=\"0\" Entries=\"{}\">\n",
        xml_escape(crate_name),
        tracks.len()
    ));
    for idx in 1..=tracks.len() {
        xml.push_str(&format!("        <TRACK Key=\"{}\"/>\n", idx));
    }
    xml.push_str("      </NODE>\n    </NODE>\n  </PLAYLISTS>\n</DJ_PLAYLISTS>\n");
    xml
}

pub fn generate_virtualdj_exports(
    crate_name: &str,
    tracks: &[ExportTrackItem],
) -> (String, String) {
    let vdj_colors = ["#28E214", "#00D2FF", "#FF2D2D", "#FFBE00"];
    let mut folder_xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    folder_xml.push_str(&format!(
        "<VirtualFolder Name=\"{}\">\n",
        xml_escape(crate_name)
    ));

    let mut db_xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    db_xml.push_str("<VirtualDJ_Database Version=\"2026\">\n");

    for (idx, t) in tracks.iter().enumerate() {
        let full_title = match &t.version {
            Some(v) if !v.is_empty() => format!("{} ({})", t.title, v),
            _ => t.title.clone(),
        };
        let beat_interval = 60.0 / t.bpm.max(1.0);
        let cues = resolve_four_hot_cues(t);
        let cue1_sec = cues.first().map(|c| c.seconds).unwrap_or(0.25);

        folder_xml.push_str(&format!(
            "  <Song Path=\"{}\" Idx=\"{}\" Artist=\"{}\" Title=\"{}\" Bpm=\"{:.2}\" Key=\"{}\"/>\n",
            xml_escape(&t.file_path),
            idx + 1,
            xml_escape(&t.artist),
            xml_escape(&full_title),
            t.bpm,
            xml_escape(&t.camelot)
        ));

        db_xml.push_str(&format!(
            "  <Song FilePath=\"{}\" FileSize=\"{}\">\n",
            xml_escape(&t.file_path),
            t.file_size_bytes
        ));
        db_xml.push_str(&format!(
            "    <Tags Author=\"{}\" Title=\"{}\" Bpm=\"{:.2}\" Key=\"{}\" Comment=\"Hera Set: {}\"/>\n",
            xml_escape(&t.artist),
            xml_escape(&full_title),
            t.bpm,
            xml_escape(&t.camelot),
            xml_escape(crate_name)
        ));
        db_xml.push_str(&format!(
            "    <Infos SongLength=\"{:.3}\" Bitrate=\"{}\"/>\n",
            t.duration_seconds, t.bitrate_kbps
        ));
        db_xml.push_str(&format!(
            "    <Scan Version=\"801\" Bpm=\"{:.6}\" Key=\"{}\"/>\n",
            beat_interval,
            xml_escape(&t.camelot)
        ));
        db_xml.push_str(&format!(
            "    <Poi Pos=\"{:.3}\" Type=\"beatgrid\" Bpm=\"{:.6}\"/>\n",
            cue1_sec, beat_interval
        ));
        for (c_idx, cue) in cues.iter().take(4).enumerate() {
            db_xml.push_str(&format!(
                "    <Poi Name=\"{}\" Pos=\"{:.3}\" Num=\"{}\" Color=\"{}\" Type=\"cue\"/>\n",
                xml_escape(&cue.name),
                cue.seconds,
                c_idx + 1,
                vdj_colors[c_idx]
            ));
        }
        db_xml.push_str("  </Song>\n");
    }

    folder_xml.push_str("</VirtualFolder>\n");
    db_xml.push_str("</VirtualDJ_Database>\n");
    (folder_xml, db_xml)
}

pub fn generate_traktor_nml(crate_name: &str, tracks: &[ExportTrackItem]) -> String {
    let mut nml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    nml.push_str("<NML VERSION=\"19\">\n");
    nml.push_str("  <HEAD COMPANY=\"Hera DJ\" PROGRAM=\"Hera\"/>\n");
    nml.push_str(&format!(
        "  <COLLECTION ENTRIES=\"{}\">\n",
        tracks.len()
    ));

    let mut playlist_keys = Vec::with_capacity(tracks.len());

    for t in tracks {
        let full_title = match &t.version {
            Some(v) if !v.is_empty() => format!("{} [{}]", t.title, v),
            _ => t.title.clone(),
        };
        let norm_path = t.file_path.replace('\\', "/");
        let (dir_part, file_part) = match norm_path.rfind('/') {
            Some(pos) => (&norm_path[..pos], &norm_path[pos + 1..]),
            None => ("", norm_path.as_str()),
        };
        let segs: Vec<&str> = dir_part.split('/').filter(|s| !s.is_empty()).collect();
        let traktor_dir = if segs.is_empty() {
            "/:".to_string()
        } else {
            format!("/:{}/:", segs.join("/:"))
        };
        let traktor_key = format!("LocalDisk{}{}", traktor_dir, file_part);
        playlist_keys.push(traktor_key);

        nml.push_str(&format!(
            "    <ENTRY TITLE=\"{}\" ARTIST=\"{}\">\n",
            xml_escape(&full_title),
            xml_escape(&t.artist)
        ));
        nml.push_str(&format!(
            "      <LOCATION DIR=\"{}\" FILE=\"{}\" VOLUME=\"LocalDisk\" VOLUMEID=\"LocalDisk\"/>\n",
            xml_escape(&traktor_dir),
            xml_escape(file_part)
        ));
        nml.push_str(&format!(
            "      <INFO BITRATE=\"{}\" PLAYTIME=\"{}\" PLAYTIME_FLOAT=\"{:.3}\" COMMENT=\"Camelot:{} | Energy:{:.2}\"/>\n",
            t.bitrate_kbps * 1000,
            t.duration_seconds.round() as u32,
            t.duration_seconds,
            xml_escape(&t.camelot),
            t.energy
        ));
        nml.push_str(&format!(
            "      <TEMPO BPM=\"{:.2}\" BPM_QUALITY=\"100.000000\"/>\n",
            t.bpm
        ));
        nml.push_str(&format!(
            "      <MUSICAL_KEY VALUE=\"{}\"/>\n",
            xml_escape(&t.camelot)
        ));

        let cues = resolve_four_hot_cues(t);
        for (c_idx, cue) in cues.iter().take(4).enumerate() {
            let start_ms = cue.seconds * 1000.0;
            nml.push_str(&format!(
                "      <CUE_V2 NAME=\"{}\" DISPL_ORDER=\"0\" TYPE=\"0\" START=\"{:.3}\" LEN=\"0.000000\" REPEATS=\"-1\" HOTCUE=\"{}\"/>\n",
                xml_escape(&cue.name),
                start_ms,
                c_idx
            ));
        }
        nml.push_str("    </ENTRY>\n");
    }

    nml.push_str("  </COLLECTION>\n");
    nml.push_str("  <PLAYLISTS>\n");
    nml.push_str("    <NODE TYPE=\"FOLDER\" NAME=\"$ROOT\">\n");
    nml.push_str("      <SUBNODES COUNT=\"1\">\n");
    nml.push_str(&format!(
        "        <NODE TYPE=\"PLAYLIST\" NAME=\"{}\">\n",
        xml_escape(crate_name)
    ));
    nml.push_str(&format!(
        "          <PLAYLIST ENTRIES=\"{}\" TYPE=\"LIST\" UUID=\"hera-crate\">\n",
        playlist_keys.len()
    ));
    for pk in &playlist_keys {
        nml.push_str(&format!(
            "            <ENTRY><PRIMARYKEY TYPE=\"TRACK\" KEY=\"{}\"/></ENTRY>\n",
            xml_escape(pk)
        ));
    }
    nml.push_str("          </PLAYLIST>\n        </NODE>\n      </SUBNODES>\n    </NODE>\n  </PLAYLISTS>\n</NML>\n");
    nml
}

pub fn export_crate_bundle(req: &ExportCrateRequest) -> ExportCrateResponse {
    let want_all = req.formats.iter().any(|f| f.eq_ignore_ascii_case("all"));
    let has_fmt = |target: &str| {
        want_all || req.formats.iter().any(|f| f.eq_ignore_ascii_case(target))
    };

    let m3u8_content = if has_fmt("m3u8") {
        Some(generate_m3u8(&req.tracks))
    } else {
        None
    };

    let rekordbox_xml_content = if has_fmt("rekordbox_xml") || has_fmt("rekordbox") {
        Some(generate_rekordbox_xml(&req.crate_name, &req.tracks))
    } else {
        None
    };

    let (virtualdj_folder_xml, virtualdj_database_xml) =
        if has_fmt("virtualdj") || has_fmt("vdj") {
            let (f_xml, db_xml) = generate_virtualdj_exports(&req.crate_name, &req.tracks);
            (Some(f_xml), Some(db_xml))
        } else {
            (None, None)
        };

    let traktor_nml_content = if has_fmt("traktor_nml") || has_fmt("traktor") {
        Some(generate_traktor_nml(&req.crate_name, &req.tracks))
    } else {
        None
    };

    #[allow(unused_mut)]
    let mut written_files = Vec::new();

    #[cfg(not(target_arch = "wasm32"))]
    if let Some(dir_str) = &req.output_dir {
        let dir_path = std::path::Path::new(dir_str);
        if std::fs::create_dir_all(dir_path).is_ok() {
            let clean = sanitize_filename(&req.crate_name);
            if let Some(c) = &m3u8_content {
                let p = dir_path.join(format!("{}.m3u8", clean));
                if std::fs::write(&p, c).is_ok() {
                    written_files.push(p.to_string_lossy().to_string());
                }
            }
            if let Some(c) = &rekordbox_xml_content {
                let p = dir_path.join(format!("{}_rekordbox.xml", clean));
                if std::fs::write(&p, c).is_ok() {
                    written_files.push(p.to_string_lossy().to_string());
                }
            }
            if let Some(c) = &virtualdj_folder_xml {
                let p = dir_path.join(format!("{}.vdjfolder", clean));
                if std::fs::write(&p, c).is_ok() {
                    written_files.push(p.to_string_lossy().to_string());
                }
            }
            if let Some(c) = &virtualdj_database_xml {
                let p = dir_path.join(format!("{}_vdj_database.xml", clean));
                if std::fs::write(&p, c).is_ok() {
                    written_files.push(p.to_string_lossy().to_string());
                }
            }
            if let Some(c) = &traktor_nml_content {
                let p = dir_path.join(format!("{}_traktor.nml", clean));
                if std::fs::write(&p, c).is_ok() {
                    written_files.push(p.to_string_lossy().to_string());
                }
            }
        }
    }

    ExportCrateResponse {
        crate_name: req.crate_name.clone(),
        track_count: req.tracks.len(),
        m3u8_content,
        rekordbox_xml_content,
        virtualdj_folder_xml,
        virtualdj_database_xml,
        traktor_nml_content,
        written_files,
    }
}
