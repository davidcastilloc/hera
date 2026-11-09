use serde_json::{json, Value};

pub(super) fn core_tool_schemas() -> Vec<Value> {
    vec![
        json!({
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
        }),
        json!({
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
        }),
        json!({
            "name": "get_track_candidates",
            "description": "Get ranked P2P track candidates with transparent forensic score breakdown (bandwidth Hz, sub-bass phase correlation, quality score, and hera_seal).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "default": 10 }
                }
            }
        }),
        json!({
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
        }),
        json!({
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
        }),
        json!({
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
        }),
        json!({
            "name": "audit_repertoire",
            "description": "Audit the node's P2P repertoire (and optional local folder) for fake lossless transcodes, synthetic sine tones, and <120Hz sub-bass phase cancellation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "directory_path": { "type": "string", "description": "Optional local directory to scan" }
                }
            }
        }),
        json!({
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
        }),
        json!({
            "name": "get_node_manifest",
            "description": "Return Hera Rust node status, runtime target (Cloudflare Workers WASM vs Native VPS), memory footprint, and 1-Click Cloud Referral options.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
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
        }),
        json!({
            "name": "get_code_mode_sdk",
            "description": "Return the TypeScript `.d.ts` ambient SDK declarations for `hera.*` Code Mode so the agent can write strongly-typed scripts.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
    ]
}
