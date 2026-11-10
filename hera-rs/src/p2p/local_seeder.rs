use super::*;
use crate::forensics::{audit_pcm_channels, decode_audio_bytes};
use crate::structure::analyze_structure_and_style;

impl P2PMeshState {
    /// Local Laptop / Server Direct Storage & P2P Super-Seeder:
    /// Scans a local folder on disk, runs real Pure-Rust Symphonia + rustfft DSP on every audio file,
    /// rejects fake lossless/synthetic files in quarantine, stores valid tracks for direct HTTP/P2P binary streaming,
    /// and registers the local machine as an active P2P Peer Node!
    pub fn scan_local_directory(
        &self,
        req: ScanLocalDirRequest,
    ) -> Result<ScanLocalDirResponse, String> {
        let dir_path = std::path::Path::new(&req.directory_path);
        if !dir_path.exists() || !dir_path.is_dir() {
            return Err(format!(
                "Directory `{}` does not exist or is not a folder",
                req.directory_path
            ));
        }

        let handle = req.dj_handle.trim().to_string();
        let base_url = req
            .public_node_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());

        let entries = std::fs::read_dir(dir_path)
            .map_err(|e| format!("Failed to read directory `{}`: {e}", req.directory_path))?;

        let mut admitted = Vec::new();
        let mut rejected = Vec::new();
        let mut idx = 0usize;

        for entry_res in entries {
            let Ok(entry) = entry_res else { continue };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(
                ext.as_str(),
                "wav" | "flac" | "mp3" | "ogg" | "aac" | "m4a" | "aiff"
            ) {
                continue;
            }

            idx += 1;
            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown Track")
                .to_string();
            let (artist, title) = if let Some((a, t)) = file_stem.split_once(" - ") {
                (a.trim().to_string(), t.trim().to_string())
            } else {
                (handle.clone(), file_stem.clone())
            };

            let Ok(bytes) = std::fs::read(&path) else {
                rejected.push(format!("{} [REJECTED: Could not read file bytes]", file_stem));
                continue;
            };

            let decoded = match decode_audio_bytes(&bytes, Some(&ext), 35.0) {
                Ok(d) => d,
                Err(e) => {
                    rejected.push(format!("{} [REJECTED: Decode error: {}]", file_stem, e));
                    continue;
                }
            };

            let forensics = audit_pcm_channels(
                &decoded.left,
                decoded.right.as_deref(),
                decoded.sample_rate,
                &ext,
            );

            if forensics.is_synthetic_tone
                || !forensics.is_authentic_lossless
                || !forensics.mono_compatible_sub
                || forensics.quality_score < 65.0
            {
                rejected.push(format!(
                    "{} - {} [QUARANTINED: {} ({:.0} Hz, score={:.1})]",
                    artist,
                    title,
                    forensics.forensic_verdict,
                    forensics.effective_bandwidth_hz,
                    forensics.quality_score
                ));
                continue;
            }

            let mono: Vec<f32> = if let Some(ref r) = decoded.right {
                let n = decoded.left.len().min(r.len());
                decoded.left[..n]
                    .iter()
                    .zip(r[..n].iter())
                    .map(|(&l, &rv)| 0.5 * (l + rv))
                    .collect()
            } else {
                decoded.left.clone()
            };

            let structure = analyze_structure_and_style(
                &mono,
                decoded.sample_rate,
                Some(decoded.duration_seconds),
                None,
            );

            let track_id = format!(
                "local-{}-{:03}",
                handle.to_ascii_lowercase().replace(' ', "-"),
                idx
            );
            let seal = compute_hera_seal(
                &track_id,
                structure.bpm,
                &structure.camelot_key,
                forensics.quality_score,
                &forensics.forensic_verdict,
            );
            let stream_url = format!(
                "{}/api/v1/p2p/download/{}",
                base_url.trim_end_matches('/'),
                track_id
            );

            let cas_fp = compute_cas_fingerprint(&artist, &title, structure.bpm);
            let real_sha256 = compute_sha256_hex(&bytes);
            if let Ok(mut sha_guard) = self.cas_audio_sha256.write() {
                sha_guard.entry(cas_fp.clone()).or_insert(real_sha256);
            }
            if let Ok(mut heat_guard) = self.cas_heat_scores.write() {
                heat_guard.entry(cas_fp).or_insert(2.0);
            }

            let verified = P2PVerifiedTrack {
                track_id: track_id.clone(),
                title,
                artist,
                owner_dj: handle.clone(),
                bpm: structure.bpm,
                camelot_key: structure.camelot_key,
                top_subgenre: structure.top_subgenre,
                duration_seconds: decoded.duration_seconds,
                format: ext,
                effective_bandwidth_hz: forensics.effective_bandwidth_hz,
                sub_bass_phase_correlation: forensics.sub_bass_phase_correlation,
                mono_compatible_sub: forensics.mono_compatible_sub,
                quality_score: forensics.quality_score,
                forensic_verdict: forensics.forensic_verdict,
                hot_cues: structure.hot_cues,
                storage_provider: "local_disk".to_string(),
                gdrive_file_id: None,
                r2_object_key: None,
                stream_url: Some(stream_url),
                share_on_p2p: true,
                node_id: self.node_id.clone(),
                hera_seal: seal,
            };

            if let Ok(mut f_guard) = self.local_files.write() {
                f_guard.insert(track_id.clone(), path.to_string_lossy().to_string());
            }
            if let Ok(mut c_guard) = self.catalog.write() {
                c_guard.retain(|t| t.track_id != track_id);
                c_guard.push(verified.clone());
            }
            admitted.push(verified);
        }

