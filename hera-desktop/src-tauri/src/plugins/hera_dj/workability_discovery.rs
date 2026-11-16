//! DB-first `< 1 s` workable media root discovery across VirtualDJ, Rekordbox, and Traktor.
#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use hera_ipc::dj_types::DiscoveredMusicRoot;

use super::vdj_xml_parser::{normalize_lookup_path, unescape_xml};
use super::workability::{
    CachedDjSoftwareMetadata, ExternalDjSoftwareIndex, extract_xml_attr, is_junk_system_directory,
};

#[derive(Default)]
struct RootBucketAcc {
    source_db: String,
    parent_dirs: Vec<Vec<String>>,
    prefix: String,
    workable_tracks_count: usize,
    mixes_over_10m_count: usize,
    skipped_short_sfx_count: usize,
    video_tracks_count: usize,
}

fn is_video_extension(path_str: &str) -> bool {
    Path::new(path_str)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp4" | "m4v" | "mov" | "webm" | "mkv"
            )
        })
}

/// Splits a slash-normalized parent folder path into `(root_prefix, segments)` and determines
/// how many leading segments form the minimum top-level media anchor.
fn split_parent_anchor(file_path: &str) -> Option<(String, Vec<String>, usize)> {
    let slashed = file_path.replace('\\', "/");
    let trimmed = slashed.strip_prefix("//?/").unwrap_or(&slashed);
    let last_slash = trimmed.rfind('/')?;
    let parent = &trimmed[..last_slash];
    if parent.is_empty() {
        return None;
    }

    let bytes = parent.as_bytes();
    let (prefix, rest) = if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        (parent[..2].to_string(), &parent[2..])
    } else if let Some(stripped) = parent.strip_prefix('/') {
        ("/".to_string(), stripped)
    } else {
        (String::new(), parent)
    };

    let segs: Vec<String> = rest
        .split('/')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if segs.is_empty() {
        return None;
    }

    let first_low = segs[0].to_ascii_lowercase();
    let anchor_depth = if (first_low == "users" || first_low == "home") && segs.len() >= 3 {
        let third_low = segs[2].to_ascii_lowercase();
        if (third_low == "documents" || third_low == "onedrive") && segs.len() >= 4 {
            4
        } else {
            3
        }
    } else if (first_low == "volumes" || first_low == "mnt") && segs.len() >= 2 {
        segs.len().min(3)
    } else if first_low == "media" && segs.len() >= 2 {
        segs.len().min(4)
    } else {
        1
    };

    let clamped_depth = anchor_depth.min(segs.len());
    Some((prefix, segs, clamped_depth))
}

fn format_joined_path(prefix: &str, segs: &[String]) -> String {
    if prefix == "/" {
        format!("/{}", segs.join("/"))
    } else if prefix.is_empty() {
        segs.join("/")
    } else {
        format!("{prefix}/{}", segs.join("/"))
    }
}

fn longest_common_dir_prefix(dirs: &[Vec<String>], min_len: usize) -> Vec<String> {
    let Some(first) = dirs.first() else {
        return Vec::new();
    };
    let mut common_len = first.len();
    for dir in &dirs[1..] {
        let matched = first
            .iter()
            .zip(dir.iter())
            .take(common_len)
            .take_while(|(a, b)| a.eq_ignore_ascii_case(b))
            .count();
        common_len = common_len.min(matched);
    }
    first[..common_len.max(min_len.min(first.len()))].to_vec()
}

/// Discovers workable media root folders in `< 1 s` directly from installed VirtualDJ,
/// Rekordbox, and Traktor XML/NML databases without crawling arbitrary OS directories.
pub fn discover_workable_roots_from_existing_dbs() -> Vec<DiscoveredMusicRoot> {
    let index = ExternalDjSoftwareIndex::build_fast_index();
    discover_workable_roots_from_index(&index, true)
}

