use hera_node_rs::dj_brain::{compute_dj_style_dna, DjStyleDnaRequest};
use hera_node_rs::export::ExportTrackItem;
use hera_node_rs::forensics::audit_pcm_channels;
use hera_node_rs::mcp::list_mcp_tools;
use hera_node_rs::structure::{analyze_structure_and_style, HotCuePoint};
use hera_node_rs::synergy::{
    optimize_crate_2opt, optimize_crate_with_spine, CrateTrackCandidate, SetSpineConfig,
    SpineAnchor,
};
use serde_json::{json, Number, Value};
use std::f64::consts::TAU;

/// Recursively rounds floating-point JSON numbers (`f32`/`f64`) to 4 decimal places
/// while preserving integer, boolean, string, and structural fields verbatim.
fn normalize_json_floats(value: Value) -> Value {
    match value {
        Value::Number(num) => {
            if num.is_i64() || num.is_u64() {
                Value::Number(num)
            } else if let Some(v) = num.as_f64() {
                let rounded = (v * 10_000.0).round() / 10_000.0;
                let clean = if rounded == 0.0 { 0.0 } else { rounded };
                Number::from_f64(clean)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            } else {
                Value::Number(num)
            }
        }
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_json_floats).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, normalize_json_floats(v)))
                .collect(),
        ),
        other => other,
    }
}

/// Phase-wrapped sine helper (`[0, 1)` fractional turn) to ensure IEEE-754 stability across OS libm implementations.
#[inline]
fn phase_sin(freq_hz: f64, time_sec: f64) -> f64 {
    (TAU * (freq_hz * time_sec).fract()).sin()
}

/// Deterministic 4-cue fixture helper for crate candidates and library items.
fn sample_hot_cues(intro_s: f32, breakdown_s: f32, drop_s: f32, outro_s: f32) -> Vec<HotCuePoint> {
    vec![
        HotCuePoint {
            num: 1,
            name: "1.1.1 Downbeat".to_string(),
            seconds: intro_s,
            bar_index: 0,
            role: "downbeat".to_string(),
            confidence: 0.96,
        },
        HotCuePoint {
            num: 2,
            name: "Phrase In (8b)".to_string(),
            seconds: breakdown_s,
            bar_index: 8,
            role: "phrase_in".to_string(),
            confidence: 0.90,
        },
        HotCuePoint {
            num: 3,
            name: "Main Drop (16b)".to_string(),
            seconds: drop_s,
            bar_index: 16,
            role: "drop".to_string(),
            confidence: 0.94,
        },
        HotCuePoint {
            num: 4,
            name: "Mix Out (32b)".to_string(),
            seconds: outro_s,
            bar_index: 32,
            role: "mix_out".to_string(),
            confidence: 0.91,
        },
    ]
}

/// Deterministic L2-normalized 64D style embedding parameterized by harmonic seed index.
fn deterministic_embedding_64d(seed_idx: usize) -> Vec<f32> {
    let mut raw = Vec::with_capacity(64);
    for dim in 0..64 {
        let angle = ((seed_idx + 1) as f64) * 0.37 + (dim as f64) * 0.21;
        let val = (0.55 + 0.35 * angle.sin() + 0.10 * (angle * 2.0).cos()) as f32;
        raw.push(val);
    }
    let norm = raw.iter().map(|&x| x * x).sum::<f32>().sqrt().max(1e-9);
    raw.into_iter()
        .map(|x| (((x / norm) as f64 * 10_000.0).round() / 10_000.0) as f32)
        .collect()
}

#[test]
fn golden_mcp_21_tools_catalog() {
    let catalog = list_mcp_tools();
    let tools = catalog["tools"]
        .as_array()
        .expect("MCP catalog must expose a 'tools' array");
    // Empirical characterization: list_mcp_tools() exposes 22 entries (Step 0 ensure_hera_updated + 21 domain tools)
    assert_eq!(
        tools.len(),
        22,
        "HERA MCP catalog must expose all 22 registered tools"
    );

    insta::assert_json_snapshot!("mcp_21_tools_catalog", catalog);
}

