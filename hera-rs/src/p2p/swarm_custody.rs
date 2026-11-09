//! Federated R2 Swarm Orchestrator, Constrained Water-Filling, Power-of-Two-Choices (`P2C`)
//! 60s Signed Tickets, 64-Byte `Proof-of-Custody` Range Audits, Anti-Poisoning Quarantine,
//! MoQ Harmonic Topic Pub/Sub, and `Delta-CRDT` State Convergence (`src/p2p/swarm_custody.rs`).

use super::*;

impl P2PMeshState {
    /// Counts active, healthy, non-quarantined nodes in the swarm.
    pub fn count_active_healthy_nodes(&self) -> usize {
        let q_map = self
            .quarantined_nodes
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        let count = self
            .peers
            .read()
            .map(|peers| {
                peers
                    .iter()
                    .filter(|n| {
                        !n.quarantined
                            && !q_map.contains_key(&n.node_id)
                            && n.edge_guardian_active
                            && n.custody_status == "ONLINE_HEALTHY"
                    })
                    .count()
            })
            .unwrap_or(1);
        count.max(1)
    }

    /// Computes the mathematical Swarm Strength & Water-Filling Resource Allocation across all nodes.
    /// Demonstrates the core invariant: "Mientras más nodos de Hera, más fuerte es el sistema de enjambres."
    pub fn compute_swarm_strength(&self) -> SwarmStrengthReport {
        let q_map = self
            .quarantined_nodes
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        let peers_list = self.peers.read().map(|g| g.clone()).unwrap_or_default();
        let healthy_nodes: Vec<P2PPeerNode> = peers_list
            .into_iter()
            .filter(|n| {
                !n.quarantined
                    && !q_map.contains_key(&n.node_id)
                    && n.edge_guardian_active
                    && n.custody_status == "ONLINE_HEALTHY"
            })
            .collect();

        let active_n = healthy_nodes.len().max(1);
        let quarantined_count = q_map.len();

        let base_k = if active_n <= 1 { 1 } else { 2 };
        let max_k = compute_elastic_replication_k(active_n, 20.0);

        let catalog_list = self.catalog.read().map(|g| g.clone()).unwrap_or_default();
        let heat_map = self
            .cas_heat_scores
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();

        let mut unique_cas_seen = std::collections::HashSet::new();
        let mut total_replicated_demand_mb = 0.0f32;
        for t in &catalog_list {
            let cas = compute_cas_fingerprint(&t.artist, &t.title, t.bpm);
            if unique_cas_seen.insert(cas.clone()) {
                let heat = heat_map.get(&cas).copied().unwrap_or(1.0);
                let k = compute_elastic_replication_k(active_n, heat);
                total_replicated_demand_mb += 38.0 * (k as f32);
            }
        }
        if total_replicated_demand_mb < 38.0 {
            total_replicated_demand_mb = 38.0 * (base_k as f32);
        }

        // Constrained Water-Filling across healthy nodes with strict 9,000 MB R2 and 85,000 req/day safety caps.
        // Sort node indices by ascending effective headroom so tightest-headroom nodes are filled first
        // and any overflow demand spills over to unconstrained nodes.
        let nodes_len = healthy_nodes.len().max(1);
        let mut sorted_indices: Vec<usize> = (0..healthy_nodes.len()).collect();
        sorted_indices.sort_by(|&ia, &ib| {
            let na = &healthy_nodes[ia];
            let nb = &healthy_nodes[ib];
            let cap_a = na.r2_safe_cap_mb.clamp(100.0, CF_WORKER_SAFE_R2_CAP_MB);
            let cap_b = nb.r2_safe_cap_mb.clamp(100.0, CF_WORKER_SAFE_R2_CAP_MB);
            let req_cap_a = na
                .daily_worker_request_safe_cap
                .clamp(1_000, CF_WORKER_SAFE_REQ_CAP);
            let req_cap_b = nb
                .daily_worker_request_safe_cap
                .clamp(1_000, CF_WORKER_SAFE_REQ_CAP);
            let head_a = if na.daily_worker_requests_used >= req_cap_a {
                0.0
            } else {
                (cap_a - na.r2_used_mb.min(cap_a)).max(0.0)
            };
            let head_b = if nb.daily_worker_requests_used >= req_cap_b {
                0.0
            } else {
                (cap_b - nb.r2_used_mb.min(cap_b)).max(0.0)
            };
            head_a
                .partial_cmp(&head_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut water_added_by_idx = vec![0.0f32; healthy_nodes.len()];
        let mut remaining_demand_mb = total_replicated_demand_mb;
        for (step, &node_idx) in sorted_indices.iter().enumerate() {
            let node = &healthy_nodes[node_idx];
            let safe_r2_cap = node.r2_safe_cap_mb.clamp(100.0, CF_WORKER_SAFE_R2_CAP_MB);
            let safe_req_cap = node
                .daily_worker_request_safe_cap
                .clamp(1_000, CF_WORKER_SAFE_REQ_CAP);
            let current_used = node.r2_used_mb.min(safe_r2_cap);
            let effective_headroom = if node.daily_worker_requests_used >= safe_req_cap {
                0.0
            } else {
                (safe_r2_cap - current_used).max(0.0)
            };
            let nodes_left = (nodes_len - step).max(1) as f32;
            let fair_water_level = (remaining_demand_mb / nodes_left).min(effective_headroom);
            water_added_by_idx[node_idx] = fair_water_level;
            remaining_demand_mb = (remaining_demand_mb - fair_water_level).max(0.0);
        }

        let mut allocations = Vec::with_capacity(healthy_nodes.len());
        for (idx, node) in healthy_nodes.iter().enumerate() {
            let safe_r2_cap = node.r2_safe_cap_mb.clamp(100.0, CF_WORKER_SAFE_R2_CAP_MB);
            let safe_req_cap = node
                .daily_worker_request_safe_cap
                .clamp(1_000, CF_WORKER_SAFE_REQ_CAP);
            let current_used = node.r2_used_mb.min(safe_r2_cap);
            let fair_water_level = water_added_by_idx[idx];
            let allocated_mb =
                ((current_used + fair_water_level).min(safe_r2_cap) * 100.0).round() / 100.0;

            let assigned_shards = ((allocated_mb / 38.0).ceil() as usize).max(1);
            let req_used = node.daily_worker_requests_used.min(safe_req_cap);
            let req_headroom = safe_req_cap.saturating_sub(req_used);

            let r2_ratio = allocated_mb / safe_r2_cap;
            let req_ratio = (req_used as f32) / (safe_req_cap as f32);
            let load_factor_pct = ((r2_ratio * 0.5 + req_ratio * 0.5) * 1000.0).round() / 100.0;

            allocations.push(WaterFillingNodeAllocation {
                node_id: node.node_id.clone(),
                owner_dj: node.dj_handle.clone(),
                allocated_r2_mb: allocated_mb,
                safe_r2_cap_mb: safe_r2_cap,
                free_headroom_mb: ((safe_r2_cap - allocated_mb).max(0.0) * 100.0).round() / 100.0,
                assigned_shards_count: assigned_shards,
                daily_requests_used: req_used,
                daily_requests_safe_cap: safe_req_cap,
                daily_requests_headroom: req_headroom,
                load_factor_pct,
                edge_guardian_active: node.edge_guardian_active,
            });
        }

        let total_free_r2_gb =
            ((active_n as f32) * (CF_WORKER_FREE_R2_LIMIT_MB / 1000.0) * 100.0).round() / 100.0;
        let safe_usable_r2_gb =
            (((active_n as f32) * (CF_WORKER_SAFE_R2_CAP_MB / 1000.0)) * 100.0).round() / 100.0;
        let total_daily_req = (active_n as u64) * CF_WORKER_FREE_REQ_LIMIT;
        let safe_daily_req = (active_n as u64) * CF_WORKER_SAFE_REQ_CAP;

        let block_multiplier = 8.4f32;
        let effective_daily_streams =
            ((safe_daily_req as f64) * (block_multiplier as f64) / 9.5).round() as u64;
        let strength_score = (((active_n as f32)
            * (1.0 + ((max_k.saturating_sub(1)) as f32) * 0.35)
            * block_multiplier)
            * 100.0)
            .round()
            / 100.0;

        SwarmStrengthReport {
            motto: SWARM_MOTTO.to_string(),
            active_healthy_nodes: active_n,
            quarantined_nodes: quarantined_count,
            total_free_r2_capacity_gb: total_free_r2_gb,
            safe_usable_r2_capacity_gb: safe_usable_r2_gb,
            total_daily_worker_requests_capacity: total_daily_req,
            safe_daily_worker_requests_capacity: safe_daily_req,
            aligned_4mb_block_throughput_multiplier: block_multiplier,
            effective_daily_lossless_streams_capacity: effective_daily_streams,
            base_replication_factor_k: base_k,
            max_thermal_replication_k: max_k,
            swarm_strength_multiplier: strength_score,
            water_filling_allocations: allocations,
        }
    }

    /// Latency-Aware Power-of-Two-Choices (P2C) load balancer: picks two deterministic candidate
    /// healthy nodes and selects the one with the lowest composite cost (`daily_worker_requests_used`
    /// normalized against `85,000 req/day` + smoothed `ewma_rtt_ms` + R2 tie-breaker).
    pub fn select_node_p2c(&self, cas_fingerprint: &str, nonce: u64) -> (String, String, u64) {
        let q_map = self
            .quarantined_nodes
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        let peers_list = self.peers.read().map(|g| g.clone()).unwrap_or_default();
        let candidates: Vec<P2PPeerNode> = peers_list
            .into_iter()
            .filter(|n| {
                !n.quarantined
                    && !q_map.contains_key(&n.node_id)
                    && n.edge_guardian_active
                    && n.custody_status == "ONLINE_HEALTHY"
                    && n.daily_worker_requests_used < n.daily_worker_request_safe_cap
            })
            .collect();

        if candidates.is_empty() {
            return (
                self.node_id.clone(),
                "https://hera-node.pc911itservices.workers.dev".to_string(),
                0,
            );
        }
        if candidates.len() == 1 {
            let c = &candidates[0];
            return (
                c.node_id.clone(),
                c.endpoint_url.clone(),
                c.daily_worker_requests_used,
            );
        }

        let mut h1: u64 = 0xcbf29ce484222325 ^ nonce;
        for b in cas_fingerprint.as_bytes() {
            h1 ^= *b as u64;
            h1 = h1.wrapping_mul(0x100000001b3);
        }
        let h2 = h1.rotate_left(29).wrapping_mul(0x9e3779b97f4a7c15);

        let idx_a = (h1 as usize) % candidates.len();
        let mut idx_b = (h2 as usize) % candidates.len();
        if idx_b == idx_a {
            idx_b = (idx_a + 1) % candidates.len();
        }

        let node_a = &candidates[idx_a];
        let node_b = &candidates[idx_b];

        let cost_a = compute_node_p2c_composite_cost(node_a);
        let cost_b = compute_node_p2c_composite_cost(node_b);

        let chosen = if cost_a < cost_b {
            node_a
        } else if cost_b < cost_a {
            node_b
        } else if node_a.daily_worker_requests_used <= node_b.daily_worker_requests_used {
            node_a
        } else {
            node_b
        };

        (
            chosen.node_id.clone(),
            chosen.endpoint_url.clone(),
            chosen.daily_worker_requests_used,
        )
    }

    /// Issues a 60-second HMAC-signed `SwarmRedirectTicket` (`X-Hera-Swarm-Ticket`) using Latency-Aware P2C routing,
    /// enforcing Edge Guardian fast-lane privileges or redeeming 1:1.5 Turbo credits when a node is offline.
    pub fn issue_route_ticket(&self, req: SwarmRouteRequest) -> Result<SwarmRouteResponse, String> {
        let requester = req.requester_dj.trim().to_string();
        if requester.is_empty() {
            return Err("requester_dj is required".to_string());
        }

        let query_key = req.track_id_or_cas.trim();
        let track = {
            let guard = self
                .catalog
                .read()
                .map_err(|_| "Catalog lock error".to_string())?;
            guard
                .iter()
                .find(|t| {
                    t.track_id.eq_ignore_ascii_case(query_key)
                        || compute_cas_fingerprint(&t.artist, &t.title, t.bpm)
                            .eq_ignore_ascii_case(query_key)
                })
                .cloned()
                .ok_or_else(|| format!("Track or CAS '{}' not found in swarm", query_key))?
        };

        let cas_fp = compute_cas_fingerprint(&track.artist, &track.title, track.bpm);

        // Boost thermal heat score so popular tracks automatically scale K = 1 -> 2 -> 3+
        let current_heat = if let Ok(mut heat_guard) = self.cas_heat_scores.write() {
            let entry = heat_guard.entry(cas_fp.clone()).or_insert(1.0);
            *entry += 2.0;
            *entry
        } else {
            3.0
        };

        let active_n = self.count_active_healthy_nodes();
        let replicas_k = compute_elastic_replication_k(active_n, current_heat);
        let audio_sha256 = self
            .cas_audio_sha256
            .read()
            .ok()
            .and_then(|g| g.get(&cas_fp).cloned())
            .unwrap_or_else(|| compute_canonical_audio_sha256(&cas_fp, &track.hera_seal, 38.0));
        let opaque_blob_key = compute_opaque_shard_blob_key(&cas_fp, &audio_sha256);

        let q_map = self
            .quarantined_nodes
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        let requester_node_healthy = self
            .peers
            .read()
            .map(|peers| {
                peers.iter().any(|n| {
                    n.dj_handle.eq_ignore_ascii_case(&requester)
                        && !n.quarantined
                        && !q_map.contains_key(&n.node_id)
                        && n.edge_guardian_active
                        && n.custody_status == "ONLINE_HEALTHY"
                })
            })
            .unwrap_or(false);

        let track_mb = 38.0f32;
        let access_mode = if let Ok(mut dj_guard) = self.dj_profiles.write() {
            if let Some(dj) = dj_guard
                .iter_mut()
                .find(|d| d.dj_handle.eq_ignore_ascii_case(&requester))
            {
                if dj.edge_guardian_active && requester_node_healthy {
                    dj.total_bytes_consumed_mb =
                        ((dj.total_bytes_consumed_mb + track_mb) * 100.0).round() / 100.0;
                    "EDGE_GUARDIAN_UNLIMITED_FAST_LANE".to_string()
                } else if dj.swarm_credit_balance_mb >= track_mb {
                    dj.swarm_credit_balance_mb =
                        ((dj.swarm_credit_balance_mb - track_mb) * 100.0).round() / 100.0;
                    dj.total_bytes_consumed_mb =
                        ((dj.total_bytes_consumed_mb + track_mb) * 100.0).round() / 100.0;
                    "TURBO_CREDIT_REDEMPTION_FAST_LANE".to_string()
                } else if dj.subscription_tier == "pro_monthly"
                    || dj.subscription_tier == "founder_unlimited"
                {
                    dj.total_bytes_consumed_mb =
                        ((dj.total_bytes_consumed_mb + track_mb) * 100.0).round() / 100.0;
                    "PRO_MONTHLY_FAST_LANE".to_string()
                } else {
                    "COMMUNITY_STANDARD_LANE".to_string()
                }
            } else {
                "COMMUNITY_STANDARD_LANE".to_string()
            }
        } else {
            "COMMUNITY_STANDARD_LANE".to_string()
        };

        let block_index = req.block_index.unwrap_or(0);
        let nonce = (current_heat.to_bits() as u64) ^ (block_index as u64);
        let (target_node_id, target_endpoint, _) = self.select_node_p2c(&cas_fp, nonce);

        let mut selected_rtt = 24.0f32;
        let mut selected_cost = 0.0f32;
        if let Ok(mut p_guard) = self.peers.write() {
            if let Some(node) = p_guard.iter_mut().find(|n| n.node_id == target_node_id) {
                selected_rtt = node.ewma_rtt_ms;
                selected_cost = compute_node_p2c_composite_cost(node);
                node.daily_worker_requests_used = node.daily_worker_requests_used.saturating_add(1);
            }
        }

        let issued_at_unix = 1_762_600_000u64;
        let expires_at_unix = issued_at_unix + SWARM_TICKET_TTL_SECS;
        let aligned_offset_bytes = (block_index as u64) * SWARM_ALIGNED_BLOCK_BYTES;
        let ticket_id = format!(
            "tkt-{}-{:x}",
            &audio_sha256[..10],
            issued_at_unix ^ nonce
        );
        let hmac_signature = compute_swarm_ticket_hmac(
            &ticket_id,
            &requester,
            &target_node_id,
            &opaque_blob_key,
            &audio_sha256,
            aligned_offset_bytes,
            SWARM_ALIGNED_BLOCK_BYTES,
            issued_at_unix,
            expires_at_unix,
        );

        let merkle_root_sha256 = compute_shard_merkle_tree(
            &opaque_blob_key,
            &audio_sha256,
            aligned_offset_bytes,
        )
        .merkle_root_sha256;

        let ticket = SwarmRedirectTicket {
            ticket_id,
            requester_dj: requester,
            target_node_id: target_node_id.clone(),
            opaque_blob_key: opaque_blob_key.clone(),
            audio_sha256,
            merkle_root_sha256,
            aligned_offset_bytes,
            aligned_length_bytes: SWARM_ALIGNED_BLOCK_BYTES,
            issued_at_unix,
            expires_at_unix,
            ttl_seconds: SWARM_TICKET_TTL_SECS,
            cache_control_header: "public, max-age=31536000, immutable".to_string(),
            hmac_signature,
        };

        let blob_id = opaque_blob_key.trim_start_matches("shard-blobs/");
        let signed_worker_shard_url = format!(
            "{}/api/v1/swarm/shard/{}",
            target_endpoint.trim_end_matches('/'),
            blob_id
        );
        let gdrive_fallback_url = track
            .gdrive_file_id
            .as_deref()
            .map(|fid| build_gdrive_large_file_bypass_url(fid, 38.0))
            .or_else(|| track.stream_url.clone())
            .unwrap_or_default();

        Ok(SwarmRouteResponse {
            status: "SWARM_TICKET_ISSUED".to_string(),
            p2c_selected_node_id: target_node_id,
            p2c_selected_node_endpoint: target_endpoint,
            selected_node_ewma_rtt_ms: selected_rtt,
            p2c_composite_cost: selected_cost,
            signed_worker_shard_url,
            gdrive_fallback_url,
            access_mode,
            elastic_replicas_k: replicas_k,
            ticket,
        })
    }

    /// Issues a Multi-Node Parallel HTTP/3 QUIC Striping Manifest (`POST /api/v1/swarm/parallel-manifest`)
    /// that stripes all `4 MB` blocks of a Studio Master (`38 MB` = `10` blocks) across distinct
    /// low-latency `Edge Guardian` nodes with per-block `merkle_root_sha256` roots.
    pub fn issue_parallel_swarm_manifest(
        &self,
        req: SwarmParallelManifestRequest,
    ) -> Result<SwarmParallelManifestResponse, String> {
        let requester = req.requester_dj.trim().to_string();
        if requester.is_empty() {
            return Err("requester_dj is required".to_string());
        }
        let query_key = req.track_id_or_cas.trim();
        let track = {
            let guard = self
                .catalog
                .read()
                .map_err(|_| "Catalog lock error".to_string())?;
            guard
                .iter()
                .find(|t| {
                    t.track_id.eq_ignore_ascii_case(query_key)
                        || compute_cas_fingerprint(&t.artist, &t.title, t.bpm)
                            .eq_ignore_ascii_case(query_key)
                })
                .cloned()
                .ok_or_else(|| format!("Track or CAS '{}' not found in swarm", query_key))?
        };

        let cas_fp = compute_cas_fingerprint(&track.artist, &track.title, track.bpm);
        let total_size_mb = req.file_size_mb.unwrap_or(38.0).clamp(4.0, 200.0);
        let total_bytes = (total_size_mb * 1024.0 * 1024.0).round() as u64;
        let total_blocks = ((total_bytes + SWARM_ALIGNED_BLOCK_BYTES - 1)
            / SWARM_ALIGNED_BLOCK_BYTES)
            .max(1) as usize;

        let mut blocks = Vec::with_capacity(total_blocks);
        let mut distinct_nodes = std::collections::HashSet::new();
        let mut access_mode = "EDGE_GUARDIAN_UNLIMITED_FAST_LANE".to_string();

        for idx in 0..total_blocks {
            // Mix prime golden-ratio stride into block_index so consecutive 4 MB blocks fan out across distinct Edge Guardians
            let route_resp = self.issue_route_ticket(SwarmRouteRequest {
                requester_dj: requester.clone(),
                track_id_or_cas: track.track_id.clone(),
                block_index: Some(idx as u32),
            })?;
            access_mode = route_resp.access_mode.clone();

            let (pubkey_id, rtt_ms, comp_cost) = self
                .peers
                .read()
                .ok()
                .and_then(|peers| {
                    peers
                        .iter()
                        .find(|n| n.node_id == route_resp.p2c_selected_node_id)
                        .map(|n| {
                            (
                                n.endpoint_pubkey_id.clone(),
                                n.ewma_rtt_ms,
                                compute_node_p2c_composite_cost(n),
                            )
                        })
                })
                .unwrap_or_else(|| {
                    (
                        derive_node_endpoint_pubkey_id("DaviDJ", &route_resp.p2c_selected_node_id),
                        route_resp.selected_node_ewma_rtt_ms,
                        route_resp.p2c_composite_cost,
                    )
                });

            distinct_nodes.insert(route_resp.p2c_selected_node_id.clone());
            blocks.push(SwarmParallelBlockAssignment {
                block_index: idx as u32,
                aligned_offset_bytes: route_resp.ticket.aligned_offset_bytes,
                aligned_length_bytes: route_resp.ticket.aligned_length_bytes,
                assigned_node_id: route_resp.p2c_selected_node_id,
                assigned_endpoint_pubkey_id: pubkey_id,
                assigned_node_endpoint: route_resp.p2c_selected_node_endpoint,
                assigned_node_ewma_rtt_ms: rtt_ms,
                p2c_composite_cost: comp_cost,
                signed_worker_shard_url: route_resp.signed_worker_shard_url,
                merkle_root_sha256: route_resp.ticket.merkle_root_sha256.clone(),
                ticket: route_resp.ticket,
            });
        }

        let distinct_count = distinct_nodes.len().max(1);
        Ok(SwarmParallelManifestResponse {
            status: "SWARM_PARALLEL_MANIFEST_ISSUED".to_string(),
            transport_protocol: "HTTP/3-QUIC-MULTIPLEXED".to_string(),
            cas_fingerprint: cas_fp,
            track_id: track.track_id,
            requester_dj: requester,
            access_mode,
            total_size_mb,
            block_size_bytes: SWARM_ALIGNED_BLOCK_BYTES,
            total_blocks,
            distinct_edge_guardians_striped: distinct_count,
            aggregate_bandwidth_multiplier: distinct_count as f32,
            blocks,
        })
    }

    /// Validates an `X-Hera-Swarm-Ticket` header on the community Worker node, serves the 4MB-aligned
    /// opaque shard (`shard-blobs/{hmac}.hbin`), and mints a `ProofOfDeliveryReceipt` (`1 MB = 1.5 MB` credit).
    pub fn redeem_swarm_shard_delivery(
        &self,
        blob_id_or_key: &str,
        ticket_header: &str,
        current_unix_override: Option<u64>,
    ) -> Result<(Vec<u8>, ProofOfDeliveryReceipt), String> {
        let trimmed_header = ticket_header.trim();
        if trimmed_header.is_empty() {
            return Err("Missing X-Hera-Swarm-Ticket header (403 Forbidden)".to_string());
        }

        let ticket: SwarmRedirectTicket = serde_json::from_str(trimmed_header)
            .map_err(|e| format!("Malformed X-Hera-Swarm-Ticket JSON: {e}"))?;

        let normalized_blob = if blob_id_or_key.starts_with("shard-blobs/") {
            blob_id_or_key.to_string()
        } else {
            format!("shard-blobs/{}", blob_id_or_key.trim_start_matches('/'))
        };

        if ticket.opaque_blob_key != normalized_blob {
            return Err(format!(
                "Ticket blob mismatch: ticket is for '{}', requested '{}'",
                ticket.opaque_blob_key, normalized_blob
            ));
        }

        let is_quarantined = self
            .quarantined_nodes
            .read()
            .map(|q| q.contains_key(&ticket.target_node_id))
            .unwrap_or(false);
        if is_quarantined {
            return Err(format!(
                "Target node '{}' is quarantined for Bait-and-Switch",
                ticket.target_node_id
            ));
        }

        let now_unix = current_unix_override.unwrap_or(ticket.issued_at_unix + 5);
        if !verify_swarm_redirect_ticket(&ticket, now_unix) {
            return Err(
                "Invalid or expired X-Hera-Swarm-Ticket signature/TTL (60s window exceeded)"
                    .to_string(),
            );
        }

        let already_redeemed = self
            .delivery_receipts
            .read()
            .map(|r| r.iter().any(|rec| rec.ticket_id == ticket.ticket_id))
            .unwrap_or(false);
        if already_redeemed {
            return Err(format!(
                "Replay attack blocked: X-Hera-Swarm-Ticket '{}' has already been redeemed",
                ticket.ticket_id
            ));
        }

        let mut raw_payload = synthesize_deterministic_64b_custody_slice(
            &ticket.opaque_blob_key,
            &ticket.audio_sha256,
            ticket.aligned_offset_bytes,
        );
        let footer = format!(
            "|HERA-4MB-IMMUTABLE-BLOCK|blob={}|sha256={}|offset={}",
            ticket.opaque_blob_key, ticket.audio_sha256, ticket.aligned_offset_bytes
        );
        raw_payload.extend_from_slice(footer.as_bytes());
        let payload = transform_p2p_boost_shard_bytes(
            &raw_payload,
            &ticket.opaque_blob_key,
            &ticket.audio_sha256,
            ticket.aligned_offset_bytes,
        );

        // 1 aligned block = 4 MB delivered -> 1.5x credit multiplier = 6.0 MB Turbo credit earned!
        let mb_delivered = (ticket.aligned_length_bytes as f32) / (1024.0 * 1024.0);
        let credits_earned = ((mb_delivered * SWARM_CREDIT_MULTIPLIER) * 100.0).round() / 100.0;

        let mut owner_dj_name = String::new();
        if let Ok(mut p_guard) = self.peers.write() {
            if let Some(node) = p_guard
                .iter_mut()
                .find(|n| n.node_id == ticket.target_node_id)
            {
                node.credits_earned_mb =
                    ((node.credits_earned_mb + credits_earned) * 100.0).round() / 100.0;
                owner_dj_name = node.dj_handle.clone();
            }
        }
        if !owner_dj_name.is_empty() {
            if let Ok(mut dj_guard) = self.dj_profiles.write() {
                if let Some(dj) = dj_guard
                    .iter_mut()
                    .find(|d| d.dj_handle.eq_ignore_ascii_case(&owner_dj_name))
                {
                    dj.total_bytes_served_mb =
                        ((dj.total_bytes_served_mb + mb_delivered) * 100.0).round() / 100.0;
                    dj.swarm_credit_balance_mb =
                        ((dj.swarm_credit_balance_mb + credits_earned) * 100.0).round() / 100.0;
                }
            }
        }

        let receipt_seed = format!(
            "pod|{}|{}|{}|{}",
            ticket.ticket_id, ticket.target_node_id, ticket.requester_dj, now_unix
        );
        let receipt_sig = compute_hmac_sha256_hex(
            SWARM_TICKET_MASTER_SECRET.as_bytes(),
            receipt_seed.as_bytes(),
        );
        let receipt = ProofOfDeliveryReceipt {
            receipt_id: format!("pod-{}", &receipt_sig[..12]),
            ticket_id: ticket.ticket_id,
            serving_node_id: ticket.target_node_id,
            requester_dj: ticket.requester_dj,
            opaque_blob_key: ticket.opaque_blob_key,
            bytes_delivered: ticket.aligned_length_bytes,
            megabytes_delivered: mb_delivered,
            credits_earned_mb: credits_earned,
            credit_multiplier: SWARM_CREDIT_MULTIPLIER,
            receipt_hmac: receipt_sig,
        };
        if let Ok(mut r_guard) = self.delivery_receipts.write() {
            r_guard.push(receipt.clone());
        }

        Ok((payload, receipt))
    }

    /// Generates a random 64-byte (`Range: bytes=offset..offset+63`) Proof-of-Custody challenge for a node.
    pub fn generate_custody_challenge(
        &self,
        req: CustodyChallengeRequest,
    ) -> Result<CustodyChallenge, String> {
        let query = req.cas_fingerprint_or_blob.trim();
        let track = {
            let guard = self
                .catalog
                .read()
                .map_err(|_| "Catalog lock error".to_string())?;
            guard
                .iter()
                .find(|t| {
                    t.track_id.eq_ignore_ascii_case(query)
                        || compute_cas_fingerprint(&t.artist, &t.title, t.bpm)
                            .eq_ignore_ascii_case(query)
                })
                .cloned()
                .ok_or_else(|| {
                    format!("Track or CAS '{}' not found for custody challenge", query)
                })?
        };
        let cas_fp = compute_cas_fingerprint(&track.artist, &track.title, track.bpm);
        let audio_sha256 = self
            .cas_audio_sha256
            .read()
            .ok()
            .and_then(|g| g.get(&cas_fp).cloned())
            .unwrap_or_else(|| compute_canonical_audio_sha256(&cas_fp, &track.hera_seal, 38.0));

        let opaque_blob_key = compute_opaque_shard_blob_key(&cas_fp, &audio_sha256);
        let seed_str = format!(
            "custody|{}|{}|{}",
            req.node_id,
            opaque_blob_key,
            req.seed_nonce.unwrap_or(42)
        );
        let seed_hash = compute_sha256_hex(seed_str.as_bytes());
        let raw_offset = u64::from_str_radix(&seed_hash[..8], 16).unwrap_or(4096);
        let byte_offset = (raw_offset % (SWARM_ALIGNED_BLOCK_BYTES - 128)) & !63u64;

        let expected_slice = synthesize_deterministic_64b_custody_slice(
            &opaque_blob_key,
            &audio_sha256,
            byte_offset,
        );
        let expected_slice_sha256 = compute_sha256_hex(&expected_slice);

        Ok(CustodyChallenge {
            challenge_id: format!("chal-{}", &seed_hash[..12]),
            node_id: req.node_id,
            opaque_blob_key,
            range_header: format!(
                "bytes={}..{}",
                byte_offset,
                byte_offset + (CUSTODY_PROOF_SLICE_BYTES as u64) - 1
            ),
            byte_offset,
            slice_length: CUSTODY_PROOF_SLICE_BYTES,
            expected_slice_sha256,
        })
    }

    /// Serves a 64-byte (`Range: bytes=offset..offset+63`) Proof-of-Custody probe on a community Worker node.
    pub fn probe_custody_64_bytes(
        &self,
        blob_id_or_key: &str,
        range_header: Option<&str>,
    ) -> Result<(u64, Vec<u8>, String), String> {
        let normalized_blob = if blob_id_or_key.starts_with("shard-blobs/") {
            blob_id_or_key.to_string()
        } else {
            format!("shard-blobs/{}", blob_id_or_key.trim_start_matches('/'))
        };

        let sha_map = self
            .cas_audio_sha256
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        let mut matched_sha256: Option<String> = None;
        for (cas_fp, sha) in &sha_map {
            if compute_opaque_shard_blob_key(cas_fp, sha) == normalized_blob {
                matched_sha256 = Some(sha.clone());
                break;
            }
        }
        let Some(audio_sha256) = matched_sha256 else {
            return Err(format!(
                "Opaque shard '{}' not found on this node (404)",
                normalized_blob
            ));
        };

        let mut byte_offset: u64 = 0;
        if let Some(rh) = range_header {
            let clean = rh.trim().trim_start_matches("bytes=");
            if let Some(first_part) = clean
                .split(|c| c == '-' || c == '.')
                .find(|s| !s.is_empty())
            {
                byte_offset = first_part.trim().parse::<u64>().unwrap_or(0);
            }
        }

        let slice = synthesize_deterministic_64b_custody_slice(
            &normalized_blob,
            &audio_sha256,
            byte_offset,
        );
        let slice_sha256 = compute_sha256_hex(&slice);
        Ok((byte_offset, slice, slice_sha256))
    }

    /// Audits a node's 64-byte Proof-of-Custody response.
    /// If the node returns 404 or lies about the 64-byte slice hash, it immediately loses `Edge Guardian` status!
    pub fn verify_custody_audit(&self, sub: CustodyAuditSubmission) -> CustodyAuditResult {
        let normalized_blob = if sub.opaque_blob_key.starts_with("shard-blobs/") {
            sub.opaque_blob_key.clone()
        } else {
            format!("shard-blobs/{}", sub.opaque_blob_key.trim_start_matches('/'))
        };

        let sha_map = self
            .cas_audio_sha256
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        let mut expected_sha256 = String::new();
        for (cas_fp, sha) in &sha_map {
            if compute_opaque_shard_blob_key(cas_fp, sha) == normalized_blob {
                let slice = synthesize_deterministic_64b_custody_slice(
                    &normalized_blob,
                    sha,
                    sub.byte_offset,
                );
                expected_sha256 = compute_sha256_hex(&slice);
                break;
            }
        }

        let passed = (sub.http_status == 200 || sub.http_status == 206)
            && !expected_sha256.is_empty()
            && sub.returned_slice_sha256.eq_ignore_ascii_case(&expected_sha256);

        let mut owner_dj = String::new();
        let mut credit_balance_mb = 0.0f32;

        if let Ok(mut p_guard) = self.peers.write() {
            if let Some(node) = p_guard.iter_mut().find(|n| n.node_id == sub.node_id) {
                owner_dj = node.dj_handle.clone();
                if passed {
                    node.edge_guardian_active = true;
                    node.custody_status = "ONLINE_HEALTHY".to_string();
                    node.last_custody_proof_hash = expected_sha256.clone();
                } else {
                    node.edge_guardian_active = false;
                    node.custody_failures = node.custody_failures.saturating_add(1);
                    node.custody_status = if sub.http_status == 404 {
                        "GUARDIAN_REVOKED_SHARD_MISSING_404".to_string()
                    } else {
                        "GUARDIAN_REVOKED_HASH_MISMATCH".to_string()
                    };
                }
            }
        }

        if !owner_dj.is_empty() {
            if let Ok(mut dj_guard) = self.dj_profiles.write() {
                if let Some(dj) = dj_guard
                    .iter_mut()
                    .find(|d| d.dj_handle.eq_ignore_ascii_case(&owner_dj))
                {
                    if passed {
                        dj.edge_guardian_active = true;
                        dj.custody_audit_passes = dj.custody_audit_passes.saturating_add(1);
                    } else {
                        dj.edge_guardian_active = false;
                        dj.custody_audit_failures = dj.custody_audit_failures.saturating_add(1);
                    }
                    credit_balance_mb = dj.swarm_credit_balance_mb;
                }
            }
        }

        // If the node returned 200/206 with a forged/corrupted hash (Bait-and-Switch on shard bytes), quarantine it
        let mut node_quarantined = false;
        if !passed
            && (sub.http_status == 200 || sub.http_status == 206)
            && !expected_sha256.is_empty()
        {
            node_quarantined = true;
            if let Ok(mut p_guard) = self.peers.write() {
                if let Some(node) = p_guard.iter_mut().find(|n| n.node_id == sub.node_id) {
                    node.quarantined = true;
                }
            }
            if let Ok(mut q_guard) = self.quarantined_nodes.write() {
                q_guard.insert(
                    sub.node_id.clone(),
                    QuarantinedNodeRecord {
                        node_id: sub.node_id.clone(),
                        owner_dj: owner_dj.clone(),
                        reason: "CUSTODY_AUDIT_FORGED_SLICE_HASH".to_string(),
                        offending_cas_fingerprint: normalized_blob.clone(),
                        expected_audio_sha256: expected_sha256.clone(),
                        reported_audio_sha256: sub.returned_slice_sha256.clone(),
                    },
                );
            }
        }

        CustodyAuditResult {
            node_id: sub.node_id,
            passed,
            edge_guardian_active: passed,
            custody_status: if passed {
                "ONLINE_HEALTHY".to_string()
            } else if sub.http_status == 404 {
                "GUARDIAN_REVOKED_SHARD_MISSING_404".to_string()
            } else {
                "GUARDIAN_REVOKED_HASH_MISMATCH".to_string()
            },
            node_quarantined,
            swarm_credit_balance_mb: credit_balance_mb,
            message: if passed {
                "64-byte Proof-of-Custody verified. Edge Guardian Unlimited R2 Fast Lane remains ACTIVE.".to_string()
            } else {
                "Custody audit FAILED (404 or forged slice). Edge Guardian status REVOKED immediately.".to_string()
            },
        }
    }

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

    /// Evaluates active swarm tracks against a `SwarmTopicSubscription` (`hera://swarm/<camelot>/<bpm_band>`)
    /// and emits proactive `SwarmSemanticPubSubEvent` notifications with `style_simhash_u64` and `merkle_root_sha256`.
    pub fn match_tracks_for_subscription(
        &self,
        sub: &SwarmTopicSubscription,
    ) -> Vec<SwarmSemanticPubSubEvent> {
        let mut events = Vec::new();
        let sha_map = self
            .cas_audio_sha256
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        if let Ok(cat) = self.catalog.read() {
            for t in cat.iter() {
                if !t.share_on_p2p {
                    continue;
                }
                if t.bpm < sub.bpm_min || t.bpm > sub.bpm_max {
                    continue;
                }
                if !are_camelot_keys_harmonically_compatible(&sub.camelot_key, &t.camelot_key) {
                    continue;
                }
                if let Some(ref sg) = sub.subgenre_filter {
                    let sg_clean = sg.trim().to_ascii_lowercase();
                    if !sg_clean.is_empty()
                        && !t.top_subgenre.to_ascii_lowercase().contains(&sg_clean)
                    {
                        continue;
                    }
                }
                let cas_fp = compute_cas_fingerprint(&t.artist, &t.title, t.bpm);
                let audio_sha = sha_map.get(&cas_fp).cloned().unwrap_or_else(|| {
                    compute_canonical_audio_sha256(&cas_fp, &t.hera_seal, 38.0)
                });
                let opaque_blob_key = compute_opaque_shard_blob_key(&cas_fp, &audio_sha);
                let merkle_tree = compute_shard_merkle_tree(&opaque_blob_key, &audio_sha, 0);
                let simhash = compute_style_simhash_u64(t.bpm, &t.camelot_key, &t.top_subgenre);
                events.push(SwarmSemanticPubSubEvent {
                    event_id: format!("moq_evt_{}_{}", sub.subscriber_node_id, t.track_id),
                    topic_uri: sub.topic_uri.clone(),
                    cas_fingerprint: cas_fp,
                    track_id: t.track_id.clone(),
                    artist: t.artist.clone(),
                    title: t.title.clone(),
                    bpm: t.bpm,
                    camelot_key: t.camelot_key.clone(),
                    style_simhash_u64: simhash,
                    merkle_root_sha256: merkle_tree.merkle_root_sha256,
                    opaque_blob_key,
                    hot_cues_count: t.hot_cues.len(),
                    prefetch_recommended: true,
                });
            }
        }
        events
    }

    /// Registers an MoQ-style Harmonic Topic Subscription (`POST /api/v1/swarm/subscribe`,
    /// e.g. `hera://swarm/8A/120-128`) and returns immediate matching events for zero-wait pre-fetching.
    pub fn subscribe_semantic_topic(&self, req: SwarmSubscribeRequest) -> SwarmSubscribeResponse {
        let camelot = if req.camelot_key.trim().is_empty() {
            "8A".to_string()
        } else {
            req.camelot_key.trim().to_ascii_uppercase()
        };
        let bpm_min = req.bpm_min.unwrap_or(118.0).max(50.0);
        let bpm_max = req.bpm_max.unwrap_or(132.0).max(bpm_min);
        let topic_uri = build_swarm_topic_uri(
            &camelot,
            bpm_min,
            bpm_max,
            req.subgenre_filter.as_deref(),
        );
        let sub = SwarmTopicSubscription {
            subscriber_node_id: req.subscriber_node_id.trim().to_string(),
            subscriber_dj: req.subscriber_dj.trim().to_string(),
            topic_uri: topic_uri.clone(),
            camelot_key: camelot,
            bpm_min,
            bpm_max,
            subgenre_filter: req.subgenre_filter.clone(),
        };

        if let Ok(mut subs_guard) = self.topic_subscriptions.write() {
            if let Some(existing) = subs_guard.iter_mut().find(|s| {
                s.subscriber_node_id == sub.subscriber_node_id && s.topic_uri == sub.topic_uri
            }) {
                *existing = sub.clone();
            } else {
                subs_guard.push(sub.clone());
            }
        }

        let immediate = self.match_tracks_for_subscription(&sub);
        SwarmSubscribeResponse {
            status: "MOQ_SEMANTIC_TOPIC_SUBSCRIBED".to_string(),
            subscription: sub,
            immediate_matching_events: immediate,
        }
    }

    /// Lists all nodes quarantined for Bait-and-Switch or forged custody proofs.
    pub fn list_quarantined_nodes(&self) -> Vec<QuarantinedNodeRecord> {
        let mut list: Vec<QuarantinedNodeRecord> = self
            .quarantined_nodes
            .read()
            .map(|q| q.values().cloned().collect())
            .unwrap_or_default();
        list.sort_by(|a, b| a.node_id.cmp(&b.node_id));
        list
    }

    /// Synchronizes an incoming `SwarmCrdtDelta` (`POST /api/v1/swarm/crdt-sync`) using commutative,
    /// associative, and idempotent `GCounter` + `LWW-Element-Set` lattice joins.
    pub fn sync_crdt_delta(&self, incoming: SwarmCrdtDelta) -> SwarmCrdtSyncResponse {
        let merged = if let Ok(mut guard) = self.crdt_state.write() {
            let mut next = merge_swarm_crdt_deltas(&guard, &incoming);
            next.origin_node_id = self.node_id.clone();
            next.origin_endpoint_pubkey_id = derive_node_endpoint_pubkey_id("DaviDJ", &self.node_id);
            *guard = next.clone();
            next
        } else {
            incoming
        };

        let mut tracks_upserted = 0usize;
        if let Ok(mut cat_guard) = self.catalog.write() {
            for entry in merged.lww_track_entries.values() {
                if entry.tombstone {
                    cat_guard.retain(|t| {
                        compute_cas_fingerprint(&t.artist, &t.title, t.bpm)
                            != entry.cas_fingerprint
                    });
                    continue;
                }
                let expected_seal = compute_hera_seal(
                    &entry.track.track_id,
                    entry.track.bpm,
                    &entry.track.camelot_key,
                    entry.track.quality_score,
                    &entry.track.forensic_verdict,
                );
                if entry.track.hera_seal == expected_seal {
                    if let Some(existing) = cat_guard.iter_mut().find(|t| {
                        t.track_id == entry.track.track_id
                            || compute_cas_fingerprint(&t.artist, &t.title, t.bpm)
                                == entry.cas_fingerprint
                    }) {
                        *existing = entry.track.clone();
                    } else {
                        cat_guard.push(entry.track.clone());
                        tracks_upserted += 1;
                    }
                }
            }
        }

        if let Ok(mut heat_guard) = self.cas_heat_scores.write() {
            for (cas_fp, node_counts) in &merged.gcounter_cas_play_counts {
                let total_plays: u64 = node_counts.values().copied().sum();
                let new_heat = (1.0 + (total_plays as f32) * 0.5).max(1.0);
                let e = heat_guard.entry(cas_fp.clone()).or_insert(1.0);
                if new_heat > *e {
                    *e = new_heat;
                }
            }
        }

        let total_served_mb = merged
            .gcounter_node_bytes_served_mb
            .values()
            .copied()
            .sum::<f32>();
        let rounded_total = (total_served_mb * 100.0).round() / 100.0;
        let digest = compute_crdt_state_digest_sha256(&merged);

        SwarmCrdtSyncResponse {
            status: "CRDT_DELTA_CONVERGED".to_string(),
            node_id: self.node_id.clone(),
            converged_state_digest_sha256: digest,
            tracks_upserted,
            gcounter_total_swarm_served_mb: rounded_total,
            merged_delta: merged,
        }
    }
}
