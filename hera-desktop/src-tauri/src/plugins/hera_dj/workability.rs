use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Cached external DJ metadata extracted in Stage 1 (< 3s, 0% CPU).
#[derive(Debug, Clone, Default)]
pub struct CachedDjSoftwareMetadata {
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub cues_count: usize,
}

/// Evaluates if a file or folder path should be discarded outright:
/// - Junk/system folders: `$RECYCLE.BIN`, `System Volume Information`, `node_modules`, `.git`, WhatsApp notes
/// - Audios < 25s: FX, jingles, samples, voice memos
pub fn is_junk_system_directory(path: &Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    s.contains("$recycle.bin")
        || s.contains("system volume information")
        || s.contains("node_modules")
        || s.contains(".git")
        || s.contains("whatsapp audio")
        || s.contains("whatsapp voice notes")
        || s.contains(".trashes")
        || s.contains(".spotlight-v100")
}

/// Check if duration represents workable DJ material:
/// - `< 25.0s`: Discarded as short SFX / voice memo.
/// - `25.0s ..= 600.0s`: Standard workable DJ track.
/// - `> 600.0s`: Extended session / recorded live mix -> Mixes (> 10m) area.
pub fn classify_track_duration(duration_seconds: f64) -> DurationCategory {
    if duration_seconds < 25.0 {
        DurationCategory::ShortSfxOrDiscard
    } else if duration_seconds > 600.0 {
        DurationCategory::MixOver10m
    } else {
        DurationCategory::WorkableTrack
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationCategory {
    ShortSfxOrDiscard,
    WorkableTrack,
    MixOver10m,
}

/// Multi-software external DB index containing pre-computed BPM, Key and Cues
/// from VirtualDJ database.xml and Rekordbox XML.
#[derive(Debug, Default)]
pub struct ExternalDjSoftwareIndex {
    pub vdj_entries: HashMap<String, CachedDjSoftwareMetadata>,
    pub rekordbox_entries: HashMap<String, CachedDjSoftwareMetadata>,
}

impl ExternalDjSoftwareIndex {
    /// Loads metadata from detected VirtualDJ and Rekordbox databases in < 3s.
    pub fn build_fast_index() -> Self {
        let mut index = Self::default();
        index.load_virtualdj_databases();
        index.load_rekordbox_xmls();
        index
    }

    fn load_virtualdj_databases(&mut self) {
        for home in candidate_virtualdj_homes() {
            let db_xml_path = home.join("database.xml");
            if db_xml_path.is_file() {
                self.parse_vdj_database_file(&db_xml_path);
            }
        }
    }

    fn parse_vdj_database_file(&mut self, path: &Path) {
        let Ok(file) = File::open(path) else { return };
        let reader = BufReader::new(file);

        let mut current_filepath: Option<String> = None;
        let mut current_bpm: Option<f64> = None;
        let mut current_key: Option<String> = None;
        let mut current_cues = 0usize;

        for line_res in reader.lines() {
            let Ok(line) = line_res else { continue };
            let trimmed = line.trim();

            if trimmed.starts_with("<Song ") {
                if let Some(fp) = current_filepath.take() {
                    let key = fp.replace('\\', "/").to_lowercase();
                    self.vdj_entries.insert(
                        key,
                        CachedDjSoftwareMetadata {
                            bpm: current_bpm,
                            key: current_key.take(),
                            cues_count: current_cues,
                        },
                    );
                }
                current_bpm = None;
                current_key = None;
                current_cues = 0;

                if let Some(pos) = trimmed.find("FilePath=\"") {
                    let rest = &trimmed[pos + 10..];
                    if let Some(end) = rest.find('"') {
                        current_filepath = Some(rest[..end].to_string());
                    }
                }
            } else if trimmed.starts_with("<Bpm ") {
                if let Some(pos) = trimmed.find("Value=\"") {
                    let rest = &trimmed[pos + 7..];
                    if let Some(end) = rest.find('"') {
                        if let Ok(val) = rest[..end].parse::<f64>() {
                            if val > 30.0 && val < 300.0 {
                                current_bpm = Some(val);
                            }
                        }
                    }
                }
            } else if trimmed.starts_with("<Key ") {
                if let Some(pos) = trimmed.find("Value=\"") {
                    let rest = &trimmed[pos + 7..];
                    if let Some(end) = rest.find('"') {
                        current_key = Some(rest[..end].to_string());
                    }
                }
            } else if trimmed.starts_with("<Poi ") {
                current_cues += 1;
            }
        }

        if let Some(fp) = current_filepath {
            let key = fp.replace('\\', "/").to_lowercase();
            self.vdj_entries.insert(
                key,
                CachedDjSoftwareMetadata {
                    bpm: current_bpm,
                    key: current_key,
                    cues_count: current_cues,
                },
            );
        }
    }

    fn load_rekordbox_xmls(&mut self) {
        // Look in standard Rekordbox export or home locations
        if let Ok(home) = std::env::var("HOME") {
            let rb_path = PathBuf::from(home)
                .join("PIONEER")
                .join("rekordbox")
                .join("rekordbox.xml");
            if rb_path.is_file() {
                self.parse_rekordbox_xml_file(&rb_path);
            }
        }
    }

    fn parse_rekordbox_xml_file(&mut self, path: &Path) {
        let Ok(file) = File::open(path) else { return };
        let reader = BufReader::new(file);

        for line_res in reader.lines() {
            let Ok(line) = line_res else { continue };
            let trimmed = line.trim();
            if trimmed.starts_with("<TRACK ") && trimmed.contains("Location=\"") {
                let location = extract_xml_attr(trimmed, "Location=\"");
                let bpm = extract_xml_attr(trimmed, "AverageBpm=\"").and_then(|b| b.parse::<f64>().ok());
                let tonality = extract_xml_attr(trimmed, "Tonality=\"");

                if let Some(loc) = location {
                    let clean_path = loc
                        .replace("file://localhost/", "")
                        .replace('\\', "/")
                        .to_lowercase();
                    self.rekordbox_entries.insert(
                        clean_path,
                        CachedDjSoftwareMetadata {
                            bpm,
                            key: tonality,
                            cues_count: 0,
                        },
                    );
                }
            }
        }
    }

    /// Retrieve pre-computed metadata for a track path (VirtualDJ or Rekordbox).
    pub fn lookup(&self, path: &Path) -> Option<&CachedDjSoftwareMetadata> {
        let norm = path.to_string_lossy().replace('\\', "/").to_lowercase();
        self.vdj_entries
            .get(&norm)
            .or_else(|| self.rekordbox_entries.get(&norm))
    }
}

fn extract_xml_attr(line: &str, attr_prefix: &str) -> Option<String> {
    let pos = line.find(attr_prefix)?;
    let rest = &line[pos + attr_prefix.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn candidate_virtualdj_homes() -> Vec<PathBuf> {
    let mut homes = Vec::new();
    if let Ok(override_dir) = std::env::var("HERA_VDJ_HOME")
        && !override_dir.trim().is_empty()
    {
        homes.push(PathBuf::from(override_dir));
    }
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        homes.push(PathBuf::from(&user_profile).join("Documents").join("VirtualDJ"));
    }
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        homes.push(PathBuf::from(local_app_data).join("VirtualDJ"));
    }
    if let Ok(home) = std::env::var("HOME") {
        homes.push(PathBuf::from(home).join("Documents").join("VirtualDJ"));
    }
    homes
}

use std::sync::atomic::{AtomicBool, Ordering};
static SCAN_CANCEL_FLAG: AtomicBool = AtomicBool::new(false);

/// Signals background library scanner to halt immediately (< 100ms).
pub fn cancel_scan_flag() {
    SCAN_CANCEL_FLAG.store(true, Ordering::SeqCst);
}

/// Resets the scan cancellation flag before starting a new scan.
pub fn reset_scan_flag() {
    SCAN_CANCEL_FLAG.store(false, Ordering::SeqCst);
}

/// Checks whether the user has requested to cancel the scan.
pub fn is_scan_cancelled() -> bool {
    SCAN_CANCEL_FLAG.load(Ordering::Relaxed)
}

