use serde_json::{json, Value};

pub(super) fn swarm_tool_schemas() -> Vec<Value> {
    vec![
        json!({
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
        }),
        json!({
            "name": "get_swarm_strength",
            "description": "Compute Federated R2 Edge Swarm strength, Constrained Water-Filling shard allocations (9,000 MB R2 & 85,000 req/day safety caps), and elastic thermal replication K across all healthy Edge Guardian nodes.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
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
        }),
        json!({
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
        }),
        json!({
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
        }),
        json!({
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
        }),
        json!({
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
        }),
        json!({
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
        }),
        json!({
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
        }),
        json!({
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
        }),
        json!({
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
        }),
    ]
}
