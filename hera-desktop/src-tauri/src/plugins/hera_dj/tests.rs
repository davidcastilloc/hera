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
    assert_eq!(report.commands_checked, 57);
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
