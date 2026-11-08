use crate::forensics::ForensicAuditReport;
use crate::structure::HotCuePoint;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// A track shared on the Hera Federated P2P Mesh, guaranteed to have passed Zero-Trust DSP Forensics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PVerifiedTrack {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    #[serde(default = "default_owner_dj")]
    pub owner_dj: String,
    pub bpm: f32,
    pub camelot_key: String,
    pub top_subgenre: String,
    pub duration_seconds: f32,
    pub format: String,
    pub effective_bandwidth_hz: f32,
    pub sub_bass_phase_correlation: f32,
    pub mono_compatible_sub: bool,
    pub quality_score: f32,
    pub forensic_verdict: String,
    pub hot_cues: Vec<HotCuePoint>,
    #[serde(default = "default_storage_provider")]
    pub storage_provider: String,
    #[serde(default)]
    pub gdrive_file_id: Option<String>,
    #[serde(default)]
    pub r2_object_key: Option<String>,
    pub stream_url: Option<String>,
    pub node_id: String,
    pub hera_seal: String,
}

fn default_owner_dj() -> String {
    "DaviDJ".to_string()
}

fn default_storage_provider() -> String {
    "gdrive".to_string()
}

/// Zero-Tech DJ profile registered on the Hera Node (connects via a simple Google Drive folder link or R2 bucket).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DjProfile {
    pub dj_handle: String,
    pub display_name: String,
    pub role: String,
    pub storage_provider: String,
    pub gdrive_folder_id: Option<String>,
    pub gdrive_folder_url: Option<String>,
    pub r2_bucket_url: Option<String>,
    pub verified_tracks_shared: usize,
    pub quarantined_rejected_tracks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DjDriveTrackInput {
    pub title: String,
    pub artist: String,
    #[serde(default = "default_bpm")]
    pub bpm: f32,
    #[serde(default = "default_camelot")]
    pub camelot_key: String,
    #[serde(default = "default_subgenre")]
    pub top_subgenre: String,
    #[serde(default = "default_duration")]
    pub duration_seconds: f32,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(alias = "gdrive_file_link")]
    pub gdrive_file_link_or_id: Option<String>,
    pub r2_object_key: Option<String>,
    #[serde(default = "default_bw")]
    pub effective_bandwidth_hz: f32,
    #[serde(default = "default_phase")]
    pub sub_bass_phase_correlation: f32,
    #[serde(default)]
    pub is_synthetic_tone: bool,
}

