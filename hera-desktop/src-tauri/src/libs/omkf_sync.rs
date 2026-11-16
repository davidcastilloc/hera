//! OMKF v1 folder capsule sync, Windows `.hera` hiding, boot seal check & Zero-Import USB.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use hera_node_rs::genre_intelligence::BoothGenreFamily;
use hera_node_rs::scanner::omkf_capsule as omkf;
use sqlx::{Connection, SqliteConnection};
use uuid::Uuid;

use super::database::SUPPORTED_TRACKS_EXTENSIONS;
use super::fast_library_scan::{
    pack_omkf_header_and_entries, sql_load_folder_omkf_entries, sql_upsert_folder_crate,
    sql_upsert_hydrated_omkf_entry_tx, summarize_omkf_crate_md,
};
use super::track::{Track, get_track_id_for_path};
use super::utils::is_file_valid;

#[cfg(windows)]
unsafe extern "system" {
    fn SetFileAttributesW(lpFileName: *const u16, dwFileAttributes: u32) -> i32;
}

fn dirty_folders_slot() -> &'static Mutex<HashSet<PathBuf>> {
    static SLOTS: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    SLOTS.get_or_init(|| Mutex::new(HashSet::new()))
}

pub fn mark_folder_dirty(dir: &Path) {
    if !dir.as_os_str().is_empty()
        && let Ok(mut g) = dirty_folders_slot().lock()
    {
        g.insert(dir.to_path_buf());
    }
}

pub fn mark_crate_dirty_for_track_path(track_path: &str) {
    if let Some(p) = Path::new(track_path).parent() {
        mark_folder_dirty(p);
    }
}

pub fn hide_hera_dir_if_windows(hera_dir: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = hera_dir.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: `wide` is NUL-terminated UTF-16 valid for `SetFileAttributesW` (`FILE_ATTRIBUTE_HIDDEN = 2`).
        unsafe {
            let _ = SetFileAttributesW(wide.as_ptr(), 0x2);
        }
    }
    #[cfg(not(windows))]
    let _ = hera_dir;
}