/// Pure, deterministic workable root aggregator over an `ExternalDjSoftwareIndex`.
pub fn discover_workable_roots_from_index(
    index: &ExternalDjSoftwareIndex,
    check_exists: bool,
) -> Vec<DiscoveredMusicRoot> {
    let mut buckets: BTreeMap<(String, String), RootBucketAcc> = BTreeMap::new();
    let mut exists_cache: HashMap<String, bool> = HashMap::new();

    let sources: [(&str, &HashMap<String, CachedDjSoftwareMetadata>); 3] = [
        ("VirtualDJ", &index.vdj_entries),
        ("Rekordbox", &index.rekordbox_entries),
        ("Traktor", &index.traktor_entries),
    ];

    for (source_name, entries) in sources {
        for (norm_key, meta) in entries {
            let raw_path = meta.original_path.as_deref().unwrap_or(norm_key.as_str());
            if is_junk_system_directory(Path::new(raw_path)) {
                continue;
            }
            if check_exists {
                let parent_key = Path::new(raw_path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                let parent_exists = *exists_cache
                    .entry(parent_key.clone())
                    .or_insert_with(|| Path::new(&parent_key).is_dir());
                if !parent_exists || !Path::new(raw_path).is_file() {
                    continue;
                }
            }

            let Some((prefix, segs, anchor_depth)) = split_parent_anchor(raw_path) else {
                continue;
            };
            let anchor_key = format_joined_path(&prefix, &segs[..anchor_depth]).to_lowercase();
            let bucket = buckets
                .entry((source_name.to_string(), anchor_key))
                .or_insert_with(|| RootBucketAcc {
                    source_db: source_name.to_string(),
                    parent_dirs: Vec::new(),
                    prefix: prefix.clone(),
                    ..Default::default()
                });

            bucket.parent_dirs.push(segs);

            let is_vid = is_video_extension(raw_path);
            match meta.duration_sec {
                Some(d) if d > 0.0 && d < 25.0 => {
                    bucket.skipped_short_sfx_count += 1;
                }
                Some(d) if d > 600.0 => {
                    bucket.mixes_over_10m_count += 1;
                }
                _ => {
                    bucket.workable_tracks_count += 1;
                    if is_vid {
                        bucket.video_tracks_count += 1;
                    }
                }
            }
        }
    }

    let mut out: Vec<DiscoveredMusicRoot> = buckets
        .into_values()
        .filter(|b| b.workable_tracks_count > 0 || b.mixes_over_10m_count > 0)
        .map(|b| {
            let min_depth = b
                .parent_dirs
                .first()
                .map(|s| {
                    let full = format!("{}/dummy.mp3", format_joined_path(&b.prefix, s));
                    split_parent_anchor(&full).map(|t| t.2).unwrap_or(1)
                })
                .unwrap_or(1);
            let common = longest_common_dir_prefix(&b.parent_dirs, min_depth);
            DiscoveredMusicRoot {
                root_path: format_joined_path(&b.prefix, &common),
                source_db: b.source_db,
                workable_tracks_count: b.workable_tracks_count,
                mixes_over_10m_count: b.mixes_over_10m_count,
                skipped_short_sfx_count: b.skipped_short_sfx_count,
                video_tracks_count: b.video_tracks_count,
            }
        })
        .collect();

    out.sort_by(|a, b| {
        b.workable_tracks_count
            .cmp(&a.workable_tracks_count)
            .then_with(|| a.root_path.cmp(&b.root_path))
    });
    out
}

pub(super) fn load_traktor_collections_into(
    target: &mut HashMap<String, CachedDjSoftwareMetadata>,
) {
    let mut candidates = Vec::new();
    if let Ok(custom) = std::env::var("HERA_TRAKTOR_NML")
        && !custom.trim().is_empty()
    {
        candidates.push(PathBuf::from(custom));
    }
    for env_key in ["USERPROFILE", "HOME"] {
        if let Ok(base) = std::env::var(env_key) {
            let ni_dir = PathBuf::from(base)
                .join("Documents")
                .join("Native Instruments");
            if let Ok(entries) = std::fs::read_dir(ni_dir) {
                for entry in entries.flatten() {
                    let p = entry.path().join("collection.nml");
                    if p.is_file() {
                        candidates.push(p);
                    }
                }
            }
        }
    }

    for nml_path in candidates {
        let Ok(file) = File::open(&nml_path) else {
            continue;
        };
        let reader = BufReader::new(file);
        let mut pending_path: Option<String> = None;
        let mut pending_dur: Option<f64> = None;
        for line in reader.lines().map_while(Result::ok) {
            let t = line.trim();
            if t.starts_with("<LOCATION ") {
                let vol = extract_xml_attr(t, "VOLUME=\"").unwrap_or_default();
                let dir = extract_xml_attr(t, "DIR=\"").unwrap_or_default();
                let fname = extract_xml_attr(t, "FILE=\"").unwrap_or_default();
                if !fname.is_empty() {
                    let clean_dir = unescape_xml(&dir).replace("/:", "/");
                    let clean_file = unescape_xml(&fname);
                    let prefix = if vol.ends_with(':') {
                        vol
                    } else {
                        String::new()
                    };
                    pending_path = Some(format!("{prefix}{clean_dir}{clean_file}"));
                }
            } else if t.starts_with("<INFO ") {
                pending_dur =
                    extract_xml_attr(t, "PLAYTIME=\"").and_then(|s| s.parse::<f64>().ok());
            } else if t.starts_with("</ENTRY>")
                && let Some(orig) = pending_path.take()
            {
                let key = normalize_lookup_path(&orig);
                target.insert(
                    key,
                    CachedDjSoftwareMetadata {
                        duration_sec: pending_dur.take(),
                        original_path: Some(orig),
                        ..Default::default()
                    },
                );
            }
        }
    }
}
