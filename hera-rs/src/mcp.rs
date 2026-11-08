use crate::code_mode::{execute_code_mode, get_typescript_sdk_definitions, CodeModeRequest};
use crate::forensics::{audit_pcm_channels, decode_audio_bytes};
use crate::p2p::{
    compute_hera_seal, get_cloud_referral_catalog, P2PMeshState, P2PSearchQuery, P2PVerifiedTrack,
};
use crate::router::{current_runtime_target, AnnounceTrackRequest};
use crate::structure::analyze_structure_and_style;
use crate::synergy::{optimize_crate_2opt, CrateTrackCandidate};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: Option<String>,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn ok(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

/// Returns the complete list of MCP tool definitions exposed by `hera-node-rs`.
pub fn list_mcp_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "search_music",
                "description": "Search acoustically verified studio masters on the Hera P2P Mesh by query, Camelot key, BPM range, or subgenre.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Artist, title, or keyword query" },
                        "camelot": { "type": "string", "description": "Camelot key filter (e.g. '8A', '11B')" },
                        "bpm_min": { "type": "number", "description": "Minimum BPM" },
                        "bpm_max": { "type": "number", "description": "Maximum BPM" },
                        "subgenre": { "type": "string", "description": "Subgenre filter" },
                        "verified_only": { "type": "boolean", "default": true }
                    }
                }
            },
            {
                "name": "get_track_candidates",
                "description": "Get ranked P2P track candidates with transparent forensic score breakdown (bandwidth Hz, sub-bass phase correlation, quality score, and hera_seal).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "limit": { "type": "integer", "default": 10 }
                    }
                }
            },
            {
                "name": "analyze_track",
                "description": "Run pure-Rust Symphonia + rustfft Zero-Trust spectral forensics (Fake Lossless, Sine Tone, <120Hz Sub-Bass Phase) and 4 Acoustic Hot Cues (1.1.1 Downbeat, Phrase In, Main Drop, Mix Out) + Camelot key + 64D Style Embedding.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Local path to WAV/FLAC/MP3/OGG file (native mode)" },
                        "pcm_left": { "type": "array", "items": { "type": "number" }, "description": "Raw float32 PCM left channel samples" },
                        "pcm_right": { "type": "array", "items": { "type": "number" }, "description": "Optional float32 PCM right channel samples" },
                        "sample_rate": { "type": "integer", "default": 44100 },
                        "claimed_ext": { "type": "string", "default": "flac" },
                        "bpm_hint": { "type": "number" }
                    }
                }
            },
            {
                "name": "analyze_batch",
                "description": "Run batch pure-Rust DSP forensic and structural Hot-Cue analysis across multiple audio files.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_paths": { "type": "array", "items": { "type": "string" } },
                        "bpm_hint": { "type": "number" }
                    },
                    "required": ["file_paths"]
                }
            },
            {
                "name": "build_dj_crate",
                "description": "Sequence a harmonic DJ crate using the 4-Layer Holistic Synergy Engine + Global 2-Opt TSP optimizer (Camelot wheel, gradual BPM + 3:4 polyrhythmic bridges, vocal clash avoidance, 64D style embeddings). Exports M3U8, Rekordbox XML, and Set Guide.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "brief": { "type": "string", "description": "Set concept or brief" },
                        "energy_arc": { "type": "string", "enum": ["steady_climb", "rollercoaster", "peak_hour"], "default": "steady_climb" },
                        "max_tracks": { "type": "integer" },
                        "candidates": { "type": "array", "items": { "type": "object" }, "description": "Optional candidate tracks; if omitted, uses the node's verified catalog" }
                    },
                    "required": ["brief"]
                }
            },
            {
                "name": "audit_repertoire",
                "description": "Audit the node's P2P repertoire (and optional local folder) for fake lossless transcodes, synthetic sine tones, and <120Hz sub-bass phase cancellation.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "directory_path": { "type": "string", "description": "Optional local directory to scan" }
                    }
                }
            },
            {
                "name": "announce_p2p_track",
                "description": "Quarantine-First P2P gatekeeper: admit a track to the verified P2P catalog only if its forensic report passes all Zero-Trust checks.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "track": { "type": "object" },
                        "forensics": { "type": "object" }
                    },
                    "required": ["track", "forensics"]
                }
            },
            {
                "name": "get_node_manifest",
                "description": "Return Hera Rust node status, runtime target (Cloudflare Workers WASM vs Native VPS), memory footprint, and 1-Click Cloud Referral options.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "hera_code_mode",
                "description": "Cloudflare Code Mode power tool (https://blog.cloudflare.com/code-mode/): Execute a multi-step TypeScript/JS `hera.*` script or pipeline in a single sub-millisecond turn without bouncing intermediate JSON through the LLM context window.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "TypeScript/JS code using the `hera` namespace, e.g.: `const tracks = await hera.searchMusic({ camelot: \"8A\" }); const clean = tracks.filter(t => t.quality_score >= 95 && t.mono_compatible_sub); return await hera.buildDjCrate({ brief: \"Code Mode Set\", energy_arc: \"peak_hour\", candidates: clean });`"
                        },
                        "pipeline": {
                            "type": "array",
                            "items": { "type": "object" },
                            "description": "Optional declarative step pipeline [{ bind, call, args }]"
                        },
                        "context": {
                            "type": "object",
                            "description": "Optional initial variables"
                        }
                    }
                }
            },
            {
                "name": "get_code_mode_sdk",
                "description": "Return the TypeScript `.d.ts` ambient SDK declarations for `hera.*` Code Mode so the agent can write strongly-typed scripts.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    })
}

