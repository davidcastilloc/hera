use crate::p2p::{
    BrokerBatchLookupRequest, BrokerBridgeRequest, BrokerClaimCloudTrackRequest,
    BrokerSimilarQuery, ConnectDjDriveRequest, CustodyAuditSubmission, CustodyChallengeRequest,
    P2PMeshState, SetP2PSharingRequest, SwarmHeartbeatRequest, SwarmRouteRequest,
};
use serde_json::Value;

pub(super) fn execute_swarm_sdk_call(
    raw_call: &str,
    normalized: &str,
    args: &Value,
    state: &P2PMeshState,
    log: &mut Vec<String>,
) -> Result<Value, String> {
    match normalized {
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
        "hera.listPeers" | "listPeers" | "list_peers" => {
            let peers = state.list_peers();
            log.push(format!(
                "hera.listPeers -> {} active P2P nodes",
                peers.len()
            ));
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
        "hera.getSwarmStrength" | "getSwarmStrength" | "get_swarm_strength" => {
            let report = state.compute_swarm_strength();
            log.push(format!(
                "hera.getSwarmStrength -> {} active nodes ({:.1} GB safe R2, K_max={})",
                report.active_healthy_nodes,
                report.safe_usable_r2_capacity_gb,
                report.max_thermal_replication_k
            ));
            serde_json::to_value(report).map_err(|e| e.to_string())
        }
        "hera.swarmHeartbeat" | "swarmHeartbeat" | "swarm_heartbeat" => {
            let req: SwarmHeartbeatRequest = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid swarmHeartbeat payload: {e}"))?;
            let resp = state.process_swarm_heartbeat(req)?;
            log.push(format!(
                "hera.swarmHeartbeat('{}') -> guardian={}, credits={:.1} MB",
                resp.node_id, resp.edge_guardian_active, resp.swarm_credit_balance_mb
            ));
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "hera.swarmRouteTicket" | "swarmRouteTicket" | "swarm_route_ticket" => {
            let req: SwarmRouteRequest = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid swarmRouteTicket payload: {e}"))?;
            let resp = state.issue_route_ticket(req)?;
            log.push(format!(
                "hera.swarmRouteTicket -> P2C node='{}', mode='{}'",
                resp.p2c_selected_node_id, resp.access_mode
            ));
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "hera.swarmCustodyAudit" | "swarmCustodyAudit" | "swarm_custody_audit" => {
            if args.get("http_status").is_some() {
                let sub: CustodyAuditSubmission = serde_json::from_value(args.clone())
                    .map_err(|e| format!("Invalid swarmCustodyAudit submission: {e}"))?;
                let res = state.verify_custody_audit(sub);
                log.push(format!(
                    "hera.swarmCustodyAudit('{}') -> passed={}, status={}",
                    res.node_id, res.passed, res.custody_status
                ));
                serde_json::to_value(res).map_err(|e| e.to_string())
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
                log.push(format!(
                    "hera.swarmCustodyAudit('{}') -> issued 64B challenge '{}'",
                    chal.node_id, chal.range_header
                ));
                serde_json::to_value(chal).map_err(|e| e.to_string())
            }
        }
        "hera.swarmParallelManifest" | "swarmParallelManifest" | "swarm_parallel_manifest" => {
            let req: crate::p2p::SwarmParallelManifestRequest =
                serde_json::from_value(args.clone())
                    .map_err(|e| format!("Invalid swarmParallelManifest payload: {e}"))?;
            let resp = state.issue_parallel_swarm_manifest(req)?;
            log.push(format!(
                "hera.swarmParallelManifest('{}') -> striped {} blocks across {} Edge Guardians",
                resp.cas_fingerprint, resp.total_blocks, resp.distinct_edge_guardians_striped
            ));
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "hera.swarmCrdtSync" | "swarmCrdtSync" | "swarm_crdt_sync" => {
            let req: crate::p2p::SwarmCrdtDelta = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid swarmCrdtSync payload: {e}"))?;
            let resp = state.sync_crdt_delta(req);
            log.push(format!(
                "hera.swarmCrdtSync -> digest='{}', total_served={:.1} MB",
                &resp.converged_state_digest_sha256[..16],
                resp.gcounter_total_swarm_served_mb
            ));
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "hera.swarmSubscribe" | "swarmSubscribe" | "swarm_subscribe" => {
            let req: crate::p2p::SwarmSubscribeRequest = serde_json::from_value(args.clone())
                .map_err(|e| format!("Invalid swarmSubscribe payload: {e}"))?;
            let resp = state.subscribe_semantic_topic(req);
            log.push(format!(
                "hera.swarmSubscribe('{}') -> {} immediate matching events",
                resp.subscription.topic_uri,
                resp.immediate_matching_events.len()
            ));
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        _ => Err(format!("Unsupported Code Mode SDK method `{raw_call}`")),
    }
}
