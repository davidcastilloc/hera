use std::path::{Path, PathBuf};

use hera_ipc::dj_types::OrganizeFolderRequest;
use hera_node_rs::scanner::omkf_capsule::{
    OmkfCrateMdMetadata, OmkfHeader, OmkfTrackEntry, OmkfTrackSlot, camelot_to_code,
    write_omkf_capsule_atomic,
};
use uuid::Uuid;

use crate::libs::db_scanner_queries::ScannerRowUpdate;
use crate::libs::omkf_fs_catalog::OmkfFsCatalog;
use crate::libs::omkf_fs_organize::{
    organize_folder_fs, save_folder_rules_fs, upsert_scanner_outcomes_to_omkf,
};
use crate::libs::omkf_sync::compute_folder_crate_id;
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
    // Pass overlapping child root alongside ancestor root (in reverse order too)
    let roots = vec![repertorio.clone(), music.clone()];
    let folders = catalog.list_folders(&roots);

    let rep_matches: Vec<_> = folders
        .iter()
        .filter(|f| f.display_name == "_REPERTORIO_DJ")
        .collect();
    assert_eq!(
        rep_matches.len(),
        1,
        "_REPERTORIO_DJ must appear once despite nested overlapping root"
    );
    let rep_summary = rep_matches[0];
    assert_eq!(rep_summary.depth, 1);
    assert_eq!(rep_summary.track_count, 0);
    assert_eq!(rep_summary.subfolder_count, 2);

    let music_id = compute_folder_crate_id(&music);
    assert_eq!(
        rep_summary.parent_crate_id.as_deref(),
        Some(music_id.as_str())
    );
    assert!(
        !folders.iter().any(|f| f.display_name == "Empty_No_Media"),
        "Empty non-root directories with 0 descendant media must be pruned"
    );

    let (detail_opt, unanalyzed) = catalog.read_folder(&rep_summary.crate_id, &roots);
    let detail = detail_opt.expect("_REPERTORIO_DJ detail");
    assert_eq!(detail.subfolders.len(), 2);
    assert_eq!(detail.subfolders[0].display_name, "01_Afro_House");
    assert_eq!(detail.subfolders[1].display_name, "03_Reggaeton");
    assert!(detail.tracks.is_empty());
    assert!(unanalyzed.is_empty());
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
    let junk_dot = club.join("._junk.mp3");
    let junk_mac = macosx.join("ghost.mp3");

    std::fs::write(&analyzed_file, b"analyzed-audio").unwrap();
    std::fs::write(&unanalyzed_file, b"live-unanalyzed-audio").unwrap();
    std::fs::write(&video_file, b"hd-video-container").unwrap();
    std::fs::write(&junk_dot, b"appledouble").unwrap();
    std::fs::write(&junk_mac, b"macosx-junk").unwrap();

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

    assert!(detail.hydrated_from_omkf);
    assert!(detail.disk_online);
    assert_eq!(detail.tracks.len(), 3, "must skip ._junk.mp3 and __MACOSX");
    assert_eq!(detail.summary.video_count, 1);

    let move_t = detail
        .tracks
        .iter()
        .find(|t| t.title == "Move")
        .expect("Move track from capsule");
    assert_eq!(move_t.artist, "Keinemusik");
    assert_eq!(move_t.bpm, Some(120.0));
    assert_eq!(move_t.camelot.as_deref(), Some("4B"));
    assert_eq!(move_t.scan_level, 2);
    assert_eq!(move_t.hot_cues.len(), 4);
    assert!((move_t.hot_cues[0].seconds - 15.5).abs() < 1e-6);

    let live_t = detail
        .tracks
        .iter()
        .find(|t| t.file_name == "Artist - Song [96 BPM - 12B].mp3")
        .expect("live unanalyzed track");
    assert_eq!(live_t.artist, "Artist");
    assert_eq!(live_t.title, "Song");
    assert_eq!(live_t.bpm, Some(96.0));
    assert_eq!(live_t.camelot.as_deref(), Some("12B"));
    assert_eq!(live_t.scan_level, 0);

    // Unanalyzed list includes the two scan_level == 0 files, excluding Move (scan_level == 2)
    assert_eq!(unanalyzed.len(), 2);
    let expected_live_id = get_track_id_for_path(&unanalyzed_file).unwrap();
    assert!(unanalyzed.iter().any(|(id, _)| id == &expected_live_id));

    // Verify get_tracks_by_ids resolves directly from RAM without SQLite
    let resolved = catalog.get_tracks_by_ids(&[move_t.id.clone(), live_t.id.clone()]);
    assert_eq!(resolved.len(), 2);
    assert_eq!(resolved[0].title, "Move");
    assert_eq!(resolved[1].title, "Song");
}

