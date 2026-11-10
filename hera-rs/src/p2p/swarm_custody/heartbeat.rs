use super::super::*;

impl P2PMeshState {
    /// Processes a community node's heartbeat (`POST /api/v1/swarm/heartbeat`), mints 1:1.5 Turbo credits,
    /// recalculates Water-Filling allocations, and issues `outbound_pull_tickets` for CGNAT nodes.
    pub fn process_swarm_heartbeat(
        &self,
        req: SwarmHeartbeatRequest,
    ) -> Result<SwarmHeartbeatResponse, String> {
        let node_id = req.node_id.trim().to_string();
        if node_id.is_empty() {
            return Err("node_id cannot be empty".to_string());
        }
        if let Ok(q_guard) = self.quarantined_nodes.read() {
            if let Some(q) = q_guard.get(&node_id) {
                return Err(format!(
                    "Node '{}' is quarantined for '{}' and cannot participate in the swarm",
                    node_id, q.reason
                ));
            }
        }

        let served_delta = req.megabytes_served_delta.unwrap_or(0.0).max(0.0);
        let earned_credits = ((served_delta * SWARM_CREDIT_MULTIPLIER) * 100.0).round() / 100.0;
        let behind_cgnat = req.behind_cgnat.unwrap_or(false);
        let r2_used = req
            .r2_used_mb
            .unwrap_or(114.0)
            .clamp(0.0, CF_WORKER_SAFE_R2_CAP_MB);
        let reqs_used = req
            .daily_worker_requests_used
            .unwrap_or(100)
            .min(CF_WORKER_SAFE_REQ_CAP);

        let mut updated_credit_balance = 150.0 + earned_credits;
        if let Ok(mut dj_guard) = self.dj_profiles.write() {
            if let Some(dj) = dj_guard
                .iter_mut()
                .find(|d| d.dj_handle.eq_ignore_ascii_case(&req.owner_dj))
            {
                if served_delta > 0.0 {
                    dj.total_bytes_served_mb =
                        ((dj.total_bytes_served_mb + served_delta) * 100.0).round() / 100.0;
                    dj.swarm_credit_balance_mb =
                        ((dj.swarm_credit_balance_mb + earned_credits) * 100.0).round() / 100.0;
                }
                dj.edge_guardian_active = true;
                updated_credit_balance = dj.swarm_credit_balance_mb;
            } else {
                dj_guard.push(DjProfile {
                    dj_handle: req.owner_dj.clone(),
                    display_name: format!("{} (Swarm Edge Guardian)", req.owner_dj),
                    role: "swarm_edge_guardian".to_string(),
                    storage_provider: "r2".to_string(),
                    gdrive_folder_id: None,
                    gdrive_folder_url: None,
                    r2_bucket_url: Some(req.endpoint_url.clone()),
                    local_node_dir: None,
                    subscription_tier: "beta_mvp".to_string(),
                    r2_quota_mb: 5_120.0,
                    r2_used_mb: r2_used,
                    bounty_refunded_mb: 0.0,
                    p2p_sharing_enabled: true,
                    can_disable_p2p_sharing: false,
                    verified_tracks_shared: 3,
                    quarantined_rejected_tracks: 0,
                    edge_guardian_active: true,
                    swarm_credit_balance_mb: updated_credit_balance,
                    total_bytes_served_mb: served_delta,
                    total_bytes_consumed_mb: 0.0,
                    custody_audit_passes: 1,
                    custody_audit_failures: 0,
                });
            }
        }

        let pubkey_id = req
            .endpoint_pubkey_id
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| derive_node_endpoint_pubkey_id(&req.owner_dj, &node_id));
        let relay_url = if behind_cgnat {
            Some(build_cgnat_relay_url(&pubkey_id))
        } else {
            None
        };
        let observed_rtt = req.observed_rtt_ms.unwrap_or(24.0).clamp(1.0, 2000.0);

        let mut final_node_id = node_id.clone();
        let mut net_gen: u64 = 1;
        let mut active_ewma_rtt: f32 = observed_rtt;

        let catalog_len = self.catalog.read().map(|c| c.len()).unwrap_or(3);
        if let Ok(mut p_guard) = self.peers.write() {
            if let Some(existing) = p_guard.iter_mut().find(|n| {
                n.node_id == node_id
                    || (!n.endpoint_pubkey_id.is_empty() && n.endpoint_pubkey_id == pubkey_id)
            }) {
                if existing.endpoint_url != req.endpoint_url {
                    existing.network_generation = existing.network_generation.saturating_add(1);
                }
                existing.endpoint_url = req.endpoint_url.clone();
                existing.dj_handle = req.owner_dj.clone();
                existing.endpoint_pubkey_id = pubkey_id.clone();
                existing.relay_assisted_url = relay_url.clone();
                if req.observed_rtt_ms.is_some() {
                    existing.ewma_rtt_ms =
                        ((existing.ewma_rtt_ms * 0.70 + observed_rtt * 0.30) * 100.0).round()
                            / 100.0;
                }
                existing.r2_used_mb = r2_used;
                existing.daily_worker_requests_used = reqs_used;
                existing.edge_guardian_active = true;
                existing.custody_status = "ONLINE_HEALTHY".to_string();
                existing.credits_earned_mb =
                    ((existing.credits_earned_mb + earned_credits) * 100.0).round() / 100.0;
                existing.behind_cgnat = behind_cgnat;
                if let Some(proof) = &req.custody_proof_slice_sha256 {
                    existing.last_custody_proof_hash = proof.clone();
                }
                final_node_id = existing.node_id.clone();
                net_gen = existing.network_generation;
                active_ewma_rtt = existing.ewma_rtt_ms;
            } else {
                p_guard.push(P2PPeerNode {
                    node_id: node_id.clone(),
                    dj_handle: req.owner_dj.clone(),
                    endpoint_url: req.endpoint_url.clone(),
                    runtime_target: "cloudflare-workers-wasm32".to_string(),
                    storage_mode: "r2_edge_guardian".to_string(),
                    verified_tracks_count: catalog_len,
                    bandwidth_mbps: 1000,
                    r2_free_tier_limit_mb: CF_WORKER_FREE_R2_LIMIT_MB,
                    r2_safe_cap_mb: CF_WORKER_SAFE_R2_CAP_MB,
                    r2_used_mb: r2_used,
                    daily_worker_request_limit: CF_WORKER_FREE_REQ_LIMIT,
                    daily_worker_request_safe_cap: CF_WORKER_SAFE_REQ_CAP,
                    daily_worker_requests_used: reqs_used,
                    edge_guardian_active: true,
                    custody_status: "ONLINE_HEALTHY".to_string(),
                    last_custody_proof_hash: req
                        .custody_proof_slice_sha256
                        .clone()
                        .unwrap_or_else(|| "custody-proof-verified".to_string()),
                    custody_failures: 0,
                    credits_earned_mb: earned_credits,
                    behind_cgnat,
                    quarantined: false,
                    endpoint_pubkey_id: pubkey_id.clone(),
                    network_generation: 1,
                    relay_assisted_url: relay_url.clone(),
                    ewma_rtt_ms: observed_rtt,
                });
            }
        }