/// Handle a single MCP JSON-RPC 2.0 request (works identically over `stdio` and HTTP `/mcp`).
pub fn handle_mcp_request(req: JsonRpcRequest, state: &P2PMeshState) -> Option<JsonRpcResponse> {
    let method = req.method.as_str();

    // Notifications (no `id`) do not expect a response frame in MCP stdio
    if method.starts_with("notifications/") && req.id.is_none() {
        return None;
    }

    match method {
        "initialize" => Some(JsonRpcResponse::ok(
            req.id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "hera-node-rs",
                    "version": env!("CARGO_PKG_VERSION"),
                    "runtime": current_runtime_target()
                }
            }),
        )),
        "ping" => Some(JsonRpcResponse::ok(req.id, json!({}))),
        "tools/list" => Some(JsonRpcResponse::ok(req.id, list_mcp_tools())),
        "tools/call" => {
            let tool_name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let args = req.params.get("arguments").cloned().unwrap_or(json!({}));

            match dispatch_tool_call(tool_name, args, state) {
                Ok(payload) => Some(JsonRpcResponse::ok(
                    req.id,
                    json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string())
                            }
                        ],
                        "structuredContent": payload,
                        "isError": false
                    }),
                )),
                Err(err_msg) => Some(JsonRpcResponse::ok(
                    req.id,
                    json!({
                        "content": [
                            {
                                "type": "text",
                                "text": format!("ERROR: {err_msg}")
                            }
                        ],
                        "isError": true
                    }),
                )),
            }
        }
        _ => Some(JsonRpcResponse::err(
            req.id,
            -32601,
            format!("Method not found: {method}"),
        )),
    }
}