#[test]
fn golden_forensics_and_4_hot_cues() {
    let sample_rate: u32 = 44_100;
    let duration_sec: f32 = 36.0;
    let total_frames: usize = (sample_rate as usize) * 36; // 1_587_600 frames
    let bpm: f64 = 124.0;
    let beat_sec: f64 = 60.0 / bpm;

    let mut left = Vec::with_capacity(total_frames);
    let mut right = Vec::with_capacity(total_frames);
    let mut mono = Vec::with_capacity(total_frames);

    // Deterministic 64-bit LCG for airy broadband studio percussion / hi-hat transients
    let mut lcg_state: u64 = 0x5EED_CAFE_8A12_4001;

    for i in 0..total_frames {
        let t = (i as f64) / (sample_rate as f64);
        let beat_pos = (t / beat_sec).fract();
        let offbeat_pos = ((t / beat_sec) + 0.5).fract();

        lcg_state = lcg_state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let noise_l = (((lcg_state >> 32) as u32 as f64) / (u32::MAX as f64)) * 2.0 - 1.0;

        lcg_state = lcg_state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let noise_r = (((lcg_state >> 32) as u32 as f64) / (u32::MAX as f64)) * 2.0 - 1.0;

        // Sharp punchy kick envelope on every quarter-note beat
        let kick_env = if beat_pos < 0.22 {
            let x = 1.0 - (beat_pos / 0.22);
            x * x * x
        } else {
            0.0
        };

        // Crisp 16th/offbeat hi-hat transient envelope
        let hat_env = if offbeat_pos < 0.14 {
            let x = 1.0 - (offbeat_pos / 0.14);
            x * x
        } else {
            0.03
        };

        // A-minor (8A) chordal & harmonic synth voices
        let a_minor_pad = 0.36 * phase_sin(220.00, t)
            + 0.28 * phase_sin(261.63, t)
            + 0.22 * phase_sin(329.63, t)
            + 0.14 * phase_sin(440.00, t);

        // High-frequency studio sheen harmonics extending cleanly up to 21 kHz
        let hf_sheen = 0.28 * phase_sin(6_592.6, t)
            + 0.24 * phase_sin(11_880.0, t)
            + 0.22 * phase_sin(16_720.0, t)
            + 0.20 * phase_sin(20_460.0, t);

        let (mono_core, stereo_air_gain) = if t < 8.0 {
            // 0.0..8.0s (Intro): gentle 124 BPM four-on-the-floor kick pulse + light hi-hat shimmer
            let kick = (0.30 * phase_sin(55.0, t) + 0.10 * phase_sin(110.0, t)) * kick_env;
            let shimmer = 0.04 * hf_sheen * hat_env;
            (kick + shimmer, 0.035 * hat_env)
        } else if t < 16.0 {
            // 8.0..16.0s (Breakdown): warm A-minor (8A) chordal harmonic pad without heavy sub kick
            let pad = 0.34 * a_minor_pad + 0.06 * phase_sin(880.0, t);
            let air = 0.03 * hf_sheen;
            (pad + air, 0.025)
        } else if t < 28.0 {
            // 16.0..28.0s (Main Drop): strong 124 BPM mono-compatible sub kick + A-minor synth lead + broadband air up to 21 kHz
            let sub_kick = (0.54 * phase_sin(55.0, t) + 0.20 * phase_sin(110.0, t)) * kick_env;
            let lead = (0.22 * a_minor_pad + 0.10 * phase_sin(659.25, t)) * (0.65 + 0.35 * hat_env);
            let sheen = 0.08 * hf_sheen * (0.45 + 0.55 * hat_env);
            (sub_kick + lead + sheen, 0.095 * hat_env)
        } else {
            // 28.0..36.0s (Outro): stripped percussion mix-out tail
            let kick = 0.26 * phase_sin(55.0, t) * kick_env;
            let tail_hat = 0.03 * hf_sheen * hat_env;
            (kick + tail_hat, 0.03 * hat_env)
        };

        // Sub-bass (< 120 Hz) remains 100% phase-locked across L/R while broadband airy hats add subtle stereo width
        let sample_l = (mono_core + stereo_air_gain * (0.75 * noise_l + 0.25 * noise_r))
            .clamp(-0.92, 0.92) as f32;
        let sample_r = (mono_core + stereo_air_gain * (0.25 * noise_l + 0.75 * noise_r))
            .clamp(-0.92, 0.92) as f32;

        left.push(sample_l);
        right.push(sample_r);
        mono.push(0.5 * (sample_l + sample_r));
    }

    let forensics = audit_pcm_channels(&left, Some(&right), sample_rate, "flac");
    let mut structure =
        analyze_structure_and_style(&mono, sample_rate, Some(duration_sec), Some(124.0));

    assert!(
        forensics.is_authentic_lossless,
        "Synthesized studio club master must pass authentic lossless check: {forensics:?}"
    );
    assert!(
        forensics.effective_bandwidth_hz >= 19_500.0,
        "Expected broadband bandwidth >= 19.5 kHz, got {}",
        forensics.effective_bandwidth_hz
    );
    assert!(
        forensics.mono_compatible_sub,
        "Sub-bass must be mono-compatible for club subwoofers"
    );
    assert_eq!(structure.hot_cues.len(), 4);
    let cue_nums: Vec<u8> = structure.hot_cues.iter().map(|c| c.num).collect();
    assert_eq!(cue_nums, vec![1, 2, 3, 4]);

    for v in &mut structure.embedding_64d {
        *v = (((*v as f64) * 10_000.0).round() / 10_000.0) as f32;
    }

    let combined = json!({
        "synthetic_fixture": "library/verified/afro_sunrise.flac",
        "forensics": forensics,
        "structure": structure,
    });
    let normalized = normalize_json_floats(combined);

    insta::assert_json_snapshot!("forensics_and_4_hot_cues", normalized);
}

