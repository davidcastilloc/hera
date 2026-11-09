pub mod cloudflare_edge;
pub mod code_mode;
pub mod dj_brain;
pub mod error;
pub mod export;
pub mod forensics;
pub mod mcp;
pub mod p2p;
pub mod p2p_exchange;
pub mod router;
pub mod structure;
pub mod synergy;

#[cfg(feature = "cloudflare")]
use std::sync::OnceLock;
#[cfg(feature = "cloudflare")]
use tower_service::Service;
#[cfg(feature = "cloudflare")]
use worker::{event, Context, Env, HttpRequest, Result as WorkerResult};

#[cfg(feature = "cloudflare")]
static ISOLATE_MESH_STATE: OnceLock<p2p::P2PMeshState> = OnceLock::new();

/// Cloudflare Workers WASM Entrypoint (`npx wrangler deploy --features cloudflare --no-default-features`).
/// Uses the exact same `axum::Router` as the Native VPS binary, while persisting `P2PMeshState`
/// across warm isolate invocations.
#[cfg(feature = "cloudflare")]
#[event(fetch)]
pub async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> WorkerResult<http::Response<axum::body::Body>> {
    let state = ISOLATE_MESH_STATE
        .get_or_init(|| {
            let node_id = env
                .var("HERA_NODE_ID")
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "hera-cf-edge-node".to_string());
            p2p::P2PMeshState::new(node_id)
        })
        .clone();

    let mut router = router::build_hera_router(state);
    Ok(router.call(req).await?)
}

#[cfg(test)]
mod tests {
    use crate::forensics::{audit_pcm_channels, decode_audio_bytes};
    use crate::p2p::{P2PMeshState, P2PVerifiedTrack};
    use crate::router::{build_hera_router, AnnounceTrackRequest, BuildCrateRequest};
    use crate::structure::analyze_structure_and_style;
    use crate::synergy::{compute_bpm_synergy, optimize_crate_2opt, CrateTrackCandidate};
    use axum::body::Body;
    use http::{Request, StatusCode};
    use tower_service::Service;

    /// Helper to encode stereo 16-bit PCM into a valid RIFF WAV byte buffer in memory.
    fn encode_stereo_wav(left: &[f32], right: &[f32], sample_rate: u32) -> Vec<u8> {
        let n = left.len().min(right.len());
        let num_channels: u16 = 2;
        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * (num_channels as u32) * (bits_per_sample as u32 / 8);
        let block_align = num_channels * (bits_per_sample / 8);
        let data_size = (n as u32) * (block_align as u32);
        let riff_size = 36 + data_size;

        let mut buf = Vec::with_capacity((44 + data_size) as usize);
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&riff_size.to_le_bytes());
        buf.extend_from_slice(b"WAVEfmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
        buf.extend_from_slice(&num_channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_align.to_le_bytes());
        buf.extend_from_slice(&bits_per_sample.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());

        for i in 0..n {
            let l_i16 = (left[i].clamp(-1.0, 1.0) * 32767.0) as i16;
            let r_i16 = (right[i].clamp(-1.0, 1.0) * 32767.0) as i16;
            buf.extend_from_slice(&l_i16.to_le_bytes());
            buf.extend_from_slice(&r_i16.to_le_bytes());
        }
        buf
    }

