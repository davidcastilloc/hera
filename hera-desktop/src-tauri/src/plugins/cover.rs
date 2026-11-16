use std::path::Path;
use std::sync::{Mutex, OnceLock};

use base64::prelude::{BASE64_STANDARD, Engine as _};
use indexmap::IndexMap;
use lofty::file::TaggedFileExt;
use lofty::picture::{MimeType, PictureType};
use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

use crate::libs::error::AnyResult;
use crate::libs::utils::scan_dir;

pub(crate) const MAX_COVER_CACHE_ENTRIES: usize = 256;
const SUPPORTED_COVER_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "bmp", "gif"];
const SUPPORTED_COVER_NAMES: [&str; 5] = ["album", "albumart", "folder", "cover", "front"];

static COVER_LRU_CACHE: OnceLock<Mutex<IndexMap<String, Option<String>>>> = OnceLock::new();

fn cover_cache() -> &'static Mutex<IndexMap<String, Option<String>>> {
    COVER_LRU_CACHE.get_or_init(|| Mutex::new(IndexMap::with_capacity(MAX_COVER_CACHE_ENTRIES)))
}

fn should_skip_id3_probe(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("webm") || ext.eq_ignore_ascii_case("mkv"))
}

fn get_cover_from_id3(path: &Path) -> Option<String> {
    if should_skip_id3_probe(path) {
        return None;
    }

    let tagged_file = lofty::read_from_path(path).ok()?;
    let primary_tag = tagged_file.primary_tag()?;

    let cover = primary_tag
        .get_picture_type(PictureType::CoverFront)
        .or_else(|| primary_tag.get_picture_type(PictureType::Illustration))
        .or_else(|| primary_tag.get_picture_type(PictureType::Other))
        .or_else(|| primary_tag.get_picture_type(PictureType::Icon))
        .or_else(|| primary_tag.get_picture_type(PictureType::OtherIcon))?;

    let format = match cover.mime_type() {
        Some(MimeType::Png) => "png",
        Some(MimeType::Jpeg) => "jpg",
        Some(MimeType::Tiff) => "tiff",
        Some(MimeType::Bmp) => "bmp",
        Some(MimeType::Gif) => "gif",
        _ => return None,
    };

    let cover_base64 = BASE64_STANDARD.encode(cover.data());
    Some(format!("data:{format};base64,{cover_base64}"))
}

fn get_cover_from_filesystem(path: &Path) -> Option<String> {
    let dir_path = path.parent()?.to_path_buf();
    if !dir_path.is_dir() {
        return None;
    }

    scan_dir(&dir_path, &SUPPORTED_COVER_EXTENSIONS)
        .into_iter()
        .find(|file| {
            file.file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|stem| {
                    let lower = stem.to_ascii_lowercase();
                    SUPPORTED_COVER_NAMES.contains(&lower.as_str())
                })
        })
        .and_then(|found| found.to_str().map(str::to_owned))
}

pub(crate) fn lookup_or_compute_cover(path: &str) -> Option<String> {
    {
        if let Ok(mut map) = cover_cache().lock()
            && let Some(cached) = map.shift_remove(path)
        {
            map.insert(path.to_owned(), cached.clone());
            return cached;
        }
    }

    let path_ref = Path::new(path);
    let computed = get_cover_from_id3(path_ref).or_else(|| get_cover_from_filesystem(path_ref));

    if let Ok(mut map) = cover_cache().lock() {
        let _ = map.shift_remove(path);
        if map.len() >= MAX_COVER_CACHE_ENTRIES {
            let _ = map.shift_remove_index(0);
        }
        map.insert(path.to_owned(), computed.clone());
    }

    computed
}

#[cfg(test)]
pub(crate) fn cover_cache_len_for_test() -> usize {
    cover_cache().lock().map(|m| m.len()).unwrap_or(0)
}

#[cfg(test)]
pub(crate) fn cover_cache_contains_for_test(key: &str) -> bool {
    cover_cache()
        .lock()
        .map(|m| m.contains_key(key))
        .unwrap_or(false)
}

#[cfg(test)]
pub(crate) fn clear_cover_cache_for_test() {
    if let Ok(mut m) = cover_cache().lock() {
        m.clear();
    }
}

#[tauri::command]
pub async fn get_cover(path: String) -> AnyResult<Option<String>> {
    let resolved = tokio::task::spawn_blocking(move || lookup_or_compute_cover(&path))
        .await
        .map_err(|e| anyhow::anyhow!("get_cover spawn_blocking join error: {e}"))?;
    Ok(resolved)
}

/**
 * Module in charge of assisting the UI with cover retrieval
 */
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("cover")
        .invoke_handler(tauri::generate_handler![get_cover])
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cover_lru_evicts_past_256_skips_webm_and_handles_root_path_without_panic() {
        clear_cover_cache_for_test();
        assert_eq!(cover_cache_len_for_test(), 0);

        // Root paths on POSIX and Windows must return None cleanly without panicking
        assert_eq!(lookup_or_compute_cover("/"), None);
        assert_eq!(lookup_or_compute_cover("C:\\"), None);
        assert_eq!(cover_cache_len_for_test(), 2);

        // Video containers (.webm / .mkv) skip lofty ID3 probing cleanly
        assert_eq!(
            lookup_or_compute_cover("/nonexistent_dir_xyz/clip.WEBM"),
            None
        );
        assert_eq!(
            lookup_or_compute_cover("/nonexistent_dir_xyz/clip.mkv"),
            None
        );

        clear_cover_cache_for_test();
        for idx in 0..300 {
            let key = format!("/virtual_nonexistent_library/track_{idx:04}.mp3");
            assert_eq!(lookup_or_compute_cover(&key), None);
        }
        assert_eq!(cover_cache_len_for_test(), MAX_COVER_CACHE_ENTRIES);
        assert!(!cover_cache_contains_for_test(
            "/virtual_nonexistent_library/track_0000.mp3"
        ));
        assert!(cover_cache_contains_for_test(
            "/virtual_nonexistent_library/track_0044.mp3"
        ));

        // Accessing track_0044 promotes it to MRU, so inserting one more evicts track_0045
        let _ = lookup_or_compute_cover("/virtual_nonexistent_library/track_0044.mp3");
        let _ = lookup_or_compute_cover("/virtual_nonexistent_library/track_0300.mp3");
        assert_eq!(cover_cache_len_for_test(), MAX_COVER_CACHE_ENTRIES);
        assert!(cover_cache_contains_for_test(
            "/virtual_nonexistent_library/track_0044.mp3"
        ));
        assert!(!cover_cache_contains_for_test(
            "/virtual_nonexistent_library/track_0045.mp3"
        ));
    }
}