#[test]
fn golden_dj_style_dna_signature() {
    let tracks = vec![
        ExportTrackItem {
            track_id: "dna_01".to_string(),
            artist: "Keinemusik".to_string(),
            title: "Afro Sunrise".to_string(),
            version: Some("Extended Club Mix".to_string()),
            file_path: "library/verified/afro_sunrise.flac".to_string(),
            bpm: 123.0,
            camelot: "8A".to_string(),
            duration_seconds: 372.0,
            bitrate_kbps: 1411,
            energy: 0.76,
            file_size_bytes: 42_500_000,
            is_video: false,
            is_own_production: false,
            hot_cues: sample_hot_cues(0.5, 31.0, 62.0, 310.0),
        },
        ExportTrackItem {
            track_id: "dna_02".to_string(),
            artist: "Keinemusik".to_string(),
            title: "Brisa del Magdalena".to_string(),
            version: Some("Original Mix".to_string()),
            file_path: "library/verified/brisa_del_magdalena.flac".to_string(),
            bpm: 124.0,
            camelot: "8A".to_string(),
            duration_seconds: 356.0,
            bitrate_kbps: 1411,
            energy: 0.80,
            file_size_bytes: 40_800_000,
            is_video: false,
            is_own_production: false,
            hot_cues: sample_hot_cues(0.4, 30.8, 61.6, 298.0),
        },
        ExportTrackItem {
            track_id: "dna_03".to_string(),
            artist: "Hugel".to_string(),
            title: "Tambores Latin Tech".to_string(),
            version: Some("Club Dub".to_string()),
            file_path: "library/verified/tambores_latin_tech.flac".to_string(),
            bpm: 126.0,
            camelot: "9A".to_string(),
            duration_seconds: 318.0,
            bitrate_kbps: 1411,
            energy: 0.86,
            file_size_bytes: 36_400_000,
            is_video: false,
            is_own_production: false,
            hot_cues: sample_hot_cues(0.2, 29.5, 59.0, 268.0),
        },
        ExportTrackItem {
            track_id: "dna_04".to_string(),
            artist: "Hugel".to_string(),
            title: "Cartagena Sunset Video Edit".to_string(),
            version: Some("VJ Club Cut".to_string()),
            file_path: "library/verified/cartagena_sunset_video.flac".to_string(),
            bpm: 125.0,
            camelot: "8B".to_string(),
            duration_seconds: 304.0,
            bitrate_kbps: 1411,
            energy: 0.84,
            file_size_bytes: 58_200_000,
            is_video: true,
            is_own_production: false,
            hot_cues: sample_hot_cues(0.3, 30.0, 60.0, 256.0),
        },
        ExportTrackItem {
            track_id: "dna_05".to_string(),
            artist: "Gordo".to_string(),
            title: "Noche de Sabana".to_string(),
            version: Some("Extended".to_string()),
            file_path: "library/verified/noche_de_sabana.flac".to_string(),
            bpm: 127.0,
            camelot: "7A".to_string(),
            duration_seconds: 340.0,
            bitrate_kbps: 1411,
            energy: 0.88,
            file_size_bytes: 39_100_000,
            is_video: false,
            is_own_production: false,
            hot_cues: sample_hot_cues(0.2, 28.8, 57.6, 290.0),
        },
        ExportTrackItem {
            track_id: "dna_06".to_string(),
            artist: "DaviDJ".to_string(),
            title: "Guaracha Estelar VIP".to_string(),
            version: Some("DaviDJ VIP Dubplate".to_string()),
            file_path: "library/verified/guaracha_estelar_vip.flac".to_string(),
            bpm: 130.0,
            camelot: "10A".to_string(),
            duration_seconds: 292.0,
            bitrate_kbps: 1411,
            energy: 0.95,
            file_size_bytes: 34_900_000,
            is_video: false,
            is_own_production: true,
            hot_cues: sample_hot_cues(0.1, 27.7, 55.4, 248.0),
        },
    ];

    let req = DjStyleDnaRequest {
        tracks,
        dj_handle: Some("DaviDJ".to_string()),
        preferred_dj_software: Some("VirtualDJ".to_string()),
        golden_rules: Some("Priorizar vocales limpias y puentes armónicos Camelot".to_string()),
    };

    let profile = compute_dj_style_dna(&req);
    let serialized = serde_json::to_value(&profile).expect("DjStyleDnaProfile must serialize");
    let normalized = normalize_json_floats(serialized);

    insta::assert_json_snapshot!("dj_style_dna_signature", normalized);
}

