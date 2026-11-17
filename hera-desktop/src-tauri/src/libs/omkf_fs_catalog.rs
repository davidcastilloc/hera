//! In-memory hierarchical OMKF Filesystem Catalog (`Config.library_folders` + `.hera/tracks.omkf`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use hera_ipc::dj_types::{FolderCrateDetail, FolderCrateSummary, OmkfFsTrackDto};

use super::omkf_fs_capsule_upsert::{deduplicate_roots, norm_cmp_key};
use super::omkf_fs_hydrate::{CatalogTrackRecord, hydrate_directory};
use super::omkf_fs_search::{
    assign_global_copy_counts, query_harmonic_candidates, search_catalog_deduplicated,
};
use super::omkf_sync::compute_folder_crate_id;
use super::track::Track;
use crate::plugins::hera_dj::workability::is_junk_system_directory;

#[derive(Debug, Clone)]
pub(crate) struct CatalogFolderNode {
    pub dir: PathBuf,
    pub summary: FolderCrateSummary,
    pub child_ids: Vec<String>,
    pub track_ids: Vec<String>,
}

#[derive(Default)]
pub(crate) struct CatalogInner {
    pub roots_sig: Vec<String>,
    pub stale: bool,
    pub folders_order: Vec<String>,
    pub folders_by_id: HashMap<String, CatalogFolderNode>,
    pub crate_id_by_norm_path: HashMap<String, String>,
    pub tracks_by_id: HashMap<String, CatalogTrackRecord>,
}

pub struct OmkfFsCatalog {
    pub(crate) inner: RwLock<CatalogInner>,
}

impl Default for OmkfFsCatalog {
    fn default() -> Self {
        Self::new()
    }
}

pub fn global_omkf_catalog() -> &'static OmkfFsCatalog {
    static CATALOG: OnceLock<OmkfFsCatalog> = OnceLock::new();
    CATALOG.get_or_init(OmkfFsCatalog::new)
}

fn should_skip_dir_name(name: &str, full_path: &Path) -> bool {
    name.starts_with('.')
        || name.eq_ignore_ascii_case("__macosx")
        || name.eq_ignore_ascii_case("$recycle.bin")
        || name.eq_ignore_ascii_case("system volume information")
        || name.eq_ignore_ascii_case("node_modules")
        || is_junk_system_directory(full_path)
}

fn walk_folder_tree(
    dir: &Path,
    parent_id: Option<String>,
    rel_path: String,
    depth: usize,
    inner: &mut CatalogInner,
) -> Option<String> {
    let cid = compute_folder_crate_id(dir);
    let slot_idx = inner.folders_order.len();
    inner.folders_order.push(cid.clone());

    let mut subdirs: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if !p.is_dir() {
                return None;
            }
            let nm = e.file_name().to_string_lossy().into_owned();
            (!should_skip_dir_name(&nm, &p)).then_some((nm, p))
        })
        .collect();
    subdirs.sort_by_key(|a| a.0.to_ascii_lowercase());

    let mut child_ids = Vec::new();
    for (nm, sub_p) in subdirs {
        let child_rel = format!("{rel_path}/{nm}");
        if let Some(sub_cid) =
            walk_folder_tree(&sub_p, Some(cid.clone()), child_rel, depth + 1, inner)
        {
            child_ids.push(sub_cid);
        }
    }

    let hyd = hydrate_directory(dir, &cid, parent_id, rel_path, depth, child_ids.len());
    if depth > 0 && hyd.records.is_empty() && child_ids.is_empty() {
        inner.folders_order.truncate(slot_idx);
        return None;
    }

    let mut track_ids = Vec::with_capacity(hyd.records.len());
    for r in hyd.records {
        track_ids.push(r.dto.id.clone());
        inner.tracks_by_id.insert(r.dto.id.clone(), r);
    }
    inner
        .crate_id_by_norm_path
        .insert(norm_cmp_key(dir), cid.clone());
    inner.folders_by_id.insert(
        cid.clone(),
        CatalogFolderNode {
            dir: dir.to_path_buf(),
            summary: hyd.summary,
            child_ids,
            track_ids,
        },
    );
    Some(cid)
}

