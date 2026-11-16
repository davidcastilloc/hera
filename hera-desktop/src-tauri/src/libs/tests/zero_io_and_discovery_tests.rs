use std::io::Cursor;
use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::libs::db_agent_queries::{AgentTrackFilter, query_agent_tracks_filtered};
use crate::libs::db_scanner_queries::{ScannerRowUpdate, apply_scanner_batch_updates};
use crate::libs::fast_library_scan::import_tracks_with_index;
use crate::libs::track::Track;
use crate::plugins::db::setup_at_path;
use crate::plugins::hera_dj::vdj_xml_parser::parse_vdj_database_reader;
use crate::plugins::hera_dj::workability::{
    CachedDjSoftwareMetadata, ExternalDjSoftwareIndex, discover_workable_roots_from_index,
};

fn build_test_wav_bytes(duration_secs: f32) -> Vec<u8> {
    let sr: u32 = 8_000;
    let frames = (sr as f32 * duration_secs) as usize;
    let data_bytes = (frames * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_bytes as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&sr.to_le_bytes());
    out.extend_from_slice(&(sr * 2).to_le_bytes());
    out.extend_from_slice(&2_u16.to_le_bytes());
    out.extend_from_slice(&16_u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes.to_le_bytes());
    for i in 0..frames {
        let s = (((i as f32 * 0.09).sin()) * 14_000.0) as i16;
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[tokio::test]
async fn test_migration_08_and_query_agent_tracks_filtered_under_5ms() {
    let dir = std::env::temp_dir().join(format!("hera_mig08_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("hera.db");

    let state = setup_at_path(&db_path).await.expect("setup_at_path");
    {
        let mut writer = state.get_lock().await;
        // Verify new Migration 08 tables exist
        sqlx::query(
            "INSERT INTO producer_watch_folders (id, folder_path, enabled, created_at) VALUES ('pw-1', 'D:/Renders', 1, 100)",
        )
        .execute(&mut writer.connection)
        .await
        .expect("producer_watch_folders must exist");
        sqlx::query(
            "INSERT INTO acoustic_landmarks (hash_u64, track_id, time_offset_ms) VALUES (424242, 'clean-1', 1250)",
        )
        .execute(&mut writer.connection)
        .await
        .expect("acoustic_landmarks must exist");

        // Seed: 2 clean workable 8A tracks, 1 wrong-key track (9A), 1 quarantined (-1), 1 scratched (skips=1), 1 >600s mix (720s)
        sqlx::query(
            r#"
            INSERT INTO tracks (
                id, path, title, album, album_artist, artists, genres, year, duration,
                is_compilation, bpm, camelot, scan_level, has_audio_skips, is_mix_over_10m,
                inferred_genre_family, inferred_era_year
            ) VALUES
                ('clean-1', '/club/clean1.flac', 'Perreo Limpio 1', 'Club', 'Feid', '["Feid"]', '["Reggaeton"]', 2025, 198, 0, 124.5, '8A', 2, 0, 0, 'Reggaeton', 2025),
                ('clean-2', '/club/clean2.flac', 'Perreo Limpio 2', 'Club', 'Karol G', '["Karol G"]', '["Reggaeton"]', 2024, 215, 0, 126.0, '8A', 1, 0, 0, 'Reggaeton', 2024),
                ('wrong-key', '/club/key9a.flac', 'Otro Tono', 'Club', 'DJ X', '["DJ X"]', '["House"]', 2025, 210, 0, 125.0, '9A', 2, 0, 0, 'House', 2025),
                ('quarantined', '/club/bad.flac', 'Corrupta', 'Club', 'Bad', '["Bad"]', '["Reggaeton"]', 2025, 200, 0, 125.0, '8A', -1, 0, 0, 'Reggaeton', 2025),
                ('scratched', '/club/skip.flac', 'Rayada CD', 'Club', 'Old CD', '["Old CD"]', '["Reggaeton"]', 2025, 205, 0, 125.0, '8A', 2, 1, 0, 'Reggaeton', 2025),
                ('long-mix', '/club/mix720.flac', 'Set En Vivo 12m', 'Mixes', 'DaviDJ', '["DaviDJ"]', '["Reggaeton"]', 2025, 720, 0, 125.0, '8A', 2, 0, 1, 'Reggaeton', 2025)
            "#,
        )
        .execute(&mut writer.connection)
        .await
        .unwrap();
    }

    let mut read_conn = state.acquire_read_conn().await.unwrap();
    let (mmap_val,): (i64,) = sqlx::query_as("PRAGMA mmap_size;")
        .fetch_one(&mut *read_conn)
        .await
        .unwrap();
    assert_eq!(mmap_val, 268_435_456);

    let filter = AgentTrackFilter {
        camelots: vec!["8A".to_string()],
        bpm_min: Some(123.0),
        bpm_max: Some(127.0),
        include_mixes_over_10m: false,
        ..Default::default()
    };
    // Warm SQLite statement cache once, then time the indexed query
    let _ = query_agent_tracks_filtered(&mut read_conn, &filter)
        .await
        .unwrap();
    let t0 = Instant::now();
    let results: Vec<Track> = query_agent_tracks_filtered(&mut read_conn, &filter)
        .await
        .unwrap();
    let elapsed = t0.elapsed();

    assert!(
        elapsed < Duration::from_millis(5),
        "expected < 5ms indexed SQL query, took {elapsed:?}"
    );
    let ids: Vec<&str> = results.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, vec!["clean-1", "clean-2"]);

    drop(read_conn);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_vdj_8_cue_import_preservation_zero_io_metrics_and_moved_path_heal() {
    let dir = std::env::temp_dir().join(format!("hera_vdj8_{}", Uuid::new_v4()));
    let folder_a = dir.join("SetA");
    let folder_b = dir.join("SetB");
    std::fs::create_dir_all(&folder_a).unwrap();
    std::fs::create_dir_all(&folder_b).unwrap();

    let track_path_a = folder_a.join("Tony Vega - Aparentemente.wav");
    std::fs::write(&track_path_a, build_test_wav_bytes(30.0)).unwrap();

    let xml = format!(
        r#"<VirtualDJ_Database Version="8.5">
  <Song FilePath="{}" FileSize="480044">
    <Tags Author="Tony Vega" Title="Aparentemente" />
    <Infos SongLength="242.0" Bitrate="320" />
    <Scan Version="801" Bpm="126.0" Key="7A" />
    <Poi Name="Intro" Pos="8.2" Num="1" Type="cue" />
    <Poi Name="Verso" Pos="32.0" Num="2" Type="cue" />
    <Poi Name="Coro" Pos="64.5" Num="3" Type="cue" />
    <Poi Name="Pregón" Pos="96.0" Num="4" Type="cue" />
    <Poi Name="Mambo" Pos="128.4" Num="5" Type="cue" />
    <Poi Name="Moña" Pos="152.0" Num="6" Type="cue" />
    <Poi Name="Climax" Pos="174.8" Num="7" Type="cue" />
    <Poi Name="Cierre" Pos="196.2" Num="8" Type="cue" />
  </Song>
</VirtualDJ_Database>"#,
        track_path_a.to_string_lossy()
    );
    let parsed = parse_vdj_database_reader(Cursor::new(xml));
    let mut index = ExternalDjSoftwareIndex::default();
    for (k, entry) in parsed {
        index.vdj_entries.insert(
            k,
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

    let imported = import_tracks_with_index(std::slice::from_ref(&track_path_a), &index);
    assert_eq!(imported.len(), 1);
    let trk = &imported[0];
    let cues = trk.hot_cues.as_ref().expect("8 cues must be imported");
    assert_eq!(cues.len(), 8);
    assert!(
        cues.iter()
            .all(|c| c.origin.as_deref() == Some("dj_manual"))
    );
    let cue5 = cues.iter().find(|c| c.num == 5).expect("Cue 5 present");
    let cue8 = cues.iter().find(|c| c.num == 8).expect("Cue 8 present");
    assert!((cue5.seconds - 128.4).abs() < 1e-3);
    assert!((cue8.seconds - 196.2).abs() < 1e-3);
    assert_eq!(trk.legacy_bpm, Some(126.0));
    assert_eq!(trk.bitrate_kbps, Some(320));

    // Persist into DB, then apply FullPass update with BPM trust diff (94.5 vs 126.0) + audio skip + auto cues
    let db_path = dir.join("hera.db");
    let state = setup_at_path(&db_path).await.unwrap();
    let orig_id = trk.id.clone();
    {
        let mut writer = state.get_lock().await;
        writer
            .upsert_scanned_tracks_batch(&imported, false)
            .await
            .unwrap();

        // Level-1 FastScan flags `is_fake_320kbps = true`
        let mut lvl1_upd = ScannerRowUpdate::failure(orig_id.clone(), Some(242), 1);
        lvl1_upd.is_fake_320kbps = true;
        apply_scanner_batch_updates(&mut writer.connection, &[lvl1_upd])
            .await
            .unwrap();
        assert_eq!(
            writer
                .get_track(&orig_id)
                .await
                .unwrap()
                .unwrap()
                .is_fake_320kbps,
            Some(true)
        );

        let full_res = hera_node_rs::scanner::full_pass::run_full_pass_from_pcm(
            &vec![0.25_f32; 8_000 * 10],
            8_000,
            None,
        );
        let (mut upd, _) = ScannerRowUpdate::from_full_pass(orig_id.clone(), full_res, Some(242));
        upd.bpm = Some(94.5);
        upd.camelot = Some("7A".to_string());
        upd.replay_gain_db = Some(-3.4);
        upd.loudness_lufs = Some(-7.6);
        upd.true_peak_dbfs = Some(-0.4);
        upd.waveform_peaks_b64 = Some("AAECAw==".to_string());
        upd.embedding_64d_b64 = Some("AQIDBA==".to_string());
        upd.has_intro_vocals = Some(true);
        upd.has_outro_vocals = Some(false);
        upd.has_audio_skips = Some(true);
        upd.glitch_details = Some("Skip at 42.10s".to_string());
        upd.hot_cues_json = Some(
            serde_json::to_string(&vec![crate::libs::track::HotCuePin {
                num: 5,
                name: "5:Auto".to_string(),
                seconds: 140.0,
                color: "#a855f7".to_string(),
                role: "Phrase".to_string(),
                origin: Some("hera_auto".to_string()),
            }])
            .unwrap(),
        );

        apply_scanner_batch_updates(&mut writer.connection, &[upd])
            .await
            .unwrap();

        let reloaded = writer.get_track(&orig_id).await.unwrap().unwrap();
        assert_eq!(
            reloaded.is_fake_320kbps,
            Some(true),
            "sticky is_fake_320kbps flag from Level-1 must survive Level-2 FullPass update"
        );
        let reloaded_cues = reloaded.hot_cues.unwrap();
        assert_eq!(reloaded_cues.len(), 8);
        let reloaded_c5 = reloaded_cues.iter().find(|c| c.num == 5).unwrap();
        assert!((reloaded_c5.seconds - 128.4).abs() < 1e-3);
        assert_eq!(reloaded_c5.origin.as_deref(), Some("dj_manual"));
        assert_eq!(reloaded.has_bpm_trust_diff, Some(true));
        assert_eq!(reloaded.has_audio_skips, Some(true));
        assert_eq!(reloaded.replay_gain_db, Some(-3.4));
        assert_eq!(reloaded.waveform_peaks_b64.as_deref(), Some("AAECAw=="));

        let doctor_kinds: Vec<String> = sqlx::query_scalar(
            "SELECT issue_kind FROM library_doctor_issues WHERE track_id = ? ORDER BY issue_kind",
        )
        .bind(&orig_id)
        .fetch_all(&mut writer.connection)
        .await
        .unwrap();
        assert_eq!(
            doctor_kinds,
            vec![
                "AUDIO_GLITCH_SKIP".to_string(),
                "BPM_TRUST_DIFF".to_string()
            ]
        );

        // Move file from SetA -> SetB and re-scan: original track ID must be healed & preserved!
        let track_path_b = folder_b.join("Tony Vega - Aparentemente.wav");
        std::fs::rename(&track_path_a, &track_path_b).unwrap();
        let moved_scan = import_tracks_with_index(std::slice::from_ref(&track_path_b), &index);
        writer
            .upsert_scanned_tracks_batch(&moved_scan, false)
            .await
            .unwrap();

        let healed = writer.get_track(&orig_id).await.unwrap().unwrap();
        assert_eq!(healed.path, track_path_b.to_string_lossy());
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_discover_workable_roots_from_index_groups_counts_and_ignores_junk() {
    let mut index = ExternalDjSoftwareIndex::default();
    let entries = [
        ("D:/Musica/Salsa/Grupo Niche - Cali.mp3", Some(245.0)),
        ("D:/Musica/Reggaeton/Feid - Luna.flac", Some(192.0)),
        ("D:/Musica/Videos/Karol G - Si Antes.mp4", Some(218.0)),
        ("D:/Musica/Mixes/Set Viernes Club.mp3", Some(780.0)),
        ("D:/Musica/SFX/Airhorn.wav", Some(4.5)),
        ("D:/$RECYCLE.BIN/deleted_song.mp3", Some(210.0)),
    ];
    for (path, dur) in entries {
        index.vdj_entries.insert(
            path.to_lowercase(),
            CachedDjSoftwareMetadata {
                bpm: Some(124.0),
                key: Some("8A".to_string()),
                cues_count: 4,
                author: Some("Artist".to_string()),
                title: Some("Title".to_string()),
                duration_sec: dur,
                bitrate_kbps: Some(320),
                cues: Vec::new(),
                original_path: Some(path.to_string()),
            },
        );
    }

    let roots = discover_workable_roots_from_index(&index, false);
    assert_eq!(roots.len(), 1);
    let r = &roots[0];
    assert_eq!(r.root_path, "D:/Musica");
    assert_eq!(r.source_db, "VirtualDJ");
    assert_eq!(r.workable_tracks_count, 3);
    assert_eq!(r.mixes_over_10m_count, 1);
    assert_eq!(r.skipped_short_sfx_count, 1);
    assert_eq!(r.video_tracks_count, 1);

    // 2-segment external USB root paths (`/Volumes/USB_STICK/track.mp3` vs `/Volumes/BACKUP_USB/track.mp3`)
    // must anchor at `/Volumes/<Volume>` instead of collapsing two distinct drives into `/Volumes`!
    let mut usb_index = ExternalDjSoftwareIndex::default();
    for (p, dur) in [
        ("/Volumes/USB_STICK/club_opener.mp3", Some(210.0)),
        ("/Volumes/BACKUP_USB/other_drive.mp3", Some(225.0)),
    ] {
        usb_index.vdj_entries.insert(
            p.to_lowercase(),
            CachedDjSoftwareMetadata {
                duration_sec: dur,
                original_path: Some(p.to_string()),
                ..Default::default()
            },
        );
    }
    let usb_roots = discover_workable_roots_from_index(&usb_index, false);
    let usb_paths: Vec<&str> = usb_roots.iter().map(|x| x.root_path.as_str()).collect();
    assert_eq!(usb_paths, vec!["/Volumes/BACKUP_USB", "/Volumes/USB_STICK"]);
}
