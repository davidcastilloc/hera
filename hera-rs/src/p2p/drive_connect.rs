use super::tier_policy::{build_default_drive_hot_cues, populate_tracks_from_gdrive_html};
use super::*;

impl P2PMeshState {
    /// Zero-Tech Onboarding: A non-technical DJ (`DJFoo`) simply pastes their DJ name and Google Drive folder link
    /// (or raw Zero-OAuth embedded folder HTML, Dropbox link, or Cloudflare R2 bucket).
    /// Enforces Cloudflare R2 storage quotas (`free` = 100 MB vs `pro_monthly` = 50 GB) and Zero-Trust DSP Forensics.
    pub fn connect_dj_drive(&self, mut req: ConnectDjDriveRequest) -> ConnectDjDriveResponse {
        let handle = req.dj_handle.trim().to_string();
        let display_name = req
            .display_name
            .clone()
            .unwrap_or_else(|| format!("{} (Community DJ)", handle));

        let gdrive_folder_id = req
            .gdrive_folder_link
            .as_deref()
            .map(extract_gdrive_id)
            .filter(|s| !s.is_empty());

        populate_tracks_from_gdrive_html(&mut req, &handle);

        let sub_tier = req
            .subscription_tier
            .clone()
            .unwrap_or_else(|| "beta_mvp".to_string());
        let r2_quota_mb = resolve_tier_r2_quota_mb(&sub_tier);
        let mut r2_used_mb = 0.0f32;
        let can_disable_p2p = tier_allows_private_vault(&sub_tier);
        // By default, P2P sharing is ALWAYS ON (`true`). Only higher plans (`pro_monthly` / `founder_unlimited`) can turn it OFF.
        let p2p_sharing_enabled = if can_disable_p2p {
            req.p2p_sharing_enabled.unwrap_or(true)
        } else {
            true
        };

        let storage_provider = if req.r2_bucket_url.is_some() && gdrive_folder_id.is_some() {
            "gdrive+r2".to_string()
        } else if req.r2_bucket_url.is_some() {
            "r2".to_string()
        } else {
            "gdrive".to_string()
        };

        let mut admitted = Vec::new();
        let mut rejected = Vec::new();
        let mut cas_deduplicated_hits = 0usize;
        let mut physical_r2_mb_saved = 0.0f32;

        for (idx, item) in req.tracks.iter().enumerate() {
            let is_lossless = matches!(
                item.format.to_ascii_lowercase().as_str(),
                "flac" | "wav" | "aiff" | "alac"
            );
            let is_fake_lossless = is_lossless && item.effective_bandwidth_hz < 16800.0;
            let mono_ok = item.sub_bass_phase_correlation >= 0.25;

            if item.is_synthetic_tone {
                rejected.push(format!(
                    "{} - {} [REJECTED: Synthetic sine/test tone detected]",
                    item.artist, item.title
                ));
                continue;
            }
            if is_fake_lossless {
                rejected.push(format!(
                    "{} - {} [REJECTED: Fake .{} transcode brickwalled at {:.0} Hz]",
                    item.artist, item.title, item.format, item.effective_bandwidth_hz
                ));
                continue;
            }
            if !mono_ok {
                rejected.push(format!(
                    "{} - {} [REJECTED: Sub-bass (<120Hz) phase cancellation corr={:.2}]",
                    item.artist, item.title, item.sub_bass_phase_correlation
                ));
                continue;
            }

            // Global CAS (Content-Addressed Storage) Internal P2P Deduplication:
            // If another DJ on the Hera Cloud already uploaded this exact master (matching canonical artist+title+BPM),
            // reuse the existing R2/P2P master with ZERO extra physical R2 megabytes and ZERO extra DSP CPU!
            let cas_key = compute_cas_fingerprint(&item.artist, &item.title, item.bpm);
            let existing_cas_master = self.catalog.read().ok().and_then(|cat| {
                cat.iter()
                    .find(|t| compute_cas_fingerprint(&t.artist, &t.title, t.bpm) == cas_key)
                    .cloned()
            });

            // Enforce Cloudflare R2 Hard Quota ONLY if this track is NOT already deduplicated in the CAS pool
            let wants_r2 = item.r2_object_key.is_some()
                || (storage_provider == "r2" && item.gdrive_file_link_or_id.is_none());
            if wants_r2 {
                let sz = item.file_size_mb.max(1.0);
                if existing_cas_master.is_some() {
                    // Internal P2P CAS Hit! 0 MB physical R2 storage consumed!
                    cas_deduplicated_hits += 1;
                    physical_r2_mb_saved += sz;
                } else {
                    if r2_used_mb + sz > r2_quota_mb {
                        rejected.push(format!(
                            "{} - {} [R2_QUOTA_EXCEEDED: Needs {:.1} MB, but {} tier has {:.1}/{:.1} MB used. Upgrade to `pro_monthly` (50 GB R2) or use Google Drive / Local P2P Node for $0]",
                            item.artist, item.title, sz, sub_tier, r2_used_mb, r2_quota_mb
                        ));
                        continue;
                    }
                    r2_used_mb += sz;
                }
            } else if existing_cas_master.is_some() {
                cas_deduplicated_hits += 1;
            }

            let file_id = item
                .gdrive_file_link_or_id
                .as_deref()
                .map(extract_gdrive_id)
                .or_else(|| {
                    if !wants_r2 && item.cloud_share_url.is_none() {
                        gdrive_folder_id
                            .as_ref()
                            .map(|fid| format!("{fid}_trk_{:02}", idx + 1))
                    } else {
                        None
                    }
                });

            let (item_provider, stream_url) = if let Some(ref c_url) = item.cloud_share_url {
                let (prov, direct) = normalize_cloud_share_link(c_url);
                (prov, Some(direct))
            } else if let Some(ref fid) = file_id {
                (
                    "gdrive".to_string(),
                    Some(build_gdrive_direct_download_url(fid)),
                )
            } else if let (Some(ref r2_base), Some(ref key)) =
                (&req.r2_bucket_url, &item.r2_object_key)
            {
                (
                    "r2".to_string(),
                    Some(format!("{}/{}", r2_base.trim_end_matches('/'), key)),
                )
            } else {
                (storage_provider.clone(), None)
            };

            let track_id = format!(
                "dj-{}-{:03}",
                handle.to_ascii_lowercase().replace(' ', "-"),
                idx + 1
            );
            let hot_cues = build_default_drive_hot_cues(item.bpm, item.duration_seconds);

            let quality_score = (90.0
                + ((item.effective_bandwidth_hz - 18000.0).max(0.0) / 1000.0) * 2.0
                + item.sub_bass_phase_correlation * 4.0)
                .clamp(70.0, 99.5);
            let q_round = (quality_score * 10.0).round() / 10.0;

            let seal = compute_hera_seal(
                &track_id,
                item.bpm,
                &item.camelot_key,
                q_round,
                "VERIFIED_STUDIO_MASTER",
            );

            let verified_track = P2PVerifiedTrack {
                track_id,
                title: item.title.clone(),
                artist: item.artist.clone(),
                owner_dj: handle.clone(),
                bpm: item.bpm,
                camelot_key: item.camelot_key.clone(),
                top_subgenre: item.top_subgenre.clone(),
                duration_seconds: item.duration_seconds,
                format: item.format.clone(),
                effective_bandwidth_hz: item.effective_bandwidth_hz,
                sub_bass_phase_correlation: item.sub_bass_phase_correlation,
                mono_compatible_sub: true,
                quality_score: q_round,
                forensic_verdict: "VERIFIED_STUDIO_MASTER".to_string(),
                hot_cues,
                storage_provider: item_provider,
                gdrive_file_id: file_id,
                r2_object_key: item.r2_object_key.clone(),
                stream_url,
                share_on_p2p: if can_disable_p2p {
                    item.share_on_p2p.unwrap_or(p2p_sharing_enabled)
                } else {
                    true
                },
                node_id: self.node_id.clone(),
                hera_seal: seal,
            };

            if let Ok(mut sha_guard) = self.cas_audio_sha256.write() {
                let sha = item
                    .audio_sha256
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| {
                        compute_canonical_audio_sha256(
                            &cas_key,
                            &verified_track.hera_seal,
                            item.file_size_mb,
                        )
                    });
                sha_guard.entry(cas_key.clone()).or_insert(sha);
            }
            if let Ok(mut heat_guard) = self.cas_heat_scores.write() {
                let h = heat_guard.entry(cas_key).or_insert(1.0);
                *h += 1.0;
            }
            if let Ok(mut guard) = self.catalog.write() {
                guard.retain(|t| t.track_id != verified_track.track_id);
                guard.push(verified_track.clone());
            }
            admitted.push(verified_track);
        }