        let strength = self.compute_swarm_strength();
        let node_alloc = strength
            .water_filling_allocations
            .iter()
            .find(|a| a.node_id == final_node_id)
            .cloned()
            .unwrap_or(WaterFillingNodeAllocation {
                node_id: final_node_id.clone(),
                owner_dj: req.owner_dj.clone(),
                allocated_r2_mb: r2_used,
                safe_r2_cap_mb: CF_WORKER_SAFE_R2_CAP_MB,
                free_headroom_mb: (CF_WORKER_SAFE_R2_CAP_MB - r2_used).max(0.0),
                assigned_shards_count: 1,
                daily_requests_used: reqs_used,
                daily_requests_safe_cap: CF_WORKER_SAFE_REQ_CAP,
                daily_requests_headroom: CF_WORKER_SAFE_REQ_CAP.saturating_sub(reqs_used),
                load_factor_pct: 5.0,
                edge_guardian_active: true,
            });

        // Generate outbound pull tickets so CGNAT nodes or under-loaded nodes can pull hot shards outbound
        let active_n = strength.active_healthy_nodes;
        let catalog_snapshot = self.catalog.read().map(|g| g.clone()).unwrap_or_default();
        let heat_map = self
            .cas_heat_scores
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        let sha_map = self
            .cas_audio_sha256
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();

        let mut outbound_pull_tickets = Vec::new();
        for t in catalog_snapshot.iter().take(3) {
            let cas = compute_cas_fingerprint(&t.artist, &t.title, t.bpm);
            let heat = heat_map.get(&cas).copied().unwrap_or(1.0);
            let target_k = compute_elastic_replication_k(active_n, heat);
            if behind_cgnat || target_k >= 2 {
                let sha = sha_map
                    .get(&cas)
                    .cloned()
                    .unwrap_or_else(|| compute_canonical_audio_sha256(&cas, &t.hera_seal, 38.0));
                let opaque_blob_key = compute_opaque_shard_blob_key(&cas, &sha);
                let pull_id = format!("pull-{}-{}", &sha[..8], final_node_id);
                let sig = compute_hmac_sha256_hex(
                    SWARM_TICKET_MASTER_SECRET.as_bytes(),
                    format!("{}|{}|{}", pull_id, opaque_blob_key, sha).as_bytes(),
                );
                outbound_pull_tickets.push(OutboundPullTicket {
                    pull_ticket_id: pull_id,
                    cas_fingerprint: cas,
                    opaque_blob_key: opaque_blob_key.clone(),
                    audio_sha256: sha,
                    source_url: format!(
                        "https://hera-cloud.pc911itservices.workers.dev/api/v1/swarm/shard/{}",
                        opaque_blob_key.trim_start_matches("shard-blobs/")
                    ),
                    file_size_mb: 38.0,
                    target_replicas_k: target_k,
                    expires_at_unix: 1_762_600_060,
                    hmac_signature: sig,
                });
            }
        }

        let mut semantic_pubsub_events = Vec::new();
        if let Ok(subs_guard) = self.topic_subscriptions.read() {
            let mut seen_cas = std::collections::HashSet::new();
            for sub in subs_guard.iter() {
                if sub.subscriber_node_id == final_node_id
                    || sub.subscriber_dj.eq_ignore_ascii_case(&req.owner_dj)
                {
                    for evt in self.match_tracks_for_subscription(sub) {
                        if seen_cas.insert(evt.cas_fingerprint.clone()) {
                            semantic_pubsub_events.push(evt);
                        }
                    }
                }
            }
        }

        Ok(SwarmHeartbeatResponse {
            status: "SWARM_HEARTBEAT_ACK".to_string(),
            node_id: final_node_id,
            endpoint_pubkey_id: pubkey_id,
            network_generation: net_gen,
            active_endpoint_url: req.endpoint_url,
            relay_assisted_url: relay_url,
            ewma_rtt_ms: active_ewma_rtt,
            edge_guardian_active: true,
            custody_status: "ONLINE_HEALTHY".to_string(),
            swarm_credit_balance_mb: updated_credit_balance,
            credit_multiplier: SWARM_CREDIT_MULTIPLIER,
            water_filling_allocation: node_alloc,
            outbound_pull_tickets,
            semantic_pubsub_events,
            swarm_strength: strength,
        })
    }
}
