use crate::code_mode::{execute_code_mode, get_typescript_sdk_definitions, CodeModeRequest};
use crate::forensics::{audit_pcm_channels, decode_audio_bytes};
use crate::p2p::{
    compute_hera_seal, get_cloud_referral_catalog, ConnectDjDriveRequest, CustodyAuditSubmission,
    CustodyChallengeRequest, P2PMeshState, P2PSearchQuery, P2PVerifiedTrack,
    SwarmHeartbeatRequest, SwarmRouteRequest,
};
use crate::router::{current_runtime_target, AnnounceTrackRequest};
use crate::structure::analyze_structure_and_style;
use crate::synergy::{
    optimize_crate_with_spine, CrateTrackCandidate, SetSpineConfig, SpineAnchor,
};
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
                "name": "ensure_hera_updated",
                "description": "[MANDATORY STEP 0] Verify if Hera repository, native Rust binary (hera-node), Python environment, companion binaries (slskd, rclone, ffmpeg), and global AI agent skills are installed and updated to the latest version, auto-updating them if needed.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "auto_compile_rust": { "type": "boolean", "default": true },
                        "sync_skills": { "type": "boolean", "default": true },
                        "pull_remote": { "type": "boolean", "default": true }
                    }
                }
            },
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
            },
            {
                "name": "connect_dj_drive",
                "description": "Zero-Tech DJ Onboarding: Connect a non-technical DJ (`DJFoo`) to the P2P network by simply passing their DJ handle and a Google Drive folder link (or Cloudflare R2 bucket). Automatically audits tracks, quarantines fake FLACs, and grants access to the shared DJ repertoire.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "dj_handle": { "type": "string", "description": "DJ name/handle (e.g. 'DJFoo')" },
                        "display_name": { "type": "string" },
                        "gdrive_folder_link": { "type": "string", "description": "Any Google Drive folder share link" },
                        "r2_bucket_url": { "type": "string", "description": "Optional Cloudflare R2 public URL" },
                        "tracks": { "type": "array", "items": { "type": "object" }, "description": "Optional list of tracks in the DJ's folder" }
                    },
                    "required": ["dj_handle"]
                }
            },
            {
                "name": "get_swarm_strength",
                "description": "Compute Federated R2 Edge Swarm strength, Constrained Water-Filling shard allocations (9,000 MB R2 & 85,000 req/day safety caps), and elastic thermal replication K across all healthy Edge Guardian nodes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "swarm_heartbeat",
                "description": "Register or update an Edge Guardian node heartbeat, credit 1:1.5 Turbo Credits for megabytes served, and return Constrained Water-Filling allocations + outbound pull tickets.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node_id": { "type": "string" },
                        "owner_dj": { "type": "string" },
                        "endpoint_url": { "type": "string" },
                        "r2_used_mb": { "type": "number" },
                        "daily_worker_requests_used": { "type": "integer" },
                        "megabytes_served_delta": { "type": "number" }
                    },
                    "required": ["node_id", "owner_dj", "endpoint_url"]
                }
            },
            {
                "name": "swarm_route_ticket",
                "description": "Route a track/shard download via Power-of-Two-Choices (P2C) to the healthiest Edge Guardian node with lowest request load, returning a single-use 60-second HMAC-signed X-Hera-Swarm-Ticket.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "requester_dj": { "type": "string" },
                        "track_id_or_cas": { "type": "string" },
                        "block_index": { "type": "integer" }
                    },
                    "required": ["requester_dj", "track_id_or_cas"]
                }
            },
            {
                "name": "swarm_custody_audit",
                "description": "Issue or verify a 64-byte (Range: bytes=offset..offset+63) Proof-of-Custody challenge against an Edge Guardian node's R2 bucket.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node_id": { "type": "string" },
                        "cas_fingerprint_or_blob": { "type": "string" },
                        "opaque_blob_key": { "type": "string" },
                        "byte_offset": { "type": "integer" },
                        "http_status": { "type": "integer" },
                        "returned_slice_sha256": { "type": "string" }
                    },
                    "required": ["node_id"]
                }
            },
            {
                "name": "swarm_parallel_manifest",
                "description": "Issue an HTTP/3 QUIC Multi-Node Parallel Striping Manifest that stripes all 4 MB aligned blocks of a Studio Master across distinct low-latency Edge Guardian nodes using Latency-Aware P2C.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "requester_dj": { "type": "string" },
                        "track_id_or_cas": { "type": "string" },
                        "file_size_mb": { "type": "number" }
                    },
                    "required": ["requester_dj", "track_id_or_cas"]
                }
            },
            {
                "name": "swarm_crdt_sync",
                "description": "Synchronize an offline-first Delta-CRDT state (GCounter + LWW-Element-Set) with commutative, associative, and idempotent lattice convergence.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "origin_node_id": { "type": "string" },
                        "origin_endpoint_pubkey_id": { "type": "string" },
                        "hlc_timestamp_ms": { "type": "integer" },
                        "gcounter_node_bytes_served_mb": { "type": "object" },
                        "gcounter_cas_play_counts": { "type": "object" },
                        "lww_track_entries": { "type": "object" }
                    }
                }
            },
            {
                "name": "swarm_subscribe",
                "description": "Subscribe an Edge Guardian or DJ node to an MoQ-style harmonic topic (e.g. hera://swarm/8A/120-128) for proactive low-latency track notifications and pre-fetching.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "subscriber_node_id": { "type": "string" },
                        "subscriber_dj": { "type": "string" },
                        "camelot_key": { "type": "string" },
                        "bpm_min": { "type": "number" },
                        "bpm_max": { "type": "number" },
                        "subgenre_filter": { "type": "string" }
                    },
                    "required": ["subscriber_node_id", "subscriber_dj", "camelot_key"]
                }
            },
            {
                "name": "export_dj_crate",
                "description": "Pure-Rust native exporter for VirtualDJ (.vdjfolder + _vdj_database.xml with 4 <Poi> Hot Cues), Pioneer Rekordbox XML, Native Instruments Traktor Pro (.nml with <CUE_V2>), and M3U8.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "crate_name": { "type": "string" },
                        "tracks": { "type": "array", "items": { "type": "object" } },
                        "formats": {
                            "type": "array",
                            "items": { "type": "string", "enum": ["virtualdj", "rekordbox", "traktor", "m3u8"] }
                        },
                        "output_dir": { "type": "string" }
                    },
                    "required": ["crate_name", "tracks"]
                }
            },
            {
                "name": "compute_dj_style_dna",
                "description": "Zero-Typing DJ Style DNA Auto-Profiler & Persistent DJ Brain generator in Pure Rust (computes BPM zones, dominant Camelot keys, minor key ratio, top artists, and ~250-token DJ Brain prompt block).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "dj_handle": { "type": "string" },
                        "preferred_software": { "type": "string", "default": "virtualdj" },
                        "preferred_transition_style": { "type": "string", "default": "open_format_4_hotcues" },
                        "golden_rules": { "type": "array", "items": { "type": "string" } },
                        "tracks": { "type": "array", "items": { "type": "object" } }
                    },
                    "required": ["dj_handle", "tracks"]
                }
            },
            {
                "name": "chat_with_dj_copilot",
                "description": "Evaluate a DJ Copilot conversational turn using Cloudflare Workers AI Free Tier (@cf/meta/llama-3.1-8b-instruct-fp8-fast, 10,000 Neurons/day free at $0.00, 6.6 Neurons/turn, AI Gateway cache & :free fallback).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "dj_handle": { "type": "string" },
                        "message": { "type": "string" },
                        "backend": { "type": "string", "default": "cloudflare_free" },
                        "model": { "type": "string" },
                        "daily_neurons_used_so_far": { "type": "number", "default": 0.0 }
                    },
                    "required": ["dj_handle", "message"]
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
        "ensure_hera_updated" => {
            let pull_remote = args
                .get("pull_remote")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let mut git_pulled = false;
            #[cfg(not(target_arch = "wasm32"))]
            if pull_remote {
                if let Ok(output) = std::process::Command::new("git")
                    .args(["pull", "--rebase", "origin", "main", "--quiet"])
                    .output()
                {
                    git_pulled = output.status.success();
                }
            }
            let _ = pull_remote;
            Ok(json!({
                "status": "ok",
                "engine": "hera-node-rs (100% Pure Rust)",
                "version": env!("CARGO_PKG_VERSION"),
                "runtime_target": current_runtime_target(),
                "node_id": state.node_id,
                "git_pulled": git_pulled,
                "python_dependency": "none (0 MB)"
            }))
        }
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
                owner_dj: args
                    .get("owner_dj")
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
                owner_dj: None,
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
                    owner_dj: None,
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

            let anchors: Vec<SpineAnchor> = args
                .get("anchors")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let custom_energy_curve: Vec<f32> = args
                .get("custom_energy_curve")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let spine = SetSpineConfig {
                anchors,
                custom_energy_curve,
            };
            let res = optimize_crate_with_spine(brief, energy_arc, candidates, max_tracks, &spine);
            Ok(serde_json::to_value(res).map_err(|e| e.to_string())?)
        }
        "audit_repertoire" => {
            let catalog_tracks = state.search(&P2PSearchQuery {
                q: None,
                camelot: None,
                bpm_min: None,
                bpm_max: None,
                subgenre: None,
                owner_dj: None,
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
        "connect_dj_drive" => {
            let req: ConnectDjDriveRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `connect_dj_drive` payload: {e}"))?;
            let resp = state.connect_dj_drive(req);
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }
        "get_swarm_strength" => {
            let report = state.compute_swarm_strength();
            Ok(serde_json::to_value(report).map_err(|e| e.to_string())?)
        }
        "swarm_heartbeat" => {
            let req: SwarmHeartbeatRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `swarm_heartbeat` payload: {e}"))?;
            let resp = state.process_swarm_heartbeat(req)?;
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }
        "swarm_route_ticket" => {
            let req: SwarmRouteRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `swarm_route_ticket` payload: {e}"))?;
            let resp = state.issue_route_ticket(req)?;
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }
        "swarm_custody_audit" => {
            if args.get("http_status").is_some() {
                let sub: CustodyAuditSubmission = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid `swarm_custody_audit` submission: {e}"))?;
                let res = state.verify_custody_audit(sub);
                Ok(serde_json::to_value(res).map_err(|e| e.to_string())?)
            } else {
                let req = CustodyChallengeRequest {
                    node_id: args
                        .get("node_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("hera-edge-node")
                        .to_string(),
                    cas_fingerprint_or_blob: args
                        .get("cas_fingerprint_or_blob")
                        .or_else(|| args.get("opaque_blob_key"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("hera-seed-001")
                        .to_string(),
                    seed_nonce: args.get("seed_nonce").and_then(|v| v.as_u64()),
                };
                let chal = state.generate_custody_challenge(req)?;
                Ok(serde_json::to_value(chal).map_err(|e| e.to_string())?)
            }
        }
        "swarm_parallel_manifest" => {
            let req: crate::p2p::SwarmParallelManifestRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `swarm_parallel_manifest` payload: {e}"))?;
            let resp = state.issue_parallel_swarm_manifest(req)?;
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }
        "swarm_crdt_sync" => {
            let req: crate::p2p::SwarmCrdtDelta = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `swarm_crdt_sync` payload: {e}"))?;
            let resp = state.sync_crdt_delta(req);
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }
        "swarm_subscribe" => {
            let req: crate::p2p::SwarmSubscribeRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `swarm_subscribe` payload: {e}"))?;
            let resp = state.subscribe_semantic_topic(req);
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }
        "export_dj_crate" => {
            let req: crate::export::ExportCrateRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `export_dj_crate` payload: {e}"))?;
            let resp = crate::export::export_crate_bundle(&req);
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }
        "compute_dj_style_dna" => {
            let req: crate::dj_brain::DjStyleDnaRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `compute_dj_style_dna` payload: {e}"))?;
            let resp = crate::dj_brain::compute_dj_style_dna(&req);
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }
        "chat_with_dj_copilot" => {
            let req: crate::dj_brain::CloudflareFreeAiTurnRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `chat_with_dj_copilot` payload: {e}"))?;
            let resp = crate::dj_brain::evaluate_cloudflare_free_ai_turn(&req);
            Ok(serde_json::to_value(resp).map_err(|e| e.to_string())?)
        }
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
            let decoded = decode_audio_bytes(&bytes, Some(ext), 90.0)?;
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