impl OmkfFsCatalog {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(CatalogInner::default()),
        }
    }

    pub fn mark_stale(&self) {
        if let Ok(mut g) = self.inner.write() {
            g.stale = true;
        }
    }

    pub fn ensure_roots(&self, roots: &[PathBuf]) {
        let deduped = deduplicate_roots(roots);
        let sig: Vec<String> = deduped.iter().map(|p| norm_cmp_key(p)).collect();
        let needs_refresh = self
            .inner
            .read()
            .map(|g| g.stale || g.roots_sig != sig)
            .unwrap_or(true);
        if !needs_refresh {
            return;
        }
        let mut next = CatalogInner {
            roots_sig: sig,
            stale: false,
            ..Default::default()
        };
        for root in deduped {
            let root_label = root
                .file_name()
                .and_then(|s| s.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("Raíz")
                .to_string();
            let _ = walk_folder_tree(&root, None, root_label, 0, &mut next);
        }
        assign_global_copy_counts(&mut next.tracks_by_id);
        if let Ok(mut g) = self.inner.write() {
            *g = next;
        }
    }

    pub fn invalidate_folder_path(&self, dir: &Path) {
        let key = norm_cmp_key(dir);
        let Ok(mut g) = self.inner.write() else {
            return;
        };
        let Some(cid) = g.crate_id_by_norm_path.get(&key).cloned() else {
            g.stale = true;
            return;
        };
        let Some(old) = g.folders_by_id.get(&cid).cloned() else {
            g.stale = true;
            return;
        };
        if !dir.is_dir() {
            g.stale = true;
            return;
        };
        for tid in &old.track_ids {
            g.tracks_by_id.remove(tid);
        }
        let hyd = hydrate_directory(
            &old.dir,
            &cid,
            old.summary.parent_crate_id.clone(),
            old.summary.rel_path.clone(),
            old.summary.depth,
            old.child_ids.len(),
        );
        let mut new_tids = Vec::with_capacity(hyd.records.len());
        for r in hyd.records {
            new_tids.push(r.dto.id.clone());
            g.tracks_by_id.insert(r.dto.id.clone(), r);
        }
        if let Some(node) = g.folders_by_id.get_mut(&cid) {
            node.summary = hyd.summary;
            node.track_ids = new_tids;
        }
        assign_global_copy_counts(&mut g.tracks_by_id);
    }

    pub fn list_folders(&self, roots: &[PathBuf]) -> Vec<FolderCrateSummary> {
        self.ensure_roots(roots);
        let Ok(g) = self.inner.read() else {
            return Vec::new();
        };
        g.folders_order
            .iter()
            .filter_map(|cid| g.folders_by_id.get(cid).map(|n| n.summary.clone()))
            .collect()
    }

    pub fn read_folder(
        &self,
        query_or_crate_id: &str,
        roots: &[PathBuf],
    ) -> (Option<FolderCrateDetail>, Vec<(String, String)>) {
        self.ensure_roots(roots);
        if let Some(q) = query_or_crate_id.strip_prefix("__search__:") {
            let Ok(g) = self.inner.read() else {
                return (None, Vec::new());
            };
            return (Some(search_catalog_deduplicated(&g, q, 200)), Vec::new());
        }
        if let Some(spec) = query_or_crate_id.strip_prefix("__harmonic__:") {
            let Ok(g) = self.inner.read() else {
                return (None, Vec::new());
            };
            return (query_harmonic_candidates(&g, spec, 60), Vec::new());
        }
        let node_opt = self
            .inner
            .read()
            .ok()
            .and_then(|g| g.folders_by_id.get(query_or_crate_id).cloned());
        let Some(node) = node_opt else {
            return (None, Vec::new());
        };
        let online = node.dir.is_dir();
        let hyd = hydrate_directory(
            &node.dir,
            query_or_crate_id,
            node.summary.parent_crate_id.clone(),
            node.summary.rel_path.clone(),
            node.summary.depth,
            node.child_ids.len(),
        );
        let mut tracks: Vec<OmkfFsTrackDto> = Vec::with_capacity(hyd.records.len());
        let mut track_ids: Vec<String> = Vec::with_capacity(hyd.records.len());
        let mut subfolders = Vec::with_capacity(node.child_ids.len());
        if let Ok(mut g) = self.inner.write() {
            for old_tid in &node.track_ids {
                g.tracks_by_id.remove(old_tid);
            }
            for r in hyd.records {
                track_ids.push(r.dto.id.clone());
                g.tracks_by_id.insert(r.dto.id.clone(), r);
            }
            assign_global_copy_counts(&mut g.tracks_by_id);
            for tid in &track_ids {
                if let Some(rec) = g.tracks_by_id.get(tid) {
                    tracks.push(rec.dto.clone());
                }
            }
            for cid in &node.child_ids {
                if let Some(cn) = g.folders_by_id.get(cid) {
                    subfolders.push(cn.summary.clone());
                }
            }
            if let Some(n) = g.folders_by_id.get_mut(query_or_crate_id) {
                n.summary = hyd.summary.clone();
                n.track_ids = track_ids.clone();
            }
        }
        let detail = FolderCrateDetail {
            summary: hyd.summary,
            rules_markdown: hyd.rules_md,
            ledger_entries: hyd.ledger,
            track_ids,
            hydrated_from_omkf: hyd.from_omkf,
            disk_online: online,
            subfolders,
            tracks,
        };
        (Some(detail), hyd.unanalyzed)
    }

    pub fn get_tracks_by_ids(&self, ids: &[String]) -> Vec<Track> {
        let Ok(g) = self.inner.read() else {
            return Vec::new();
        };
        ids.iter()
            .filter_map(|id| g.tracks_by_id.get(id).map(CatalogTrackRecord::to_track))
            .collect()
    }

    pub(crate) fn folder_path_for_crate_id(
        &self,
        crate_id: &str,
        roots: &[PathBuf],
    ) -> Option<PathBuf> {
        self.ensure_roots(roots);
        self.inner
            .read()
            .ok()?
            .folders_by_id
            .get(crate_id)
            .map(|n| n.dir.clone())
    }
}