        let peer = P2PPeerNode {
            node_id: self.node_id.clone(),
            dj_handle: handle.clone(),
            endpoint_url: base_url,
            runtime_target: "native-local-p2p-seeder".to_string(),
            storage_mode: "local_disk_p2p_seeder".to_string(),
            verified_tracks_count: admitted.len(),
            bandwidth_mbps: req.bandwidth_mbps.unwrap_or(800),
            r2_free_tier_limit_mb: CF_WORKER_FREE_R2_LIMIT_MB,
            r2_safe_cap_mb: CF_WORKER_SAFE_R2_CAP_MB,
            r2_used_mb: (admitted.len() as f32) * 38.0,
            daily_worker_request_limit: CF_WORKER_FREE_REQ_LIMIT,
            daily_worker_request_safe_cap: CF_WORKER_SAFE_REQ_CAP,
            daily_worker_requests_used: 0,
            edge_guardian_active: true,
            custody_status: "ONLINE_HEALTHY".to_string(),
            last_custody_proof_hash: "local-disk-custody-verified".to_string(),
            custody_failures: 0,
            credits_earned_mb: (admitted.len() as f32) * 38.0 * SWARM_CREDIT_MULTIPLIER,
            behind_cgnat: false,
            quarantined: false,
            endpoint_pubkey_id: derive_node_endpoint_pubkey_id(&handle, &self.node_id),
            network_generation: 1,
            relay_assisted_url: None,
            ewma_rtt_ms: 12.0,
        };
        self.register_peer_node(peer.clone());

        let profile = DjProfile {
            dj_handle: handle.clone(),
            display_name: format!("{} (Local P2P Seeder Node)", handle),
            role: "local_p2p_seeder_node".to_string(),
            storage_provider: "local_disk".to_string(),
            gdrive_folder_id: None,
            gdrive_folder_url: None,
            r2_bucket_url: None,
            local_node_dir: Some(req.directory_path.clone()),
            subscription_tier: "local_unlimited".to_string(),
            r2_quota_mb: 100.0,
            r2_used_mb: 0.0,
            bounty_refunded_mb: 0.0,
            p2p_sharing_enabled: true,
            can_disable_p2p_sharing: false,
            verified_tracks_shared: admitted.len(),
            quarantined_rejected_tracks: rejected.len(),
            edge_guardian_active: true,
            swarm_credit_balance_mb: ((admitted.len() as f32) * 38.0 * SWARM_CREDIT_MULTIPLIER).max(150.0),
            total_bytes_served_mb: (admitted.len() as f32) * 38.0,
            total_bytes_consumed_mb: 0.0,
            custody_audit_passes: 1,
            custody_audit_failures: 0,
        };
        if let Ok(mut p_guard) = self.dj_profiles.write() {
            p_guard.retain(|p| !p.dj_handle.eq_ignore_ascii_case(&handle));
            p_guard.push(profile);
        }

        Ok(ScanLocalDirResponse {
            status: "local_p2p_seeder_active".to_string(),
            node_id: self.node_id.clone(),
            dj_handle: handle,
            scanned_directory: req.directory_path,
            admitted_tracks: admitted,
            rejected_quarantine_tracks: rejected,
            registered_peer: peer,
        })
    }

    /// Reads raw audio bytes from the local disk for P2P peer-to-peer HTTP binary transfer.
    pub fn get_local_file_bytes(&self, track_id: &str) -> Result<(String, Vec<u8>), String> {
        let path_str = self
            .local_files
            .read()
            .map_err(|_| "Lock error".to_string())?
            .get(track_id)
            .cloned()
            .ok_or_else(|| format!("Track `{track_id}` is not stored on this local disk node"))?;
        let bytes = std::fs::read(&path_str)
            .map_err(|e| format!("Failed to read local P2P file `{path_str}`: {e}"))?;
        Ok((path_str, bytes))
    }

    /// Reads only an aligned `chunk_size` slice at `chunk_idx * chunk_size` using `Seek` + `Read::take`,
    /// avoiding loading the entire 40 MB lossless audio file into RAM on every 8 KiB UDP datagram.
    pub fn get_local_file_chunk_slice(
        &self,
        track_id: &str,
        chunk_idx: usize,
        chunk_size: usize,
    ) -> Result<Vec<u8>, String> {
        use std::io::{Read, Seek, SeekFrom};

        let path_str = self
            .local_files
            .read()
            .map_err(|_| "Lock error".to_string())?
            .get(track_id)
            .cloned()
            .ok_or_else(|| format!("Track `{track_id}` is not stored on this local disk node"))?;

        let mut file = std::fs::File::open(&path_str)
            .map_err(|e| format!("Failed to open local P2P file `{path_str}`: {e}"))?;
        let file_len = file.metadata().map(|m| m.len() as usize).unwrap_or(0);
        let start = (chunk_idx.saturating_mul(chunk_size)).min(file_len);
        let to_read = chunk_size.min(file_len.saturating_sub(start));
        if to_read == 0 {
            return Ok(Vec::new());
        }

        file.seek(SeekFrom::Start(start as u64))
            .map_err(|e| format!("Failed to seek local P2P file `{path_str}`: {e}"))?;
        let mut buf = Vec::with_capacity(to_read);
        file.take(to_read as u64)
            .read_to_end(&mut buf)
            .map_err(|e| format!("Failed to read slice from `{path_str}`: {e}"))?;
        Ok(buf)
    }
}
