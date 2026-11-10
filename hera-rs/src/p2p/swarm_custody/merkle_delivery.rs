use super::super::*;

impl P2PMeshState {
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

        let now_unix = current_unix_override.unwrap_or_else(|| self.current_unix_secs());
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
}
