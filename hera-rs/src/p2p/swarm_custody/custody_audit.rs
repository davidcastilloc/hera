use super::super::*;

impl P2PMeshState {
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
}
