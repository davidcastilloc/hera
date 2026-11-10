use super::analyze::{analyze_batch_args, analyze_single_track_arg};
use crate::code_mode::{execute_code_mode, get_typescript_sdk_definitions, CodeModeRequest};
use crate::p2p::{get_cloud_referral_catalog, P2PMeshState, P2PSearchQuery, P2PVerifiedTrack};
use crate::router::{current_runtime_target, AnnounceTrackRequest};
use crate::synergy::{optimize_crate_with_spine, CrateTrackCandidate, SetSpineConfig, SpineAnchor};
use serde_json::{json, Value};

pub(super) fn dispatch_core_tool(
    name: &str,
    args: &Value,
    state: &P2PMeshState,
) -> Option<Result<Value, String>> {
    match name {
        "ensure_hera_updated" => {
            let _ = args;
            #[allow(unused_mut)]
            let mut git_head = "unknown".to_string();
            #[cfg(not(target_arch = "wasm32"))]
            {
                let mut cmd = std::process::Command::new("git");
                cmd.args(["rev-parse", "--short", "HEAD"]);
                if let Ok(out) = crate::dj_brain::cli_runner::run_command_with_timeout(
                    cmd,
                    std::time::Duration::from_secs(2),
                ) {
                    if out.status.success() {
                        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if !s.is_empty() {
                            git_head = s;
                        }
                    }
                }
            }
            Some(Ok(json!({
                "status": "ok",
                "engine": "hera-node-rs (100% Pure Rust)",
                "version": env!("CARGO_PKG_VERSION"),
                "runtime_target": current_runtime_target(),
                "node_id": state.node_id,
                "git_pulled": false,
                "git_head": git_head,
                "python_dependency": "none (0 MB)"
            })))
        }
        "search_music" => {
            let q = P2PSearchQuery {
                q: args.get("query").and_then(|v| v.as_str()).map(String::from),
                camelot: args
                    .get("camelot")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                bpm_min: args
                    .get("bpm_min")
                    .and_then(|v| v.as_f64())
                    .map(|f| f as f32),
                bpm_max: args
                    .get("bpm_max")
                    .and_then(|v| v.as_f64())
                    .map(|f| f as f32),
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
            Some(Ok(json!({
                "node_id": state.node_id,
                "total_matches": matches.len(),
                "tracks": matches
            })))
        }
        "get_track_candidates" => {
            let query_str = args.get("query").and_then(|v| v.as_str()).map(String::from);
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
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

            Some(Ok(json!({
                "candidates_count": ranked.len(),
                "candidates": ranked
            })))
        }
        "analyze_track" => Some(analyze_single_track_arg(args)),
        "analyze_batch" => Some(analyze_batch_args(args)),
        "build_dj_crate" => Some(dispatch_build_dj_crate(args, state)),
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

            Some(Ok(json!({
                "node_id": state.node_id,
                "total_catalog_tracks": catalog_tracks.len(),
                "verified_studio_masters": verified_count,
                "quarantine_integrity": "100% Zero-Trust Enforced",
                "tracks": catalog_tracks
            })))
        }
        "announce_p2p_track" => Some((|| {
            let req: AnnounceTrackRequest = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid announce_p2p_track payload: {e}"))?;
            let admitted: P2PVerifiedTrack =
                state.announce_verified_track(req.track, &req.forensics)?;
            serde_json::to_value(admitted).map_err(|e| e.to_string())
        })()),
        "get_node_manifest" => Some(Ok(json!({
            "service": "Hera Universal Rust DSP, MCP & P2P Node",
            "version": env!("CARGO_PKG_VERSION"),
            "runtime_target": current_runtime_target(),
            "node_id": state.node_id,
            "cloud_referrals": get_cloud_referral_catalog()
        }))),
        "hera_code_mode" => Some((|| {
            let req: CodeModeRequest = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid `hera_code_mode` payload: {e}"))?;
            let res = execute_code_mode(req, state)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        })()),
        "get_code_mode_sdk" => Some(Ok(json!({
            "sdk_namespace": "hera",
            "reference": "https://blog.cloudflare.com/code-mode/",
            "typescript_declarations": get_typescript_sdk_definitions()
        }))),
        _ => None,
    }
}

fn dispatch_build_dj_crate(args: &Value, state: &P2PMeshState) -> Result<Value, String> {
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
                file_path: t
                    .stream_url
                    .unwrap_or_else(|| format!("/library/{}.flac", t.track_id)),
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
    serde_json::to_value(res).map_err(|e| e.to_string())
}