#[test]
fn golden_2opt_tsp_and_set_spine_routes() {
    // Intentionally provided in non-sequential order so 2-Opt TSP actively optimizes the route
    let candidates = vec![
        CrateTrackCandidate {
            track_id: "trk_04".to_string(),
            title: "Latin Tech Horizon".to_string(),
            artist: "Hugel".to_string(),
            file_path: "library/verified/trk_04_latin_tech_horizon.flac".to_string(),
            duration_seconds: 324.0,
            bpm: 125.0,
            camelot_key: "9A".to_string(),
            energy_score: 0.84,
            punch_index: 0.78,
            groove_type: "four_on_floor".to_string(),
            top_subgenre: "Latin Tech".to_string(),
            has_intro_vocals: false,
            has_outro_vocals: true,
            embedding_64d: deterministic_embedding_64d(3),
            hot_cues: sample_hot_cues(0.2, 30.7, 61.4, 276.0),
        },
        CrateTrackCandidate {
            track_id: "trk_01".to_string(),
            title: "Deep Pacific Warmup".to_string(),
            artist: "Keinemusik".to_string(),
            file_path: "library/verified/trk_01_deep_pacific_warmup.flac".to_string(),
            duration_seconds: 368.0,
            bpm: 122.0,
            camelot_key: "7A".to_string(),
            energy_score: 0.58,
            punch_index: 0.56,
            groove_type: "afro_syncopated".to_string(),
            top_subgenre: "Afro House".to_string(),
            has_intro_vocals: false,
            has_outro_vocals: false,
            embedding_64d: deterministic_embedding_64d(0),
            hot_cues: sample_hot_cues(0.4, 31.5, 63.0, 314.0),
        },
        CrateTrackCandidate {
            track_id: "trk_06".to_string(),
            title: "Medellin Peak Finale".to_string(),
            artist: "DaviDJ".to_string(),
            file_path: "library/verified/trk_06_medellin_peak_finale.flac".to_string(),
            duration_seconds: 298.0,
            bpm: 128.0,
            camelot_key: "10A".to_string(),
            energy_score: 0.96,
            punch_index: 0.90,
            groove_type: "four_on_floor".to_string(),
            top_subgenre: "Guaracha / Tribal".to_string(),
            has_intro_vocals: false,
            has_outro_vocals: false,
            embedding_64d: deterministic_embedding_64d(5),
            hot_cues: sample_hot_cues(0.1, 28.1, 56.2, 252.0),
        },
        CrateTrackCandidate {
            track_id: "trk_02".to_string(),
            title: "Afro Sunrise Groove".to_string(),
            artist: "Rampa".to_string(),
            file_path: "library/verified/trk_02_afro_sunrise_groove.flac".to_string(),
            duration_seconds: 352.0,
            bpm: 123.0,
            camelot_key: "8A".to_string(),
            energy_score: 0.68,
            punch_index: 0.64,
            groove_type: "afro_syncopated".to_string(),
            top_subgenre: "Afro House".to_string(),
            has_intro_vocals: true,
            has_outro_vocals: false,
            embedding_64d: deterministic_embedding_64d(1),
            hot_cues: sample_hot_cues(0.3, 31.2, 62.4, 300.0),
        },
        CrateTrackCandidate {
            track_id: "trk_05".to_string(),
            title: "Euphoric Terrace Lift".to_string(),
            artist: "Mochakk".to_string(),
            file_path: "library/verified/trk_05_euphoric_terrace_lift.flac".to_string(),
            duration_seconds: 312.0,
            bpm: 126.5,
            camelot_key: "9B".to_string(),
            energy_score: 0.90,
            punch_index: 0.83,
            groove_type: "four_on_floor".to_string(),
            top_subgenre: "Tech House".to_string(),
            has_intro_vocals: false,
            has_outro_vocals: false,
            embedding_64d: deterministic_embedding_64d(4),
            hot_cues: sample_hot_cues(0.2, 29.8, 59.6, 264.0),
        },
        CrateTrackCandidate {
            track_id: "trk_03".to_string(),
            title: "Golden Chord Bridge".to_string(),
            artist: "Adam Port".to_string(),
            file_path: "library/verified/trk_03_golden_chord_bridge.flac".to_string(),
            duration_seconds: 342.0,
            bpm: 124.0,
            camelot_key: "8B".to_string(),
            energy_score: 0.75,
            punch_index: 0.70,
            groove_type: "four_on_floor".to_string(),
            top_subgenre: "Melodic Afro".to_string(),
            has_intro_vocals: false,
            has_outro_vocals: false,
            embedding_64d: deterministic_embedding_64d(2),
            hot_cues: sample_hot_cues(0.3, 30.9, 61.9, 290.0),
        },
    ];

    let route_2opt = optimize_crate_2opt(
        "Club Journey Peak Set",
        "club_journey",
        candidates.clone(),
        None,
    );

    let spine_cfg = SetSpineConfig {
        anchors: vec![
            SpineAnchor {
                track_id: "trk_01".to_string(),
                role: Some("opener".to_string()),
                slot: None,
                position: None,
            },
            SpineAnchor {
                track_id: "trk_06".to_string(),
                role: Some("peak".to_string()),
                slot: None,
                position: None,
            },
            SpineAnchor {
                track_id: "trk_03".to_string(),
                role: Some("closer".to_string()),
                slot: None,
                position: None,
            },
        ],
        custom_energy_curve: vec![0.58, 0.72, 0.85, 0.96, 0.76],
    };

    let route_spine = optimize_crate_with_spine(
        "Pinned Spine Set",
        "club_journey",
        candidates,
        Some(5),
        &spine_cfg,
    );

    let combined = json!({
        "two_opt_unconstrained": route_2opt,
        "pinned_spine_constrained": route_spine,
    });
    let normalized = normalize_json_floats(combined);

    insta::assert_json_snapshot!("curator_2opt_tsp_and_spine_routes", normalized);
}
