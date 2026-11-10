use super::*;

impl P2PMeshState {
    /// Pre-Upload CAS Deduplication Check & Zero-Upload Claim (`POST /api/v1/cloud/check-cas`):
    /// Before a client uploads a 38 MB FLAC to Cloudflare R2, it asks `hera-cloud` if the `cas_fingerprint`
    /// already exists in the swarm.
    /// - If it exists: the client skips the 38 MB upload (`upload_required = false`), links the master in `<1 ms`,
    ///   and triggers the `+38 MB` Curator Quota Refund Bounty for the pioneer DJ!
    /// - If it does not exist: returns `upload_required = true` and the canonical R2 CAS object key (`cas-vault/{cas}.flac`).
    pub fn check_cas_before_upload(&self, req: CasCheckRequest) -> CasCheckResponse {
        let cas = req
            .cas_fingerprint
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| compute_cas_fingerprint(&req.artist, &req.title, req.bpm));
        let r2_key = format!("cas-vault/{cas}.flac");
        let file_mb = req.file_size_mb.unwrap_or(38.0).max(1.0);

        let exists_in_swarm = self
            .catalog
            .read()
            .map(|cat| {
                cat.iter().any(|t| {
                    t.share_on_p2p
                        && t.forensic_verdict == "VERIFIED_STUDIO_MASTER"
                        && compute_cas_fingerprint(&t.artist, &t.title, t.bpm)
                            .eq_ignore_ascii_case(&cas)
                })
            })
            .unwrap_or(false);

