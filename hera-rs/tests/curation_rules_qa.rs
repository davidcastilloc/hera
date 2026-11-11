use hera_node_rs::curation_rules::{DjCurationRuleSet, GoldenTransitionPair};
use hera_node_rs::structure::HotCuePoint;
use hera_node_rs::synergy::{
    optimize_crate_with_dj_rules, CrateTrackCandidate, SetSpineConfig, SpineAnchor,
};

fn sample_hot_cues() -> Vec<HotCuePoint> {
    vec![
        HotCuePoint {
            num: 1,
            name: "1.1.1 Downbeat".to_string(),
            seconds: 0.2,
            bar_index: 0,
            role: "downbeat".to_string(),
            confidence: 0.98,
        },
        HotCuePoint {
            num: 2,
            name: "Phrase In (8b)".to_string(),
            seconds: 15.0,
            bar_index: 8,
            role: "breakdown".to_string(),
            confidence: 0.92,
        },
    ]
}

fn build_track(
    id: &str,
    title: &str,
    artist: &str,
    file_path: &str,
    duration: f32,
    bpm: f32,
    key: &str,
    energy: f32,
) -> CrateTrackCandidate {
    CrateTrackCandidate {
        track_id: id.to_string(),
        title: title.to_string(),
        artist: artist.to_string(),
        file_path: file_path.to_string(),
        duration_seconds: duration,
        bpm,
        camelot_key: key.to_string(),
        energy_score: energy,
        punch_index: 0.70,
        groove_type: "four_on_floor".to_string(),
        top_subgenre: "Latin / House".to_string(),
        has_intro_vocals: false,
        has_outro_vocals: false,
        embedding_64d: vec![0.1; 64],
        hot_cues: sample_hot_cues(),
    }
}

#[test]
fn test_curation_rules_routes_over_10m_mixes_without_disk_moves() {
    let candidates = vec![
        build_track("trk_01", "Perdóname mi vida", "Grupo Niche", "music/niche.mp3", 245.0, 100.0, "8A", 0.65),
        build_track("trk_02", "Conquístame", "Los Titanes", "music/titanes.mp3", 280.0, 100.0, "8A", 0.70),
        // Live set / session: 15 minutes = 900 seconds > 600s
        build_track("mix_01", "Live Salsa Festival 2026 Session", "DJ Live", "mixes/fest_session.mp3", 900.0, 100.0, "8A", 0.80),
        build_track("trk_03", "Cali Pachanguero", "Grupo Niche", "music/cali.mp3", 310.0, 102.0, "9A", 0.85),
    ];

    let rules = DjCurationRuleSet::default();
    let spine = SetSpineConfig::default();

    let result = optimize_crate_with_dj_rules("Latin Crate", "steady_climb", candidates, None, &spine, &rules);

    assert_eq!(result.routed_to_mixes_over_10m, vec!["mix_01"]);
    assert_eq!(result.base_result.total_tracks, 3);
    assert!(!result.base_result.sequence.iter().any(|t| t.track_id == "mix_01"));
}

#[test]
fn test_curation_rules_excludes_blacklisted_and_title_patterns() {
    let candidates = vec![
        build_track("trk_01", "Perdóname mi vida", "Grupo Niche", "music/niche.mp3", 245.0, 100.0, "8A", 0.65),
        build_track("trk_02", "Clavelito ya no", "Orquesta Sonora", "music/clavelito.mp3", 210.0, 98.0, "7A", 0.60),
        build_track("trk_03", "Conquístame", "Los Titanes", "music/titanes.mp3", 280.0, 100.0, "8A", 0.70),
        build_track("trk_04", "Blacklisted ID Track", "Artist X", "music/x.mp3", 220.0, 100.0, "8A", 0.65),
    ];

    let rules = DjCurationRuleSet {
        excluded_track_ids: vec!["trk_04".to_string()],
        excluded_title_patterns: vec!["Clavelito".to_string()],
        ..Default::default()
    };
    let spine = SetSpineConfig::default();

    let result = optimize_crate_with_dj_rules("Latin Crate", "steady_climb", candidates, None, &spine, &rules);

    assert_eq!(result.excluded_tracks.len(), 2);
    assert!(result.excluded_tracks.contains(&"trk_02".to_string()));
    assert!(result.excluded_tracks.contains(&"trk_04".to_string()));
    assert_eq!(result.base_result.total_tracks, 2);
    assert_eq!(result.base_result.sequence[0].track_id, "trk_01");
    assert_eq!(result.base_result.sequence[1].track_id, "trk_03");
}

#[test]
fn test_curation_rules_locks_golden_transition_pair() {
    let candidates = vec![
        build_track("trk_01", "Perdóname mi vida", "Grupo Niche", "music/niche.mp3", 245.0, 100.0, "8A", 0.65),
        build_track("trk_02", "Conquístame", "Los Titanes", "music/titanes.mp3", 280.0, 100.0, "8A", 0.70),
        build_track("trk_03", "Una Aventura", "Grupo Niche", "music/aventura.mp3", 290.0, 101.0, "8A", 0.75),
    ];

    let golden_pair = GoldenTransitionPair {
        from_track_id: "trk_01".to_string(),
        to_track_id: "trk_02".to_string(),
        custom_bonus: Some(0.45),
        note: Some("Salsa Romántica Classic Bridge".to_string()),
    };

    let rules = DjCurationRuleSet {
        golden_pairs: vec![golden_pair],
        ..Default::default()
    };

    let spine = SetSpineConfig {
        anchors: vec![SpineAnchor {
            track_id: "trk_01".to_string(),
            role: Some("opener".to_string()),
            slot: Some(0),
            position: None,
        }],
        custom_energy_curve: vec![],
    };

    let result = optimize_crate_with_dj_rules("Romantic Set", "steady_climb", candidates, None, &spine, &rules);

    assert_eq!(result.base_result.sequence[0].track_id, "trk_01");
    assert_eq!(result.base_result.sequence[1].track_id, "trk_02");
    assert_eq!(result.golden_pairs_locked, 1);
}

#[test]
fn test_curation_rules_supports_mixed_audio_and_video() {
    let candidates = vec![
        build_track("trk_01", "Perdóname mi vida", "Grupo Niche", "videos/niche_official.mp4", 245.0, 100.0, "8A", 0.65),
        build_track("trk_02", "Conquístame", "Los Titanes", "music/titanes_master.flac", 280.0, 100.0, "8A", 0.70),
        build_track("trk_03", "Rebellion Live HD", "Joe Arroyo", "videos/rebellion.mkv", 320.0, 105.0, "9A", 0.85),
    ];

    let rules = DjCurationRuleSet::default();
    let spine = SetSpineConfig::default();

    let result = optimize_crate_with_dj_rules("Mixed Video Crate", "steady_climb", candidates, None, &spine, &rules);

    assert_eq!(result.base_result.total_tracks, 3);
    assert_eq!(result.video_tracks_count, 2); // .mp4 and .mkv
}
