use crate::p2p::{
    ConnectDjDriveRequest, CustodyAuditSubmission, CustodyChallengeRequest, P2PMeshState,
    SwarmHeartbeatRequest, SwarmRouteRequest,
};
use serde_json::Value;

pub(super) fn dispatch_swarm_tool(
    name: &str,
    args: Value,
    state: &P2PMeshState,
) -> Result<Value, String> {
    match name {
        "connect_dj_drive" => {
            let req: ConnectDjDriveRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `connect_dj_drive` payload: {e}"))?;
            let resp = state.connect_dj_drive(req);
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "get_swarm_strength" => {
            let report = state.compute_swarm_strength();
            serde_json::to_value(report).map_err(|e| e.to_string())
        }
        "swarm_heartbeat" => {
            let req: SwarmHeartbeatRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `swarm_heartbeat` payload: {e}"))?;
            let resp = state.process_swarm_heartbeat(req)?;
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "swarm_route_ticket" => {
            let req: SwarmRouteRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `swarm_route_ticket` payload: {e}"))?;
            let resp = state.issue_route_ticket(req)?;
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "swarm_custody_audit" => {
            if args.get("http_status").is_some() {
                let sub: CustodyAuditSubmission = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid `swarm_custody_audit` submission: {e}"))?;
                let res = state.verify_custody_audit(sub);
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
                serde_json::to_value(chal).map_err(|e| e.to_string())
            }
        }
        "swarm_parallel_manifest" => {
            let req: crate::p2p::SwarmParallelManifestRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `swarm_parallel_manifest` payload: {e}"))?;
            let resp = state.issue_parallel_swarm_manifest(req)?;
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "swarm_crdt_sync" => {
            let req: crate::p2p::SwarmCrdtDelta = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `swarm_crdt_sync` payload: {e}"))?;
            let resp = state.sync_crdt_delta(req);
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "swarm_subscribe" => {
            let req: crate::p2p::SwarmSubscribeRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `swarm_subscribe` payload: {e}"))?;
            let resp = state.subscribe_semantic_topic(req);
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "export_dj_crate" => {
            let req: crate::export::ExportCrateRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `export_dj_crate` payload: {e}"))?;
            let resp = crate::export::export_crate_bundle(&req);
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "compute_dj_style_dna" => {
            let req: crate::dj_brain::DjStyleDnaRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `compute_dj_style_dna` payload: {e}"))?;
            let resp = crate::dj_brain::compute_dj_style_dna(&req);
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        "chat_with_dj_copilot" => {
            let req: crate::dj_brain::CloudflareFreeAiTurnRequest = serde_json::from_value(args)
                .map_err(|e| format!("Invalid `chat_with_dj_copilot` payload: {e}"))?;
            let resp = crate::dj_brain::evaluate_agent_harness_turn(&req, state);
            serde_json::to_value(resp).map_err(|e| e.to_string())
        }
        _ => Err(format!("Unknown MCP tool `{name}`")),
    }
}