#[test]
fn test_search_deduplicates_copies_preferring_repertorio_and_harmonic_sentinel() {
    let tmp = TempDirGuard::new("search");
    let root = tmp.path().join("Music");
    let active_dir = root.join("_REPERTORIO_DJ").join("03_Reggaeton");
    let backup_dir = root.join("BACKUP").join("Old");
    let usb_dir = root.join("_USB_PIONEER");

    std::fs::create_dir_all(&active_dir).unwrap();
    std::fs::create_dir_all(&backup_dir).unwrap();
    std::fs::create_dir_all(&usb_dir).unwrap();

    let copy_active = active_dir.join("Wisin & Yandel - Mírala Bien [96 BPM - 8A].mp3");
    let copy_backup = backup_dir.join("Wisin & Yandel - Mirala Bien.mp3");
    let copy_usb = usb_dir.join("Wisin & Yandel - Mirala Bien (Official Video).mp4");
    let compat_house = active_dir.join("Disclosure - Latch [122 BPM - 4B].flac");
    let compat_afro = active_dir.join("Black Coffee - Drive [121 BPM - 11B].flac");
    let clash_track = active_dir.join("Random - Clash [155 BPM - 1A].mp3");

    for p in [
        &copy_active,
        &copy_backup,
        &copy_usb,
        &compat_house,
        &compat_afro,
        &clash_track,
    ] {
        std::fs::write(p, b"audio-bytes").unwrap();
    }

    let catalog = OmkfFsCatalog::new();
    let roots = vec![root];

    let (search_detail, _) = catalog.read_folder("__search__:mirala bien", &roots);
    let hits = search_detail.expect("search detail").tracks;
    assert_eq!(hits.len(), 1, "3 copies must collapse into 1 result");
    assert_eq!(hits[0].copy_count, 3);
    assert!(
        hits[0].path.contains("_REPERTORIO_DJ"),
        "must prefer active _REPERTORIO_DJ path over BACKUP and _USB_PIONEER, got: {}",
        hits[0].path
    );

    let (harm_detail, _) = catalog.read_folder("__harmonic__:120:4B", &roots);
    let harm_tracks = harm_detail.expect("harmonic detail").tracks;
    assert!(harm_tracks.iter().any(|t| t.title == "Latch"));
    assert!(harm_tracks.iter().any(|t| t.title == "Drive"));
    assert!(!harm_tracks.iter().any(|t| t.title == "Clash"));
}

#[test]
fn test_organize_folder_fs_rules_dry_run_apply_rollback_and_capsule_upsert() {
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
    let (d0, _) = catalog.read_folder(&cid, &roots);
    assert_eq!(d0.unwrap().rules_markdown, "Solo club >= 120 BPM");

    // Upsert a background scanner full-pass outcome directly into .hera/tracks.omkf
    let fast_id = get_track_id_for_path(&fast_wav).unwrap();
    let mut upd = ScannerRowUpdate::failure(fast_id.clone(), Some(210), 2);
    upd.bpm = Some(134.0);
    upd.camelot = Some("8A".to_string());
    let pins = vec![HotCuePin {
        num: 1,
        name: "1:Intro".to_string(),
        seconds: 8.0,
        color: "#22c55e".to_string(),
        role: "Intro".to_string(),
        origin: Some("hera_auto".to_string()),
    }];
    let touched = upsert_scanner_outcomes_to_omkf(
        &catalog,
        &[(fast_wav.to_string_lossy().to_string(), upd, Some(pins))],
    );
    assert_eq!(touched, vec![cid.clone()]);

    // Dry run moves nothing
    let preview = organize_folder_fs(
        &catalog,
        &roots,
        OrganizeFolderRequest {
            crate_id: cid.clone(),
            strategy: "by_tempo_zone".to_string(),
            dry_run: true,
            rollback_tx_id: None,
        },
    )
    .unwrap();
    assert_eq!(preview.moves.len(), 2);
    assert_eq!(preview.applied_count, 0);
    assert!(fast_wav.is_file() && warm_wav.is_file());

    // Apply moves into _128_140 and _100_118
    let applied = organize_folder_fs(
        &catalog,
        &roots,
        OrganizeFolderRequest {
            crate_id: cid.clone(),
            strategy: "by_tempo_zone".to_string(),
            dry_run: false,
            rollback_tx_id: None,
        },
    )
    .unwrap();
    assert_eq!(applied.applied_count, 2);
    assert!(!fast_wav.exists() && !warm_wav.exists());
    assert!(
        crate_dir
            .join("_128_140")
            .join("DJ Snake - Propagate [134 BPM - 8A].wav")
            .is_file()
    );
    assert!(
        crate_dir
            .join("_100_118")
            .join("Sade - Smooth [108 BPM - 5A].wav")
            .is_file()
    );

    // Rollback restores original files via # Ledger Reversible without SQLite
    let rb = organize_folder_fs(
        &catalog,
        &roots,
        OrganizeFolderRequest {
            crate_id: cid,
            strategy: "by_tempo_zone".to_string(),
            dry_run: false,
            rollback_tx_id: Some(applied.tx_id),
        },
    )
    .unwrap();
    assert!(rb.rolled_back);
    assert_eq!(rb.applied_count, 2);
    assert!(fast_wav.is_file() && warm_wav.is_file());
}