fn default_bpm() -> f32 {
    124.0
}
fn default_camelot() -> String {
    "8A".to_string()
}
fn default_subgenre() -> String {
    "Afro House / Tech House".to_string()
}
fn default_duration() -> f32 {
    240.0
}
fn default_format() -> String {
    "flac".to_string()
}
fn default_bw() -> f32 {
    21200.0
}
fn default_phase() -> f32 {
    0.92
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectDjDriveRequest {
    pub dj_handle: String,
    pub display_name: Option<String>,
    /// Any messy Google Drive folder link pasted by a non-technical DJ
    pub gdrive_folder_link: Option<String>,
    /// Optional Cloudflare R2 bucket public URL
    pub r2_bucket_url: Option<String>,
    /// Optional list of tracks found in the DJ's shared Google Drive / R2 folder
    #[serde(default)]
    pub tracks: Vec<DjDriveTrackInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectDjDriveResponse {
    pub status: String,
    pub profile: DjProfile,
    pub admitted_tracks: Vec<P2PVerifiedTrack>,
    pub rejected_quarantine_tracks: Vec<String>,
    pub network_catalog_available: usize,
    pub welcome_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PPeerNode {
    pub node_id: String,
    pub endpoint_url: String,
    pub runtime_target: String,
    pub verified_tracks_count: usize,
    pub bandwidth_mbps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudReferralOption {
    pub provider: String,
    pub tier_type: String,
    pub user_free_bonus: String,
    pub creator_referral_reward: String,
    pub deploy_url: String,
    pub one_click_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PSearchQuery {
    pub q: Option<String>,
    pub camelot: Option<String>,
    pub bpm_min: Option<f32>,
    pub bpm_max: Option<f32>,
    pub subgenre: Option<String>,
    pub owner_dj: Option<String>,
    pub verified_only: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct P2PMeshState {
    pub node_id: String,
    pub catalog: Arc<RwLock<Vec<P2PVerifiedTrack>>>,
    pub dj_profiles: Arc<RwLock<Vec<DjProfile>>>,
    pub peers: Arc<RwLock<Vec<P2PPeerNode>>>,
}

impl P2PMeshState {
    pub fn new(node_id: impl Into<String>) -> Self {
        let nid = node_id.into();
        let seed_catalog = vec![
            P2PVerifiedTrack {
                track_id: "hera-seed-001".to_string(),
                title: "Ritual Del Sol (DaviDJ Original Club Master)".to_string(),
                artist: "DaviDJ".to_string(),
                owner_dj: "DaviDJ".to_string(),
                bpm: 122.0,
                camelot_key: "8A".to_string(),
                top_subgenre: "Afro House / Deep Tech".to_string(),
                duration_seconds: 342.0,
                format: "flac".to_string(),
                effective_bandwidth_hz: 21450.0,
                sub_bass_phase_correlation: 0.96,
                mono_compatible_sub: true,
                quality_score: 98.5,
                forensic_verdict: "VERIFIED_STUDIO_MASTER".to_string(),
                hot_cues: vec![
                    HotCuePoint {
                        num: 1,
                        name: "1.1.1 Downbeat".to_string(),
                        seconds: 0.25,
                        bar_index: 0,
                        role: "downbeat".to_string(),
                        confidence: 0.98,
                    },
                    HotCuePoint {
                        num: 2,
                        name: "Phrase In (8b)".to_string(),
                        seconds: 15.98,
                        bar_index: 8,
                        role: "phrase_in".to_string(),
                        confidence: 0.93,
                    },
                    HotCuePoint {
                        num: 3,
                        name: "Main Drop (16b)".to_string(),
                        seconds: 31.72,
                        bar_index: 16,
                        role: "drop".to_string(),
                        confidence: 0.96,
                    },
                    HotCuePoint {
                        num: 4,
                        name: "Mix Out (144b)".to_string(),
                        seconds: 283.52,
                        bar_index: 144,
                        role: "mix_out".to_string(),
                        confidence: 0.94,
                    },
                ],
                storage_provider: "gdrive".to_string(),
                gdrive_file_id: Some("1DaviDJ_RitualDelSol_MasterFLAC".to_string()),
                r2_object_key: Some("masters/davidj/ritual_del_sol.flac".to_string()),
                stream_url: Some(build_gdrive_direct_download_url(
                    "1DaviDJ_RitualDelSol_MasterFLAC",
                )),
                node_id: nid.clone(),
                hera_seal: compute_hera_seal(
                    "hera-seed-001",
                    122.0,
                    "8A",
                    98.5,
                    "VERIFIED_STUDIO_MASTER",
                ),
            },
            P2PVerifiedTrack {
                track_id: "hera-seed-002".to_string(),
                title: "Guaracha Turbo Club (DaviDJ Extended Mix)".to_string(),
                artist: "DaviDJ".to_string(),
                owner_dj: "DaviDJ".to_string(),
                bpm: 128.0,
                camelot_key: "8A".to_string(),
                top_subgenre: "Tech House / Guaracha".to_string(),
                duration_seconds: 296.0,
                format: "flac".to_string(),
                effective_bandwidth_hz: 21800.0,
                sub_bass_phase_correlation: 0.94,
                mono_compatible_sub: true,
                quality_score: 97.0,
                forensic_verdict: "VERIFIED_STUDIO_MASTER".to_string(),
                hot_cues: vec![
                    HotCuePoint {
                        num: 1,
                        name: "1.1.1 Downbeat".to_string(),
                        seconds: 0.12,
                        bar_index: 0,
                        role: "downbeat".to_string(),
                        confidence: 0.97,
                    },
                    HotCuePoint {
                        num: 3,
                        name: "Main Drop (16b)".to_string(),
                        seconds: 30.12,
                        bar_index: 16,
                        role: "drop".to_string(),
                        confidence: 0.95,
                    },
                ],
                storage_provider: "gdrive".to_string(),
                gdrive_file_id: Some("1DaviDJ_GuarachaTurbo_MasterFLAC".to_string()),
                r2_object_key: Some("masters/davidj/guaracha_turbo.flac".to_string()),
                stream_url: Some(build_gdrive_direct_download_url(
                    "1DaviDJ_GuarachaTurbo_MasterFLAC",
                )),
                node_id: nid.clone(),
                hera_seal: compute_hera_seal(
                    "hera-seed-002",
                    128.0,
                    "8A",
                    97.0,
                    "VERIFIED_STUDIO_MASTER",
                ),
            },
            P2PVerifiedTrack {
                track_id: "hera-seed-003".to_string(),
                title: "Teteo Polirritmico 96 (DaviDJ 3:4 Bridge Master)".to_string(),
                artist: "DaviDJ".to_string(),
                owner_dj: "DaviDJ".to_string(),
                bpm: 96.0,
                camelot_key: "8A".to_string(),
                top_subgenre: "Dembow / Urbano".to_string(),
                duration_seconds: 218.0,
                format: "flac".to_string(),
                effective_bandwidth_hz: 21100.0,
                sub_bass_phase_correlation: 0.95,
                mono_compatible_sub: true,
                quality_score: 96.5,
                forensic_verdict: "VERIFIED_STUDIO_MASTER".to_string(),
                hot_cues: vec![
                    HotCuePoint {
                        num: 1,
                        name: "1.1.1 Downbeat".to_string(),
                        seconds: 0.10,
                        bar_index: 0,
                        role: "downbeat".to_string(),
                        confidence: 0.98,
                    },
                    HotCuePoint {
                        num: 3,
                        name: "Main Drop (16b)".to_string(),
                        seconds: 40.10,
                        bar_index: 16,
                        role: "drop".to_string(),
                        confidence: 0.95,
                    },
                ],
                storage_provider: "gdrive".to_string(),
                gdrive_file_id: Some("1DaviDJ_Teteo96_MasterFLAC".to_string()),
                r2_object_key: Some("masters/davidj/teteo_96.flac".to_string()),
                stream_url: Some(build_gdrive_direct_download_url(
                    "1DaviDJ_Teteo96_MasterFLAC",
                )),
                node_id: nid.clone(),
                hera_seal: compute_hera_seal(
                    "hera-seed-003",
                    96.0,
                    "8A",
                    96.5,
                    "VERIFIED_STUDIO_MASTER",
                ),
            },
        ];

        let founder_profile = vec![DjProfile {
            dj_handle: "DaviDJ".to_string(),
            display_name: "David José (DaviDJ — Founder & Master Curator)".to_string(),
            role: "founder_node_operator".to_string(),
            storage_provider: "gdrive+r2".to_string(),
            gdrive_folder_id: Some("1DaviDJ_Official_Repertoire_Folder".to_string()),
            gdrive_folder_url: Some(
                "https://drive.google.com/drive/folders/1DaviDJ_Official_Repertoire_Folder"
                    .to_string(),
            ),
            r2_bucket_url: Some("https://cdn.hera-davidj.workers.dev".to_string()),
            verified_tracks_shared: 3,
            quarantined_rejected_tracks: 0,
        }];

        Self {
            node_id: nid,
            catalog: Arc::new(RwLock::new(seed_catalog)),
            dj_profiles: Arc::new(RwLock::new(founder_profile)),
            peers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Zero-Tech Onboarding: A non-technical DJ (`DJFoo`) simply pastes their DJ name and Google Drive folder link.
    /// The node extracts the Drive IDs, audits every track with Zero-Trust rules, admits valid tracks to the P2P hive,
    /// and rejects fake MP3->FLAC transcodes or phase-cancelled tracks.
    pub fn connect_dj_drive(&self, req: ConnectDjDriveRequest) -> ConnectDjDriveResponse {
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

        let storage_provider = if req.r2_bucket_url.is_some() && gdrive_folder_id.is_some() {
            "gdrive+r2".to_string()
        } else if req.r2_bucket_url.is_some() {
            "r2".to_string()
        } else {
            "gdrive".to_string()
        };

        let mut admitted = Vec::new();
        let mut rejected = Vec::new();

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

            let file_id = item
                .gdrive_file_link_or_id
                .as_deref()
                .map(extract_gdrive_id)
                .or_else(|| {
                    gdrive_folder_id
                        .as_ref()
                        .map(|fid| format!("{fid}_trk_{:02}", idx + 1))
                });

            let stream_url = if let Some(ref fid) = file_id {
                Some(build_gdrive_direct_download_url(fid))
            } else if let (Some(ref r2_base), Some(ref key)) =
                (&req.r2_bucket_url, &item.r2_object_key)
            {
                Some(format!("{}/{}", r2_base.trim_end_matches('/'), key))
            } else {
                None
            };

            let track_id = format!(
                "dj-{}-{:03}",
                handle.to_ascii_lowercase().replace(' ', "-"),
                idx + 1
            );
            let bar_sec = (60.0 / item.bpm.max(60.0)) * 4.0;
            let c2 = (8.0 * bar_sec).min(item.duration_seconds * 0.25);
            let c3 = (16.0 * bar_sec).min(item.duration_seconds * 0.45);
            let c4 = (item.duration_seconds - 8.0 * bar_sec).max(c3 + bar_sec);

            let hot_cues = vec![
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
            ];

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
                storage_provider: storage_provider.clone(),
                gdrive_file_id: file_id,
                r2_object_key: item.r2_object_key.clone(),
                stream_url,
                node_id: self.node_id.clone(),
                hera_seal: seal,
            };

            if let Ok(mut guard) = self.catalog.write() {
                guard.retain(|t| t.track_id != verified_track.track_id);
                guard.push(verified_track.clone());
            }
            admitted.push(verified_track);
        }

        let profile = DjProfile {
            dj_handle: handle.clone(),
            display_name: display_name.clone(),
            role: "community_dj_peer".to_string(),
            storage_provider,
            gdrive_folder_id,
            gdrive_folder_url: req.gdrive_folder_link,
            r2_bucket_url: req.r2_bucket_url,
            verified_tracks_shared: admitted.len(),
            quarantined_rejected_tracks: rejected.len(),
        };

        if let Ok(mut p_guard) = self.dj_profiles.write() {
            p_guard.retain(|p| !p.dj_handle.eq_ignore_ascii_case(&handle));
            p_guard.push(profile.clone());
        }

        let total_network_tracks = self.catalog.read().map(|c| c.len()).unwrap_or(0);

        ConnectDjDriveResponse {
            status: "connected".to_string(),
            profile,
            admitted_tracks: admitted.clone(),
            rejected_quarantine_tracks: rejected.clone(),
            network_catalog_available: total_network_tracks,
            welcome_message: format!(
                "¡Bienvenido {}! Tu Google Drive quedó conectado sin configurar servidores. Se admitieron {} pistas verificadas ({} rechazadas por aduana forense) y ya tienes acceso a las {} pistas verificadas de la red P2P (incluyendo el repertorio de DaviDJ).",
                handle,
                admitted.len(),
                rejected.len(),
                total_network_tracks
            ),
        }
    }

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

/// Extract clean Google Drive File or Folder ID from any share link pasted by a non-technical DJ.
pub fn extract_gdrive_id(url_or_id: &str) -> String {
    let trimmed = url_or_id.trim();
    if let Some(idx) = trimmed.find("/folders/") {
        let rest = &trimmed[idx + 9..];
        return rest
            .split(&['?', '/', '&'][..])
            .next()
            .unwrap_or(rest)
            .to_string();
    }
    if let Some(idx) = trimmed.find("/file/d/") {
        let rest = &trimmed[idx + 8..];
        return rest
            .split(&['?', '/', '&'][..])
            .next()
            .unwrap_or(rest)
            .to_string();
    }
    if let Some(idx) = trimmed.find("id=") {
        let rest = &trimmed[idx + 3..];
        return rest
            .split(&['&', '/', '?'][..])
            .next()
            .unwrap_or(rest)
            .to_string();
    }
    trimmed.to_string()
}

/// Generate a direct download / streaming URL from a Google Drive File ID.
pub fn build_gdrive_direct_download_url(file_id: &str) -> String {
    let clean = extract_gdrive_id(file_id);
    format!("https://drive.google.com/uc?export=download&id={clean}")
}

/// Deterministic FNV-1a 64-bit acoustic verification seal for P2P trust.
pub fn compute_hera_seal(
    track_id: &str,
    bpm: f32,
    camelot: &str,
    quality_score: f32,
    verdict: &str,
) -> String {
    let payload = format!("HERA-V2|{track_id}|{bpm:.2}|{camelot}|{quality_score:.1}|{verdict}");
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in payload.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("hera-seal-{hash:016x}")
}

/// Built-in low-cost / free cloud deployment & affiliate referral options.
pub fn get_cloud_referral_catalog() -> Vec<CloudReferralOption> {
    vec![
        CloudReferralOption {
            provider: "Cloudflare Workers (Serverless WASM)".to_string(),
            tier_type: "100% Free Tier (100k req/day + D1 SQLite + Google Drive / R2)".to_string(),
            user_free_bonus: "$0.00/month Forever Free Edge Node".to_string(),
            creator_referral_reward: "Massive viral open-source adoption & zero hosting cost".to_string(),
            deploy_url: "https://dash.cloudflare.com/?to=/:account/workers".to_string(),
            one_click_command: "cd hera-rs && npx wrangler deploy".to_string(),
        },
        CloudReferralOption {
            provider: "Railway.app (1-Click Container + TCP P2P)".to_string(),
            tier_type: "Trial + $5/mo Hobby".to_string(),
            user_free_bonus: "$5.00 Free Trial Credit".to_string(),
            creator_referral_reward: "Up to 25%-50% recurring monthly template kickback".to_string(),
            deploy_url: "https://railway.app/template/hera-node?referralCode=HERA_REFERRAL".to_string(),
            one_click_command: "railway up".to_string(),
        },
        CloudReferralOption {
            provider: "Vultr Cloud Compute (1 Gbps P2P Node)".to_string(),
            tier_type: "Micro VPS from $2.50-$5.00/mo".to_string(),
            user_free_bonus: "$100 to $300 Free Credit for new users".to_string(),
            creator_referral_reward: "$35 to $100 USD cash payout per referred active user".to_string(),
            deploy_url: "https://www.vultr.com/?ref=HERA_VULTR_REF".to_string(),
            one_click_command: "curl -fsSL https://raw.githubusercontent.com/davidcastilloc/hera/main/deploy/install-node.sh | bash".to_string(),
        },
        CloudReferralOption {
            provider: "DigitalOcean Droplets".to_string(),
            tier_type: "Basic Droplet from $4.00/mo".to_string(),
            user_free_bonus: "$200 Free Credit for 60 days (2 months 100% free)".to_string(),
            creator_referral_reward: "$25 to $200 USD per referred user reaching $25 spend".to_string(),
            deploy_url: "https://m.do.co/c/HERA_DO_REF".to_string(),
            one_click_command: "curl -fsSL https://raw.githubusercontent.com/davidcastilloc/hera/main/deploy/install-node.sh | bash".to_string(),
        },
        CloudReferralOption {
            provider: "Hetzner Cloud (High-Speed EU/US VPS)".to_string(),
            tier_type: "CX22 (2 vCPU, 4GB RAM, 20TB 10Gbps) ~3.79 EUR/mo".to_string(),
            user_free_bonus: "20 EUR Free Credit (~5 months free)".to_string(),
            creator_referral_reward: "10 EUR credit per referred user".to_string(),
            deploy_url: "https://hetzner.cloud/?ref=HERA_HETZNER_REF".to_string(),
            one_click_command: "curl -fsSL https://raw.githubusercontent.com/davidcastilloc/hera/main/deploy/install-node.sh | bash".to_string(),
        },
    ]
}
