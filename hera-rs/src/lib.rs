pub mod code_mode;
pub mod forensics;
pub mod mcp;
pub mod p2p;
pub mod router;
pub mod structure;
pub mod synergy;

#[cfg(feature = "cloudflare")]
use tower_service::Service;
#[cfg(feature = "cloudflare")]
use worker::{event, Context, Env, HttpRequest, Result as WorkerResult};

/// Cloudflare Workers WASM Entrypoint (`npx wrangler deploy --features cloudflare --no-default-features`).
/// Uses the exact same `axum::Router` as the Native VPS binary!
#[cfg(feature = "cloudflare")]
#[event(fetch)]
pub async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> WorkerResult<http::Response<axum::body::Body>> {
    let node_id = env
        .var("HERA_NODE_ID")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "hera-cf-edge-node".to_string());
    let state = p2p::P2PMeshState::new(node_id);
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
                stream_url: None,
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
}
