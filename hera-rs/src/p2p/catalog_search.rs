use super::*;
use crate::forensics::ForensicAuditReport;

impl P2PMeshState {
    pub fn list_dj_profiles(&self) -> Vec<DjProfile> {
        self.dj_profiles
            .read()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn find_track_by_id(&self, track_id: &str) -> Option<P2PVerifiedTrack> {
        self.catalog
            .read()
            .ok()?
            .iter()
            .find(|t| t.track_id == track_id)
            .cloned()
    }

    /// Quarantine-First Gatekeeper: Only admits a track to the P2P catalog if it passes Zero-Trust DSP Forensics.
    pub fn announce_verified_track(
        &self,
        mut track: P2PVerifiedTrack,
        forensic: &ForensicAuditReport,
    ) -> Result<P2PVerifiedTrack, String> {
        if forensic.is_synthetic_tone {
            return Err(
                "QUARANTINE REJECT: Track contains synthetic sine/test tone and cannot be shared on Hera P2P."
                    .to_string(),
            );
        }
        if !forensic.is_authentic_lossless {
            return Err(format!(
                "QUARANTINE REJECT: Fake lossless transcode detected ({:.0} Hz cutoff, {}).",
                forensic.effective_bandwidth_hz, forensic.suspected_source_codec
            ));
        }
        if !forensic.mono_compatible_sub {
            return Err(format!(
                "QUARANTINE REJECT: Sub-bass (<120 Hz) phase cancellation detected (corr={:.2}).",
                forensic.sub_bass_phase_correlation
            ));
        }
        if forensic.quality_score < 65.0 {
            return Err(format!(
                "QUARANTINE REJECT: Quality score {:.1} is below minimum P2P threshold (65.0).",
                forensic.quality_score
            ));
        }

        if let Some(ref fid) = track.gdrive_file_id {
            let clean_id = extract_gdrive_id(fid);
            track.gdrive_file_id = Some(clean_id.clone());
            if track.stream_url.is_none() {
                track.stream_url = Some(build_gdrive_direct_download_url(&clean_id));
            }
        }

        track.node_id = self.node_id.clone();
        track.effective_bandwidth_hz = forensic.effective_bandwidth_hz;
        track.sub_bass_phase_correlation = forensic.sub_bass_phase_correlation;
        track.mono_compatible_sub = forensic.mono_compatible_sub;
        track.quality_score = forensic.quality_score;
        track.forensic_verdict = forensic.forensic_verdict.clone();
        track.hera_seal = compute_hera_seal(
            &track.track_id,
            track.bpm,
            &track.camelot_key,
            track.quality_score,
            &track.forensic_verdict,
        );

        if let Ok(mut guard) = self.catalog.write() {
            guard.retain(|t| t.track_id != track.track_id);
            guard.push(track.clone());
        }
        Ok(track)
    }

    pub fn search(&self, params: &P2PSearchQuery) -> Vec<P2PVerifiedTrack> {
        let Ok(guard) = self.catalog.read() else {
            return vec![];
        };
        let q_lower = params
            .q
            .as_ref()
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        let cam_upper = params
            .camelot
            .as_ref()
            .map(|s| s.trim().to_ascii_uppercase())
            .unwrap_or_default();
        let sub_lower = params
            .subgenre
            .as_ref()
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        let dj_lower = params
            .owner_dj
            .as_ref()
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        let verified_only = params.verified_only.unwrap_or(true);

        guard
            .iter()
            .filter(|t| {
                if !t.share_on_p2p {
                    return false;
                }
                if verified_only && t.forensic_verdict != "VERIFIED_STUDIO_MASTER" {
                    return false;
                }
                if !dj_lower.is_empty() && t.owner_dj.to_ascii_lowercase() != dj_lower {
                    return false;
                }
                if !q_lower.is_empty() {
                    let hay = format!(
                        "{} {} {} {} {}",
                        t.artist.to_ascii_lowercase(),
                        t.title.to_ascii_lowercase(),
                        t.owner_dj.to_ascii_lowercase(),
                        t.top_subgenre.to_ascii_lowercase(),
                        t.camelot_key.to_ascii_lowercase()
                    );
                    if !hay.contains(&q_lower) {
                        return false;
                    }
                }
                if !cam_upper.is_empty() && t.camelot_key.to_ascii_uppercase() != cam_upper {
                    return false;
                }
                if !sub_lower.is_empty()
                    && !t.top_subgenre.to_ascii_lowercase().contains(&sub_lower)
                {
                    return false;
                }
                if let Some(bmin) = params.bpm_min {
                    if t.bpm < bmin {
                        return false;
                    }
                }
                if let Some(bmax) = params.bpm_max {
                    if t.bpm > bmax {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }
}
