use super::super::*;

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
}
