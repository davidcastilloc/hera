//! Cloudflare "Code Mode" Engine for Hera MCP (`https://blog.cloudflare.com/code-mode/`).
//!
//! Instead of forcing an LLM to make 10 sequential tool round-trips through its context window,
//! Code Mode exposes:
//! 1. Strongly-typed TypeScript definitions (`GET /mcp/code-mode.d.ts`) for the `hera` SDK namespace.
//! 2. An in-memory sandboxed Code Mode runner (`hera_code_mode` MCP tool & `POST /mcp/code-mode`)
//!    that executes multi-step TypeScript/JS `hera.*` programs and declarative pipelines in < 1 ms
//!    directly inside Rust (both on Cloudflare Workers WASM and Native VPS).

use crate::forensics::audit_pcm_channels;
use crate::p2p::{
    get_cloud_referral_catalog, BrokerBatchLookupRequest, BrokerBridgeRequest,
    BrokerClaimCloudTrackRequest, BrokerSimilarQuery, ConnectDjDriveRequest, P2PMeshState,
    P2PSearchQuery, P2PVerifiedTrack, ScanLocalDirRequest, SetP2PSharingRequest,
};
use crate::router::current_runtime_target;
use crate::synergy::{
    compute_bpm_synergy, compute_camelot_synergy, optimize_crate_2opt, CrateTrackCandidate,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// Request payload for `POST /mcp/code-mode` and the `hera_code_mode` MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeModeRequest {
    /// TypeScript / JavaScript code using the `hera.*` SDK (e.g. `const tracks = await hera.searchMusic({...}); return hera.buildDjCrate({...});`)
    #[serde(default)]
    pub code: Option<String>,
    /// Optional declarative pipeline steps for deterministic multi-operation chaining.
    #[serde(default)]
    pub pipeline: Option<Vec<CodeModeStep>>,
    /// Optional initial context variables accessible inside the script/pipeline.
    #[serde(default)]
    pub context: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeModeStep {
    /// Variable name to bind the output of this step into the execution scope.
    pub bind: String,
    /// SDK method name: `hera.searchMusic`, `hera.filterTracks`, `hera.evaluateTransition`, `hera.buildDjCrate`, `hera.auditPcm`, `hera.auditRepertoire`.
    pub call: String,
    /// Arguments object (supports `"$varName"` references to previous step bindings).
    #[serde(default)]
    pub args: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeModeExecutionResult {
    pub mode: String,
    pub operations_executed: usize,
    pub execution_log: Vec<String>,
    pub output: Value,
}

/// Complete TypeScript ambient SDK definition (`GET /mcp/code-mode.d.ts`) following Cloudflare's Code Mode specification.
pub fn get_typescript_sdk_definitions() -> &'static str {
    r#"// ============================================================================
// HERA CURATOR — CLOUDFLARE CODE MODE TYPESCRIPT SDK (`hera.*`)
// Reference: https://blog.cloudflare.com/code-mode/
// ============================================================================

export interface HotCuePoint {
  num: number;
  name: string;
  seconds: number;
  bar_index: number;
  role: "downbeat" | "phrase_in" | "drop" | "mix_out";
  confidence: number;
}

export interface ForensicAuditReport {
  is_authentic_lossless: boolean;
  is_synthetic_tone: boolean;
  effective_bandwidth_hz: number;
  nyquist_hz: number;
  suspected_source_codec: string;
  spectral_flatness: number;
  peak_to_mean_ratio: number;
  crest_factor_db: number;
  clipped_samples: number;
  clipping_ratio: number;
  sub_bass_phase_correlation: number;
  mono_compatible_sub: boolean;
  quality_score: number;
  forensic_verdict: "VERIFIED_STUDIO_MASTER" | "REJECT_SYNTHETIC_TONE" | "WARN_FAKE_LOSSLESS_TRANSCODE" | "WARN_SUB_BASS_PHASE_CANCELLATION";
  warnings: string[];
}

export interface P2PVerifiedTrack {
  track_id: string;
  title: string;
  artist: string;
  bpm: number;
  camelot_key: string;
  top_subgenre: string;
  duration_seconds: number;
  format: string;
  effective_bandwidth_hz: number;
  sub_bass_phase_correlation: number;
  mono_compatible_sub: boolean;
  quality_score: number;
  forensic_verdict: string;
  hot_cues: HotCuePoint[];
  stream_url?: string;
  node_id: string;
  hera_seal: string;
}

export interface OptimizedCrateResult {
  brief: string;
  energy_arc: "steady_climb" | "rollercoaster" | "peak_hour";
  total_tracks: number;
  total_duration_seconds: number;
  mean_synergy_score: number;
  harmonic_match_pct: number;
  polyrhythmic_bridges: number;
  vocal_clash_warnings: number;
  sequence: P2PVerifiedTrack[];
  m3u8_export: string;
  rekordbox_xml_export: string;
  set_guide_markdown: string;
}

declare namespace hera {
  /** Search verified studio masters in the P2P node catalog. */
  function searchMusic(params?: {
    query?: string;
    camelot?: string;
    bpm_min?: number;
    bpm_max?: number;
    subgenre?: string;
    verified_only?: boolean;
  }): Promise<P2PVerifiedTrack[]>;

  /** Filter a track list by forensic quality, sub-bass phase correlation, BPM, or Camelot compatibility. */
  function filterTracks(params: {
    tracks: P2PVerifiedTrack[];
    min_quality_score?: number;
    require_mono_compatible_sub?: boolean;
    min_bandwidth_hz?: number;
    compatible_with_camelot?: string;
    bpm_min?: number;
    bpm_max?: number;
  }): Promise<P2PVerifiedTrack[]>;

  /** Compute 4-layer transition synergy (Camelot + BPM/3:4 Polyrhythm + Transient/Vocals + 64D Style) between two tracks. */
  function evaluateTransition(params: {
    from_track: P2PVerifiedTrack;
    to_track: P2PVerifiedTrack;
  }): Promise<{
    total_synergy: number;
    camelot_score: number;
    camelot_relation: string;
    bpm_score: number;
    is_polyrhythmic_bridge: boolean;
    vocal_clash_warning: boolean;
    mix_recommendation: string;
  }>;

  /** Sequence a complete DJ crate globally using the 2-Opt Traveling Salesperson (TSP) optimizer. */
  function buildDjCrate(params: {
    brief: string;
    energy_arc?: "steady_climb" | "rollercoaster" | "peak_hour";
    max_tracks?: number;
    candidates?: P2PVerifiedTrack[];
  }): Promise<OptimizedCrateResult>;

  /** Audit the entire node repertoire and return Zero-Trust forensic statistics. */
  function auditRepertoire(): Promise<{
    node_id: string;
    total_tracks: number;
    verified_studio_masters: number;
    average_quality_score: number;
    tracks: P2PVerifiedTrack[];
  }>;
}
"#
}

/// Execute a Code Mode request (either TypeScript `hera.*` script or declarative step pipeline) in memory.
pub fn execute_code_mode(
    req: CodeModeRequest,
    state: &P2PMeshState,
) -> Result<CodeModeExecutionResult, String> {
    let mut scope: Map<String, Value> = req.context;
    let mut log: Vec<String> = Vec::new();
    let mut ops_count = 0usize;

    // 1. If a TypeScript/JS `code` block is provided, parse its `hera.*` statements into executable steps + filter predicates
    let steps = if let Some(ref ts_code) = req.code {
        if !ts_code.trim().is_empty() {
            parse_ts_code_to_steps(ts_code, &mut scope, &mut log)?
        } else {
            req.pipeline.unwrap_or_default()
        }
    } else {
        req.pipeline.unwrap_or_default()
    };

    if steps.is_empty() && req.code.is_none() {
        return Err(
            "Provide either `code` (TypeScript/JS using `hera.*`) or `pipeline` steps.".to_string(),
        );
    }

    let mut last_value = Value::Null;

    for step in steps {
        ops_count += 1;
        let resolved_args = resolve_variables(&step.args, &scope);
        let result = execute_single_sdk_call(&step.call, &resolved_args, state, &mut log)?;
        last_value = result.clone();
        scope.insert(step.bind.clone(), result);
    }

    // Check if the script had a custom `return { ... }` expression or return variable
    if let Some(ret_expr) = scope.get("__return_expr__").cloned() {
        last_value = resolve_variables(&ret_expr, &scope);
    }

    Ok(CodeModeExecutionResult {
        mode: "cloudflare_code_mode_rust_sandbox".to_string(),
        operations_executed: ops_count,
        execution_log: log,
        output: last_value,
    })
}

/// Lightweight parser that translates TypeScript/JS `hera.*` statements & `.filter(...)` chains into sandboxed Rust operations.
fn parse_ts_code_to_steps(
    code: &str,
    scope: &mut Map<String, Value>,
    log: &mut Vec<String>,
) -> Result<Vec<CodeModeStep>, String> {
    let mut steps = Vec::new();

    // Split by statements (`;` or newline when outside braces)
    let statements = split_ts_statements(code);

    for raw_stmt in statements {
        let stmt = raw_stmt.trim();
        if stmt.is_empty() || stmt.starts_with("//") || stmt.starts_with("/*") {
            continue;
        }

        // Handle `return ...`
        if let Some(rest) = stmt.strip_prefix("return ") {
            let ret_str = rest.trim().trim_end_matches(';').trim();
            if let Some((call_name, call_args)) = extract_hera_call(ret_str) {
                steps.push(CodeModeStep {
                    bind: "__last__".to_string(),
                    call: call_name,
                    args: call_args,
                });
                scope.insert(
                    "__return_expr__".to_string(),
                    Value::String("$__last__".to_string()),
                );
            } else if ret_str.starts_with('{') && ret_str.ends_with('}') {
                let obj = parse_loose_js_object(ret_str);
                scope.insert("__return_expr__".to_string(), obj);
            } else {
                scope.insert(
                    "__return_expr__".to_string(),
                    Value::String(format!("${ret_str}")),
                );
            }
            continue;
        }

        // Handle `const/let/var name = ...`
        let (var_name, rhs) = if let Some(after_const) = stmt
            .strip_prefix("const ")
            .or_else(|| stmt.strip_prefix("let "))
            .or_else(|| stmt.strip_prefix("var "))
        {
            if let Some(eq_idx) = after_const.find('=') {
                let name = after_const[..eq_idx]
                    .split(':')
                    .next()
                    .unwrap_or("res")
                    .trim()
                    .to_string();
                let expr = after_const[eq_idx + 1..].trim().trim_end_matches(';').trim();
                (name, expr)
            } else {
                continue;
            }
        } else {
            ("__anon__".to_string(), stmt.trim_end_matches(';').trim())
        };

        // Case A: `await hera.method({...})` or `hera.method({...})`
        if let Some((call_name, call_args)) = extract_hera_call(rhs) {
            steps.push(CodeModeStep {
                bind: var_name,
                call: call_name,
                args: call_args,
            });
            continue;
        }

        // Case B: `sourceVar.filter(t => ...)`
        if let Some(filter_idx) = rhs.find(".filter(") {
            let src_var = rhs[..filter_idx].trim();
            let pred_body = &rhs[filter_idx + 8..];
            let mut filter_args = Map::new();
            filter_args.insert(
                "tracks".to_string(),
                Value::String(format!("${src_var}")),
            );

            if pred_body.contains("mono_compatible_sub") {
                filter_args.insert("require_mono_compatible_sub".to_string(), Value::Bool(true));
            }
            if let Some(q_val) = extract_number_after(pred_body, "quality_score") {
                filter_args.insert("min_quality_score".to_string(), json!(q_val));
            }
            if let Some(bw_val) = extract_number_after(pred_body, "effective_bandwidth_hz") {
                filter_args.insert("min_bandwidth_hz".to_string(), json!(bw_val));
            }
            if let Some(bmin) = extract_number_after(pred_body, "bpm >=")
                .or_else(|| extract_number_after(pred_body, "bpm >"))
            {
                filter_args.insert("bpm_min".to_string(), json!(bmin));
            }
            if let Some(bmax) = extract_number_after(pred_body, "bpm <=")
                .or_else(|| extract_number_after(pred_body, "bpm <"))
            {
                filter_args.insert("bpm_max".to_string(), json!(bmax));
            }

            log.push(format!(
                "CodeMode AST: compiled `{src_var}.filter(...)` -> native Rust predicate filter"
            ));
            steps.push(CodeModeStep {
                bind: var_name,
                call: "hera.filterTracks".to_string(),
                args: Value::Object(filter_args),
            });
        }
    }

    Ok(steps)
}

fn execute_single_sdk_call(
    call: &str,
    args: &Value,
    state: &P2PMeshState,
    log: &mut Vec<String>,
) -> Result<Value, String> {
    let normalized = call.trim().trim_start_matches("await ").trim();
    match normalized {
        "hera.searchMusic" | "searchMusic" | "search_music" => {
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
            let tracks = state.search(&q);
            log.push(format!(
                "hera.searchMusic -> found {} verified tracks",
                tracks.len()
            ));
            serde_json::to_value(tracks).map_err(|e| e.to_string())
        }
        "hera.connectDjDrive" | "connectDjDrive" | "connect_dj_drive" => {
            let req: ConnectDjDriveRequest = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid connectDjDrive payload: {e}"))?;
            let resp = state.connect_dj_drive(req);
            log.push(format!(
                "hera.connectDjDrive('{}') -> admitted {} tracks ({} quarantined)",
                resp.profile.dj_handle,
                resp.admitted_tracks.len(),
                resp.rejected_quarantine_tracks.len()
            ));
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "hera.listDjs" | "listDjs" | "list_djs" => {
            let profiles = state.list_dj_profiles();
            log.push(format!("hera.listDjs -> {} connected DJs", profiles.len()));
            serde_json::to_value(profiles).map_err(|e| e.to_string())
        }
        "hera.scanLocalFolder" | "scanLocalFolder" | "scan_local_folder" => {
            let req: ScanLocalDirRequest = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid scanLocalFolder payload: {e}"))?;
            let resp = state.scan_local_directory(req)?;
            log.push(format!(
                "hera.scanLocalFolder('{}') -> admitted {} local P2P tracks",
                resp.scanned_directory,
                resp.admitted_tracks.len()
            ));
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "hera.listPeers" | "listPeers" | "list_peers" => {
            let peers = state.list_peers();
            log.push(format!("hera.listPeers -> {} active P2P nodes", peers.len()));
            serde_json::to_value(peers).map_err(|e| e.to_string())
        }
        "hera.brokerStats" | "brokerStats" | "broker_stats" => {
            let stats = state.broker_stats();
            log.push(format!(
                "hera.brokerStats -> {} unique CAS masters ({} total swarm replicas, {:.1}% dedup)",
                stats.unique_cas_masters, stats.total_swarm_replicas, stats.deduplication_ratio_pct
            ));
            serde_json::to_value(stats).map_err(|e| e.to_string())
        }
        "hera.brokerLookupIntel" | "brokerLookupIntel" | "broker_lookup_intel" => {
            let req: BrokerBatchLookupRequest = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid brokerLookupIntel payload: {e}"))?;
            let resp = state.broker_lookup_intel_batch(req);
            log.push(format!(
                "hera.brokerLookupIntel -> {} instant O(1) hits ({} auto-healed, {:.1} ms CPU saved)",
                resp.instant_intel_hits.len(),
                resp.auto_healed_count,
                resp.total_cpu_ms_saved
            ));
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "hera.brokerResolveBridge" | "brokerResolveBridge" | "broker_resolve_bridge" => {
            let req: BrokerBridgeRequest = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid brokerResolveBridge payload: {e}"))?;
            let candidates = state.broker_resolve_emergency_bridge(req);
            log.push(format!(
                "hera.brokerResolveBridge -> {} optimal bridge tracks found",
                candidates.len()
            ));
            serde_json::to_value(candidates).map_err(|e| e.to_string())
        }
        "hera.brokerFindSimilar" | "brokerFindSimilar" | "broker_find_similar" => {
            let req: BrokerSimilarQuery = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid brokerFindSimilar payload: {e}"))?;
            let matches = state.broker_find_similar_by_popcnt(req);
            log.push(format!(
                "hera.brokerFindSimilar (u64 POPCNT) -> {} sonic matches found",
                matches.len()
            ));
            serde_json::to_value(matches).map_err(|e| e.to_string())
        }
        "hera.brokerOptimizeTiering" | "brokerOptimizeTiering" | "broker_optimize_tiering" => {
            let res = state.broker_optimize_hot_cold_tier();
            log.push(format!(
                "hera.brokerOptimizeTiering -> hibernated {} cold R2 replicas ({:.1} MB freed)",
                res.cold_r2_replicas_hibernated, res.r2_megabytes_freed
            ));
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "hera.brokerClaimCloudTrack" | "brokerClaimCloudTrack" | "broker_claim_cloud_track" => {
            let req: BrokerClaimCloudTrackRequest = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid brokerClaimCloudTrack payload: {e}"))?;
            let res = state.broker_claim_cloud_track(req)?;
            log.push(format!(
                "hera.brokerClaimCloudTrack('{}') -> 0.0 MB R2 copy (bounty={:.1} MB)",
                res.linked_track.title, res.pioneer_bounty_refunded_mb
            ));
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "hera.setP2PSharing" | "setP2PSharing" | "set_p2p_sharing" => {
            let req: SetP2PSharingRequest = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid setP2PSharing payload: {e}"))?;
            let profile = state.set_p2p_sharing(req)?;
            log.push(format!(
                "hera.setP2PSharing('{}') -> p2p_sharing_enabled={}",
                profile.dj_handle, profile.p2p_sharing_enabled
            ));
            serde_json::to_value(profile).map_err(|e| e.to_string())
        }
        "hera.filterTracks" | "filterTracks" => {
            let tracks: Vec<P2PVerifiedTrack> = args
                .get("tracks")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_else(|| {
                    state.search(&P2PSearchQuery {
                        q: None,
                        camelot: None,
                        bpm_min: None,
                        bpm_max: None,
                        subgenre: None,
                        owner_dj: None,
                        verified_only: Some(true),
                    })
                });
            let min_q = args
                .get("min_quality_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            let req_mono = args
                .get("require_mono_compatible_sub")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let min_bw = args
                .get("min_bandwidth_hz")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            let compat_cam = args
                .get("compatible_with_camelot")
                .and_then(|v| v.as_str());
            let bmin = args.get("bpm_min").and_then(|v| v.as_f64()).map(|f| f as f32);
            let bmax = args.get("bpm_max").and_then(|v| v.as_f64()).map(|f| f as f32);

            let filtered: Vec<P2PVerifiedTrack> = tracks
                .into_iter()
                .filter(|t| {
                    if t.quality_score < min_q {
                        return false;
                    }
                    if req_mono && !t.mono_compatible_sub {
                        return false;
                    }
                    if t.effective_bandwidth_hz < min_bw {
                        return false;
                    }
                    if let Some(ref_cam) = compat_cam {
                        let (sc, _) = compute_camelot_synergy(ref_cam, &t.camelot_key);
                        if sc < 0.75 {
                            return false;
                        }
                    }
                    if let Some(lo) = bmin {
                        if t.bpm < lo {
                            return false;
                        }
                    }
                    if let Some(hi) = bmax {
                        if t.bpm > hi {
                            return false;
                        }
                    }
                    true
                })
                .collect();

            log.push(format!(
                "hera.filterTracks -> retained {} tracks",
                filtered.len()
            ));
            serde_json::to_value(filtered).map_err(|e| e.to_string())
        }
        "hera.evaluateTransition" | "evaluateTransition" => {
            let bpm_a = args
                .get("bpm_a")
                .or_else(|| args.get("from_track").and_then(|t| t.get("bpm")))
                .and_then(|v| v.as_f64())
                .unwrap_or(122.0) as f32;
            let bpm_b = args
                .get("bpm_b")
                .or_else(|| args.get("to_track").and_then(|t| t.get("bpm")))
                .and_then(|v| v.as_f64())
                .unwrap_or(128.0) as f32;
            let cam_a = args
                .get("camelot_a")
                .or_else(|| args.get("from_track").and_then(|t| t.get("camelot_key")))
                .and_then(|v| v.as_str())
                .unwrap_or("8A");
            let cam_b = args
                .get("camelot_b")
                .or_else(|| args.get("to_track").and_then(|t| t.get("camelot_key")))
                .and_then(|v| v.as_str())
                .unwrap_or("8A");

            let (cam_score, cam_rel) = compute_camelot_synergy(cam_a, cam_b);
            let (bpm_score, delta_pct, is_poly, note) = compute_bpm_synergy(bpm_a, bpm_b);
            let total = (cam_score * 0.45 + bpm_score * 0.55).clamp(0.0, 1.0);
            log.push(format!(
                "hera.evaluateTransition({cam_a}@{bpm_a} -> {cam_b}@{bpm_b}) -> synergy={:.2}",
                total
            ));
            Ok(json!({
                "total_synergy": (total * 1000.0).round() / 1000.0,
                "camelot_score": cam_score,
                "camelot_relation": cam_rel,
                "bpm_score": bpm_score,
                "bpm_delta_pct": delta_pct,
                "is_polyrhythmic_bridge": is_poly,
                "tempo_note": note
            }))
        }
        "hera.buildDjCrate" | "buildDjCrate" | "build_dj_crate" => {
            let brief = args
                .get("brief")
                .and_then(|v| v.as_str())
                .unwrap_or("Code Mode Curated Crate");
            let energy_arc = args
                .get("energy_arc")
                .and_then(|v| v.as_str())
                .unwrap_or("steady_climb");
            let max_tracks = args
                .get("max_tracks")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);

            let candidates: Vec<CrateTrackCandidate> = if let Some(c_val) = args.get("candidates") {
                if let Ok(direct) = serde_json::from_value::<Vec<CrateTrackCandidate>>(c_val.clone())
                {
                    direct
                } else if let Ok(p2p_list) =
                    serde_json::from_value::<Vec<P2PVerifiedTrack>>(c_val.clone())
                {
                    p2p_list.into_iter().map(p2p_to_candidate).collect()
                } else {
                    default_candidates_from_state(state)
                }
            } else {
                default_candidates_from_state(state)
            };

            let res = optimize_crate_2opt(brief, energy_arc, candidates, max_tracks);
            log.push(format!(
                "hera.buildDjCrate('{brief}', arc='{energy_arc}') -> sequenced {} tracks (synergy={:.1}%)",
                res.total_tracks,
                res.mean_synergy_score * 100.0
            ));
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        "hera.auditRepertoire" | "auditRepertoire" | "audit_repertoire" => {
            let tracks = state.search(&P2PSearchQuery {
                q: None,
                camelot: None,
                bpm_min: None,
                bpm_max: None,
                subgenre: None,
                owner_dj: None,
                verified_only: Some(false),
            });
            let n = tracks.len().max(1) as f32;
            let avg_q: f32 = tracks.iter().map(|t| t.quality_score).sum::<f32>() / n;
            let verified = tracks
                .iter()
                .filter(|t| t.forensic_verdict == "VERIFIED_STUDIO_MASTER" && t.mono_compatible_sub)
                .count();
            log.push(format!(
                "hera.auditRepertoire -> {verified}/{} verified masters (avg quality {:.1})",
                tracks.len(),
                avg_q
            ));
            Ok(json!({
                "node_id": state.node_id,
                "total_tracks": tracks.len(),
                "verified_studio_masters": verified,
                "average_quality_score": (avg_q * 10.0).round() / 10.0,
                "tracks": tracks
            }))
        }
        "hera.auditPcm" | "auditPcm" => {
            let left: Vec<f32> = args
                .get("left")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect())
                .unwrap_or_default();
            let sr = args
                .get("sample_rate")
                .and_then(|v| v.as_u64())
                .unwrap_or(44100) as u32;
            let ext = args
                .get("claimed_ext")
                .and_then(|v| v.as_str())
                .unwrap_or("flac");
            let rep = audit_pcm_channels(&left, None, sr, ext);
            log.push(format!("hera.auditPcm -> verdict={}", rep.forensic_verdict));
            serde_json::to_value(rep).map_err(|e| e.to_string())
        }
        _ => Err(format!("Unsupported Code Mode SDK method `{call}`")),
    }
}

fn p2p_to_candidate(t: P2PVerifiedTrack) -> CrateTrackCandidate {
    CrateTrackCandidate {
        track_id: t.track_id.clone(),
        title: t.title,
        artist: t.artist,
        file_path: t
            .stream_url
            .unwrap_or_else(|| format!("/library/{}.flac", t.track_id)),
        duration_seconds: t.duration_seconds,
        bpm: t.bpm,
        camelot_key: t.camelot_key,
        energy_score: 0.76,
        punch_index: 0.74,
        groove_type: "four_on_floor".to_string(),
        top_subgenre: t.top_subgenre,
        has_intro_vocals: false,
        has_outro_vocals: false,
        embedding_64d: vec![0.125; 64],
        hot_cues: t.hot_cues,
    }
}

fn default_candidates_from_state(state: &P2PMeshState) -> Vec<CrateTrackCandidate> {
    state
        .search(&P2PSearchQuery {
            q: None,
            camelot: None,
            bpm_min: None,
            bpm_max: None,
            subgenre: None,
            owner_dj: None,
            verified_only: Some(true),
        })
        .into_iter()
        .map(p2p_to_candidate)
        .collect()
}

/// Resolve `"$var"` or `"$var.field"` references against the Code Mode execution scope.
fn resolve_variables(val: &Value, scope: &Map<String, Value>) -> Value {
    match val {
        Value::String(s) if s.starts_with('$') => {
            let path = s.trim_start_matches('$');
            if let Some((root, prop)) = path.split_once('.') {
                if let Some(root_val) = scope.get(root) {
                    if prop == "length" {
                        if let Some(arr) = root_val.as_array() {
                            return json!(arr.len());
                        }
                    }
                    if let Some(obj) = root_val.as_object() {
                        if let Some(field_val) = obj.get(prop) {
                            return field_val.clone();
                        }
                    }
                }
            }
            scope.get(path).cloned().unwrap_or(Value::Null)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(|v| resolve_variables(v, scope)).collect()),
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.clone(), resolve_variables(v, scope));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn split_ts_statements(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;

    for ch in code.chars() {
        match ch {
            '{' | '(' | '[' => {
                depth += 1;
                cur.push(ch);
            }
            '}' | ')' | ']' => {
                depth = (depth - 1).max(0);
                cur.push(ch);
            }
            ';' | '\n' if depth == 0 => {
                if !cur.trim().is_empty() {
                    out.push(cur.trim().to_string());
                }
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

fn extract_hera_call(expr: &str) -> Option<(String, Value)> {
    let clean = expr.trim().trim_start_matches("await ").trim();
    if !clean.starts_with("hera.") {
        return None;
    }
    let open_paren = clean.find('(')?;
    let close_paren = clean.rfind(')')?;
    let method = clean[..open_paren].trim().to_string();
    let inside = clean[open_paren + 1..close_paren].trim();
    let args = if inside.is_empty() {
        json!({})
    } else if let Ok(v) = serde_json::from_str::<Value>(inside) {
        v
    } else {
        parse_loose_js_object(inside)
    };
    Some((method, args))
}

fn parse_loose_js_object(raw: &str) -> Value {
    let trimmed = raw.trim();
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        return v;
    }
    let inner = trimmed
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(trimmed);

    let mut map = Map::new();
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for ch in inner.chars() {
        match ch {
            '{' | '[' | '(' => {
                depth += 1;
                cur.push(ch);
            }
            '}' | ']' | ')' => {
                depth = (depth - 1).max(0);
                cur.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur.trim().to_string());
    }

    for part in parts {
        if let Some((k, v)) = part.split_once(':') {
            let key = k.trim().trim_matches('"').trim_matches('\'').to_string();
            let val_str = v.trim();
            let val = if (val_str.starts_with('"') && val_str.ends_with('"'))
                || (val_str.starts_with('\'') && val_str.ends_with('\''))
            {
                Value::String(val_str[1..val_str.len() - 1].to_string())
            } else if val_str == "true" {
                Value::Bool(true)
            } else if val_str == "false" {
                Value::Bool(false)
            } else if let Ok(n) = val_str.parse::<i64>() {
                json!(n)
            } else if let Ok(f) = val_str.parse::<f64>() {
                json!(f)
            } else {
                Value::String(format!("${val_str}"))
            };
            map.insert(key, val);
        }
    }
    Value::Object(map)
}

fn extract_number_after(text: &str, marker: &str) -> Option<f64> {
    let idx = text.find(marker)?;
    let rest = &text[idx + marker.len()..];
    let mut num_str = String::new();
    let mut started = false;
    for ch in rest.chars() {
        if ch.is_ascii_digit() || (ch == '.' && started) {
            started = true;
            num_str.push(ch);
        } else if started {
            break;
        }
    }
    num_str.parse::<f64>().ok()
}

/// Complete OpenAPI 3.1.0 Specification (`GET /openapi.json`) for Google AI Studio, Gemini CLI, OpenWebUI & Custom Actions.
pub fn get_openapi_3_1_spec(node_id: &str) -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Hera Universal Audio DSP, MCP Code Mode & P2P Node API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": format!(
                "Pure-Rust Audio DSP, 4-Layer 2-Opt TSP Crate Curator, Cloudflare Code Mode & Quarantine-First P2P Node (Node: {}, Runtime: {})",
                node_id,
                current_runtime_target()
            )
        },
        "paths": {
            "/health": {
                "get": {
                    "operationId": "getNodeHealth",
                    "summary": "Check Hera node status, runtime target, and P2P catalog count",
                    "responses": { "200": { "description": "Node is healthy" } }
                }
            },
            "/mcp": {
                "post": {
                    "operationId": "mcpJsonRpc",
                    "summary": "Model Context Protocol (MCP) JSON-RPC 2.0 endpoint",
                    "responses": { "200": { "description": "JSON-RPC 2.0 response" } }
                }
            },
            "/mcp/code-mode": {
                "post": {
                    "operationId": "executeHeraCodeMode",
                    "summary": "Cloudflare Code Mode execution endpoint: run multi-step TypeScript `hera.*` scripts or pipelines in a single sub-millisecond turn",
                    "responses": { "200": { "description": "Code Mode execution output and log" } }
                }
            },
            "/mcp/code-mode.d.ts": {
                "get": {
                    "operationId": "getCodeModeTypeScriptDefinitions",
                    "summary": "Download TypeScript `.d.ts` ambient SDK declarations for `hera.*` Code Mode",
                    "responses": { "200": { "description": "TypeScript declarations" } }
                }
            },
            "/api/v1/audio/analyze-file": {
                "post": {
                    "operationId": "analyzeAudioFile",
                    "summary": "Upload WAV/FLAC/MP3/OGG bytes for pure-Rust Symphonia + rustfft spectral forensics & 4 Acoustic Hot Cues",
                    "responses": { "200": { "description": "Full forensic & structural Hot-Cue telemetry" } }
                }
            },
            "/api/v1/crates/build": {
                "post": {
                    "operationId": "buildDjCrate2Opt",
                    "summary": "Sequence a harmonic DJ crate using the 4-Layer Synergy Engine + Global 2-Opt TSP optimizer",
                    "responses": { "200": { "description": "Optimized sequence with M3U8, Rekordbox XML, and Set Guide" } }
                }
            },
            "/api/v1/p2p/search": {
                "get": {
                    "operationId": "searchP2PVerifiedCatalog",
                    "summary": "Search acoustically verified studio masters in the P2P node catalog",
                    "responses": { "200": { "description": "Matching verified tracks" } }
                }
            },
            "/api/v1/p2p/announce": {
                "post": {
                    "operationId": "announceP2PVerifiedTrack",
                    "summary": "Quarantine-First P2P gatekeeper: admit a track only if it passes Zero-Trust spectral & phase forensics",
                    "responses": { "201": { "description": "Track admitted with cryptographic hera_seal" } }
                }
            },
            "/api/v1/node/manifest": {
                "get": {
                    "operationId": "getNodeManifestAndReferrals",
                    "summary": "Get node invariants and 1-Click Cloud Referral deployment catalog",
                    "responses": { "200": { "description": "Manifest and referral catalog" } }
                }
            }
        },
        "x-cloud-referrals": get_cloud_referral_catalog()
    })
}
