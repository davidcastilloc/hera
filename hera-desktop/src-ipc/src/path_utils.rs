use std::path::{Component, Path, PathBuf};

/// Formats a track path for an M3U playlist entry.
///
/// When `track_path.parent()` and `playlist_parent` share a common prefix root,
/// it computes the relative path from the playlist directory to the track file.
/// If they do not share a common root (e.g. cross-drive paths on Windows like
/// `D:/ClubSets/afro.flac` vs `C:/Playlists`, or differing root components),
/// it falls back cleanly to `track_path.to_string_lossy().into_owned()` without panicking.
pub fn format_m3u_entry_path(track_path: &Path, playlist_parent: &Path) -> String {
    if let Some(relative) = pure_diff_paths(track_path, playlist_parent) {
        relative.to_string_lossy().into_owned()
    } else {
        track_path.to_string_lossy().into_owned()
    }
}

/// Pure component-based relative path diff between `path` and `base`.
/// Returns `None` if they don't share identical root/prefix components.
fn pure_diff_paths(path: &Path, base: &Path) -> Option<PathBuf> {
    let mut path_comps = path.components();
    let mut base_comps = base.components();

    let mut path_common = Vec::new();
    let mut base_common = Vec::new();

    loop {
        match (path_comps.next(), base_comps.next()) {
            (Some(p), Some(b)) if p == b => {
                path_common.push(p);
                base_common.push(b);
            }
            (p, b) => {
                // If the first component (root / prefix) differed, diff is impossible
                if path_common.is_empty() {
                    return None;
                }
                let mut result = PathBuf::new();
                if let Some(b_comp) = b {
                    if matches!(b_comp, Component::Prefix(_) | Component::RootDir) {
                        return None;
                    }
                    result.push(Component::ParentDir.as_os_str());
                }
                for b_comp in base_comps {
                    if matches!(b_comp, Component::Prefix(_) | Component::RootDir) {
                        return None;
                    }
                    result.push(Component::ParentDir.as_os_str());
                }
                if let Some(p_comp) = p {
                    result.push(p_comp.as_os_str());
                }
                for p_comp in path_comps {
                    result.push(p_comp.as_os_str());
                }
                return Some(result);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_m3u_entry_falls_back_cleanly_when_diff_paths_is_none() {
        let p = format_m3u_entry_path(
            Path::new("D:/ClubSets/afro.flac"),
            Path::new("/var/playlists"),
        );
        assert!(p.contains("afro.flac"));
        assert_eq!(p, "D:/ClubSets/afro.flac");
    }

    #[test]
    fn format_m3u_entry_computes_relative_path_for_same_tree() {
        let p = format_m3u_entry_path(
            Path::new("/home/music/afro.flac"),
            Path::new("/home/music/playlists"),
        );
        assert_eq!(p, "../afro.flac");
    }

    #[test]
    fn format_m3u_entry_same_directory() {
        let p = format_m3u_entry_path(Path::new("/home/music/afro.flac"), Path::new("/home/music"));
        assert_eq!(p, "afro.flac");
    }
}
