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
    pub stream_url: Option<String>,
    pub node_id: String,
    pub hera_seal: String,
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
    pub verified_only: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct P2PMeshState {
    pub node_id: String,
    pub catalog: Arc<RwLock<Vec<P2PVerifiedTrack>>>,
    pub peers: Arc<RwLock<Vec<P2PPeerNode>>>,
}

impl P2PMeshState {
    pub fn new(node_id: impl Into<String>) -> Self {
        let nid = node_id.into();
        let seed_catalog = vec![
            P2PVerifiedTrack {
                track_id: "hera-seed-001".to_string(),
                title: "Ritual Del Sol (Original Club Master)".to_string(),
                artist: "Hera Collective".to_string(),
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
                stream_url: Some("/api/v1/p2p/stream/hera-seed-001".to_string()),
                node_id: nid.clone(),
                hera_seal: compute_hera_seal("hera-seed-001", 122.0, "8A", 98.5, "VERIFIED_STUDIO_MASTER"),
            },
            P2PVerifiedTrack {
                track_id: "hera-seed-002".to_string(),
                title: "Guaracha Turbo Club (Extended Mix)".to_string(),
                artist: "Hera Collective".to_string(),
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
                stream_url: Some("/api/v1/p2p/stream/hera-seed-002".to_string()),
                node_id: nid.clone(),
                hera_seal: compute_hera_seal("hera-seed-002", 128.0, "8A", 97.0, "VERIFIED_STUDIO_MASTER"),
            },
        ];

        Self {
            node_id: nid,
            catalog: Arc::new(RwLock::new(seed_catalog)),
            peers: Arc::new(RwLock::new(Vec::new())),
        }
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
        let verified_only = params.verified_only.unwrap_or(true);

        guard
            .iter()
            .filter(|t| {
                if verified_only && t.forensic_verdict != "VERIFIED_STUDIO_MASTER" {
                    return false;
                }
                if !q_lower.is_empty() {
                    let hay = format!(
                        "{} {} {} {}",
                        t.artist.to_ascii_lowercase(),
                        t.title.to_ascii_lowercase(),
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

/// Deterministic FNV-1a 64-bit acoustic verification seal for P2P trust.
pub fn compute_hera_seal(
    track_id: &str,
    bpm: f32,
    camelot: &str,
    quality_score: f32,
    verdict: &str,
) -> String {
    let payload = format!(
        "HERA-V2|{track_id}|{bpm:.2}|{camelot}|{quality_score:.1}|{verdict}"
    );
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
            tier_type: "100% Free Tier (100k req/day + D1 SQLite + R2 10GB)".to_string(),
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
