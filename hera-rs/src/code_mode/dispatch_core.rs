use crate::forensics::audit_pcm_channels;
use crate::p2p::{P2PMeshState, P2PSearchQuery, P2PVerifiedTrack, ScanLocalDirRequest};
use crate::synergy::{
    compute_bpm_synergy, compute_camelot_synergy, optimize_crate_with_spine, CrateTrackCandidate,
    SetSpineConfig, SpineAnchor,
};
use serde_json::{json, Value};

pub(super) fn execute_core_sdk_call(
    normalized: &str,
    args: &Value,
    state: &P2PMeshState,
    log: &mut Vec<String>,
) -> Option<Result<Value, String>> {
    match normalized {
        "hera.searchMusic" | "searchMusic" | "search_music" => {
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
            let tracks = state.search(&q);
            log.push(format!(
                "hera.searchMusic -> found {} verified tracks",
                tracks.len()
            ));
            Some(serde_json::to_value(tracks).map_err(|e| e.to_string()))
        }
        "hera.scanLocalFolder" | "scanLocalFolder" | "scan_local_folder" => Some((|| {
            let req: ScanLocalDirRequest = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid scanLocalFolder payload: {e}"))?;
            let resp = state.scan_local_directory(req)?;
            log.push(format!(
                "hera.scanLocalFolder('{}') -> admitted {} local P2P tracks",
                resp.scanned_directory,
                resp.admitted_tracks.len()
            ));
            serde_json::to_value(resp).map_err(|e| e.to_string())
        })()),
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
            let compat_cam = args.get("compatible_with_camelot").and_then(|v| v.as_str());
            let bmin = args
                .get("bpm_min")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32);
            let bmax = args
                .get("bpm_max")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32);

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
            Some(serde_json::to_value(filtered).map_err(|e| e.to_string()))
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
            Some(Ok(json!({
                "total_synergy": (total * 1000.0).round() / 1000.0,
                "camelot_score": cam_score,
                "camelot_relation": cam_rel,
                "bpm_score": bpm_score,
                "bpm_delta_pct": delta_pct,
                "is_polyrhythmic_bridge": is_poly,
                "tempo_note": note
            })))
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
                if let Ok(direct) =
                    serde_json::from_value::<Vec<CrateTrackCandidate>>(c_val.clone())
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
            log.push(format!(
                "hera.buildDjCrate('{brief}', arc='{energy_arc}') -> sequenced {} tracks (synergy={:.1}%)",
                res.total_tracks,
                res.mean_synergy_score * 100.0
            ));
            Some(serde_json::to_value(res).map_err(|e| e.to_string()))
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
            Some(Ok(json!({
                "node_id": state.node_id,
                "total_tracks": tracks.len(),
                "verified_studio_masters": verified,
                "average_quality_score": (avg_q * 10.0).round() / 10.0,
                "tracks": tracks
            })))
        }
        "hera.auditPcm" | "auditPcm" => {
            let left: Vec<f32> = args
                .get("left")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_f64().map(|f| f as f32))
                        .collect()
                })
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
            Some(serde_json::to_value(rep).map_err(|e| e.to_string()))
        }
        _ => None,
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
