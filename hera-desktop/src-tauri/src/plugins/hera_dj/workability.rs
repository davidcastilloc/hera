use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[allow(unused_imports)]
pub use super::workability_discovery::{
    discover_workable_roots_from_existing_dbs, discover_workable_roots_from_index,
};
#[allow(unused_imports)]
pub use hera_ipc::dj_types::DiscoveredMusicRoot;

/// Cached external DJ metadata extracted in Stage 1 (< 3s, 0% CPU).
#[derive(Debug, Clone, Default)]
pub struct CachedDjSoftwareMetadata {
    pub bpm: Option<f64>,
    pub key: Option<String>,
    #[allow(dead_code)]
    pub cues_count: usize,
    pub author: Option<String>,
    pub title: Option<String>,
    pub duration_sec: Option<f64>,
    pub bitrate_kbps: Option<u32>,
    pub cues: Vec<super::vdj_xml_parser::LegacyCue>,
    pub original_path: Option<String>,
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
#[allow(dead_code)]
pub fn classify_track_duration(duration_seconds: f64) -> DurationCategory {
    if duration_seconds < 25.0 {
        DurationCategory::ShortSfxOrDiscard
    } else if duration_seconds > 600.0 {
        DurationCategory::MixOver10m
    } else {
        DurationCategory::WorkableTrack
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationCategory {
    ShortSfxOrDiscard,
    WorkableTrack,
    MixOver10m,
}

/// Multi-software external DB index containing pre-computed BPM, Key and Cues
/// from VirtualDJ `database.xml`, Rekordbox XML, and Traktor `collection.nml`.
#[derive(Debug, Default)]
pub struct ExternalDjSoftwareIndex {
    pub vdj_entries: HashMap<String, CachedDjSoftwareMetadata>,
    pub rekordbox_entries: HashMap<String, CachedDjSoftwareMetadata>,
    pub traktor_entries: HashMap<String, CachedDjSoftwareMetadata>,
}

impl ExternalDjSoftwareIndex {
    /// Loads metadata from detected VirtualDJ, Rekordbox, and Traktor databases in < 3s.
    pub fn build_fast_index() -> Self {
        let mut index = Self::default();
        index.load_virtualdj_databases();
        index.load_rekordbox_xmls();
        index.load_traktor_nmls();
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
        let parsed = super::vdj_xml_parser::parse_vdj_database_reader(BufReader::new(file));
        for (key, entry) in parsed {
            self.vdj_entries.insert(
                key,
                CachedDjSoftwareMetadata {
                    bpm: entry.bpm,
                    key: entry.key,
                    cues_count: entry.cues_count,
                    author: entry.author,
                    title: entry.title,
                    duration_sec: entry.duration_sec,
                    bitrate_kbps: entry.bitrate_kbps,
                    cues: entry.cues,
                    original_path: entry.original_path,
                },
            );
        }
    }

    fn load_rekordbox_xmls(&mut self) {
        for env_key in ["HERA_REKORDBOX_XML", "USERPROFILE", "HOME"] {
            let Ok(val) = std::env::var(env_key) else {
                continue;
            };
            let rb_path = if env_key == "HERA_REKORDBOX_XML" {
                PathBuf::from(val)
            } else {
                PathBuf::from(val)
                    .join("PIONEER")
                    .join("rekordbox")
                    .join("rekordbox.xml")
            };
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
                let bpm =
                    extract_xml_attr(trimmed, "AverageBpm=\"").and_then(|b| b.parse::<f64>().ok());
                let tonality = extract_xml_attr(trimmed, "Tonality=\"");
                let duration_sec =
                    extract_xml_attr(trimmed, "TotalTime=\"").and_then(|d| d.parse::<f64>().ok());

                if let Some(loc) = location {
                    let orig = loc.replace("file://localhost/", "").replace('\\', "/");
                    let clean_path = orig.to_lowercase();
                    self.rekordbox_entries.insert(
                        clean_path,
                        CachedDjSoftwareMetadata {
                            bpm,
                            key: tonality,
                            cues_count: 0,
                            author: None,
                            title: None,
                            duration_sec,
                            bitrate_kbps: None,
                            cues: Vec::new(),
                            original_path: Some(orig),
                        },
                    );
                }
            }
        }
    }

    fn load_traktor_nmls(&mut self) {
        super::workability_discovery::load_traktor_collections_into(&mut self.traktor_entries);
    }

    /// Retrieve pre-computed metadata for a track path (VirtualDJ, Rekordbox, or Traktor).
    pub fn lookup(&self, path: &Path) -> Option<&CachedDjSoftwareMetadata> {
        let norm = super::vdj_xml_parser::normalize_lookup_path(&path.to_string_lossy());
        self.vdj_entries
            .get(&norm)
            .or_else(|| self.rekordbox_entries.get(&norm))
            .or_else(|| self.traktor_entries.get(&norm))
    }
}

pub(super) fn extract_xml_attr(line: &str, attr_prefix: &str) -> Option<String> {
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
        homes.push(
            PathBuf::from(&user_profile)
                .join("Documents")
                .join("VirtualDJ"),
        );
    }
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        homes.push(PathBuf::from(local_app_data).join("VirtualDJ"));
    }
    if let Ok(home) = std::env::var("HOME") {
        homes.push(PathBuf::from(home).join("Documents").join("VirtualDJ"));
    }
    homes
}

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
