use std::path::{Path, PathBuf};

use hera_ipc::dj_types::OrganizeFolderRequest;
use hera_node_rs::scanner::omkf_capsule::{
    OmkfCrateMdMetadata, OmkfHeader, OmkfTrackEntry, OmkfTrackSlot, camelot_to_code,
    read_omkf_capsule, write_omkf_capsule_atomic,
};
use sqlx::{Connection, SqliteConnection};
use uuid::Uuid;

use crate::libs::db_scanner_queries::ScannerRowUpdate;
use crate::libs::omkf_fs_catalog::OmkfFsCatalog;
use crate::libs::omkf_fs_organize::{
    organize_folder_fs, save_folder_rules_fs, upsert_scanner_outcomes_to_omkf,
};
use crate::libs::omkf_sync::{compute_folder_crate_id, write_folder_omkf_from_db};
use crate::libs::track::{HotCuePin, get_track_id_for_path};

struct TempDirGuard(PathBuf);

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let p = std::env::temp_dir().join(format!("hera_omkf_fs_{prefix}_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&p).expect("create temp dir");
        Self(p)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn org_req(cid: &str, dry_run: bool, rollback_tx_id: Option<String>) -> OrganizeFolderRequest {
    OrganizeFolderRequest {
        crate_id: cid.to_string(),
        strategy: "by_tempo_zone".to_string(),
        dry_run,
        rollback_tx_id,
    }
}

#[test]
fn test_nested_roots_deduplicate_and_container_folders_retained() {
    let tmp = TempDirGuard::new("nested");
    let music = tmp.path().join("Music");
    let repertorio = music.join("_REPERTORIO_DJ");
    let afro = repertorio.join("01_Afro_House");
    let reggaeton = repertorio.join("03_Reggaeton");
    let empty_dir = music.join("Empty_No_Media");

    std::fs::create_dir_all(&afro).unwrap();
    std::fs::create_dir_all(&reggaeton).unwrap();
    std::fs::create_dir_all(&empty_dir).unwrap();

    std::fs::write(afro.join("Keinemusik - Move.flac"), b"flac-data").unwrap();
    std::fs::write(reggaeton.join("Feid - Luna.mp3"), b"mp3-data").unwrap();
    std::fs::write(empty_dir.join("notes.txt"), b"not audio").unwrap();

    let catalog = OmkfFsCatalog::new();
    let roots = vec![repertorio.clone(), music.clone()];
    let folders = catalog.list_folders(&roots);

    let rep_matches: Vec<_> = folders
        .iter()
        .filter(|f| f.display_name == "_REPERTORIO_DJ")
        .collect();
    assert_eq!(rep_matches.len(), 1);
    let rep_summary = rep_matches[0];
    assert_eq!(
        (
            rep_summary.depth,
            rep_summary.track_count,
            rep_summary.subfolder_count
        ),
        (1, 0, 2)
    );

    let music_id = compute_folder_crate_id(&music);
    assert_eq!(
        rep_summary.parent_crate_id.as_deref(),
        Some(music_id.as_str())
    );
    assert!(!folders.iter().any(|f| f.display_name == "Empty_No_Media"));

    let (detail_opt, unanalyzed) = catalog.read_folder(&rep_summary.crate_id, &roots);
    let detail = detail_opt.expect("_REPERTORIO_DJ detail");
    assert_eq!(detail.subfolders.len(), 2);
    assert_eq!(detail.subfolders[0].display_name, "01_Afro_House");
    assert_eq!(detail.subfolders[1].display_name, "03_Reggaeton");
    assert!(detail.tracks.is_empty() && unanalyzed.is_empty());
}

#[test]
fn test_read_folder_merges_omkf_capsule_with_live_unanalyzed_and_skips_junk() {
    let tmp = TempDirGuard::new("merge");
    let club = tmp.path().join("01_Afro_House");
    let macosx = club.join("__MACOSX");
    std::fs::create_dir_all(&macosx).unwrap();

    let analyzed_file = club.join("Keinemusik - Move.flac");
    let unanalyzed_file = club.join("Artist - Song [96 BPM - 12B].mp3");
    let video_file = club.join("Rauw Alejandro - Todo De Ti.mp4");
    std::fs::write(&analyzed_file, b"analyzed-audio").unwrap();
    std::fs::write(&unanalyzed_file, b"live-unanalyzed-audio").unwrap();
    std::fs::write(&video_file, b"hd-video-container").unwrap();
    std::fs::write(club.join("._junk.mp3"), b"appledouble").unwrap();
    std::fs::write(macosx.join("ghost.mp3"), b"macosx-junk").unwrap();

    let slot = OmkfTrackSlot {
        bpm_x100: 12000,
        camelot_code: camelot_to_code("4B"),
        scan_level: 2,
        duration_sec_u16: 352,
        hot_cues_ms: [15_500, 64_000, 128_000, 280_000, 0, 0, 0, 0],
        ..Default::default()
    };
    let entry = OmkfTrackEntry {
        slot,
        filename: "Keinemusik - Move.flac".to_string(),
        artist: "Keinemusik".to_string(),
        title: "Move".to_string(),
    };
    let hdr = OmkfHeader {
        tree_seal_64: 0xCAFE_BABE_1234_5678,
        bpm_min_x100: 12000,
        bpm_max_x100: 12000,
        ..Default::default()
    };
    let md = OmkfCrateMdMetadata {
        crate_id: compute_folder_crate_id(&club),
        tree_seal_64: hdr.tree_seal_64,
        ..Default::default()
    };
    write_omkf_capsule_atomic(&club.join(".hera"), &hdr, &[entry], &md).unwrap();

    let catalog = OmkfFsCatalog::new();
    let roots = vec![club.clone()];
    let cid = compute_folder_crate_id(&club);
    let (detail_opt, unanalyzed) = catalog.read_folder(&cid, &roots);
    let detail = detail_opt.expect("hydrated folder detail");

    assert!(detail.hydrated_from_omkf && detail.disk_online);
    assert_eq!((detail.tracks.len(), detail.summary.video_count), (3, 1));

    let move_t = detail.tracks.iter().find(|t| t.title == "Move").unwrap();
    assert_eq!(
        (
            move_t.artist.as_str(),
            move_t.bpm,
            move_t.camelot.as_deref()
        ),
        ("Keinemusik", Some(120.0), Some("4B"))
    );
    assert_eq!((move_t.scan_level, move_t.hot_cues.len()), (2, 4));
    assert!((move_t.hot_cues[0].seconds - 15.5).abs() < 1e-6);

    let live_t = detail
        .tracks
        .iter()
        .find(|t| t.file_name == "Artist - Song [96 BPM - 12B].mp3")
        .unwrap();
    assert_eq!(
        (live_t.artist.as_str(), live_t.title.as_str()),
        ("Artist", "Song")
    );
    assert_eq!(
        (live_t.bpm, live_t.camelot.as_deref(), live_t.scan_level),
        (Some(96.0), Some("12B"), 0)
    );

    assert_eq!(unanalyzed.len(), 2);
    let expected_live_id = get_track_id_for_path(&unanalyzed_file).unwrap();
    assert!(unanalyzed.iter().any(|(id, _)| id == &expected_live_id));

    let resolved = catalog.get_tracks_by_ids(&[move_t.id.clone(), live_t.id.clone()]);
    assert_eq!(
        (resolved[0].title.as_str(), resolved[1].title.as_str()),
        ("Move", "Song")
    );
}

#[test]
fn test_search_deduplicates_meta_and_hash_and_harmonic_excludes_backup() {
    let tmp = TempDirGuard::new("search");
    let root = tmp.path().join("Music");
    let active_dir = root.join("_REPERTORIO_DJ").join("03_Reggaeton");
    let backup_dir = root.join("BACKUP").join("Old");
    let usb_dir = root.join("_USB_PIONEER");

    std::fs::create_dir_all(&active_dir).unwrap();
    std::fs::create_dir_all(&backup_dir).unwrap();
    std::fs::create_dir_all(&usb_dir).unwrap();

    for p in [
        active_dir.join("Wisin & Yandel - Mírala Bien [96 BPM - 8A].mp3"),
        backup_dir.join("Wisin & Yandel - Mirala Bien.mp3"),
        usb_dir.join("Wisin & Yandel - Mirala Bien (Official Video).mp4"),
        active_dir.join("Disclosure - Latch [122 BPM - 4B].flac"),
        active_dir.join("Black Coffee - Drive [121 BPM - 11B].flac"),
        backup_dir.join("Backup Only - Ghost [120 BPM - 4B].mp3"),
        active_dir.join("Random - Clash [155 BPM - 1A].mp3"),
        active_dir.join("DJ One - Secret Dubplate.flac"),
        usb_dir.join("99_raw_dubplate.flac"),
    ] {
        std::fs::write(p, b"audio-bytes").unwrap();
    }

    // Write identical non-zero partial_hash_64kb on two differently named dubplate files
    let shared_hash = [0xAB_u8; 16];
    for (dir, fname, art, tit) in [
        (
            &active_dir,
            "DJ One - Secret Dubplate.flac",
            "DJ One",
            "Secret Dubplate",
        ),
        (&usb_dir, "99_raw_dubplate.flac", "", "99_raw_dubplate"),
    ] {
        let slot = OmkfTrackSlot {
            partial_hash_64kb: shared_hash,
            scan_level: 2,
            duration_sec_u16: 215,
            ..Default::default()
        };
        let e = OmkfTrackEntry {
            slot,
            filename: fname.into(),
            artist: art.into(),
            title: tit.into(),
        };
        let md = OmkfCrateMdMetadata {
            crate_id: compute_folder_crate_id(dir),
            ..Default::default()
        };
        write_omkf_capsule_atomic(&dir.join(".hera"), &OmkfHeader::default(), &[e], &md).unwrap();
    }

    let catalog = OmkfFsCatalog::new();
    let roots = vec![root];
    let tracks_for = |q: &str| catalog.read_folder(q, &roots).0.unwrap().tracks;

    let hits = tracks_for("__search__:mirala bien");
    assert_eq!((hits.len(), hits[0].copy_count), (1, 3));
    assert!(hits[0].path.contains("_REPERTORIO_DJ"));

    let hash_hits = tracks_for("__search__:dubplate");
    assert_eq!((hash_hits.len(), hash_hits[0].copy_count), (1, 2));
    assert!(hash_hits[0].path.contains("_REPERTORIO_DJ"));

    let harm_tracks = tracks_for("__harmonic__:120:4B");
    assert!(harm_tracks.iter().any(|t| t.title == "Latch"));
    assert!(harm_tracks.iter().any(|t| t.title == "Drive"));
    assert!(!harm_tracks.iter().any(|t| t.title == "Ghost"));
    assert!(!harm_tracks.iter().any(|t| t.title == "Clash"));
}

#[tokio::test]
async fn test_organize_fs_overwrite_guard_rollback_and_non_sql_capsule_survival() {
    let tmp = TempDirGuard::new("organize");
    let crate_dir = tmp.path().join("Set_Viernes");
    std::fs::create_dir_all(&crate_dir).unwrap();

    let fast_wav = crate_dir.join("DJ Snake - Propagate [134 BPM - 8A].wav");
    let warm_wav = crate_dir.join("Sade - Smooth [108 BPM - 5A].wav");
    std::fs::write(&fast_wav, b"fast-pcm-bytes").unwrap();
    std::fs::write(&warm_wav, b"warm-pcm-bytes").unwrap();

    let catalog = OmkfFsCatalog::new();
    let roots = vec![crate_dir.clone()];
    let cid = compute_folder_crate_id(&crate_dir);

    assert!(save_folder_rules_fs(&catalog, &roots, &cid, "Solo club >= 120 BPM").unwrap());
    assert_eq!(
        catalog.read_folder(&cid, &roots).0.unwrap().rules_markdown,
        "Solo club >= 120 BPM"
    );

    // Upsert non-SQLite outcome into .hera/tracks.omkf and verify survival across empty-DB write_folder_omkf_from_db
    let fast_id = get_track_id_for_path(&fast_wav).unwrap();
    let mut upd = ScannerRowUpdate::failure(fast_id, Some(210), 2);
    (upd.bpm, upd.camelot) = (Some(134.0), Some("8A".to_string()));
    let pin = HotCuePin {
        num: 1,
        name: "1:Intro".into(),
        seconds: 8.0,
        color: "#22c55e".into(),
        role: "Intro".into(),
        origin: None,
    };
    let batch = [(fast_wav.to_string_lossy().into(), upd, Some(vec![pin]))];
    assert_eq!(
        upsert_scanner_outcomes_to_omkf(&catalog, &batch),
        vec![cid.clone()]
    );

    let mut empty_conn = SqliteConnection::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("src/migrations")
        .run_direct(&mut empty_conn)
        .await
        .unwrap();
    write_folder_omkf_from_db(&mut empty_conn, &crate_dir)
        .await
        .unwrap();
    let survived = read_omkf_capsule(&crate_dir.join(".hera/tracks.omkf")).unwrap();
    assert_eq!(
        (survived.tracks.len(), survived.tracks[0].slot.bpm_x100),
        (1, 13_400)
    );

    // Sacred Disk Invariant: pre-existing destination file must never be overwritten
    let conflict_dir = crate_dir.join("_128_140");
    std::fs::create_dir_all(&conflict_dir).unwrap();
    let conflict_dst = conflict_dir.join("DJ Snake - Propagate [134 BPM - 8A].wav");
    std::fs::write(&conflict_dst, b"SACRED_EXISTING_FILE").unwrap();

    let guarded = organize_folder_fs(&catalog, &roots, org_req(&cid, false, None)).unwrap();
    assert_eq!(guarded.applied_count, 1);
    assert_eq!(
        std::fs::read(&conflict_dst).unwrap(),
        b"SACRED_EXISTING_FILE"
    );
    assert!(fast_wav.is_file() && !warm_wav.exists());

    std::fs::remove_file(&conflict_dst).unwrap();
    let rb1 =
        organize_folder_fs(&catalog, &roots, org_req(&cid, false, Some(guarded.tx_id))).unwrap();
    assert!(rb1.rolled_back && warm_wav.is_file());

    let preview = organize_folder_fs(&catalog, &roots, org_req(&cid, true, None)).unwrap();
    assert_eq!((preview.moves.len(), preview.applied_count), (2, 0));

    let applied = organize_folder_fs(&catalog, &roots, org_req(&cid, false, None)).unwrap();
    assert_eq!(applied.applied_count, 2);
    assert!(!fast_wav.exists() && !warm_wav.exists());

    let rb2 =
        organize_folder_fs(&catalog, &roots, org_req(&cid, false, Some(applied.tx_id))).unwrap();
    assert!(rb2.rolled_back && rb2.applied_count == 2 && fast_wav.is_file() && warm_wav.is_file());
}