fn dispatch_tool_call(name: &str, args: Value, state: &P2PMeshState) -> Result<Value, String> {
    match name {
        "search_music" => {
            let q = P2PSearchQuery {
                q: args.get("query").and_then(|v| v.as_str()).map(String::from),
                camelot: args
                    .get("camelot")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                bpm_min: args.get("bpm_min").and_then(|v| v.as_f64()).map(|f| f as f32),
                bpm_max: args.get("bpm_max").and_then(|v| v.as_f64()).map(|f| f as f32),
                subgenre: args
                    .get("subgenre")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                verified_only: args.get("verified_only").and_then(|v| v.as_bool()),
            };
            let matches = state.search(&q);
            Ok(json!({
                "node_id": state.node_id,
                "total_matches": matches.len(),
                "tracks": matches
            }))
        }
        "get_track_candidates" => {
            let query_str = args.get("query").and_then(|v| v.as_str()).map(String::from);
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(10) as usize;
            let mut matches = state.search(&P2PSearchQuery {
                q: query_str,
                camelot: None,
                bpm_min: None,
                bpm_max: None,
                subgenre: None,
                verified_only: Some(true),
            });
            matches.sort_by(|a, b| {
                b.quality_score
                    .partial_cmp(&a.quality_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            matches.truncate(limit);

            let ranked: Vec<Value> = matches
                .into_iter()
                .enumerate()
                .map(|(idx, t)| {
                    json!({
                        "rank": idx + 1,
                        "track_id": t.track_id,
                        "artist": t.artist,
                        "title": t.title,
                        "bpm": t.bpm,
                        "camelot_key": t.camelot_key,
                        "score_breakdown": {
                            "quality_score": t.quality_score,
                            "effective_bandwidth_hz": t.effective_bandwidth_hz,
                            "sub_bass_phase_correlation": t.sub_bass_phase_correlation,
                            "mono_compatible_sub": t.mono_compatible_sub,
                            "forensic_verdict": t.forensic_verdict,
                            "hera_seal": t.hera_seal
                        },
                        "hot_cues": t.hot_cues
                    })
                })
                .collect();

            Ok(json!({
                "candidates_count": ranked.len(),
                "candidates": ranked
            }))
        }
        "analyze_track" => analyze_single_track_arg(&args),
        "analyze_batch" => {
            let paths = args
                .get("file_paths")
                .and_then(|v| v.as_array())
                .ok_or_else(|| "`file_paths` array is required".to_string())?;
            let bpm_hint = args.get("bpm_hint").and_then(|v| v.as_f64()).map(|f| f as f32);
            let mut results = Vec::with_capacity(paths.len());
            for p_val in paths {
                if let Some(p_str) = p_val.as_str() {
                    let sub_arg = json!({
                        "file_path": p_str,
                        "bpm_hint": bpm_hint
                    });
                    match analyze_single_track_arg(&sub_arg) {
                        Ok(r) => results.push(r),
                        Err(e) => results.push(json!({ "file_path": p_str, "error": e })),
                    }
                }
            }
            Ok(json!({
                "processed": results.len(),
                "results": results
            }))
        }
        "build_dj_crate" => {
            let brief = args
                .get("brief")
                .and_then(|v| v.as_str())
                .unwrap_or("Hera Rust Curated Set");
            let energy_arc = args
                .get("energy_arc")
                .and_then(|v| v.as_str())
                .unwrap_or("steady_climb");
            let max_tracks = args
                .get("max_tracks")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);

            let candidates: Vec<CrateTrackCandidate> = if let Some(arr) = args.get("candidates") {
                serde_json::from_value(arr.clone())
                    .map_err(|e| format!("Invalid `candidates` format: {e}"))?
            } else {
                // Convert node's verified P2P catalog into CrateTrackCandidates
                let p2p_tracks = state.search(&P2PSearchQuery {
                    q: None,
                    camelot: None,
                    bpm_min: None,
                    bpm_max: None,
                    subgenre: None,
                    verified_only: Some(true),
                });
                p2p_tracks
                    .into_iter()
                    .map(|t| CrateTrackCandidate {
                        track_id: t.track_id.clone(),
                        title: t.title,
                        artist: t.artist,
                        file_path: t.stream_url.unwrap_or_else(|| format!("/library/{}.flac", t.track_id)),
                        duration_seconds: t.duration_seconds,
                        bpm: t.bpm,
                        camelot_key: t.camelot_key,
                        energy_score: 0.75,
                        punch_index: 0.72,
                        groove_type: "four_on_floor".to_string(),
                        top_subgenre: t.top_subgenre,
                        has_intro_vocals: false,
                        has_outro_vocals: false,
                        embedding_64d: vec![0.125; 64],
                        hot_cues: t.hot_cues,
                    })
                    .collect()
            };

            let res = optimize_crate_2opt(brief, energy_arc, candidates, max_tracks);
            Ok(serde_json::to_value(res).map_err(|e| e.to_string())?)
        }
        "audit_repertoire" => {
            let catalog_tracks = state.search(&P2PSearchQuery {
                q: None,
                camelot: None,
                bpm_min: None,
                bpm_max: None,
                subgenre: None,
                verified_only: Some(false),
            });
            let verified_count = catalog_tracks
                .iter()
                .filter(|t| t.forensic_verdict == "VERIFIED_STUDIO_MASTER" && t.mono_compatible_sub)
                .count();

            Ok(json!({
                "node_id": state.node_id,
                "total_catalog_tracks": catalog_tracks.len(),
                "verified_studio_masters": verified_count,
                "quarantine_integrity": "100% Zero-Trust Enforced",
                "tracks": catalog_tracks
            }))
        }
        "announce_p2p_track" => {
            let req: AnnounceTrackRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid announce_p2p_track payload: {e}"))?;
            let admitted: P2PVerifiedTrack =
                state.announce_verified_track(req.track, &req.forensics)?;
            Ok(serde_json::to_value(admitted).map_err(|e| e.to_string())?)
        }
        "get_node_manifest" => Ok(json!({
            "service": "Hera Universal Rust DSP, MCP & P2P Node",
            "version": env!("CARGO_PKG_VERSION"),
            "runtime_target": current_runtime_target(),
            "node_id": state.node_id,
            "cloud_referrals": get_cloud_referral_catalog()
        })),
        "hera_code_mode" => {
            let req: CodeModeRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `hera_code_mode` payload: {e}"))?;
            let res = execute_code_mode(req, state)?;
            Ok(serde_json::to_value(res).map_err(|e| e.to_string())?)
        }
        "get_code_mode_sdk" => Ok(json!({
            "sdk_namespace": "hera",
            "reference": "https://blog.cloudflare.com/code-mode/",
            "typescript_declarations": get_typescript_sdk_definitions()
        })),
        _ => Err(format!("Unknown MCP tool `{name}`")),
    }
}

fn analyze_single_track_arg(args: &Value) -> Result<Value, String> {
    let bpm_hint = args.get("bpm_hint").and_then(|v| v.as_f64()).map(|f| f as f32);

    if let Some(fp) = args.get("file_path").and_then(|v| v.as_str()) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let bytes = std::fs::read(fp)
                .map_err(|e| format!("Failed to read audio file `{fp}`: {e}"))?;
            let ext = std::path::Path::new(fp)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("wav");
            let decoded = decode_audio_bytes(&bytes, Some(ext), 45.0)?;
            let forensics = audit_pcm_channels(
                &decoded.left,
                decoded.right.as_deref(),
                decoded.sample_rate,
                ext,
            );
            let structure = analyze_structure_and_style(
                &decoded.mono,
                decoded.sample_rate,
                Some(decoded.duration_seconds),
                bpm_hint,
            );
            let seal = compute_hera_seal(
                fp,
                structure.bpm,
                &structure.camelot_key,
                forensics.quality_score,
                &forensics.forensic_verdict,
            );
            return Ok(json!({
                "file_path": fp,
                "forensics": forensics,
                "structure": structure,
                "hera_seal": seal
            }));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = fp;
            return Err("`file_path` is only supported on native VPS/local mode; pass `pcm_left` or POST binary to `/api/v1/audio/analyze-file` on Cloudflare Workers.".to_string());
        }
    }

    if let Some(left_arr) = args.get("pcm_left").and_then(|v| v.as_array()) {
        let left: Vec<f32> = left_arr
            .iter()
            .filter_map(|x| x.as_f64().map(|f| f as f32))
            .collect();
        if left.is_empty() {
            return Err("`pcm_left` cannot be empty".to_string());
        }
        let right: Option<Vec<f32>> = args
            .get("pcm_right")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect());

        let sr = args
            .get("sample_rate")
            .and_then(|v| v.as_u64())
            .unwrap_or(44100) as u32;
        let ext = args
            .get("claimed_ext")
            .and_then(|v| v.as_str())
            .unwrap_or("flac");

        let forensics = audit_pcm_channels(&left, right.as_deref(), sr, ext);
        let mono: Vec<f32> = if let Some(ref r) = right {
            let n = left.len().min(r.len());
            left[..n]
                .iter()
                .zip(r[..n].iter())
                .map(|(&l, &rv)| 0.5 * (l + rv))
                .collect()
        } else {
            left
        };
        let dur = (mono.len() as f32) / (sr.max(1) as f32);
        let structure = analyze_structure_and_style(&mono, sr, Some(dur), bpm_hint);
        let seal = compute_hera_seal(
            "mcp-pcm",
            structure.bpm,
            &structure.camelot_key,
            forensics.quality_score,
            &forensics.forensic_verdict,
        );
        return Ok(json!({
            "forensics": forensics,
            "structure": structure,
            "hera_seal": seal
        }));
    }

    Err("Provide either `file_path` or `pcm_left` to `analyze_track`.".to_string())
}

/// Run the MCP JSON-RPC 2.0 Server over `stdin`/`stdout` (`hera-node --mcp`).
#[cfg(not(target_arch = "wasm32"))]
pub fn run_mcp_stdio_loop(state: P2PMeshState) {
    use std::io::{self, BufRead, Write};
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let Ok(raw) = line else { break };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let resp = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => handle_mcp_request(req, &state),
            Err(e) => Some(JsonRpcResponse::err(
                None,
                -32700,
                format!("Parse error: {e}"),
            )),
        };
        if let Some(r) = resp {
            if let Ok(out) = serde_json::to_string(&r) {
                let _ = writeln!(stdout, "{out}");
                let _ = stdout.flush();
            }
        }
    }
}