        let initial_credits = ((admitted.len() as f32) * 38.0 * SWARM_CREDIT_MULTIPLIER).max(150.0);
        let profile = DjProfile {
            dj_handle: handle.clone(),
            display_name: display_name.clone(),
            role: "community_dj_peer".to_string(),
            storage_provider,
            gdrive_folder_id,
            gdrive_folder_url: req.gdrive_folder_link,
            r2_bucket_url: req.r2_bucket_url,
            local_node_dir: None,
            subscription_tier: sub_tier,
            r2_quota_mb,
            r2_used_mb,
            bounty_refunded_mb: 0.0,
            p2p_sharing_enabled,
            can_disable_p2p_sharing: can_disable_p2p,
            verified_tracks_shared: admitted.len(),
            quarantined_rejected_tracks: rejected.len(),
            edge_guardian_active: true,
            swarm_credit_balance_mb: initial_credits,
            total_bytes_served_mb: (admitted.len() as f32) * 38.0,
            total_bytes_consumed_mb: 0.0,
            custody_audit_passes: 1,
            custody_audit_failures: 0,
        };

        if let Ok(mut p_guard) = self.dj_profiles.write() {
            p_guard.retain(|p| !p.dj_handle.eq_ignore_ascii_case(&handle));
            p_guard.push(profile.clone());
        }

        let total_network_tracks = self
            .catalog
            .read()
            .map(|c| c.iter().filter(|t| t.share_on_p2p).count())
            .unwrap_or(0);

        ConnectDjDriveResponse {
            status: "connected".to_string(),
            profile,
            admitted_tracks: admitted.clone(),
            rejected_quarantine_tracks: rejected.clone(),
            cas_deduplicated_hits,
            physical_r2_mb_saved,
            network_catalog_available: total_network_tracks,
            welcome_message: format!(
                "¡Bienvenido {}! Tu almacenamiento quedó conectado (Compartir P2P activo por defecto: {}). Se admitieron {} pistas verificadas ({} deduplicadas por CAS ahorrando {:.1} MB físicos en R2; {} rechazadas por aduana forense o cuota R2) y ya tienes acceso a las {} pistas compartidas de la red P2P.",
                handle,
                p2p_sharing_enabled,
                admitted.len(),
                cas_deduplicated_hits,
                physical_r2_mb_saved,
                rejected.len(),
                total_network_tracks
            ),
        }
    }
}
