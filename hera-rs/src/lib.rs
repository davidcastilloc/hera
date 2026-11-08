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

        // Upgrade DJR2TrialUser to `pro_monthly` (50 GB = 51,200 MB R2 quota)
        let req_up = Request::builder()
            .method("POST")
            .uri("/api/v1/dj/upgrade-tier")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "dj_handle": "DJR2TrialUser",
                    "tier": "pro_monthly"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_up = app.call(req_up).await.unwrap();
        assert_eq!(resp_up.status(), StatusCode::OK);
        let j_up: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_up.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_up["profile"]["r2_quota_mb"], 51200.0);

        // Verify Cloudflare Beta Economics endpoint (`GET /api/v1/cloud/economics?djs=10&gb_per_dj=5`)
        let req_econ = Request::builder()
            .uri("/api/v1/cloud/economics?djs=10&gb_per_dj=5")
            .body(Body::empty())
            .unwrap();
        let resp_econ = app.call(req_econ).await.unwrap();
        assert_eq!(resp_econ.status(), StatusCode::OK);
        let j_econ: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_econ.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_econ["cohort_djs_count"], 10);
        assert_eq!(j_econ["total_r2_storage_gb"], 50.0);
        // 50 GB total - 10 GB Free R2 = 40 GB billable * $0.015 = $0.60 USD/month total ($0.06/DJ)!
        assert_eq!(j_econ["total_monthly_cohort_cost_usd"], 0.6);

        // 3. Test Local Laptop / Server Direct Storage & P2P Seeder (`POST /api/v1/node/scan-local-dir` + `GET /api/v1/p2p/download/:track_id`)
        let temp_dir = std::env::temp_dir().join("hera_local_p2p_test_dir");
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

        // 6. Test Safe Atomic Hot/Cold R2 Eviction (`POST /api/v1/broker/optimize-tiering`)
        let req_tier = Request::builder()
            .method("POST")
            .uri("/api/v1/broker/optimize-tiering")
            .body(Body::empty())
            .unwrap();
        let resp_tier = app.call(req_tier).await.unwrap();
        assert_eq!(resp_tier.status(), StatusCode::OK);
        let j_tier: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_tier.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_tier["status"], "hot_cold_tier_optimized");
        // Since `Ritual Del Sol` has a verified Google Drive backup replica from DaviDJ,
        // the redundant R2 replica from DJR2TrialUser is safely hibernated, freeing 38 MB in R2!
        assert_eq!(j_tier["cold_r2_replicas_hibernated"], 1);
        assert_eq!(j_tier["r2_megabytes_freed"], 38.0);

        // 7. Test Zero-Copy Cloud P2P Track Claim + Curator Quota Refund Bounty (`POST /api/v1/broker/claim-cloud-track`)
        let req_claim = Request::builder()
            .method("POST")
            .uri("/api/v1/broker/claim-cloud-track")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "claimer_dj": "DJCloudSubscriber",
                    "target_track_or_cas": "hera-seed-001"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_claim = app.call(req_claim).await.unwrap();
        assert_eq!(resp_claim.status(), StatusCode::CREATED);
        let j_claim: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp_claim.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(j_claim["status"], "p2p_cloud_track_claimed_zero_copy");
        assert_eq!(j_claim["physical_r2_mb_consumed"], 0.0);
        assert_eq!(j_claim["pioneer_bounty_refunded_mb"], 38.0);
        assert_eq!(
            j_claim["linked_track"]["storage_provider"],
            "r2_cas_zero_copy_link"
        );
        assert_eq!(j_claim["linked_track"]["share_on_p2p"], true);

        // 8. Test P2P Sharing ON by default, and ability to turn it OFF only on higher plans (`pro_monthly`)
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
        // Forced ON because `beta_mvp` requires P2P sharing by default!
        assert_eq!(j_free_dj["profile"]["p2p_sharing_enabled"], true);
        assert_eq!(j_free_dj["profile"]["can_disable_p2p_sharing"], false);
        assert_eq!(j_free_dj["admitted_tracks"][0]["share_on_p2p"], true);

        // 8a. Attempting to turn off P2P sharing while on `beta_mvp` returns 403 FORBIDDEN (`PLAN_UPGRADE_REQUIRED`)
        let req_toggle_blocked = Request::builder()
            .method("POST")
            .uri("/api/v1/dj/p2p-sharing")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "dj_handle": "DJBetaShared",
                    "p2p_sharing_enabled": false
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_toggle_blocked = app.call(req_toggle_blocked).await.unwrap();
        assert_eq!(resp_toggle_blocked.status(), StatusCode::FORBIDDEN);

        // 8b. Upgrade `DJBetaShared` to `pro_monthly` -> now they CAN turn P2P sharing off (Private Vault mode) or back on!
        let req_up_pro = Request::builder()
            .method("POST")
            .uri("/api/v1/dj/upgrade-tier")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "dj_handle": "DJBetaShared",
                    "tier": "pro_monthly"
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_up_pro = app.call(req_up_pro).await.unwrap();
        assert_eq!(resp_up_pro.status(), StatusCode::OK);

        let req_toggle_ok = Request::builder()
            .method("POST")
            .uri("/api/v1/dj/p2p-sharing")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "dj_handle": "DJBetaShared",
                    "p2p_sharing_enabled": false
                }))
                .unwrap(),
            ))
            .unwrap();
        let resp_toggle_ok = app.call(req_toggle_ok).await.unwrap();
        assert_eq!(resp_toggle_ok.status(), StatusCode::OK);
        let j_toggle_ok: serde_json::Value = serde_json::from_slice(
            &to_bytes(resp_toggle_ok.into_body(), 1024 * 1024)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(j_toggle_ok["profile"]["p2p_sharing_enabled"], false);
        assert_eq!(j_toggle_ok["profile"]["can_disable_p2p_sharing"], true);
    }
}
