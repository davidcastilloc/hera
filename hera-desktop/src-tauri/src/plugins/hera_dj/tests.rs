use super::*;

#[test]
fn test_parse_virtualdj_history_tracklist_and_extinf() {
    let vdj_txt = "21:05 : Avicii - Levels\n21:11 : Eric Prydz - Opus\n";
    let parsed = parse_virtualdj_history_content(vdj_txt);
    assert_eq!(parsed, Some(("Eric Prydz".to_string(), "Opus".to_string())));

    let m3u_txt = "#EXTM3U\n#EXTINF:412,Keinemusik — Cesonia\nlibrary/Cesonia.wav\n";
    let parsed_m3u = parse_virtualdj_history_content(m3u_txt);
    assert_eq!(
        parsed_m3u,
        Some(("Keinemusik".to_string(), "Cesonia".to_string()))
    );

    assert_eq!(parse_virtualdj_history_content("   \n#EXTM3U\n"), None);
}

#[test]
fn test_write_virtualdj_live_crate_generates_vdjfolder_with_hot_cues() {
    let temp_dir =
        std::env::temp_dir().join(format!("hera_vdj_overlay_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).expect("create temp vdj dir");

    let sample_track = ExportTrackItem {
        track_id: "vdj-live-01".to_string(),
        artist: "Keinemusik".to_string(),
        title: "Cesonia".to_string(),
        version: None,
        file_path: temp_dir.join("Cesonia.wav").to_string_lossy().to_string(),
        bpm: 125.0,
        camelot: "9A".to_string(),
        duration_seconds: 240.0,
        bitrate_kbps: 1411,
        energy: 0.84,
        file_size_bytes: 0,
        is_video: false,
        is_own_production: false,
        hot_cues: Vec::new(),
    };

    let written = write_virtualdj_live_crate_files(
        "HERA En Vivo",
        &[sample_track],
        std::slice::from_ref(&temp_dir),
    );
    assert!(
        written
            .iter()
            .any(|p| p.ends_with("HERA En Vivo.vdjfolder")),
        "Expected HERA En Vivo.vdjfolder in {:?}",
        written
    );
    let folder_xml =
        std::fs::read_to_string(temp_dir.join("HERA En Vivo.vdjfolder")).expect("read vdjfolder");
    assert!(folder_xml.contains("Cesonia"));
    let db_xml = std::fs::read_to_string(temp_dir.join("HERA En Vivo_vdj_database.xml"))
        .expect("read vdj database xml");
    assert!(db_xml.contains("<Poi "));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_tauri_v2_acl_triad_parity() {
    let report = hera_ipc::acl_guard::verify_tauri_v2_acl_triad(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
    .unwrap();
    assert_eq!(report.commands_checked, 63);
}

#[test]
fn test_native_audio_state_snapshot_and_telemetry_mapping() {
    use hera_node_rs::audio_engine::{AudioBackend, DeckId, HeadlessClockAudioBackend};

    let backend = HeadlessClockAudioBackend::new_deterministic(48_000);
    let snap = backend.snapshot();
    let mapped = audio_transport::map_player_snapshot(snap, backend.poll_latest_telemetry());
    assert_eq!(mapped.sample_rate, 48_000);
    assert!(!mapped.track_loaded);
    assert_eq!(mapped.hot_cue_frames.len(), 4);

    let after_vol = backend.set_master_volume(0.75);
    assert!((after_vol.volume - 0.75).abs() < 1e-6);
    let _ = DeckId::DeckA;
}

#[test]
fn test_workability_junk_folders_and_duration_classification() {
    use std::path::Path;
    use workability::{DurationCategory, classify_track_duration, is_junk_system_directory};

    // Junk folder filtering
    assert!(is_junk_system_directory(Path::new(
        "D:/$RECYCLE.BIN/deleted.mp3"
    )));
    assert!(is_junk_system_directory(Path::new(
        "/home/dj/Music/WhatsApp Audio/sample.mp3"
    )));
    assert!(is_junk_system_directory(Path::new(
        "C:/System Volume Information/track.wav"
    )));
    assert!(!is_junk_system_directory(Path::new(
        "/home/dj/Music/Salsa/Grupo Niche.mp3"
    )));

    // Duration classification
    assert_eq!(
        classify_track_duration(15.0),
        DurationCategory::ShortSfxOrDiscard
    );
    assert_eq!(
        classify_track_duration(240.0),
        DurationCategory::WorkableTrack
    );
    assert_eq!(classify_track_duration(900.0), DurationCategory::MixOver10m);
}

#[test]
fn test_format_now_playing_tray_tooltip() {
    use virtualdj_live::format_now_playing_tray_tooltip;

    let empty_tooltip = format_now_playing_tray_tooltip(None);
    assert_eq!(empty_tooltip, "HERA BETA · Cabina lista");

    let playing = VirtualDjNowPlaying {
        artist: "Joe Arroyo".to_string(),
        title: "La Rebelión".to_string(),
        source_path: "tracklist.txt".to_string(),
    };
    let playing_tooltip = format_now_playing_tray_tooltip(Some(&playing));
    assert_eq!(
        playing_tooltip,
        "HERA BETA · Sonando: Joe Arroyo - La Rebelión"
    );
}

#[test]
fn test_load_file_with_known_cues_skips_recompute_and_applies_gain() {
    use hera_node_rs::audio_engine::{
        AudioBackend, DeckId, HeadlessClockAudioBackend, TransportCommand,
    };

    // 1. ReplayGain dB clamping (-12..+6 dB) and unity fallback (1.0)
    assert!((audio_transport::replay_gain_db_to_linear(None) - 1.0).abs() < 1e-6);
    assert!((audio_transport::replay_gain_db_to_linear(Some(f32::NAN)) - 1.0).abs() < 1e-6);
    let plus_6 = audio_transport::replay_gain_db_to_linear(Some(14.0));
    assert!((plus_6 - 10.0_f32.powf(6.0 / 20.0)).abs() < 1e-5);
    let minus_12 = audio_transport::replay_gain_db_to_linear(Some(-24.0));
    assert!((minus_12 - 10.0_f32.powf(-12.0 / 20.0)).abs() < 1e-5);

    // 2. >= 4 cues maps first 4 deck pads cleanly (even when 8 cues are supplied)
    let eight_cues = vec![1.5, 16.0, 32.0, 64.0, 96.0, 128.0, 160.0, 192.0];
    assert_eq!(
        audio_transport::extract_deck_pad_cues(Some(eight_cues)),
        Some([1.5, 16.0, 32.0, 64.0])
    );
    assert_eq!(
        audio_transport::extract_deck_pad_cues(Some(vec![1.0, 2.0])),
        None
    );

    // 3. Apply SetGain onto headless AudioBackend and verify output amplitude scales & resets
    let backend = HeadlessClockAudioBackend::new_deterministic(8_000);
    // Build a minimal 0.1s 8kHz mono PCM WAV in memory
    let num_samples = 800_u32;
    let data_bytes = num_samples * 2;
    let mut wav = Vec::with_capacity(44 + data_bytes as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&8_000_u32.to_le_bytes());
    wav.extend_from_slice(&16_000_u32.to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    for _ in 0..num_samples {
        wav.extend_from_slice(&16_000_i16.to_le_bytes());
    }

    backend
        .load_bytes(DeckId::DeckA, &wav, Some("wav"))
        .unwrap();
    let atten = audio_transport::replay_gain_db_to_linear(Some(-12.0));
    backend.send_command(TransportCommand::SetGain(DeckId::DeckA, atten));
    backend.send_command(TransportCommand::Play(DeckId::DeckA));
    let quiet_frames = backend.render_deterministic_frames(64);
    let quiet_peak = quiet_frames
        .iter()
        .copied()
        .fold(0.0_f32, |a, b| a.max(b.abs()));

    // Reloading next track with None resets gain back to 1.0 unity
    backend
        .load_bytes(DeckId::DeckA, &wav, Some("wav"))
        .unwrap();
    let unity = audio_transport::replay_gain_db_to_linear(None);
    backend.send_command(TransportCommand::SetGain(DeckId::DeckA, unity));
    backend.send_command(TransportCommand::Play(DeckId::DeckA));
    let unity_frames = backend.render_deterministic_frames(64);
    let unity_peak = unity_frames
        .iter()
        .copied()
        .fold(0.0_f32, |a, b| a.max(b.abs()));

    assert!(
        unity_peak > quiet_peak * 2.5,
        "unity={unity_peak} quiet={quiet_peak}"
    );
}

#[tokio::test]
async fn test_scanner_quarantine_minus_one_persists_and_isolated_job_catches_panic() {
    let caught = scanner_governor_os::run_isolated_scan_job(|| -> u32 {
        panic!("simulated corrupt mp4 atom");
    });
    let err_msg = caught.unwrap_err();
    assert!(err_msg.contains("PANIC_IN_SCAN_WORKER"));
    assert!(err_msg.contains("simulated corrupt mp4 atom"));

    scanner_governor_os::lower_current_worker_thread_priority();

    let tmp_db =
        std::env::temp_dir().join(format!("hera_quarantine_test_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&tmp_db);
    let db_state = crate::plugins::db::setup_at_path(&tmp_db).await.unwrap();
    {
        let mut writer = db_state.get_lock().await;
        sqlx::query(
            "INSERT INTO tracks (id, path, title, album, album_artist, artists, genres, duration, scan_level)
             VALUES ('corrupt_1', '/tmp/corrupt.mp3', 'Bad', 'A', 'B', '[]', '[]', 180, 0),
                    ('pending_1', '/tmp/good.mp3', 'Good', 'A', 'B', '[]', '[]', 200, 0)",
        )
        .execute(&mut writer.connection)
        .await
        .unwrap();

        crate::libs::db_scanner_queries::apply_scanner_batch_updates(
            &mut writer.connection,
            &[crate::libs::db_scanner_queries::ScannerRowUpdate {
                track_id: "corrupt_1".to_string(),
                bpm: None,
                camelot: None,
                energy: None,
                hot_cues_json: None,
                waveform_peaks_blob: None,
                scan_level: -1,
                is_transition_candidate: false,
                is_fake_320kbps: false,
                duration_sec: None,
            }],
        )
        .await
        .unwrap();

        crate::libs::db_scanner_queries::record_doctor_issue(
            &mut writer.connection,
            "corrupt_1",
            "/tmp/corrupt.mp3",
            "CORRUPT_FILE",
            "error",
            &err_msg,
        )
        .await
        .unwrap();

        let pending =
            crate::libs::db_scanner_queries::load_pending_scanner_jobs(&mut writer.connection, 50)
                .await
                .unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, "pending_1");

        let (lvl,): (i64,) = sqlx::query_as("SELECT scan_level FROM tracks WHERE id = 'corrupt_1'")
            .fetch_one(&mut writer.connection)
            .await
            .unwrap();
        assert_eq!(lvl, -1);

        let (issue_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM library_doctor_issues WHERE track_id = 'corrupt_1' AND issue_kind = 'CORRUPT_FILE'",
        )
        .fetch_one(&mut writer.connection)
        .await
        .unwrap();
        assert_eq!(issue_count, 1);
    }
    drop(db_state);
    let _ = std::fs::remove_file(&tmp_db);
}