fn sys_ns(t: std::io::Result<SystemTime>) -> i64 {
    t.ok()
        .and_then(|s| s.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

pub fn compute_folder_crate_id(folder: &Path) -> String {
    if let Ok(md) = std::fs::read_to_string(folder.join(".hera/crate.md"))
        && let Some(line) = md.lines().find(|l| l.trim_start().starts_with("crate_id:"))
    {
        let raw = line
            .split_once(':')
            .map_or("", |x| x.1)
            .trim()
            .trim_matches('"');
        if !raw.is_empty() {
            return raw.into();
        }
    }
    let canon = std::fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf());
    let key = format!(
        "hera-crate:{}",
        canon.to_string_lossy().to_ascii_lowercase()
    );
    Uuid::new_v3(&Uuid::NAMESPACE_OID, key.as_bytes()).to_string()
}

pub struct DiskEntrySeal {
    pub path: PathBuf,
    pub filename: String,
    pub size: u64,
    pub mtime_ns: i64,
    pub seal: u64,
}

pub fn scan_dir_entry_seals(folder: &Path) -> (i64, Vec<DiskEntrySeal>, u64) {
    let dir_ns = sys_ns(std::fs::metadata(folder).and_then(|m| m.modified()));
    let mut entries = Vec::new();
    for e in std::fs::read_dir(folder).into_iter().flatten().flatten() {
        let (p, Ok(m)) = (e.path(), e.metadata()) else {
            continue;
        };
        if !p.is_file() || !is_file_valid(&p, &SUPPORTED_TRACKS_EXTENSIONS) {
            continue;
        }
        let fname = e.file_name().to_string_lossy().into_owned();
        let (sz, mt) = (m.len(), sys_ns(m.modified()));
        entries.push(DiskEntrySeal {
            seal: omkf::compute_entry_seal_u64(&fname, sz, mt),
            path: p,
            filename: fname,
            size: sz,
            mtime_ns: mt,
        });
    }
    entries.sort_by(|a, b| a.filename.cmp(&b.filename));
    let seals: Vec<u64> = entries.iter().map(|x| x.seal).collect();
    (dir_ns, entries, omkf::compute_tree_seal_u64(&seals))
}

pub fn folder_matches_omkf_capsule(folder: &Path) -> bool {
    let Ok(hdr) = omkf::read_omkf_header(&folder.join(".hera/tracks.omkf")) else {
        return false;
    };
    let (_, entries, actual) = scan_dir_entry_seals(folder);
    !entries.is_empty() && (hdr.tree_seal_64 == 0 || hdr.tree_seal_64 == actual)
}

pub async fn hydrate_folder_from_omkf_zero_dsp(
    conn: &mut SqliteConnection,
    folder: &Path,
) -> Result<usize, String> {
    let cap = omkf::read_omkf_capsule(&folder.join(".hera/tracks.omkf"))?;
    let (dir_ns, entries, actual_seal) = scan_dir_entry_seals(folder);
    if entries.is_empty()
        || (cap.header.tree_seal_64 != 0 && actual_seal != cap.header.tree_seal_64)
    {
        return Ok(0);
    }
    let cid = compute_folder_crate_id(folder);
    let by_name: HashMap<&str, &DiskEntrySeal> =
        entries.iter().map(|e| (e.filename.as_str(), e)).collect();
    let (mut tx, mut stats, mut inserted) = (
        conn.begin().await.map_err(|e| e.to_string())?,
        Vec::new(),
        0,
    );
    for e in &cap.tracks {
        let Some(disk) = by_name.get(e.filename.as_str()) else {
            continue;
        };
        let tid = get_track_id_for_path(&disk.path).unwrap_or_else(|_| Uuid::new_v4().to_string());
        let cam = omkf::code_to_camelot(e.slot.camelot_code).unwrap_or_else(|| "8A".into());
        let fam = BoothGenreFamily::from_code(e.slot.genre_family_code);
        let genre_s = (fam != BoothGenreFamily::Unknown).then(|| fam.as_str().to_string());
        let (ptx, ppath) = (&mut tx, &disk.path);
        sql_upsert_hydrated_omkf_entry_tx(ptx, &cid, &tid, ppath, e, &cam, genre_s.as_deref())
            .await?;
        let vid = (e.slot.flags_bits & omkf::OMKF_FLAG_IS_VIDEO) != 0;
        let lufs = f64::from(e.slot.lufs_x100) / 100.0;
        stats.push((
            cam,
            genre_s.unwrap_or_default(),
            e.slot.inferred_era_year,
            lufs,
            vid,
            e.slot.scan_level >= 2,
        ));
        inserted += 1;
    }
    let refs: Vec<_> = stats
        .iter()
        .map(|(c, g, e, l, v, f)| (c.as_str(), g.as_str(), *e, *l, *v, *f))
        .collect();
    let mut hdr = cap.header.clone();
    hdr.dir_mtime_ns = dir_ns;
    let (md, full_n) = summarize_omkf_crate_md(
        &cid,
        hdr.tree_seal_64,
        hdr.bpm_min_x100,
        hdr.bpm_max_x100,
        &refs,
    );
    sql_upsert_folder_crate(&mut tx, &cid, folder, &hdr, &md, full_n, now_ms()).await?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(inserted)
}

pub async fn write_folder_omkf_from_db(
    conn: &mut SqliteConnection,
    folder: &Path,
) -> Result<String, String> {
    let cid = compute_folder_crate_id(folder);
    let (dir_ns, disk, tree_seal) = scan_dir_entry_seals(folder);
    let loaded = sql_load_folder_omkf_entries(conn, &cid, folder, &disk).await?;
    let (hdr, entries, md, full_n) = pack_omkf_header_and_entries(&cid, tree_seal, dir_ns, loaded);
    let hera_dir = folder.join(".hera");
    omkf::write_omkf_capsule_atomic(&hera_dir, &hdr, &entries, &md)?;
    hide_hera_dir_if_windows(&hera_dir);
    sql_upsert_folder_crate(conn, &cid, folder, &hdr, &md, full_n, now_ms()).await?;
    Ok(cid)
}

pub async fn flush_dirty_omkf_capsules(conn: &mut SqliteConnection) -> usize {
    let dirs: Vec<PathBuf> = dirty_folders_slot()
        .lock()
        .map(|mut g| g.drain().collect())
        .unwrap_or_default();
    let mut n = 0;
    for d in dirs {
        if d.is_dir()
            && let Ok(cid) = write_folder_omkf_from_db(conn, &d).await
        {
            crate::libs::omkf_cloud_client::spawn_background_omkf_folder_cloud_sync(
                "DaviDJ".to_string(),
                cid,
                d,
            );
            n += 1;
        }
    }
    n
}

pub async fn assign_crate_ids_and_mark_dirty(conn: &mut SqliteConnection, tracks: &[Track]) {
    let mut dirs = HashSet::new();
    for t in tracks {
        if let Some(p) = Path::new(&t.path).parent() {
            let cid = compute_folder_crate_id(p);
            let _ = sqlx::query("UPDATE tracks SET crate_id = ? WHERE id = ?")
                .bind(cid)
                .bind(&t.id)
                .execute(&mut *conn)
                .await;
            dirs.insert(p.to_path_buf());
        }
    }
    for d in dirs {
        mark_folder_dirty(&d);
    }
}

pub async fn boot_verify_folder_crates(conn: &mut SqliteConnection) -> (usize, usize) {
    let q = "SELECT folder_path, tree_seal_64, dir_mtime_ns FROM folder_crates";
    let rows: Vec<(String, String, i64)> = sqlx::query_as(q)
        .fetch_all(&mut *conn)
        .await
        .unwrap_or_default();
    let (mut ok, mut dirty) = (0, 0);
    for (fp, seal_hex, old_ns) in rows {
        let dir = Path::new(&fp);
        if !dir.is_dir() {
            continue;
        }
        let hdr_ok = omkf::read_omkf_header(&dir.join(".hera/tracks.omkf"))
            .is_ok_and(|h| !seal_hex.is_empty() && format!("{:016x}", h.tree_seal_64) == seal_hex);
        if sys_ns(std::fs::metadata(dir).and_then(|m| m.modified())) == old_ns || hdr_ok {
            ok += 1;
        } else {
            mark_folder_dirty(dir);
            dirty += 1;
        }
    }
    (ok, dirty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
    use sqlx::{Connection, SqliteConnection};

    #[tokio::test]
    async fn test_zero_import_usb_hydrates_after_db_wipe() {
        let dir = std::env::temp_dir().join(format!("hera_usb_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav = dir.join("groove.wav");
        std::fs::write(&wav, vec![1_u8; 2048]).unwrap();
        let tid = get_track_id_for_path(&wav).unwrap();
        let mut c1 = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/migrations")
            .run_direct(&mut c1)
            .await
            .unwrap();
        sqlx::query("INSERT INTO tracks (id,path,title,album,album_artist,artists,genres,duration,bpm,camelot,hot_cues,scan_level,embedding_64d_b64,inferred_genre_family,inferred_era_year) VALUES (?1,?2,'Groove','EP','DaviDJ','[\"DaviDJ\"]','[]',240,128.5,'9A','[{\"num\":1,\"name\":\"1:Intro\",\"seconds\":16.0,\"color\":\"#22c55e\",\"role\":\"intro\"}]',2,?3,'AfroHouse',2024)")
            .bind(&tid).bind(wav.to_string_lossy().to_string()).bind(B64.encode([42_u8; 64])).execute(&mut c1).await.unwrap();
        let cid = write_folder_omkf_from_db(&mut c1, &dir).await.unwrap();
        assert!(dir.join(".hera/tracks.omkf").is_file() && dir.join(".hera/crate.md").is_file());
        let mut c2 = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/migrations")
            .run_direct(&mut c2)
            .await
            .unwrap();
        let n = hydrate_folder_from_omkf_zero_dsp(&mut c2, &dir)
            .await
            .unwrap();
        assert_eq!(n, 1);
        let q = "SELECT bpm, camelot, scan_level, crate_id FROM tracks WHERE id = ?";
        let r: (f64, String, i64, String) = sqlx::query_as(q)
            .bind(&tid)
            .fetch_one(&mut c2)
            .await
            .unwrap();
        assert_eq!((r.0, &*r.1, r.2, &*r.3), (128.5, "9A", 2, &*cid));
        assert_eq!(boot_verify_folder_crates(&mut c2).await.0, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