        if exists_in_swarm {
            let claim_res = if req.auto_claim_if_exists {
                self.broker_claim_cloud_track(BrokerClaimCloudTrackRequest {
                    claimer_dj: req.dj_handle.clone(),
                    target_track_or_cas: cas.clone(),
                })
                .ok()
            } else {
                None
            };

            let note = format!(
                "CAS HIT (`{}`): Este Studio Master ya existe en el CAS de Hera Cloud. `{}` ahorró {:.1} MB de subida por internet (0 MB extra en R2) y se vinculó en <1 ms.",
                cas, req.dj_handle, file_mb
            );
            CasCheckResponse {
                status: "cas_hit_zero_upload_linked".to_string(),
                cas_fingerprint: cas,
                already_in_cas: true,
                upload_required: false,
                r2_cas_object_key: r2_key,
                bandwidth_saved_mb: file_mb,
                zero_copy_claim: claim_res,
                note,
            }
        } else {
            let note = format!(
                "CAS MISS (`{}`): Pista nueva en la bóveda global. Sube el archivo una única vez a R2 bajo `{}`.",
                cas, r2_key
            );
            CasCheckResponse {
                status: "cas_miss_upload_required".to_string(),
                cas_fingerprint: cas,
                already_in_cas: false,
                upload_required: true,
                r2_cas_object_key: r2_key,
                bandwidth_saved_mb: 0.0,
                zero_copy_claim: None,
                note,
            }
        }
    }

    /// Exports the live `P2PMeshState` (DJ accounts, Global CAS objects, Verified P2P Catalog, and Federated Peers)
    /// as an idempotent Cloudflare D1 SQL transaction (`INSERT OR REPLACE INTO ...`) compatible with `migrations/0001` & `0002`.
    pub fn export_d1_sql_snapshot(&self) -> String {
        fn esc(s: &str) -> String {
            s.replace('\'', "''")
        }

        let mut sql = String::from(
            "-- Hera Cloud Control Plane — Cloudflare D1 Live State Snapshot\nBEGIN TRANSACTION;\n\n",
        );

        if let Ok(profiles) = self.dj_profiles.read() {
            for p in profiles.iter() {
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO dj_cloud_accounts (dj_handle, display_name, role, storage_provider, subscription_tier, r2_quota_mb, r2_used_mb, bounty_refunded_mb, p2p_sharing_enabled, can_disable_p2p_sharing, verified_tracks_shared, quarantined_rejected_tracks) VALUES ('{}', '{}', '{}', '{}', '{}', {:.2}, {:.2}, {:.2}, {}, {}, {}, {});\n",
                    esc(&p.dj_handle),
                    esc(&p.display_name),
                    esc(&p.role),
                    esc(&p.storage_provider),
                    esc(&p.subscription_tier),
                    p.r2_quota_mb,
                    p.r2_used_mb,
                    p.bounty_refunded_mb,
                    if p.p2p_sharing_enabled { 1 } else { 0 },
                    if p.can_disable_p2p_sharing { 1 } else { 0 },
                    p.verified_tracks_shared,
                    p.quarantined_rejected_tracks
                ));
            }
        }
        sql.push('\n');

        let swarm_index = self.build_broker_swarm_index();
        for rec in &swarm_index {
            let r2_key = format!("cas-vault/{}.flac", rec.cas_fingerprint);
            let pioneer = rec
                .available_routes
                .first()
                .map(|r| r.owner_dj.as_str())
                .unwrap_or("DaviDJ");
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO cas_global_objects (cas_fingerprint, r2_object_key, original_uploader_dj, canonical_artist, canonical_title, bpm, camelot_key, style_simhash_hex, hera_seal, swarm_replicas_count) VALUES ('{}', '{}', '{}', '{}', '{}', {:.2}, '{}', '{}', '{}', {});\n",
                esc(&rec.cas_fingerprint),
                esc(&r2_key),
                esc(pioneer),
                esc(&rec.canonical_artist),
                esc(&rec.canonical_title),
                rec.bpm,
                esc(&rec.camelot_key),
                esc(&rec.style_simhash_hex),
                esc(&rec.hera_seal),
                rec.swarm_replicas_count
            ));
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO swarm_opaque_shards (opaque_blob_key, cas_fingerprint, audio_sha256, hera_seal, aligned_block_size_bytes, heat_score, target_replicas_k, active_replicas_count) VALUES ('{}', '{}', '{}', '{}', {}, {:.2}, {}, {});\n",
                esc(&rec.opaque_blob_key),
                esc(&rec.cas_fingerprint),
                esc(&rec.audio_sha256),
                esc(&rec.hera_seal),
                rec.aligned_block_size_bytes,
                rec.heat_score,
                rec.target_replicas_k,
                rec.swarm_replicas_count
            ));
        }
        sql.push('\n');

        if let Ok(catalog) = self.catalog.read() {
            for t in catalog.iter() {
                let cas = compute_cas_fingerprint(&t.artist, &t.title, t.bpm);
                let cues_json = serde_json::to_string(&t.hot_cues).unwrap_or_else(|_| "[]".to_string());
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO verified_p2p_catalog (track_id, owner_dj, cas_fingerprint, title, artist, bpm, camelot_key, top_subgenre, duration_seconds, format, effective_bandwidth_hz, sub_bass_phase_correlation, mono_compatible_sub, quality_score, forensic_verdict, storage_provider, r2_object_key, stream_url, share_on_p2p, hot_cues_json, hera_seal) VALUES ('{}', '{}', '{}', '{}', '{}', {:.2}, '{}', '{}', {:.2}, '{}', {:.1}, {:.3}, {}, {:.2}, '{}', '{}', '{}', '{}', {}, '{}', '{}');\n",
                    esc(&t.track_id),
                    esc(&t.owner_dj),
                    esc(&cas),
                    esc(&t.title),
                    esc(&t.artist),
                    t.bpm,
                    esc(&t.camelot_key),
                    esc(&t.top_subgenre),
                    t.duration_seconds,
                    esc(&t.format),
                    t.effective_bandwidth_hz,
                    t.sub_bass_phase_correlation,
                    if t.mono_compatible_sub { 1 } else { 0 },
                    t.quality_score,
                    esc(&t.forensic_verdict),
                    esc(&t.storage_provider),
                    esc(t.r2_object_key.as_deref().unwrap_or("")),
                    esc(t.stream_url.as_deref().unwrap_or("")),
                    if t.share_on_p2p { 1 } else { 0 },
                    esc(&cues_json),
                    esc(&t.hera_seal)
                ));
            }
        }
        sql.push('\n');

        if let Ok(peers) = self.peers.read() {
            for peer in peers.iter() {
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO p2p_peer_nodes (node_id, dj_handle, endpoint_url, runtime_target, storage_mode, verified_tracks_count, bandwidth_mbps) VALUES ('{}', '{}', '{}', '{}', '{}', {}, {});\n",
                    esc(&peer.node_id),
                    esc(&peer.dj_handle),
                    esc(&peer.endpoint_url),
                    esc(&peer.runtime_target),
                    esc(&peer.storage_mode),
                    peer.verified_tracks_count,
                    peer.bandwidth_mbps
                ));
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO swarm_node_allocations (node_id, owner_dj, endpoint_url, r2_free_tier_limit_mb, r2_safe_cap_mb, r2_used_mb, daily_worker_request_limit, daily_worker_request_safe_cap, daily_worker_requests_used, edge_guardian_active, custody_status, last_custody_proof_hash, custody_failures, credits_earned_mb, behind_cgnat, quarantined) VALUES ('{}', '{}', '{}', {:.2}, {:.2}, {:.2}, {}, {}, {}, {}, '{}', '{}', {}, {:.2}, {}, {});\n",
                    esc(&peer.node_id),
                    esc(&peer.dj_handle),
                    esc(&peer.endpoint_url),
                    peer.r2_free_tier_limit_mb,
                    peer.r2_safe_cap_mb,
                    peer.r2_used_mb,
                    peer.daily_worker_request_limit,
                    peer.daily_worker_request_safe_cap,
                    peer.daily_worker_requests_used,
                    if peer.edge_guardian_active { 1 } else { 0 },
                    esc(&peer.custody_status),
                    esc(&peer.last_custody_proof_hash),
                    peer.custody_failures,
                    peer.credits_earned_mb,
                    if peer.behind_cgnat { 1 } else { 0 },
                    if peer.quarantined { 1 } else { 0 }
                ));
            }
        }

        if let Ok(quarantined) = self.quarantined_nodes.read() {
            for q in quarantined.values() {
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO swarm_quarantined_nodes (node_id, owner_dj, reason, offending_cas_fingerprint, expected_audio_sha256, reported_audio_sha256) VALUES ('{}', '{}', '{}', '{}', '{}', '{}');\n",
                    esc(&q.node_id),
                    esc(&q.owner_dj),
                    esc(&q.reason),
                    esc(&q.offending_cas_fingerprint),
                    esc(&q.expected_audio_sha256),
                    esc(&q.reported_audio_sha256)
                ));
            }
        }

        if let Ok(receipts) = self.delivery_receipts.read() {
            for r in receipts.iter() {
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO swarm_delivery_receipts (receipt_id, ticket_id, serving_node_id, requester_dj, opaque_blob_key, bytes_delivered, megabytes_delivered, credits_earned_mb, credit_multiplier, receipt_hmac) VALUES ('{}', '{}', '{}', '{}', '{}', {}, {:.2}, {:.2}, {:.2}, '{}');\n",
                    esc(&r.receipt_id),
                    esc(&r.ticket_id),
                    esc(&r.serving_node_id),
                    esc(&r.requester_dj),
                    esc(&r.opaque_blob_key),
                    r.bytes_delivered,
                    r.megabytes_delivered,
                    r.credits_earned_mb,
                    r.credit_multiplier,
                    esc(&r.receipt_hmac)
                ));
            }
        }

        sql.push_str("\nCOMMIT;\n");
        sql
    }
}
