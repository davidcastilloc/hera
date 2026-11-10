use super::*;

impl P2PMeshState {
    pub fn register_peer_node(&self, peer: P2PPeerNode) -> Vec<P2PPeerNode> {
        if let Ok(mut guard) = self.peers.write() {
            guard.retain(|p| p.node_id != peer.node_id);
            guard.push(peer);
            return guard.clone();
        }
        vec![]
    }

    pub fn list_peers(&self) -> Vec<P2PPeerNode> {
        self.peers.read().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn federate_sync(&self, req: FederatedSyncRequest) -> FederatedSyncResponse {
        let peer_node_id = req.peer.node_id.clone();
        let peer_dj = req.peer.dj_handle.clone();

        // Reject already quarantined nodes immediately
        let already_quarantined = self
            .quarantined_nodes
            .read()
            .map(|q| q.contains_key(&peer_node_id))
            .unwrap_or(false);
        if already_quarantined {
            let mut blocked_peer = req.peer;
            blocked_peer.quarantined = true;
            blocked_peer.edge_guardian_active = false;
            blocked_peer.custody_status = "QUARANTINED_BAIT_AND_SWITCH".to_string();
            return FederatedSyncResponse {
                status: "REJECTED_NODE_QUARANTINED".to_string(),
                responder_node_id: self.node_id.clone(),
                peer_registered: blocked_peer,
                tracks_accepted: 0,
                tracks_rejected_invalid_seal: req.shared_tracks.len(),
                node_quarantined_for_bait_and_switch: true,
                federated_swarm_catalog: Vec::new(),
                active_peers: self.list_peers(),
            };
        }

        let mut accepted = 0usize;
        let mut rejected = 0usize;
        let mut bait_and_switch_detected = false;

        if let Ok(mut c_guard) = self.catalog.write() {
            for mut trk in req.shared_tracks {
                let expected_seal = compute_hera_seal(
                    &trk.track_id,
                    trk.bpm,
                    &trk.camelot_key,
                    trk.quality_score,
                    &trk.forensic_verdict,
                );
                if trk.forensic_verdict != "VERIFIED_STUDIO_MASTER"
                    || !trk.mono_compatible_sub
                    || trk.effective_bandwidth_hz < 16800.0
                    || trk.hera_seal != expected_seal
                {
                    rejected += 1;
                    continue;
                }

                let cas_fp = compute_cas_fingerprint(&trk.artist, &trk.title, trk.bpm);
                let reported_sha_opt = req
                    .track_audio_sha256
                    .get(&trk.track_id)
                    .or_else(|| req.track_audio_sha256.get(&cas_fp))
                    .cloned();

                // Double cryptographic seal check (`hera_seal` + `audio_sha256`) against Bait-and-Switch!
                if let Some(reported_sha) = reported_sha_opt {
                    let mut mismatch_expected = None;
                    if let Ok(sha_guard) = self.cas_audio_sha256.read() {
                        if let Some(expected_sha) = sha_guard.get(&cas_fp) {
                            if !expected_sha.eq_ignore_ascii_case(&reported_sha) {
                                mismatch_expected = Some(expected_sha.clone());
                            }
                        }
                    }
                    if let Some(expected_sha) = mismatch_expected {
                        rejected += 1;
                        bait_and_switch_detected = true;
                        if let Ok(mut q_guard) = self.quarantined_nodes.write() {
                            q_guard.insert(
                                peer_node_id.clone(),
                                QuarantinedNodeRecord {
                                    node_id: peer_node_id.clone(),
                                    owner_dj: peer_dj.clone(),
                                    reason: "BAIT_AND_SWITCH_AUDIO_SHA256_MISMATCH".to_string(),
                                    offending_cas_fingerprint: cas_fp,
                                    expected_audio_sha256: expected_sha,
                                    reported_audio_sha256: reported_sha,
                                },
                            );
                        }
                        if let Ok(mut dj_guard) = self.dj_profiles.write() {
                            if let Some(dj) = dj_guard
                                .iter_mut()
                                .find(|d| d.dj_handle.eq_ignore_ascii_case(&peer_dj))
                            {
                                dj.edge_guardian_active = false;
                                dj.custody_audit_failures = dj.custody_audit_failures.saturating_add(1);
                            }
                        }
                        continue;
                    } else if let Ok(mut sha_guard) = self.cas_audio_sha256.write() {
                        sha_guard.entry(cas_fp.clone()).or_insert(reported_sha);
                    }
                } else if let Ok(mut sha_guard) = self.cas_audio_sha256.write() {
                    let canonical = compute_canonical_audio_sha256(&cas_fp, &trk.hera_seal, 38.0);
                    sha_guard.entry(cas_fp.clone()).or_insert(canonical);
                }

                if let Ok(mut heat_guard) = self.cas_heat_scores.write() {
                    let h = heat_guard.entry(cas_fp).or_insert(1.0);
                    *h += 1.0;
                }

                trk.share_on_p2p = true;
                c_guard.retain(|existing| existing.track_id != trk.track_id);
                c_guard.push(trk);
                accepted += 1;
            }
        }

        let mut peer = req.peer;
        if bait_and_switch_detected {
            peer.quarantined = true;
            peer.edge_guardian_active = false;
            peer.custody_status = "QUARANTINED_BAIT_AND_SWITCH".to_string();
            if let Ok(mut p_guard) = self.peers.write() {
                p_guard.retain(|p| p.node_id != peer.node_id);
            }
            return FederatedSyncResponse {
                status: "QUARANTINED_BAIT_AND_SWITCH_ATTACK".to_string(),
                responder_node_id: self.node_id.clone(),
                peer_registered: peer,
                tracks_accepted: 0,
                tracks_rejected_invalid_seal: rejected,
                node_quarantined_for_bait_and_switch: true,
                federated_swarm_catalog: Vec::new(),
                active_peers: self.list_peers(),
            };
        }

        if accepted > 0 {
            peer.verified_tracks_count = peer.verified_tracks_count.max(accepted);
        }
        let active_peers = self.register_peer_node(peer.clone());
        let federated_catalog = self.search(&P2PSearchQuery {
            q: None,
            camelot: None,
            bpm_min: None,
            bpm_max: None,
            subgenre: None,
            owner_dj: None,
            verified_only: Some(true),
        });

        FederatedSyncResponse {
            status: "federated_node_synced".to_string(),
            responder_node_id: self.node_id.clone(),
            peer_registered: peer,
            tracks_accepted: accepted,
            tracks_rejected_invalid_seal: rejected,
            node_quarantined_for_bait_and_switch: false,
            federated_swarm_catalog: federated_catalog,
            active_peers,
        }
    }

    /// Builds a signed `FederatedSyncRequest` payload containing all verified shared tracks on this node
    /// so the node can auto-federate with `hera-cloud` or another peer node.
    pub fn build_federated_sync_payload(
        &self,
        dj_handle: &str,
        public_url: Option<String>,
        bandwidth_mbps: u32,
    ) -> FederatedSyncRequest {
        let shared_tracks = self.search(&P2PSearchQuery {
            q: None,
            camelot: None,
            bpm_min: None,
            bpm_max: None,
            subgenre: None,
            owner_dj: None,
            verified_only: Some(true),
        });
        let runtime_target = if cfg!(target_arch = "wasm32") {
            "cloudflare-workers-wasm32".to_string()
        } else {
            "native-vps-tokio".to_string()
        };
        let peer = P2PPeerNode {
            node_id: self.node_id.clone(),
            dj_handle: dj_handle.to_string(),
            endpoint_url: public_url.unwrap_or_else(|| "http://127.0.0.1:8080".to_string()),
            runtime_target,
            storage_mode: "federated_p2p_node".to_string(),
            verified_tracks_count: shared_tracks.len(),
            bandwidth_mbps,
            r2_free_tier_limit_mb: CF_WORKER_FREE_R2_LIMIT_MB,
            r2_safe_cap_mb: CF_WORKER_SAFE_R2_CAP_MB,
            r2_used_mb: (shared_tracks.len() as f32) * 38.0,
            daily_worker_request_limit: CF_WORKER_FREE_REQ_LIMIT,
            daily_worker_request_safe_cap: CF_WORKER_SAFE_REQ_CAP,
            daily_worker_requests_used: 0,
            edge_guardian_active: true,
            custody_status: "ONLINE_HEALTHY".to_string(),
            last_custody_proof_hash: "federated-custody-verified".to_string(),
            custody_failures: 0,
            credits_earned_mb: (shared_tracks.len() as f32) * 38.0 * SWARM_CREDIT_MULTIPLIER,
            behind_cgnat: false,
            quarantined: false,
            endpoint_pubkey_id: derive_node_endpoint_pubkey_id(dj_handle, &self.node_id),
            network_generation: 1,
            relay_assisted_url: None,
            ewma_rtt_ms: 20.0,
        };
        let track_audio_sha256 = self
            .cas_audio_sha256
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        FederatedSyncRequest {
            peer,
            shared_tracks,
            track_audio_sha256,
        }
    }

    /// Auto-Federates this local/VPS node with a remote `hera` or `hera-cloud` HTTP endpoint (`--federate-url`)
    /// by sending `POST /api/v1/p2p/federate-sync` and merging the returned federated swarm catalog back into local RAM.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn push_federation_over_http(
        &self,
        remote_url: &str,
        dj_handle: &str,
        public_url: Option<String>,
        bandwidth_mbps: u32,
    ) -> Result<FederatedSyncResponse, String> {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        let payload = self.build_federated_sync_payload(dj_handle, public_url, bandwidth_mbps);
        let body_bytes =
            serde_json::to_vec(&payload).map_err(|e| format!("JSON serialize error: {e}"))?;

        let trimmed_remote = remote_url.trim();
        if trimmed_remote.starts_with("https://") {
            return Err("HTTPS not supported over plaintext TcpStream. Use HTTP federation endpoint.".to_string());
        }
        let clean = trimmed_remote.trim_start_matches("http://");
        let host_port = clean.split('/').next().unwrap_or("127.0.0.1:8080");
        let addr_str = if host_port.contains(':') {
            host_port.to_string()
        } else {
            format!("{host_port}:80")
        };

        use std::net::ToSocketAddrs;
        let socket_addr = addr_str
            .to_socket_addrs()
            .map_err(|e| format!("DNS resolution failed for `{addr_str}`: {e}"))?
            .next()
            .ok_or_else(|| format!("No socket addresses found for `{addr_str}`"))?;

        let timeout_5s = std::time::Duration::from_secs(5);
        let mut stream = TcpStream::connect_timeout(&socket_addr, timeout_5s)
            .map_err(|e| format!("Failed to connect to federation hub `{addr_str}`: {e}"))?;
        let _ = stream.set_read_timeout(Some(timeout_5s));
        let _ = stream.set_write_timeout(Some(timeout_5s));
        let req_str = format!(
            "POST /api/v1/p2p/federate-sync HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            host_port,
            body_bytes.len()
        );
        stream
            .write_all(req_str.as_bytes())
            .and_then(|_| stream.write_all(&body_bytes))
            .map_err(|e| format!("Failed to send federation payload: {e}"))?;

        let mut resp_buf = Vec::new();
        stream
            .read_to_end(&mut resp_buf)
            .map_err(|e| format!("Failed to read federation response: {e}"))?;
        let resp_str = String::from_utf8_lossy(&resp_buf);
        let json_body = resp_str
            .split("\r\n\r\n")
            .nth(1)
            .unwrap_or(resp_str.as_ref())
            .trim();

        let sync_resp: FederatedSyncResponse = serde_json::from_str(json_body)
            .map_err(|e| format!("Invalid federation JSON response: {e}"))?;

        if let Ok(mut c_guard) = self.catalog.write() {
            for trk in &sync_resp.federated_swarm_catalog {
                if !c_guard.iter().any(|existing| existing.track_id == trk.track_id) {
                    c_guard.push(trk.clone());
                }
            }
        }
        Ok(sync_resp)
    }
}
