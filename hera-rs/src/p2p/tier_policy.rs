use super::*;
use crate::structure::HotCuePoint;

/// Zero-OAuth Auto-Discovery: If the caller provided Google Drive's public `/embeddedfolderview?id=...#list` HTML
/// and `tracks` is empty, automatically extract every audio file ID and title without OAuth!
pub(crate) fn populate_tracks_from_gdrive_html(req: &mut ConnectDjDriveRequest, handle: &str) {
    if req.tracks.is_empty() {
        if let Some(ref html) = req.gdrive_embedded_html {
            for (fid, fname) in parse_gdrive_public_folder_html(html) {
                let ext = fname
                    .rsplit('.')
                    .next()
                    .unwrap_or("flac")
                    .to_ascii_lowercase();
                let stem = fname.trim_end_matches(&format!(".{ext}")).to_string();
                let (artist, title) = if let Some((a, t)) = stem.split_once(" - ") {
                    (a.trim().to_string(), t.trim().to_string())
                } else {
                    (handle.to_string(), stem)
                };
                req.tracks.push(DjDriveTrackInput {
                    title,
                    artist,
                    bpm: 124.0,
                    camelot_key: "8A".to_string(),
                    top_subgenre: "Club Master".to_string(),
                    duration_seconds: 300.0,
                    format: ext,
                    gdrive_file_link_or_id: Some(fid),
                    r2_object_key: None,
                    cloud_share_url: None,
                    file_size_mb: 32.0,
                    effective_bandwidth_hz: 21000.0,
                    sub_bass_phase_correlation: 0.94,
                    is_synthetic_tone: false,
                    share_on_p2p: Some(true),
                    audio_sha256: None,
                });
            }
        }
    }
}

pub(crate) fn build_default_drive_hot_cues(bpm: f32, duration_seconds: f32) -> Vec<HotCuePoint> {
    let bar_sec = (60.0 / bpm.max(60.0)) * 4.0;
    let c2 = (8.0 * bar_sec).min(duration_seconds * 0.25);
    let c3 = (16.0 * bar_sec).min(duration_seconds * 0.45);
    let c4 = (duration_seconds - 8.0 * bar_sec).max(c3 + bar_sec);

    vec![
        HotCuePoint {
            num: 1,
            name: "1.1.1 Downbeat".to_string(),
            seconds: 0.20,
            bar_index: 0,
            role: "downbeat".to_string(),
            confidence: 0.95,
        },
        HotCuePoint {
            num: 2,
            name: "Phrase In (8b)".to_string(),
            seconds: (c2 * 100.0).round() / 100.0,
            bar_index: 8,
            role: "phrase_in".to_string(),
            confidence: 0.91,
        },
        HotCuePoint {
            num: 3,
            name: "Main Drop (16b)".to_string(),
            seconds: (c3 * 100.0).round() / 100.0,
            bar_index: 16,
            role: "drop".to_string(),
            confidence: 0.94,
        },
        HotCuePoint {
            num: 4,
            name: "Mix Out".to_string(),
            seconds: (c4 * 100.0).round() / 100.0,
            bar_index: 32,
            role: "mix_out".to_string(),
            confidence: 0.92,
        },
    ]
}

impl P2PMeshState {
    /// Upgrade a DJ's Cloudflare R2 storage tier (`free` = 100 MB -> `pro_monthly` = 50,000 MB / 50 GB).
    pub fn upgrade_dj_tier(&self, dj_handle: &str, tier: &str) -> Result<DjProfile, String> {
        let mut guard = self
            .dj_profiles
            .write()
            .map_err(|_| "Failed to lock dj_profiles".to_string())?;
        let profile = guard
            .iter_mut()
            .find(|p| p.dj_handle.eq_ignore_ascii_case(dj_handle))
            .ok_or_else(|| format!("DJ profile `{dj_handle}` not found"))?;

        profile.subscription_tier = tier.to_string();
        profile.r2_quota_mb = resolve_tier_r2_quota_mb(tier);
        profile.can_disable_p2p_sharing = tier_allows_private_vault(tier);
        if !profile.can_disable_p2p_sharing {
            profile.p2p_sharing_enabled = true;
        }
        Ok(profile.clone())
    }

    /// Toggle P2P sharing for a DJ's uploaded songs (`p2p_sharing_enabled` is `true` by default).
    /// Only DJs on a higher plan (`pro_monthly`, `pro`, `founder_unlimited`) can turn P2P sharing OFF (`false`)!
    pub fn set_p2p_sharing(
        &self,
        req: SetP2PSharingRequest,
    ) -> Result<DjProfile, String> {
        let handle = req.dj_handle.trim();
        let updated_profile = {
            let mut guard = self
                .dj_profiles
                .write()
                .map_err(|_| "Failed to lock dj_profiles".to_string())?;
            let profile = guard
                .iter_mut()
                .find(|p| p.dj_handle.eq_ignore_ascii_case(handle))
                .ok_or_else(|| format!("DJ profile `{handle}` not found"))?;

            if !req.p2p_sharing_enabled && !tier_allows_private_vault(&profile.subscription_tier) {
                return Err(format!(
                    "PLAN_UPGRADE_REQUIRED: En el plan `{}` todas las canciones subidas se comparten en la red P2P por defecto. Actualiza a `pro_monthly` para poder apagar el P2P en tus canciones (Modo Bóveda Privada).",
                    profile.subscription_tier
                ));
            }

            profile.p2p_sharing_enabled = req.p2p_sharing_enabled;
            profile.can_disable_p2p_sharing = tier_allows_private_vault(&profile.subscription_tier);
            profile.clone()
        };

        if let Ok(mut cat) = self.catalog.write() {
            for trk in cat.iter_mut() {
                if trk.owner_dj.eq_ignore_ascii_case(handle) {
                    trk.share_on_p2p = req.p2p_sharing_enabled;
                }
            }
        }

        Ok(updated_profile)
    }
}
