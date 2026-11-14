use crate::bpm_engine::v2_core::audio_ingest::fast_partial_file_hash;
use crate::scanner::schema::{ScanLevel, TrackScanState};
use std::collections::HashMap;
use std::path::Path;

pub const WATCHER_DEBOUNCE_MS: u64 = 1_500;

#[derive(Default)]
pub struct LibraryIndex {
    by_path: HashMap<String, TrackScanState>,
    by_hash_size: HashMap<(String, u64), String>,
}

impl LibraryIndex {
    pub fn upsert(&mut self, state: TrackScanState) {
        self.by_hash_size.insert(
            (state.partial_hash_64kb.clone(), state.file_size_bytes),
            state.track_id.clone(),
        );
        self.by_path.insert(state.file_path.clone(), state);
    }

    /// Nivel 0 INDEXED con auto-curación de rutas movidas/renombradas por `(partial_hash_64kb, file_size_bytes)`.
    pub fn index_or_heal_file(&mut self, path: &Path, now_unix_ms: i64) -> Result<TrackScanState, String> {
        let path_str = path.to_string_lossy().to_string();
        let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
        let size = meta.len();
        let hash = fast_partial_file_hash(path).map_err(|e| format!("{e:?}"))?;

        if let Some(existing) = self.by_path.get(&path_str) {
            if existing.partial_hash_64kb == hash && existing.file_size_bytes == size {
                return Ok(existing.clone());
            }
        }

        if let Some(old_id) = self.by_hash_size.get(&(hash.clone(), size)).cloned() {
            if let Some(old_key) = self
                .by_path
                .iter()
                .find(|(_, v)| v.track_id == old_id)
                .map(|(k, _)| k.clone())
            {
                if let Some(mut healed) = self.by_path.remove(&old_key) {
                    healed.file_path = path_str.clone();
                    healed.updated_at_unix_ms = now_unix_ms;
                    self.by_path.insert(path_str, healed.clone());
                    return Ok(healed);
                }
            }
        }

        let state = TrackScanState {
            track_id: format!("trk_{}_{}", &hash[..hash.len().min(12)], size),
            file_path: path_str,
            partial_hash_64kb: hash,
            file_size_bytes: size,
            mtime_unix_ms: now_unix_ms,
            scan_level: ScanLevel::Indexed,
            bpm: None,
            camelot_key: None,
            is_transition_candidate: false,
            is_fake_320kbps: false,
            effective_bandwidth_hz: None,
            updated_at_unix_ms: now_unix_ms,
        };
        self.upsert(state.clone());
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renaming_mp3_heals_track_id_without_recomputing_dsp() {
        let tmp_dir = std::env::temp_dir().join("hera_scanner_indexer_test");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let orig = tmp_dir.join("orig_track.mp3");
        let renamed = tmp_dir.join("renamed_track.mp3");
        let _ = std::fs::remove_file(&orig);
        let _ = std::fs::remove_file(&renamed);

        std::fs::write(&orig, vec![0xAA_u8; 4096]).unwrap();

        let mut idx = LibraryIndex::default();
        let mut s1 = idx.index_or_heal_file(&orig, 1000).unwrap();
        s1.scan_level = ScanLevel::Full;
        s1.bpm = Some(126.0);
        idx.upsert(s1.clone());

        std::fs::rename(&orig, &renamed).unwrap();
        let s2 = idx.index_or_heal_file(&renamed, 2000).unwrap();
        assert_eq!(s2.track_id, s1.track_id);
        assert_eq!(s2.scan_level, ScanLevel::Full);
        assert_eq!(s2.bpm, Some(126.0));

        let _ = std::fs::remove_file(&renamed);
        let _ = std::fs::remove_dir(&tmp_dir);
    }
}