    #[test]
    fn test_rust_forensics_rejects_synthetic_sine_and_phase_cancellation() {
        let sr = 44100u32;
        let n = sr as usize * 4;
        // 1. Pure 440 Hz sine wave -> must be rejected as synthetic tone
        let sine: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * (i as f32) / (sr as f32)).sin() * 0.8)
            .collect();
        let rep_sine = audit_pcm_channels(&sine, None, sr, "flac");
        assert!(rep_sine.is_synthetic_tone);
        assert_eq!(rep_sine.forensic_verdict, "REJECT_SYNTHETIC_TONE");
        assert_eq!(rep_sine.quality_score, 0.0);

        // 2. Inverted 60 Hz sub-bass between Left and Right -> must flag phase cancellation
        let mut left = Vec::with_capacity(n);
        let mut right = Vec::with_capacity(n);
        let mut rng: u64 = 0x123456789abcdef;
        for i in 0..n {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let noise = (((rng >> 32) as u32 as f32) / (u32::MAX as f32) - 0.5) * 0.35;
            let sub = (2.0 * std::f32::consts::PI * 60.0 * (i as f32) / (sr as f32)).sin() * 0.55;
            left.push(sub + noise);
            right.push(-sub + noise); // 180-degree out of phase sub!
        }
        let rep_phase = audit_pcm_channels(&left, Some(&right), sr, "flac");
        assert!(!rep_phase.mono_compatible_sub);
        assert!(rep_phase.sub_bass_phase_correlation < -0.5);
    }

    #[test]
    fn test_rust_structure_detects_downbeat_and_drop() {
        let sr = 22050u32;
        let bpm = 120.0_f32; // 1 beat = 0.5s, 1 bar = 2.0s
        let dur_sec = 48.0_f32; // 24 bars
        let n = (sr as f32 * dur_sec) as usize;
        let mut mono = vec![0.0_f32; n];

        // Silence for first 1.0s, then kicks starting at t = 1.0s
        // Breakdown from bar 12 to 16 (t=25s..33s), massive Main Drop at bar 16 (t=33s)
        for i in 0..n {
            let t = (i as f32) / (sr as f32);
            if t < 1.0 {
                continue;
            }
            let rel_t = t - 1.0;
            let bar = (rel_t / 2.0) as usize;
            let beat_phase = (rel_t % 0.5) / 0.5;
            let kick_env = if beat_phase < 0.18 {
                (1.0 - beat_phase / 0.18).powi(2)
            } else {
                0.0
            };

            let sub_gain = if (12..16).contains(&bar) {
                0.05 // Breakdown trough
            } else if bar >= 16 {
                0.95 // Main Drop surge!
            } else {
                0.45
            };

            let sub_wave = (2.0 * std::f32::consts::PI * 58.0 * t).sin() * kick_env * sub_gain;
            let mid_wave = if bar >= 4 && bar < 20 {
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.25
            } else {
                0.0
            };
            mono[i] = sub_wave + mid_wave;
        }

        let profile = analyze_structure_and_style(&mono, sr, Some(dur_sec), Some(bpm));
        assert!((profile.first_beat_seconds - 1.0).abs() < 0.15);
        assert_eq!(profile.hot_cues.len(), 4);
        assert!(profile.main_drop_seconds > profile.phrase_in_seconds);
        assert!(profile.mix_out_seconds > profile.main_drop_seconds);
    }

    #[test]
    fn test_rust_2opt_tsp_and_polyrhythmic_bridge() {
        let (score, _delta, is_poly, _note) = compute_bpm_synergy(96.0, 128.0);
        assert!(is_poly);
        assert!(score >= 0.85);

        let candidates = vec![
            CrateTrackCandidate {
                track_id: "t1".into(),
                title: "Peak Closer".into(),
                artist: "DJ A".into(),
                file_path: "/music/t1.flac".into(),
                duration_seconds: 240.0,
                bpm: 128.0,
                camelot_key: "10A".into(),
                energy_score: 0.94,
                punch_index: 0.85,
                groove_type: "four_on_floor".into(),
                top_subgenre: "Tech House".into(),
                has_intro_vocals: false,
                has_outro_vocals: false,
                embedding_64d: vec![0.125; 64],
                hot_cues: vec![],
            },
            CrateTrackCandidate {
                track_id: "t2".into(),
                title: "Warmup Opener".into(),
                artist: "DJ B".into(),
                file_path: "/music/t2.flac".into(),
                duration_seconds: 240.0,
                bpm: 122.0,
                camelot_key: "8A".into(),
                energy_score: 0.52,
                punch_index: 0.65,
                groove_type: "four_on_floor".into(),
                top_subgenre: "Afro House".into(),
                has_intro_vocals: false,
                has_outro_vocals: false,
                embedding_64d: vec![0.125; 64],
                hot_cues: vec![],
            },
            CrateTrackCandidate {
                track_id: "t3".into(),
                title: "Mid Builder".into(),
                artist: "DJ C".into(),
                file_path: "/music/t3.flac".into(),
                duration_seconds: 240.0,
                bpm: 125.0,
                camelot_key: "9A".into(),
                energy_score: 0.74,
                punch_index: 0.75,
                groove_type: "four_on_floor".into(),
                top_subgenre: "Tech House".into(),
                has_intro_vocals: false,
                has_outro_vocals: false,
                embedding_64d: vec![0.125; 64],
                hot_cues: vec![],
            },
        ];

        let res = optimize_crate_2opt("Club Ritual", "steady_climb", candidates, None);
        assert_eq!(res.total_tracks, 3);
        // Should order Warmup (8A, 122) -> Mid (9A, 125) -> Peak (10A, 128)
        assert_eq!(res.sequence[0].track_id, "t2");
        assert_eq!(res.sequence[1].track_id, "t3");
        assert_eq!(res.sequence[2].track_id, "t1");
        assert_eq!(res.harmonic_match_pct, 100.0);
        assert!(res.rekordbox_xml_export.contains("<DJ_PLAYLISTS"));
    }

    #[tokio::test]
    async fn test_universal_axum_router_endpoints_and_wav_upload() {
        let state = P2PMeshState::new("test-node-01");
        let mut app = build_hera_router(state);

        // 1. Test /health
        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.call(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 2. Test /api/v1/audio/analyze-file with real in-memory WAV bytes via Symphonia
        let sr = 44100u32;
        let n = sr as usize * 6;
        let mut left = Vec::with_capacity(n);
        let mut right = Vec::with_capacity(n);
        let mut rng: u64 = 987654321;
        for i in 0..n {
            let t = (i as f32) / (sr as f32);
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let airy_noise = (((rng >> 32) as u32 as f32) / (u32::MAX as f32) - 0.5) * 0.25;
            let kick = (2.0 * std::f32::consts::PI * 65.0 * t).sin()
                * ((1.0 - (t % 0.5) * 2.0).max(0.0))
                * 0.55;
            left.push(kick + airy_noise);
            right.push(kick + airy_noise);
        }
        let wav_bytes = encode_stereo_wav(&left, &right, sr);
        let decoded = decode_audio_bytes(&wav_bytes, Some("wav"), 10.0).unwrap();
        assert_eq!(decoded.sample_rate, 44100);

        let req_wav = Request::builder()
            .method("POST")
            .uri("/api/v1/audio/analyze-file?ext=wav&bpm_hint=120&track_id=wav-test-01")
            .body(Body::from(wav_bytes))
            .unwrap();
        let resp_wav = app.call(req_wav).await.unwrap();
        assert_eq!(resp_wav.status(), StatusCode::OK);

        // 3. Test Quarantine-First Gatekeeper on /api/v1/p2p/announce
        let fake_forensics = audit_pcm_channels(
            &vec![0.5; 44100 * 2], // DC / synthetic
            None,
            44100,
            "flac",
        );
        let announce_payload = AnnounceTrackRequest {
            track: P2PVerifiedTrack {
                track_id: "bad-track".into(),
                title: "Fake Sine".into(),
                artist: "Spammer".into(),
                bpm: 120.0,
                camelot_key: "8A".into(),
                top_subgenre: "Test".into(),
                duration_seconds: 10.0,
                format: "flac".into(),
                effective_bandwidth_hz: 440.0,
                sub_bass_phase_correlation: 1.0,
                mono_compatible_sub: true,
                quality_score: 0.0,
                forensic_verdict: "REJECT_SYNTHETIC_TONE".into(),
                hot_cues: vec![],
                owner_dj: "Spammer".into(),
                storage_provider: "google_drive".into(),
                gdrive_file_id: None,
                r2_object_key: None,
                stream_url: None,
                share_on_p2p: true,
                node_id: "test".into(),
                hera_seal: String::new(),
            },
            forensics: fake_forensics,
        };
        let req_ann = Request::builder()
            .method("POST")
            .uri("/api/v1/p2p/announce")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&announce_payload).unwrap()))
            .unwrap();
        let resp_ann = app.call(req_ann).await.unwrap();
        assert_eq!(resp_ann.status(), StatusCode::FORBIDDEN);

        // 4. Test /api/v1/crates/build
        let crate_req = BuildCrateRequest {
            brief: "Test Crate".into(),
            energy_arc: "steady_climb".into(),
            max_tracks: Some(2),
            anchors: vec![],
            custom_energy_curve: vec![],
            candidates: vec![
                CrateTrackCandidate {
                    track_id: "c1".into(),
                    title: "Track 1".into(),
                    artist: "Artist 1".into(),
                    file_path: "/a.flac".into(),
                    duration_seconds: 200.0,
                    bpm: 124.0,
                    camelot_key: "8A".into(),
                    energy_score: 0.6,
                    punch_index: 0.7,
                    groove_type: "four_on_floor".into(),
                    top_subgenre: "House".into(),
                    has_intro_vocals: false,
                    has_outro_vocals: false,
                    embedding_64d: vec![0.125; 64],
                    hot_cues: vec![],
                },
                CrateTrackCandidate {
                    track_id: "c2".into(),
                    title: "Track 2".into(),
                    artist: "Artist 2".into(),
                    file_path: "/b.flac".into(),
                    duration_seconds: 210.0,
                    bpm: 126.0,
                    camelot_key: "9A".into(),
                    energy_score: 0.8,
                    punch_index: 0.75,
                    groove_type: "four_on_floor".into(),
                    top_subgenre: "Tech House".into(),
                    has_intro_vocals: false,
                    has_outro_vocals: false,
                    embedding_64d: vec![0.125; 64],
                    hot_cues: vec![],
                },
            ],
        };
        let req_crate = Request::builder()
            .method("POST")
            .uri("/api/v1/crates/build")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&crate_req).unwrap()))
            .unwrap();
        let resp_crate = app.call(req_crate).await.unwrap();
        assert_eq!(resp_crate.status(), StatusCode::OK);

        // 5. Test MCP JSON-RPC 2.0 endpoint (`POST /mcp` & `GET /mcp/tools`)
        let req_mcp_init = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {}
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_mcp_init = app.call(req_mcp_init).await.unwrap();
        assert_eq!(resp_mcp_init.status(), StatusCode::OK);

        let req_mcp_call = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/call",
                    "params": {
                        "name": "build_dj_crate",
                        "arguments": {
                            "brief": "MCP Rust Set",
                            "energy_arc": "steady_climb"
                        }
                    }
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_mcp_call = app.call(req_mcp_call).await.unwrap();
        assert_eq!(resp_mcp_call.status(), StatusCode::OK);

        // 6. Test OpenAPI 3.1.0 & Cloudflare Code Mode (`GET /openapi.json`, `GET /mcp/code-mode.d.ts`, `POST /mcp/code-mode`)
        let req_openapi = Request::builder()
            .uri("/openapi.json")
            .body(Body::empty())
            .unwrap();
        let resp_openapi = app.call(req_openapi).await.unwrap();
        assert_eq!(resp_openapi.status(), StatusCode::OK);

        let req_dts = Request::builder()
            .uri("/mcp/code-mode.d.ts")
            .body(Body::empty())
            .unwrap();
        let resp_dts = app.call(req_dts).await.unwrap();
        assert_eq!(resp_dts.status(), StatusCode::OK);

        let ts_script = r#"
            const tracks = await hera.searchMusic({ camelot: "8A" });
            const clean = tracks.filter(t => t.quality_score >= 95 && t.mono_compatible_sub);
            const crate = await hera.buildDjCrate({ brief: "Code Mode Peak Set", energy_arc: "peak_hour", candidates: clean });
            return { selected_count: clean.length, synergy: crate.mean_synergy_score };
        "#;
        let req_cm = Request::builder()
            .method("POST")
            .uri("/mcp/code-mode")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({ "code": ts_script })).unwrap(),
            ))
            .unwrap();
        let resp_cm = app.call(req_cm).await.unwrap();
        assert_eq!(resp_cm.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_davidj_and_djfoo_zero_tech_gdrive_and_code_mode_simulation() {
        use axum::body::to_bytes;

        let state = P2PMeshState::new("hera-davidj-hub");
        let mut app = build_hera_router(state);

        // 1. Verify DaviDJ (David José) is already registered as Founder & Core Architect
        let req_profiles = Request::builder()
            .uri("/api/v1/dj/profiles")
            .body(Body::empty())
            .unwrap();
        let resp_profiles = app.call(req_profiles).await.unwrap();
        assert_eq!(resp_profiles.status(), StatusCode::OK);
        let profiles_bytes = to_bytes(resp_profiles.into_body(), 1024 * 1024).await.unwrap();
        let profiles_json: serde_json::Value = serde_json::from_slice(&profiles_bytes).unwrap();
        assert_eq!(profiles_json["connected_djs_count"], 1);
        assert_eq!(profiles_json["profiles"][0]["dj_handle"], "DaviDJ");

        // 2. Non-technical DJFoo connects ONLY by pasting his Google Drive folder link!
        // He has 1 genuine 20,600 Hz studio FLAC master and 1 fake 14,900 Hz YouTube-rip FLAC.
        let djfoo_connect_payload = serde_json::json!({
            "dj_handle": "DJFoo",
            "display_name": "DJ Foo (New Club Resident)",
            "gdrive_folder_link": "https://drive.google.com/drive/folders/1DJFoo_My_Club_Tracks_2026?usp=sharing",
            "tracks": [
                {
                    "title": "Foo Groove",
                    "artist": "DJFoo",
                    "bpm": 125.0,
                    "camelot_key": "8A",
                    "top_subgenre": "Tech House",
                    "duration_seconds": 330.0,
                    "format": "flac",
                    "gdrive_file_link": "https://drive.google.com/file/d/1DJFoo_Groove_125BPM_8A_Master/view?usp=sharing",
                    "effective_bandwidth_hz": 20600.0,
                    "sub_bass_phase_correlation": 0.96
                },
                {
                    "title": "Fake YouTube Rip 128kbps Upscaled",
                    "artist": "Unknown",
                    "bpm": 126.0,
                    "camelot_key": "8A",
                    "top_subgenre": "Tech House",
                    "duration_seconds": 210.0,
                    "format": "flac",
                    "gdrive_file_link": "https://drive.google.com/file/d/1DJFoo_Fake_Rip/view?usp=sharing",
                    "effective_bandwidth_hz": 14900.0,
                    "sub_bass_phase_correlation": 0.85
                }
            ]
        });

        let req_connect = Request::builder()
            .method("POST")
            .uri("/api/v1/dj/connect-drive")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&djfoo_connect_payload).unwrap()))
            .unwrap();
        let resp_connect = app.call(req_connect).await.unwrap();
        assert_eq!(resp_connect.status(), StatusCode::CREATED);
        let connect_bytes = to_bytes(resp_connect.into_body(), 1024 * 1024).await.unwrap();
        let connect_json: serde_json::Value = serde_json::from_slice(&connect_bytes).unwrap();

        // Zero-Trust Quarantine must admit ONLY the authentic 20.6 kHz master and reject the 14.9 kHz fake!
        assert_eq!(connect_json["admitted_tracks"].as_array().unwrap().len(), 1);
        assert_eq!(
            connect_json["rejected_quarantine_tracks"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let djfoo_track_id = connect_json["admitted_tracks"][0]["track_id"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(
            connect_json["admitted_tracks"][0]["stream_url"]
                .as_str()
                .unwrap()
                .contains("1DJFoo_Groove_125BPM_8A_Master")
        );

        // 3. DJFoo's free Gemini CLI / Claude agent runs a SINGLE Cloudflare Code Mode script
        // to collaborate with DaviDJ's catalog and build a 4-track B2B set (DaviDJ + DJFoo)!
        let b2b_code_mode_script = r#"
            const djs = await hera.listDjs();
            const tracks = await hera.searchMusic({ camelot: "8A" });
            const clean = tracks.filter(t => t.quality_score >= 95 && t.mono_compatible_sub);
            const crate = await hera.buildDjCrate({ brief: "DaviDJ x DJFoo B2B Peak Set", energy_arc: "peak_hour", candidates: clean });
            return { dj_count: djs.length, b2b_tracks: clean.length, synergy: crate.mean_synergy_score, total_crate_tracks: crate.total_tracks };
        "#;
        let req_b2b = Request::builder()
            .method("POST")
            .uri("/mcp/code-mode")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({ "code": b2b_code_mode_script })).unwrap(),
            ))
            .unwrap();
        let resp_b2b = app.call(req_b2b).await.unwrap();
        assert_eq!(resp_b2b.status(), StatusCode::OK);
        let b2b_bytes = to_bytes(resp_b2b.into_body(), 1024 * 1024).await.unwrap();
        let b2b_json: serde_json::Value = serde_json::from_slice(&b2b_bytes).unwrap();

        assert_eq!(b2b_json["mode"], "cloudflare_code_mode_rust_sandbox");
        assert_eq!(b2b_json["output"]["dj_count"], 2);
        // 3 tracks from DaviDJ + 1 verified track from DJFoo = 4 tracks in the B2B crate!
        assert_eq!(b2b_json["output"]["b2b_tracks"], 4);
        assert_eq!(b2b_json["output"]["total_crate_tracks"], 4);
        assert!(b2b_json["output"]["synergy"].as_f64().unwrap() > 0.78);

        // 4. Verify DaviDJ can resolve DJFoo's direct Google Drive download link via /api/v1/p2p/stream/:track_id
        let req_stream = Request::builder()
            .uri(&format!("/api/v1/p2p/stream/{}", djfoo_track_id))
            .body(Body::empty())
            .unwrap();
        let resp_stream = app.call(req_stream).await.unwrap();
        assert_eq!(resp_stream.status(), StatusCode::OK);
        let stream_bytes = to_bytes(resp_stream.into_body(), 1024 * 1024).await.unwrap();
        let stream_json: serde_json::Value = serde_json::from_slice(&stream_bytes).unwrap();
        assert_eq!(stream_json["owner_dj"], "DJFoo");
        assert_eq!(stream_json["storage_provider"], "gdrive");
        assert_eq!(
            stream_json["direct_download_url"],
            "https://drive.google.com/uc?export=download&id=1DJFoo_Groove_125BPM_8A_Master"
        );
    }

    #[tokio::test]
    async fn test_multi_storage_r2_quota_zero_oauth_gdrive_and_local_p2p_seeder() {
        use axum::body::to_bytes;

        let state = P2PMeshState::new("hera-universal-storage-hub");
        let mut app = build_hera_router(state);

        // 1. Test Zero-OAuth Google Drive `/embeddedfolderview` HTML Auto-Parser
        let fake_gdrive_html = r#"
            <div class="flip-entries">
                <div class="flip-entry" id="entry-1ZeroOAuth_File_ID_001">
                    <div class="flip-entry-title">DJFoo - Midnight Caracas.flac</div>
                </div>
                <div class="flip-entry" id="entry-1ZeroOAuth_File_ID_002">
                    <div class="flip-entry-title">DJFoo - Avila Sunrise.wav</div>
                </div>
            </div>
        "#;
        let zero_oauth_req = serde_json::json!({
            "dj_handle": "DJZeroOAuth",
            "gdrive_folder_link": "https://drive.google.com/drive/folders/1ZeroOAuthFolder",
            "gdrive_embedded_html": fake_gdrive_html
        });
        let req1 = Request::builder()
            .method("POST")
            .uri("/api/v1/dj/connect-drive")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&zero_oauth_req).unwrap()))
            .unwrap();
        let resp1 = app.call(req1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::CREATED);
        let j1: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp1.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j1["admitted_tracks"].as_array().unwrap().len(), 2);
        // Verify default Founding Beta MVP DJ gets 5,120 MB (5 GB) in Cloudflare R2!
        assert_eq!(j1["profile"]["subscription_tier"], "beta_mvp");
        assert_eq!(j1["profile"]["r2_quota_mb"], 5120.0);
        assert_eq!(
            j1["admitted_tracks"][0]["stream_url"],
            "https://drive.google.com/uc?export=download&id=1ZeroOAuth_File_ID_001"
        );

        // 2. Test Cloudflare R2 Hard Quota (`trial_100mb` = 100 MB limit) + Global CAS Deduplication
        let r2_quota_req = serde_json::json!({
            "dj_handle": "DJR2TrialUser",
            "r2_bucket_url": "https://r2.hera-davidj.workers.dev",
            "subscription_tier": "trial_100mb",
            "tracks": [
                // This first track is a CAS Deduplication hit (already uploaded by DaviDJ at 122 BPM), so it costs 0 MB in R2!
                { "title": "Ritual Del Sol (DaviDJ Original Club Master)", "artist": "DaviDJ", "bpm": 122.0, "r2_object_key": "r2/dup_ritual.flac", "file_size_mb": 45.0 },
                { "title": "R2 Track 1", "artist": "DJR2", "r2_object_key": "r2/t1.flac", "file_size_mb": 45.0 },
                { "title": "R2 Track 2", "artist": "DJR2", "r2_object_key": "r2/t2.flac", "file_size_mb": 45.0 },
                { "title": "R2 Track 3 (Over 100MB Trial Quota)", "artist": "DJR2", "r2_object_key": "r2/t3.flac", "file_size_mb": 45.0 }
            ]
        });
        let req2 = Request::builder()
            .method("POST")
            .uri("/api/v1/dj/connect-drive")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&r2_quota_req).unwrap()))
            .unwrap();
        let resp2 = app.call(req2).await.unwrap();
        let j2: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp2.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        // 1 CAS-deduplicated track (0 MB physical R2) + 2 new tracks (90 MB <= 100 MB) = 3 admitted; 4th track rejected by R2_QUOTA_EXCEEDED!
        assert_eq!(j2["cas_deduplicated_hits"], 1);
        assert_eq!(j2["physical_r2_mb_saved"], 45.0);
        assert_eq!(j2["admitted_tracks"].as_array().unwrap().len(), 3);
        assert_eq!(j2["rejected_quarantine_tracks"].as_array().unwrap().len(), 1);
        assert!(
            j2["rejected_quarantine_tracks"][0]
                .as_str()
                .unwrap()
                .contains("R2_QUOTA_EXCEEDED")
        );

        // 3. Test Local Laptop / Server Direct Storage & P2P Seeder (`POST /api/v1/node/scan-local-dir` + `GET /api/v1/p2p/download/:track_id`)
        let temp_dir = std::env::temp_dir().join(format!(
            "hera_local_p2p_test_dir_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create 1 real broadband club master WAV (passes Zero-Trust DSP) and 1 pure sine WAV (quarantined)
        let sr = 44100u32;
        let n = sr as usize * 6;
        let mut left_good = Vec::with_capacity(n);
        let mut right_good = Vec::with_capacity(n);
        let mut rng: u64 = 123456789;
        for i in 0..n {
            let t = (i as f32) / (sr as f32);
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let airy = (((rng >> 32) as u32 as f32) / (u32::MAX as f32) - 0.5) * 0.25;
            let kick = (2.0 * std::f32::consts::PI * 60.0 * t).sin()
                * ((1.0 - (t % 0.5) * 2.0).max(0.0))
                * 0.55;
            left_good.push(kick + airy);
            right_good.push(kick + airy);
        }
        let good_wav = encode_stereo_wav(&left_good, &right_good, sr);
        std::fs::write(temp_dir.join("DJLaptop - Local Seeder Master.wav"), &good_wav).unwrap();

        let bad_sine: Vec<f32> = (0..sr as usize * 3)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * (i as f32 / sr as f32)).sin() * 0.8)
            .collect();
        let bad_wav = encode_stereo_wav(&bad_sine, &bad_sine, sr);
        std::fs::write(temp_dir.join("Spammer - Fake Sine Test.wav"), &bad_wav).unwrap();

        let req_scan = Request::builder()
            .method("POST")
            .uri("/api/v1/node/scan-local-dir")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "dj_handle": "DJLaptop",
                    "directory_path": temp_dir.to_string_lossy(),
                    "public_node_url": "http://192.168.1.50:8080",
                    "bandwidth_mbps": 800
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_scan = app.call(req_scan).await.unwrap();
        assert_eq!(resp_scan.status(), StatusCode::CREATED);
        let j_scan: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_scan.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();

        assert_eq!(j_scan["admitted_tracks"].as_array().unwrap().len(), 1);
        assert_eq!(j_scan["rejected_quarantine_tracks"].as_array().unwrap().len(), 1);
        assert!(!crate::p2p::tier_allows_private_vault("local_unlimited"));
        let local_track_id = j_scan["admitted_tracks"][0]["track_id"]
            .as_str()
            .unwrap()
            .to_string();

        // Verify direct P2P binary audio streaming from the laptop's local disk!
        let req_dl = Request::builder()
            .uri(&format!("/api/v1/p2p/download/{}", local_track_id))
            .body(Body::empty())
            .unwrap();
        let resp_dl = app.call(req_dl).await.unwrap();
        assert_eq!(resp_dl.status(), StatusCode::OK);
        let dl_bytes = to_bytes(resp_dl.into_body(), 64 * 1024 * 1024).await.unwrap();
        assert_eq!(dl_bytes.len(), good_wav.len());

        let _ = std::fs::remove_dir_all(&temp_dir);

        // 4. Test the Internal P2P Data Broker (`GET /api/v1/broker/stats`, `POST /api/v1/broker/lookup-intel`, `POST /api/v1/broker/resolve-bridge`)
        let req_bstats = Request::builder()
            .uri("/api/v1/broker/stats")
            .body(Body::empty())
            .unwrap();
        let resp_bstats = app.call(req_bstats).await.unwrap();
        assert_eq!(resp_bstats.status(), StatusCode::OK);
        let j_bstats: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_bstats.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        // Verify deduplication in the Broker: `Ritual Del Sol` has 2 replicas (DaviDJ + DJR2TrialUser) grouped under 1 CAS master!
        assert!(j_bstats["total_swarm_replicas"].as_u64().unwrap() > j_bstats["unique_cas_masters"].as_u64().unwrap());

        // Test Collective Crowd-DSP O(1) Lookup + Auto-Heal of a 14,800 Hz fake YouTube rip!
        let req_lookup = Request::builder()
            .method("POST")
            .uri("/api/v1/broker/lookup-intel")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "requesting_dj": "DJNewbie",
                    "tracks": [
                        {
                            "artist": "DaviDJ",
                            "title": "Ritual Del Sol",
                            "local_bandwidth_hz": 14800.0
                        }
                    ]
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_lookup = app.call(req_lookup).await.unwrap();
        assert_eq!(resp_lookup.status(), StatusCode::OK);
        let j_lookup: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_lookup.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_lookup["auto_healed_count"], 1);
        assert_eq!(j_lookup["instant_intel_hits"][0]["needs_auto_heal"], true);
        // Has 2 swarm download routes (DaviDJ's Google Drive + DJR2TrialUser's R2 CAS)!
        assert_eq!(
            j_lookup["instant_intel_hits"][0]["verified_master_intel"]["swarm_replicas_count"],
            2
        );

        // Test Emergency Booth Bridge Resolver (Jumping from 128 BPM 8A to 96 BPM 8A via 3:4 polyrhythmic bridge)
        let req_bridge = Request::builder()
            .method("POST")
            .uri("/api/v1/broker/resolve-bridge")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "from_bpm": 128.0,
                    "from_camelot": "8A",
                    "to_bpm": 96.0,
                    "to_camelot": "8A"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_bridge = app.call(req_bridge).await.unwrap();
        assert_eq!(resp_bridge.status(), StatusCode::OK);
        let j_bridge: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_bridge.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert!(j_bridge["bridge_candidates_count"].as_u64().unwrap() >= 1);

        // 5. Test 64-Bit SimHash POPCNT Sonic Similarity (`POST /api/v1/broker/similar`)
        let req_sim = Request::builder()
            .method("POST")
            .uri("/api/v1/broker/similar")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "bpm": 122.0,
                    "camelot_key": "8A",
                    "subgenre": "Afro House / Deep Tech",
                    "limit": 3
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_sim = app.call(req_sim).await.unwrap();
        assert_eq!(resp_sim.status(), StatusCode::OK);
        let j_sim: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_sim.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_sim["engine"], "u64_simhash_popcnt_in_ram");
        assert!(j_sim["matches_count"].as_u64().unwrap() >= 1);
        // Exact match (`Ritual Del Sol`, 122 BPM, 8A, Afro House / Deep Tech) must have 0 Hamming distance!
        assert_eq!(j_sim["matches"][0]["hamming_distance_bits"], 0);
        assert_eq!(j_sim["matches"][0]["popcnt_similarity_pct"], 100.0);

        // 6. Verify P2P Sharing is ALWAYS ON by default (`share_on_p2p: true`) in the public P2P node
        let req_free_dj = Request::builder()
            .method("POST")
            .uri("/api/v1/dj/connect-drive")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "dj_handle": "DJBetaShared",
                    "subscription_tier": "beta_mvp",
                    "p2p_sharing_enabled": false, // Even if a Beta DJ passes false, default sharing stays ON!
                    "tracks": [
                        {
                            "artist": "DJBetaShared",
                            "title": "Beta Club Groove",
                            "bpm": 125.0,
                            "camelot_key": "8A",
                            "format": "flac",
                            "effective_bandwidth_hz": 21500.0,
                            "sub_bass_phase_correlation": 0.95,
                            "share_on_p2p": false
                        }
                    ]
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_free_dj = app.call(req_free_dj).await.unwrap();
        assert_eq!(resp_free_dj.status(), StatusCode::CREATED);
        let j_free_dj: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_free_dj.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(j_free_dj["profile"]["p2p_sharing_enabled"], true);
        assert_eq!(j_free_dj["profile"]["can_disable_p2p_sharing"], false);
        assert_eq!(j_free_dj["admitted_tracks"][0]["share_on_p2p"], true);

        // 7. Test Federated Node Sync (`POST /api/v1/p2p/federate-sync`) from a Developer's Self-Hosted Cloudflare Worker / VPS
        let valid_seal = crate::p2p::compute_hera_seal(
            "cf-worker-dev-001",
            126.0,
            "9A",
            96.0,
            "VERIFIED_STUDIO_MASTER",
        );
        let req_fed = Request::builder()
            .method("POST")
            .uri("/api/v1/p2p/federate-sync")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "peer": {
                        "node_id": "hera-dev-cf-worker-01",
                        "dj_handle": "DJDevWorker",
                        "endpoint_url": "https://hera-node.djdev.workers.dev",
                        "runtime_target": "cloudflare-workers-wasm32",
                        "storage_mode": "cloudflare_worker_self_hosted",
                        "verified_tracks_count": 1,
                        "bandwidth_mbps": 1000
                    },
                    "shared_tracks": [
                        {
                            "track_id": "cf-worker-dev-001",
                            "title": "Edge Worker Groove",
                            "artist": "DJDevWorker",
                            "owner_dj": "DJDevWorker",
                            "bpm": 126.0,
                            "camelot_key": "9A",
                            "top_subgenre": "Tech House",
                            "duration_seconds": 280.0,
                            "format": "flac",
                            "effective_bandwidth_hz": 21200.0,
                            "sub_bass_phase_correlation": 0.94,
                            "mono_compatible_sub": true,
                            "quality_score": 96.0,
                            "forensic_verdict": "VERIFIED_STUDIO_MASTER",
                            "hot_cues": [],
                            "storage_provider": "gdrive",
                            "stream_url": "https://drive.google.com/uc?export=download&id=1DevWorkerTrack",
                            "share_on_p2p": true,
                            "node_id": "hera-dev-cf-worker-01",
                            "hera_seal": valid_seal
                        },
                        {
                            "track_id": "cf-worker-forged-002",
                            "title": "Tampered Seal Track",
                            "artist": "BadActor",
                            "owner_dj": "BadActor",
                            "bpm": 126.0,
                            "camelot_key": "9A",
                            "top_subgenre": "Tech House",
                            "duration_seconds": 280.0,
                            "format": "flac",
                            "effective_bandwidth_hz": 21200.0,
                            "sub_bass_phase_correlation": 0.94,
                            "mono_compatible_sub": true,
                            "quality_score": 96.0,
                            "forensic_verdict": "VERIFIED_STUDIO_MASTER",
                            "hot_cues": [],
                            "storage_provider": "gdrive",
                            "share_on_p2p": true,
                            "node_id": "hera-dev-cf-worker-01",
                            "hera_seal": "hera-seal-0000000000000000"
                        }
                    ]
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_fed = app.call(req_fed).await.unwrap();
        assert_eq!(resp_fed.status(), StatusCode::OK);
        let j_fed: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_fed.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_fed["status"], "federated_node_synced");
        assert_eq!(j_fed["tracks_accepted"], 1);
        assert_eq!(j_fed["tracks_rejected_invalid_seal"], 1);
        assert!(j_fed["federated_swarm_catalog"].as_array().unwrap().len() >= 4);

        // 7. Test `GET /api/v1/p2p/export-federation` payload builder
        let req_exp = Request::builder()
            .uri("/api/v1/p2p/export-federation?dj_handle=DJFoo&public_url=https://djfoo-hera.workers.dev")
            .body(Body::empty())
            .unwrap();
        let resp_exp = app.call(req_exp).await.unwrap();
        assert_eq!(resp_exp.status(), StatusCode::OK);
        let j_exp: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_exp.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_exp["peer"]["dj_handle"], "DJFoo");
        assert_eq!(
            j_exp["peer"]["endpoint_url"],
            "https://djfoo-hera.workers.dev"
        );
        assert!(j_exp["shared_tracks"].as_array().unwrap().len() >= 4);
    }

    #[tokio::test]
    async fn test_cloudflare_swarm_water_filling_p2c_tickets_custody_and_bait_switch() {
        use crate::p2p::compute_hera_seal;
        use axum::body::to_bytes;

        let state = P2PMeshState::new("hera-edge-primary");
        let mut app = build_hera_router(state);

        // 1. Baseline Swarm Strength (N = 1 active healthy node: `hera-edge-primary`)
        let req_s1 = Request::builder()
            .uri("/api/v1/swarm/strength")
            .body(Body::empty())
            .unwrap();
        let resp_s1 = app.call(req_s1).await.unwrap();
        assert_eq!(resp_s1.status(), StatusCode::OK);
        let j_s1: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_s1.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(
            j_s1["motto"],
            "Mientras más nodos de Hera, más fuerte es el sistema de enjambres"
        );
        assert_eq!(j_s1["active_healthy_nodes"], 1);
        assert_eq!(j_s1["total_free_r2_capacity_gb"], 10.0);
        assert_eq!(j_s1["safe_usable_r2_capacity_gb"], 9.0);
        assert_eq!(j_s1["safe_daily_worker_requests_capacity"], 85_000);
        assert_eq!(j_s1["base_replication_factor_k"], 1);
        let strength_n1 = j_s1["swarm_strength_multiplier"].as_f64().unwrap();

        // 2. Scale the Swarm: Add Node 2 (`DJFoo-Worker-EU`) via `POST /api/v1/swarm/heartbeat`
        let req_hb2 = Request::builder()
            .method("POST")
            .uri("/api/v1/swarm/heartbeat")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "node_id": "hera-worker-djfoo-eu",
                    "owner_dj": "DJFoo",
                    "endpoint_url": "https://hera-djfoo.workers.dev",
                    "behind_cgnat": false,
                    "r2_used_mb": 400.0,
                    "daily_worker_requests_used": 1200,
                    "megabytes_served_delta": 20.0
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_hb2 = app.call(req_hb2).await.unwrap();
        assert_eq!(resp_hb2.status(), StatusCode::OK);
        let j_hb2: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_hb2.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_hb2["edge_guardian_active"], true);
        assert_eq!(j_hb2["swarm_strength"]["active_healthy_nodes"], 2);
        assert_eq!(j_hb2["swarm_strength"]["total_free_r2_capacity_gb"], 20.0);
        assert_eq!(j_hb2["swarm_strength"]["safe_usable_r2_capacity_gb"], 18.0);
        assert_eq!(
            j_hb2["swarm_strength"]["safe_daily_worker_requests_capacity"],
            170_000
        );
        assert_eq!(j_hb2["swarm_strength"]["base_replication_factor_k"], 2);
        let strength_n2 = j_hb2["swarm_strength"]["swarm_strength_multiplier"]
            .as_f64()
            .unwrap();
        assert!(strength_n2 > strength_n1);

        // 3. Scale to N = 5 nodes (including a CGNAT node & a nearly-full free-tier Worker to test Water-Filling safety caps)
        for (node_id, owner_dj, cgnat, r2_used, req_used) in [
            ("hera-worker-davidj-us", "DaviDJ", false, 1200.0, 5000u64),
            ("hera-worker-luna-latam", "DJLuna", true, 300.0, 800u64),
            ("hera-worker-kai-apac", "DJKai", false, 8950.0, 84900u64),
        ] {
            let req_hb = Request::builder()
                .method("POST")
                .uri("/api/v1/swarm/heartbeat")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "node_id": node_id,
                        "owner_dj": owner_dj,
                        "endpoint_url": format!("https://{}.workers.dev", node_id),
                        "behind_cgnat": cgnat,
                        "r2_used_mb": r2_used,
                        "daily_worker_requests_used": req_used
                    }))
                    .unwrap(),
                ))
                .unwrap();
            let resp_hb = app.call(req_hb).await.unwrap();
            assert_eq!(resp_hb.status(), StatusCode::OK);
            let j_hb: serde_json::Value =
                serde_json::from_slice(&to_bytes(resp_hb.into_body(), 1024 * 1024).await.unwrap())
                    .unwrap();
            if cgnat {
                assert!(
                    !j_hb["outbound_pull_tickets"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                );
            }
            let alloc_mb = j_hb["water_filling_allocation"]["allocated_r2_mb"]
                .as_f64()
                .unwrap();
            let safe_cap_mb = j_hb["water_filling_allocation"]["safe_r2_cap_mb"]
                .as_f64()
                .unwrap();
            assert_eq!(safe_cap_mb, 9000.0);
            assert!(alloc_mb <= safe_cap_mb + 1e-5);
        }

        let req_s5 = Request::builder()
            .uri("/api/v1/swarm/strength")
            .body(Body::empty())
            .unwrap();
        let resp_s5 = app.call(req_s5).await.unwrap();
        let j_s5: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_s5.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_s5["active_healthy_nodes"], 5);
        assert_eq!(j_s5["total_free_r2_capacity_gb"], 50.0);
        assert_eq!(j_s5["safe_usable_r2_capacity_gb"], 45.0);
        assert_eq!(j_s5["safe_daily_worker_requests_capacity"], 425_000);
        assert_eq!(j_s5["base_replication_factor_k"], 2);
        assert!(j_s5["max_thermal_replication_k"].as_u64().unwrap() >= 3);
        let strength_n5 = j_s5["swarm_strength_multiplier"].as_f64().unwrap();
        assert!(strength_n5 > strength_n2);

        // 4. Power-of-Two-Choices (P2C) + Global Catalog Blinding (`shard-blobs/{hmac}.hbin`) + 60s Signed Ticket
        let req_ticket = Request::builder()
            .method("POST")
            .uri("/api/v1/swarm/route-ticket")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "track_id_or_cas": "hera-seed-001",
                    "requester_dj": "DJFoo",
                    "block_index": 0
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_ticket = app.call(req_ticket).await.unwrap();
        assert_eq!(resp_ticket.status(), StatusCode::OK);
        let j_ticket: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_ticket.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(j_ticket["status"], "SWARM_TICKET_ISSUED");
        assert_eq!(
            j_ticket["access_mode"],
            "EDGE_GUARDIAN_UNLIMITED_FAST_LANE"
        );
        assert_eq!(j_ticket["ticket"]["aligned_length_bytes"], 4_194_304);
        let issued = j_ticket["ticket"]["issued_at_unix"].as_u64().unwrap();
        let expires = j_ticket["ticket"]["expires_at_unix"].as_u64().unwrap();
        assert_eq!(expires - issued, 60);

        let opaque_blob_key = j_ticket["ticket"]["opaque_blob_key"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(opaque_blob_key.starts_with("shard-blobs/"));
        assert!(opaque_blob_key.ends_with(".hbin"));
        let blob_id = opaque_blob_key.trim_start_matches("shard-blobs/");
        let signed_ticket_header = serde_json::to_string(&j_ticket["ticket"]).unwrap();

        // 4a. Attempting to fetch the opaque shard WITHOUT `X-Hera-Swarm-Ticket` must be blocked (403 FORBIDDEN)
        let req_no_ticket = Request::builder()
            .uri(&format!("/api/v1/swarm/shard/{}", blob_id))
            .body(Body::empty())
            .unwrap();
        let resp_no_ticket = app.call(req_no_ticket).await.unwrap();
        assert_eq!(resp_no_ticket.status(), StatusCode::FORBIDDEN);

        // 4b. Attempting to fetch with a forged/tampered signature must be blocked (403 FORBIDDEN)
        let mut tampered_ticket = j_ticket["ticket"].clone();
        tampered_ticket["hmac_signature"] = serde_json::Value::String("0000deadbeef".to_string());
        let req_bad_ticket = Request::builder()
            .uri(&format!("/api/v1/swarm/shard/{}", blob_id))
            .header(
                "x-hera-swarm-ticket",
                serde_json::to_string(&tampered_ticket).unwrap(),
            )
            .body(Body::empty())
            .unwrap();
        let resp_bad_ticket = app.call(req_bad_ticket).await.unwrap();
        assert_eq!(resp_bad_ticket.status(), StatusCode::FORBIDDEN);

        // 4b-bis. Attempting to fetch with an EXPIRED 60s ticket (`now_unix > expires_at_unix`) must be blocked (403 FORBIDDEN)
        let req_expired_ticket = Request::builder()
            .uri(&format!(
                "/api/v1/swarm/shard/{}?now_unix={}",
                blob_id,
                expires + 1
            ))
            .header("x-hera-swarm-ticket", &signed_ticket_header)
            .body(Body::empty())
            .unwrap();
        let resp_expired_ticket = app.call(req_expired_ticket).await.unwrap();
        assert_eq!(resp_expired_ticket.status(), StatusCode::FORBIDDEN);

        // 4c. Valid 60s ticket succeeds, sets `Cache-Control: public, max-age=31536000, immutable`, and mints 1 : 1.5 Turbo credits!
        let req_good_shard = Request::builder()
            .uri(&format!("/api/v1/swarm/shard/{}", blob_id))
            .header("x-hera-swarm-ticket", &signed_ticket_header)
            .body(Body::empty())
            .unwrap();
        let resp_good_shard = app.call(req_good_shard).await.unwrap();
        assert_eq!(resp_good_shard.status(), StatusCode::OK);
        assert_eq!(
            resp_good_shard
                .headers()
                .get("cache-control")
                .unwrap()
                .to_str()
                .unwrap(),
            "public, max-age=31536000, immutable"
        );
        let receipt_hdr = resp_good_shard
            .headers()
            .get("x-hera-proof-of-delivery-receipt")
            .unwrap()
            .to_str()
            .unwrap();
        let j_delivery: serde_json::Value = serde_json::from_str(receipt_hdr).unwrap();
        assert_eq!(j_delivery["megabytes_delivered"], 4.0);
        assert_eq!(j_delivery["credits_earned_mb"], 6.0);
        assert_eq!(j_delivery["credit_multiplier"], 1.5);
        assert_eq!(
            resp_good_shard
                .headers()
                .get("x-hera-p2p-boost-cipher")
                .unwrap()
                .to_str()
                .unwrap(),
            "zk-ctr-sha256-v1"
        );
        let wire_bytes = to_bytes(resp_good_shard.into_body(), 1024 * 1024)
            .await
            .unwrap();
        assert!(!String::from_utf8_lossy(&wire_bytes).contains("HERA-4MB-IMMUTABLE-BLOCK"));
        let unsealed = crate::p2p_exchange::transform_p2p_boost_shard_bytes(
            &wire_bytes,
            &opaque_blob_key,
            j_ticket["ticket"]["audio_sha256"].as_str().unwrap(),
            j_ticket["ticket"]["aligned_offset_bytes"].as_u64().unwrap(),
        );
        assert!(String::from_utf8_lossy(&unsealed).contains("HERA-4MB-IMMUTABLE-BLOCK"));

        // 4d. Replaying the exact same `X-Hera-Swarm-Ticket` a second time must be rejected (403 FORBIDDEN) to prevent infinite credit farming!
        let req_replay_shard = Request::builder()
            .uri(&format!("/api/v1/swarm/shard/{}", blob_id))
            .header("x-hera-swarm-ticket", &signed_ticket_header)
            .body(Body::empty())
            .unwrap();
        let resp_replay_shard = app.call(req_replay_shard).await.unwrap();
        assert_eq!(resp_replay_shard.status(), StatusCode::FORBIDDEN);

        // 5. Meritocratic Autopilot (`Edge Guardian`) + 64-Byte `Proof-of-Custody` Challenge/Probe/Audit
        let req_chal = Request::builder()
            .method("POST")
            .uri("/api/v1/swarm/custody-challenge")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "node_id": "hera-worker-djfoo-eu",
                    "cas_fingerprint_or_blob": "hera-seed-001",
                    "seed_nonce": 20261108
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_chal = app.call(req_chal).await.unwrap();
        assert_eq!(resp_chal.status(), StatusCode::OK);
        let j_chal: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_chal.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_chal["slice_length"], 64);
        let range_header = j_chal["range_header"].as_str().unwrap().to_string();
        let expected_slice_sha256 = j_chal["expected_slice_sha256"]
            .as_str()
            .unwrap()
            .to_string();

        // 5a. Probe the Worker's 64-byte slice endpoint with the `Range` header
        let req_probe = Request::builder()
            .uri(&format!("/api/v1/swarm/custody-probe/{}", blob_id))
            .header("range", &range_header)
            .body(Body::empty())
            .unwrap();
        let resp_probe = app.call(req_probe).await.unwrap();
        assert_eq!(resp_probe.status(), StatusCode::PARTIAL_CONTENT);
        let probe_slice_hash = resp_probe
            .headers()
            .get("x-hera-custody-slice-sha256")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let probe_bytes = to_bytes(resp_probe.into_body(), 1024).await.unwrap();
        assert_eq!(probe_bytes.len(), 64);
        assert_eq!(probe_slice_hash, expected_slice_sha256);

        // 5b. Submit passing custody audit -> `Edge Guardian` remains active (`ONLINE_HEALTHY`)
        let req_audit_ok = Request::builder()
            .method("POST")
            .uri("/api/v1/swarm/custody-audit")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "challenge_id": j_chal["challenge_id"],
                    "node_id": "hera-worker-djfoo-eu",
                    "opaque_blob_key": &opaque_blob_key,
                    "byte_offset": j_chal["byte_offset"],
                    "http_status": 206,
                    "returned_slice_sha256": probe_slice_hash
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_audit_ok = app.call(req_audit_ok).await.unwrap();
        assert_eq!(resp_audit_ok.status(), StatusCode::OK);
        let j_audit_ok: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_audit_ok.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(j_audit_ok["passed"], true);
        assert_eq!(j_audit_ok["edge_guardian_active"], true);
        assert_eq!(j_audit_ok["custody_status"], "ONLINE_HEALTHY");

        // 5c. Submit failing custody audit (node deleted the file and returned 404) -> loses `Edge Guardian` immediately!
        let req_audit_fail = Request::builder()
            .method("POST")
            .uri("/api/v1/swarm/custody-audit")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "challenge_id": j_chal["challenge_id"],
                    "node_id": "hera-worker-djfoo-eu",
                    "opaque_blob_key": &opaque_blob_key,
                    "byte_offset": j_chal["byte_offset"],
                    "http_status": 404,
                    "returned_slice_sha256": ""
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_audit_fail = app.call(req_audit_fail).await.unwrap();
        assert_eq!(resp_audit_fail.status(), StatusCode::FORBIDDEN);
        let j_audit_fail: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_audit_fail.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(j_audit_fail["passed"], false);
        assert_eq!(j_audit_fail["edge_guardian_active"], false);
        assert_eq!(
            j_audit_fail["custody_status"],
            "GUARDIAN_REVOKED_SHARD_MISSING_404"
        );

        // Verify that the revoked node is immediately excluded from `active_healthy_nodes` and Water-Filling allocations!
        let req_s4 = Request::builder()
            .uri("/api/v1/swarm/strength")
            .body(Body::empty())
            .unwrap();
        let resp_s4 = app.call(req_s4).await.unwrap();
        let j_s4: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_s4.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_s4["active_healthy_nodes"], 4);
        assert_eq!(j_s4["total_free_r2_capacity_gb"], 40.0);
        assert_eq!(j_s4["safe_usable_r2_capacity_gb"], 36.0);

        // 5d. When DJFoo's node is offline/revoked, DJFoo automatically spends their earned 1:1.5 Turbo Credits (`38.0 MB` per track)!
        for block_idx in 1..=4 {
            let req_ticket_offline = Request::builder()
                .method("POST")
                .uri("/api/v1/swarm/route-ticket")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "track_id_or_cas": "hera-seed-001",
                        "requester_dj": "DJFoo",
                        "block_index": block_idx
                    }))
                    .unwrap(),
                ))
                .unwrap();
            let resp_ticket_offline = app.call(req_ticket_offline).await.unwrap();
            assert_eq!(resp_ticket_offline.status(), StatusCode::OK);
            let j_ticket_offline: serde_json::Value = serde_json::from_slice(
                &to_bytes(resp_ticket_offline.into_body(), 1024 * 1024)
                    .await
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(
                j_ticket_offline["access_mode"],
                "TURBO_CREDIT_REDEMPTION_FAST_LANE"
            );
        }

        // 5e. Once DJFoo's Turbo Credits are exhausted (< 38.0 MB remaining while node is still revoked), access downgrades to `COMMUNITY_STANDARD_LANE`!
        let req_ticket_standard = Request::builder()
            .method("POST")
            .uri("/api/v1/swarm/route-ticket")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "track_id_or_cas": "hera-seed-001",
                    "requester_dj": "DJFoo",
                    "block_index": 5
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_ticket_standard = app.call(req_ticket_standard).await.unwrap();
        assert_eq!(resp_ticket_standard.status(), StatusCode::OK);
        let j_ticket_standard: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_ticket_standard.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            j_ticket_standard["access_mode"],
            "COMMUNITY_STANDARD_LANE"
        );

        // 6. Anti-Bait-and-Switch Double Cryptographic Seal (`audio_sha256` + `hera_seal`) & Quarantine Blacklist
        let valid_seal = compute_hera_seal(
            "hera-bait-001",
            122.0,
            "8A",
            98.5,
            "VERIFIED_STUDIO_MASTER",
        );
        let req_bait = Request::builder()
            .method("POST")
            .uri("/api/v1/p2p/federate-sync")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "peer": {
                        "node_id": "hera-node-bait-attacker",
                        "dj_handle": "MaliciousBaitNode",
                        "endpoint_url": "https://malicious-bait.workers.dev",
                        "runtime_target": "cloudflare-workers-wasm32",
                        "storage_mode": "r2_edge_guardian",
                        "verified_tracks_count": 1,
                        "bandwidth_mbps": 1000
                    },
                    "shared_tracks": [
                        {
                            "track_id": "hera-bait-001",
                            "title": "Ritual Del Sol (DaviDJ Original Club Master)",
                            "artist": "DaviDJ",
                            "owner_dj": "MaliciousBaitNode",
                            "bpm": 122.0,
                            "camelot_key": "8A",
                            "top_subgenre": "Afro House / Deep Tech",
                            "duration_seconds": 342.0,
                            "format": "flac",
                            "effective_bandwidth_hz": 21850.0,
                            "sub_bass_phase_correlation": 0.96,
                            "mono_compatible_sub": true,
                            "quality_score": 98.5,
                            "forensic_verdict": "VERIFIED_STUDIO_MASTER",
                            "hot_cues": [],
                            "storage_provider": "r2",
                            "gdrive_file_id": None::<String>,
                            "r2_object_key": Some("masters/fake.flac"),
                            "stream_url": Some("https://malicious-bait.workers.dev/fake.flac"),
                            "share_on_p2p": true,
                            "node_id": "hera-node-bait-attacker",
                            "hera_seal": valid_seal
                        }
                    ],
                    "track_audio_sha256": {
                        "hera-bait-001": "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
                    }
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_bait = app.call(req_bait).await.unwrap();
        assert_eq!(resp_bait.status(), StatusCode::OK);
        let j_bait: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_bait.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_bait["status"], "QUARANTINED_BAIT_AND_SWITCH_ATTACK");
        assert_eq!(j_bait["tracks_accepted"], 0);
        assert_eq!(j_bait["tracks_rejected_invalid_seal"], 1);
        assert_eq!(j_bait["node_quarantined_for_bait_and_switch"], true);

        let req_q = Request::builder()
            .uri("/api/v1/swarm/quarantine")
            .body(Body::empty())
            .unwrap();
        let resp_q = app.call(req_q).await.unwrap();
        assert_eq!(resp_q.status(), StatusCode::OK);
        let j_q: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_q.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_q["quarantined_nodes_count"], 1);
        assert_eq!(
            j_q["quarantined_nodes"][0]["node_id"],
            "hera-node-bait-attacker"
        );
        assert_eq!(
            j_q["quarantined_nodes"][0]["reason"],
            "BAIT_AND_SWITCH_AUDIO_SHA256_MISMATCH"
        );

        // 7. Verify Google Drive >25 MB bypass (`&confirm=t`) and 4 MB aligned block metadata in `/api/v1/p2p/stream/hera-seed-001`
        let req_stream = Request::builder()
            .uri("/api/v1/p2p/stream/hera-seed-001")
            .body(Body::empty())
            .unwrap();
        let resp_stream = app.call(req_stream).await.unwrap();
        assert_eq!(resp_stream.status(), StatusCode::OK);
        let j_stream: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_stream.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert!(
            j_stream["gdrive_large_file_bypass_url"]
                .as_str()
                .unwrap()
                .contains("&confirm=t")
        );
        assert_eq!(j_stream["aligned_block_size_bytes"], 4_194_304);
        assert!(
            j_stream["opaque_blob_key"]
                .as_str()
                .unwrap()
                .starts_with("shard-blobs/")
        );

        // 8. Verify `crate::p2p_exchange` public helpers and Code Mode `hera.getSwarmStrength()`
        assert_eq!(crate::p2p_exchange::SWARM_TURBO_CREDIT_MULTIPLIER, 1.5);
        assert_eq!(
            crate::p2p_exchange::compute_elastic_replication_factor(5, 10.0),
            5
        );
        let blind_key = crate::p2p_exchange::derive_blind_shard_key("cas-test", "sha-test");
        assert!(blind_key.starts_with("shard-blobs/"));
        let ex_state = P2PMeshState::new("hera-exchange-helper-test");
        let wf = crate::p2p_exchange::compute_water_filling_allocations(&ex_state);
        assert_eq!(wf.len(), 1);
        let chal_helper = crate::p2p_exchange::issue_custody_challenge(
            &ex_state,
            crate::p2p_exchange::CustodyChallengeRequest {
                node_id: "hera-exchange-helper-test".to_string(),
                cas_fingerprint_or_blob: "hera-seed-001".to_string(),
                seed_nonce: Some(42),
            },
        )
        .unwrap();
        assert_eq!(chal_helper.slice_length, 64);

        let req_cm_swarm = Request::builder()
            .method("POST")
            .uri("/mcp/code-mode")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "code": "const s = await hera.getSwarmStrength(); return s;"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_cm_swarm = app.call(req_cm_swarm).await.unwrap();
        assert_eq!(resp_cm_swarm.status(), StatusCode::OK);
        let j_cm_swarm: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_cm_swarm.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(j_cm_swarm["output"]["active_healthy_nodes"], 4);

        // 9. Verify Layer 3 Post-P2P Streaming Merkle Proofs (`16 x 256 KiB = 4 MB`) & zero-kilobyte tamper rejection
        let tree = crate::p2p_exchange::compute_shard_merkle_tree(&blind_key, "sha-test", 0);
        assert_eq!(tree.leaf_count, 16);
        assert_eq!(tree.leaf_size_bytes, 262_144);
        assert_eq!(tree.leaf_hashes_sha256.len(), 16);
        assert_eq!(tree.merkle_root_sha256.len(), 64);

        for leaf_idx in 0..16 {
            let proof = crate::p2p_exchange::generate_merkle_inclusion_proof(
                &blind_key, "sha-test", 0, leaf_idx,
            )
            .unwrap();
            assert_eq!(proof.sibling_hashes_sha256.len(), 4);
            assert!(crate::p2p_exchange::verify_merkle_inclusion_proof(&proof));

            // Tampering with a single bit/nibble of the leaf hash must immediately fail verification!
            let mut tampered = proof.clone();
            tampered.leaf_hash_sha256 =
                "00000000000000000000000000000000000000000000000000000000deadbeef".to_string();
            assert!(!crate::p2p_exchange::verify_merkle_inclusion_proof(&tampered));
        }

        // 10. Verify Layer 2 Post-P2P Iroh-style `endpoint_pubkey_id` + Network Roaming (`Wi-Fi -> 5G`) + CGNAT Relay
        let expected_pubkey = crate::p2p_exchange::derive_node_endpoint_pubkey_id(
            "DJLuna",
            "hera-worker-luna-latam",
        );
        assert!(expected_pubkey.starts_with("ed25519_"));

        // DJLuna roams from her Worker URL to a mobile 5G booth endpoint behind CGNAT:
        let req_roam = Request::builder()
            .method("POST")
            .uri("/api/v1/swarm/heartbeat")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "node_id": "hera-worker-luna-latam",
                    "owner_dj": "DJLuna",
                    "endpoint_url": "https://5g-booth.luna.mobile",
                    "endpoint_pubkey_id": expected_pubkey,
                    "behind_cgnat": true,
                    "observed_rtt_ms": 18.5,
                    "megabytes_served_delta": 4.0,
                    "r2_used_mb": 420.0,
                    "daily_worker_requests_used": 1250
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_roam = app.call(req_roam).await.unwrap();
        assert_eq!(resp_roam.status(), StatusCode::OK);
        let j_roam: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_roam.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(j_roam["endpoint_pubkey_id"], expected_pubkey);
        assert_eq!(j_roam["network_generation"], 2);
        assert_eq!(j_roam["swarm_credit_balance_mb"], 162.0);
        assert!(j_roam["relay_assisted_url"]
            .as_str()
            .unwrap()
            .starts_with("https://relay.hera.audio/iroh-tunnel/ed25519_"));

        // 11. Verify Layer 1 & 2 Post-P2P Latency-Aware P2C + Multi-Node HTTP/3 QUIC Parallel Striping Manifest
        let req_pman = Request::builder()
            .method("POST")
            .uri("/api/v1/swarm/parallel-manifest")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "requester_dj": "DJFoo",
                    "track_id_or_cas": "hera-seed-001",
                    "file_size_mb": 38.0
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_pman = app.call(req_pman).await.unwrap();
        assert_eq!(resp_pman.status(), StatusCode::OK);
        let j_pman: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_pman.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(j_pman["status"], "SWARM_PARALLEL_MANIFEST_ISSUED");
        assert_eq!(j_pman["transport_protocol"], "HTTP/3-QUIC-MULTIPLEXED");
        assert_eq!(j_pman["total_blocks"], 10);
        assert!(j_pman["distinct_edge_guardians_striped"].as_u64().unwrap() >= 2);
        assert_eq!(j_pman["blocks"].as_array().unwrap().len(), 10);
        assert_eq!(j_pman["blocks"][9]["aligned_offset_bytes"], 9 * 4_194_304u64);

        // 12. Verify Layer 4 Post-P2P Delta-CRDT (`GCounter` + `LWW-Element-Set`) Commutativity (`merge(A, B) == merge(B, A)`) & Idempotence (`merge(A, A) == A`)
        let mut delta_a = crate::p2p_exchange::SwarmCrdtDelta {
            origin_node_id: "offline-booth-a".to_string(),
            origin_endpoint_pubkey_id: "ed25519_booth_a".to_string(),
            hlc_timestamp_ms: 1_762_600_100_000,
            ..Default::default()
        };
        delta_a
            .gcounter_node_bytes_served_mb
            .insert("offline-booth-a".to_string(), 120.0);
        delta_a
            .gcounter_node_bytes_served_mb
            .insert("offline-hotel-b".to_string(), 40.0);

        let mut delta_b = crate::p2p_exchange::SwarmCrdtDelta {
            origin_node_id: "offline-hotel-b".to_string(),
            origin_endpoint_pubkey_id: "ed25519_hotel_b".to_string(),
            hlc_timestamp_ms: 1_762_600_200_000,
            ..Default::default()
        };
        delta_b
            .gcounter_node_bytes_served_mb
            .insert("offline-booth-a".to_string(), 80.0);
        delta_b
            .gcounter_node_bytes_served_mb
            .insert("offline-hotel-b".to_string(), 95.0);

        let merged_ab = crate::p2p_exchange::merge_swarm_crdt_deltas(&delta_a, &delta_b);
        let merged_ba = crate::p2p_exchange::merge_swarm_crdt_deltas(&delta_b, &delta_a);
        let merged_aa = crate::p2p_exchange::merge_swarm_crdt_deltas(&delta_a, &delta_a);
        assert_eq!(
            crate::p2p_exchange::compute_crdt_state_digest_sha256(&merged_ab),
            crate::p2p_exchange::compute_crdt_state_digest_sha256(&merged_ba)
        );
        assert_eq!(
            crate::p2p_exchange::compute_crdt_state_digest_sha256(&merged_aa),
            crate::p2p_exchange::compute_crdt_state_digest_sha256(&delta_a)
        );

        let req_crdt = Request::builder()
            .method("POST")
            .uri("/api/v1/swarm/crdt-sync")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&merged_ab).unwrap()))
            .unwrap();
        let resp_crdt = app.call(req_crdt).await.unwrap();
        assert_eq!(resp_crdt.status(), StatusCode::OK);
        let j_crdt: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_crdt.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(j_crdt["status"], "CRDT_DELTA_CONVERGED");
        assert!(j_crdt["gcounter_total_swarm_served_mb"].as_f64().unwrap() >= 548.3);

        // 13. Verify Layer 5 Post-P2P MoQ-Style Semantic Harmonic Pub/Sub (`hera://swarm/8A/120-128`)
        let req_sub = Request::builder()
            .method("POST")
            .uri("/api/v1/swarm/subscribe")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "subscriber_node_id": "hera-worker-luna-latam",
                    "subscriber_dj": "DJLuna",
                    "camelot_key": "8A",
                    "bpm_min": 120.0,
                    "bpm_max": 128.0
                })
                .to_string(),
            ))
            .unwrap();
        let resp_sub = app.call(req_sub).await.unwrap();
        assert_eq!(resp_sub.status(), StatusCode::CREATED);
        let j_sub: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_sub.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(j_sub["status"], "MOQ_SEMANTIC_TOPIC_SUBSCRIBED");
        assert_eq!(j_sub["subscription"]["topic_uri"], "hera://swarm/8A/120-128");
        assert!(
            j_sub["immediate_matching_events"]
                .as_array()
                .unwrap()
                .len()
                >= 2
        );
        assert_eq!(
            j_sub["immediate_matching_events"][0]["merkle_root_sha256"]
                .as_str()
                .unwrap()
                .len(),
            64
        );

        // Verify next heartbeat for DJLuna proactively delivers `semantic_pubsub_events`
        let req_hb_sub = Request::builder()
            .method("POST")
            .uri("/api/v1/swarm/heartbeat")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "node_id": "hera-worker-luna-latam",
                    "owner_dj": "DJLuna",
                    "endpoint_url": "https://5g-roaming.djluna.workers.dev",
                    "r2_used_mb": 250.0,
                    "daily_worker_requests_used": 600,
                    "observed_rtt_ms": 18.0
                })
                .to_string(),
            ))
            .unwrap();
        let resp_hb_sub = app.call(req_hb_sub).await.unwrap();
        assert_eq!(resp_hb_sub.status(), StatusCode::OK);
        let j_hb_sub: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_hb_sub.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert!(
            j_hb_sub["semantic_pubsub_events"]
                .as_array()
                .unwrap()
                .len()
                >= 2
        );
    }

    #[tokio::test]
    async fn test_pure_rust_desktop_engine_exports_style_dna_and_cloudflare_free_ai() {
        use axum::body::to_bytes;

        let state = P2PMeshState::new("hera-desktop-pure-rust-test");
        let mut app = build_hera_router(state);

        let sample_tracks = serde_json::json!([
            {
                "track_id": "trk_keine_1",
                "artist": "Keinemusik",
                "title": "Move",
                "version": "Extended Mix",
                "file_path": "/Music/Hera/124_8A_Keinemusik - Move.flac",
                "bpm": 124.0,
                "camelot": "8A",
                "duration_seconds": 390.0,
                "bitrate_kbps": 1411,
                "energy": 0.84,
                "file_size_bytes": 42000000,
                "is_video": false,
                "is_own_production": false
            },
            {
                "track_id": "trk_david_1",
                "artist": "David Castillo",
                "title": "Caracas Dub",
                "version": "Original Master",
                "file_path": "/Music/Hera/_MIS_PRODUCCIONES/126_9A_David Castillo - Caracas Dub.flac",
                "bpm": 126.0,
                "camelot": "9A",
                "duration_seconds": 360.0,
                "bitrate_kbps": 1411,
                "energy": 0.88,
                "file_size_bytes": 38000000,
                "is_video": false,
                "is_own_production": true
            }
        ]);

        // 1. Verify Pure Rust Native Exporters (VirtualDJ .vdjfolder/<Poi>, Rekordbox XML, Traktor NML <CUE_V2>, M3U8)
        let req_exp = Request::builder()
            .method("POST")
            .uri("/api/v1/crates/export")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "crate_name": "Caracas Club Peak Set",
                    "brief": "Afro & Latin Tech House 124-126 BPM",
                    "tracks": sample_tracks,
                    "formats": ["m3u8", "rekordbox_xml", "virtualdj", "traktor_nml"]
                })
                .to_string(),
            ))
            .unwrap();
        let resp_exp = app.call(req_exp).await.unwrap();
        assert_eq!(resp_exp.status(), StatusCode::OK);
        let j_exp: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_exp.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(j_exp["track_count"], 2);
        assert!(j_exp["m3u8_content"].as_str().unwrap().contains("#EXTM3U"));
        assert!(j_exp["rekordbox_xml_content"]
            .as_str()
            .unwrap()
            .contains("<POSITION_MARK"));
        assert!(j_exp["virtualdj_folder_xml"]
            .as_str()
            .unwrap()
            .contains("<VirtualFolder"));
        assert!(j_exp["virtualdj_database_xml"]
            .as_str()
            .unwrap()
            .contains("Type=\"cue\""));
        assert!(j_exp["traktor_nml_content"]
            .as_str()
            .unwrap()
            .contains("<CUE_V2"));

        // 2. Verify Pure Rust DJ Style DNA Auto-Profiler
        let req_dna = Request::builder()
            .method("POST")
            .uri("/api/v1/dj/style-dna")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "tracks": sample_tracks,
                    "dj_handle": "DaviDJ",
                    "preferred_dj_software": "VirtualDJ",
                    "golden_rules": "Siempre incluir 1 produccion propia y respetar Camelot"
                })
                .to_string(),
            ))
            .unwrap();
        let resp_dna = app.call(req_dna).await.unwrap();
        assert_eq!(resp_dna.status(), StatusCode::OK);
        let j_dna: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_dna.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(j_dna["dominant_bpm_zone"], "club_afro_guaracha_122_132");
        assert!(j_dna["prompt_block"].as_str().unwrap().contains("VirtualDJ"));

        // 3. Verify Cloudflare Workers AI Free Default (`10,000 Neurons/day` at $0.00 + AI Gateway Cache Hit + Fallback)
        let req_ai = Request::builder()
            .method("POST")
            .uri("/api/v1/ai/chat")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "dj_handle": "DaviDJ",
                    "message": "Ármame un puente entre 8A y 9A a 125 BPM para VirtualDJ",
                    "style_dna": j_dna,
                    "daily_neurons_used_so_far": 120.0,
                    "cache_hit": false
                })
                .to_string(),
            ))
            .unwrap();
        let resp_ai = app.call(req_ai).await.unwrap();
        assert_eq!(resp_ai.status(), StatusCode::OK);
        let j_ai: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_ai.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(j_ai["backend_used"], "cloudflare_workers_ai_free");
        assert_eq!(
            j_ai["model_used"],
            "@cf/meta/llama-3.1-8b-instruct-fp8-fast"
        );
        assert_eq!(j_ai["estimated_cost_usd"], 0.0);
        assert!(j_ai["daily_neurons_remaining"].as_f64().unwrap() > 9800.0);
    }

    #[tokio::test]
    async fn test_cloudflare_edge_kv_forensic_cache_d1_dna_sync_and_vectorize_simhash() {
        use axum::body::{to_bytes, Body};
        use axum::http::{Request, StatusCode};
        use tower_service::Service;

        let state = P2PMeshState::new("hera-edge-test-01");
        let mut app = build_hera_router(state);

        // 1. Verify GET /api/v1/edge/cloudflare-capabilities returns all 6 Cloudflare Edge primitives + MoQ events
        let req_cap = Request::builder()
            .uri("/api/v1/edge/cloudflare-capabilities")
            .body(Body::empty())
            .unwrap();
        let resp_cap = app.call(req_cap).await.unwrap();
        assert_eq!(resp_cap.status(), StatusCode::OK);
        let j_cap: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_cap.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(j_cap["r2_egress_fee_usd_per_gb"], 0.0);
        assert_eq!(j_cap["pioneer_forensic_bounty_mb"], 5.0);
        assert_eq!(j_cap["primitives"].as_array().unwrap().len(), 7);
        assert!(j_cap["primitives"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "cf_calls_managed_turn"));
        assert!(!j_cap["recent_moq_harmonic_events"].as_array().unwrap().is_empty());

        // 2. Verify POST /api/v1/edge/forensic-cache/lookup hits on seeded track (< 2 ms, saves 420 ms laptop FFT CPU)
        let req_hit = Request::builder()
            .method("POST")
            .uri("/api/v1/edge/forensic-cache/lookup")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "artist": "DaviDJ",
                    "title": "Ritual Del Sol (DaviDJ Original Club Master)",
                    "bpm": 122.0
                })
                .to_string(),
            ))
            .unwrap();
        let resp_hit = app.call(req_hit).await.unwrap();
        assert_eq!(resp_hit.status(), StatusCode::OK);
        let j_hit: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_hit.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(j_hit["kv_hit"], true);
        assert_eq!(j_hit["cache_status"], "CLOUDFLARE_KV_EDGE_HIT");
        assert_eq!(j_hit["laptop_fft_cpu_ms_saved"], 420);
        assert_eq!(j_hit["entry"]["hot_cues"].as_array().unwrap().len(), 4);

        // 3. Verify POST /api/v1/edge/forensic-cache/store stores a brand-new pioneer master and credits +5.0 MB bounty
        let req_store = Request::builder()
            .method("POST")
            .uri("/api/v1/edge/forensic-cache/store")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "pioneer_dj": "DaviDJ",
                    "artist": "Rampa & Keinemusik",
                    "title": "Cesar (Hera Club VIP)",
                    "bpm": 123.0,
                    "camelot_key": "8A",
                    "top_subgenre": "Afro House / Deep Tech",
                    "duration_seconds": 368.0,
                    "format": "flac",
                    "effective_bandwidth_hz": 21900.0,
                    "sub_bass_phase_correlation": 0.96,
                    "hot_cues": j_hit["entry"]["hot_cues"].clone()
                })
                .to_string(),
            ))
            .unwrap();
        let resp_store = app.call(req_store).await.unwrap();
        assert_eq!(resp_store.status(), StatusCode::CREATED);
        let j_store: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_store.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(j_store["was_new_pioneer_entry"], true);
        assert_eq!(j_store["pioneer_bounty_credited_mb"], 5.0);

        // 4. Verify POST /api/v1/edge/d1-dna-sync generates deterministic Cloudflare D1 SQLite upsert at $0.00
        let req_d1 = Request::builder()
            .method("POST")
            .uri("/api/v1/edge/d1-dna-sync")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "dj_handle": "DaviDJ",
                    "preferred_software": "VirtualDJ",
                    "dominant_bpm_zone": "club_afro_guaracha_122_132",
                    "overall_average_bpm": 124.5,
                    "dominant_camelot_keys": ["8A", "9A", "7A"],
                    "prompt_block": "[HERA DJ STYLE DNA — DaviDJ]",
                    "pinned_anchors": [
                        { "track_id": "hera-seed-001", "title": "Ritual Del Sol", "role": "opening", "target_index": 0 }
                    ]
                })
                .to_string(),
            ))
            .unwrap();
        let resp_d1 = app.call(req_d1).await.unwrap();
        assert_eq!(resp_d1.status(), StatusCode::OK);
        let j_d1: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_d1.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(j_d1["status"], "CLOUDFLARE_D1_DNA_SYNCED_ZERO_COST");
        assert_eq!(j_d1["d1_cost_usd"], 0.0);
        assert!(j_d1["record"]["d1_upsert_sql"]
            .as_str()
            .unwrap()
            .contains("INSERT OR REPLACE INTO d1_dj_style_dna"));

        // 5. Verify POST /api/v1/edge/vectorize-match ranks swarm tracks using 64-bit u64 SimHash POPCNT + Camelot compatibility
        let req_vec = Request::builder()
            .method("POST")
            .uri("/api/v1/edge/vectorize-match")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "bpm": 122.5,
                    "camelot_key": "8A",
                    "subgenre": "Afro House / Deep Tech",
                    "limit": 4
                })
                .to_string(),
            ))
            .unwrap();
        let resp_vec = app.call(req_vec).await.unwrap();
        assert_eq!(resp_vec.status(), StatusCode::OK);
        let j_vec: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_vec.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(
            j_vec["engine"],
            "cloudflare_vectorize_u64_simhash_camelot_hybrid"
        );
        assert!(j_vec["matches_count"].as_u64().unwrap() >= 3);
        assert_eq!(j_vec["matches"][0]["camelot_compatible"], true);

        // 6. Verify POST /api/v1/p2p/turn-credentials issues Stage 3.5 Cloudflare Calls Managed TURN RFC 5766 credentials
        let req_turn = Request::builder()
            .method("POST")
            .uri("/api/v1/p2p/turn-credentials")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "dj_handle": "DaviDJ",
                    "node_id": "hera-mobile-cgnat-01",
                    "ttl_seconds": 3600
                })
                .to_string(),
            ))
            .unwrap();
        let resp_turn = app.call(req_turn).await.unwrap();
        assert_eq!(resp_turn.status(), StatusCode::OK);
        let j_turn: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_turn.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(j_turn["provider"], "cloudflare_calls_managed_turn_anycast");
        assert_eq!(j_turn["protocol"], "rfc5766_channel_data_binary_chunk_relay");
        assert_eq!(j_turn["channel_number"], 0x4000);
        assert_eq!(j_turn["r2_bytes_used"], 0);
        assert_eq!(j_turn["free_tier_allowance_gb"], 1000.0);
        assert!(!j_turn["ice_servers"].as_array().unwrap().is_empty());

        // 7. Verify POST /api/v1/p2p/turn-relay-proof executes live Stage 3.5 RFC 5766 ChannelData 0x4000 relay proof
        let req_turn_proof = Request::builder()
            .method("POST")
            .uri("/api/v1/p2p/turn-relay-proof")
            .body(Body::empty())
            .unwrap();
        let resp_turn_proof = app.call(req_turn_proof).await.unwrap();
        assert_eq!(resp_turn_proof.status(), StatusCode::OK);
        let j_turn_proof: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_turn_proof.into_body(), 1024 * 1024).await.unwrap(),
        )
        .unwrap();
        assert_eq!(
            j_turn_proof["stage_used"],
            "STAGE_3_5_CLOUDFLARE_TURN_CHANNEL_RELAY"
        );
        assert_eq!(j_turn_proof["r2_bytes_used"], 0);
        assert_eq!(j_turn_proof["audio_bytes_transferred"], 262_144);
        assert_eq!(j_turn_proof["chunks_transferred"], 32);
        assert_eq!(j_turn_proof["sha256_verified"], true);
    }

    #[test]
    fn test_live_udp_hole_punch_direct_p2p_zero_r2_and_stun_nat_probe() {
        // Generate 256 KiB of deterministic studio master audio bytes (1 Merkle Leaf)
        let mut audio_bytes = Vec::with_capacity(262_144);
        for i in 0..262_144usize {
            audio_bytes.push(((i.wrapping_mul(73) ^ (i >> 3)) & 0xFF) as u8);
        }

        let report =
            crate::p2p::run_live_udp_hole_punch_p2p_proof(&audio_bytes).expect("UDP P2P proof");
        println!(
            "\n=== HERA LIVE UDP HOLE PUNCH + DIRECT P2P PROOF ===\n{}\n===================================================\n",
            serde_json::to_string_pretty(&report).unwrap()
        );

        assert_eq!(report.r2_bytes_used, 0);
        assert_eq!(report.direct_p2p_audio_bytes_transferred, 262_144);
        assert!(report.coordinator_signaling_bytes < 200);
        assert!(report.sha256_verified);
        assert_eq!(report.expected_sha256, report.received_sha256);

        // Also test the persistent Desktop UDP P2P Seeder Daemon + Multi-STUN NAT Classifier + Port-Spray Client
        let state = std::sync::Arc::new(P2PMeshState::new("hera-desktop-daemon-test"));
        let (daemon_status, served_counter) =
            crate::p2p::spawn_desktop_udp_p2p_daemon(state).expect("spawn UDP daemon");
        println!(
            "\n=== HERA DESKTOP UDP P2P DAEMON + MULTI-STUN NAT DIAGNOSIS ===\n{}\n==============================================================\n",
            serde_json::to_string_pretty(&daemon_status).unwrap()
        );
        for chunk_idx in 0..4u32 {
            let chunk = crate::p2p::request_chunk_from_peer_with_spray(
                &daemon_status.daemon_listen_socket,
                daemon_status.nat_diagnosis.observed_port_delta,
                daemon_status.nat_diagnosis.spray_window_ports,
                "hera-seed-001",
                chunk_idx,
            )
            .expect("request chunk from daemon");
            assert_eq!(chunk.len(), 8192);
        }
        assert!(served_counter.load(std::sync::atomic::Ordering::Relaxed) >= 4);

        // Verify Stage 3.5: RFC 5766 TURN ChannelData (0x4000) binary chunk relay when Symmetric CGNAT blocks direct spray
        let turn_report = crate::p2p::run_live_turn_channel_relay_p2p_proof(&audio_bytes)
            .expect("Stage 3.5 TURN ChannelData P2P proof");
        println!(
            "\n=== HERA STAGE 3.5 RFC 5766 TURN CHANNEL-DATA BINARY RELAY PROOF ===\n{}\n====================================================================\n",
            serde_json::to_string_pretty(&turn_report).unwrap()
        );
        assert_eq!(
            turn_report.stage_used,
            "STAGE_3_5_CLOUDFLARE_TURN_CHANNEL_RELAY"
        );
        assert_eq!(turn_report.bound_channel_number, 0x4000);
        assert_eq!(turn_report.r2_bytes_used, 0);
        assert_eq!(turn_report.audio_bytes_transferred, 262_144);
        assert_eq!(turn_report.chunks_transferred, 32);
        assert!(turn_report.turn_relay_frames_forwarded >= 64);
        assert!(turn_report.sha256_verified);
        assert_eq!(turn_report.expected_sha256, turn_report.received_sha256);

        // Verify automatic cascade from Stage 3 (blocked blackhole UDP port) -> Stage 3.5 (RFC 5766 TURN ChannelData)
        let blackhole = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind blackhole");
        let blackhole_addr = blackhole.local_addr().unwrap().to_string();
        let (fallback_chunk, stage_label) = crate::p2p::request_chunk_with_turn_fallback(
            &blackhole_addr,
            1,
            2,
            Some(&daemon_status.daemon_listen_socket),
            "hera-seed-001",
            0,
        )
        .expect("cascade to Stage 3.5 TURN relay");
        assert_eq!(fallback_chunk.len(), 8192);
        assert_eq!(stage_label, "STAGE_3_5_CLOUDFLARE_TURN_CHANNEL_RELAY");
    }
}
