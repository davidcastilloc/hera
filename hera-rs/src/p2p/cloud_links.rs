//! Cloud Share Link Normalization, Zero-OAuth Google Drive HTML Parser, CAS Fingerprinting,
//! 64-bit `SimHash` POPCNT Signatures, and Cloudflare SaaS Unit Economics (`src/p2p/cloud_links.rs`).

use super::CloudReferralOption;
use serde::{Deserialize, Serialize};

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

/// Zero-OAuth Google Drive public folder view URL (requires no API key and no OAuth).
pub fn build_gdrive_embedded_folder_url(folder_id_or_url: &str) -> String {
    let clean = extract_gdrive_id(folder_id_or_url);
    format!("https://drive.google.com/embeddedfolderview?id={clean}#list")
}

/// Google Drive API v3 read-only folder listing URL using a single server API key (zero OAuth for DJs).
pub fn build_gdrive_api_v3_folder_list_url(folder_id_or_url: &str, api_key: &str) -> String {
    let clean = extract_gdrive_id(folder_id_or_url);
    format!(
        "https://www.googleapis.com/drive/v3/files?q='{clean}'+in+parents+and+trashed=false&fields=files(id,name,mimeType,size,md5Checksum)&key={api_key}"
    )
}

/// Parses Google Drive's Zero-OAuth `/embeddedfolderview?id=<FOLDER_ID>#list` HTML into `(file_id, filename)` pairs.
pub fn parse_gdrive_public_folder_html(html: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut rest = html;

    while let Some(pos) = rest.find("id=\"entry-") {
        let after_id = &rest[pos + 10..];
        let Some(end_quote) = after_id.find('"') else {
            break;
        };
        let file_id = after_id[..end_quote].trim().to_string();
        rest = &after_id[end_quote..];

        // Look for `<div class="flip-entry-title">FILENAME</div>`
        if let Some(title_pos) = rest.find("flip-entry-title\">") {
            let after_title = &rest[title_pos + 18..];
            if let Some(close_div) = after_title.find("</div>") {
                let fname = after_title[..close_div].trim().to_string();
                if !file_id.is_empty() && !fname.is_empty() {
                    results.push((file_id, fname));
                }
                rest = &after_title[close_div..];
            }
        }
    }
    results
}

/// Normalizes any cloud share link (Dropbox, OneDrive, Archive.org, Google Drive, or Cloudflare R2)
/// into `(provider_name, direct_binary_stream_url)`.
pub fn normalize_cloud_share_link(url: &str) -> (String, String) {
    let trimmed = url.trim();
    let lower = trimmed.to_ascii_lowercase();

    if lower.contains("dropbox.com") {
        let direct = if trimmed.contains("dl=0") {
            trimmed.replace("dl=0", "dl=1")
        } else if trimmed.contains('?') {
            format!("{trimmed}&dl=1")
        } else {
            format!("{trimmed}?dl=1")
        };
        return ("dropbox".to_string(), direct);
    }

    if lower.contains("drive.google.com") {
        let fid = extract_gdrive_id(trimmed);
        return ("gdrive".to_string(), build_gdrive_direct_download_url(&fid));
    }

    if lower.contains("1drv.ms") || lower.contains("onedrive.live.com") {
        let direct = if trimmed.contains('?') {
            format!("{trimmed}&download=1")
        } else {
            format!("{trimmed}?download=1")
        };
        return ("onedrive".to_string(), direct);
    }

    if lower.contains("archive.org") {
        return ("archive_org".to_string(), trimmed.to_string());
    }

    ("r2_or_direct".to_string(), trimmed.to_string())
}

/// Deterministic Content-Addressed Storage (CAS) fingerprint for internal Cloud P2P deduplication.
/// Normalizes artist + title + rounded BPM so that if 20 DJs upload the same club master,
/// Hera stores only 1 physical copy in Cloudflare R2 (`0 MB` extra R2 cost for DJs #2..#20).
pub fn compute_cas_fingerprint(artist: &str, title: &str, bpm: f32) -> String {
    let clean_a: String = artist
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let clean_t: String = title
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let bpm_int = bpm.round() as u32;
    format!("cas-{clean_a}-{clean_t}-{bpm_int}bpm")
}

/// Computes an 8-byte (64-bit) Locality-Sensitive Binary Style Signature (`SimHash`)
/// so that tracks with similar BPM bucket, Camelot harmonic zone, and subgenre share bit patterns,
/// enabling single-instruction `(a ^ b).count_ones()` (`POPCNT`) similarity search across 100k+ tracks in < 0.1 ms.
pub fn compute_style_simhash_u64(bpm: f32, camelot: &str, subgenre: &str) -> u64 {
    let mut sig: u64 = 0;

    // Bits 0..15: Thermometer code for BPM bucket (70 BPM .. 166 BPM, 6 BPM per bit)
    let bpm_clamped = bpm.clamp(70.0, 166.0);
    let bpm_steps = ((bpm_clamped - 70.0) / 6.0).round() as u32;
    for i in 0..16u32 {
        if i <= bpm_steps {
            sig |= 1u64 << i;
        }
    }

    // Bits 16..31: Circular Camelot Wheel ring neighborhood (1..12 + A/B mode)
    let cam_trim = camelot.trim().to_ascii_uppercase();
    let is_major_b = cam_trim.ends_with('B');
    let num: u32 = cam_trim
        .trim_end_matches(&['A', 'B'][..])
        .parse::<u32>()
        .unwrap_or(8)
        .clamp(1, 12);
    let prev = if num == 1 { 12 } else { num - 1 };
    let next = if num == 12 { 1 } else { num + 1 };
    sig |= 1u64 << (15 + num);
    sig |= 1u64 << (15 + prev);
    sig |= 1u64 << (15 + next);
    if is_major_b {
        sig |= 0b1111u64 << 28;
    }

    // Bits 32..63: Subgenre token locality-sensitive projection
    let mut sub_bits: u32 = 0;
    let sub_lower = subgenre.to_ascii_lowercase();
    for (idx, token) in sub_lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .enumerate()
    {
        let mut h: u32 = 0x811c9dc5 ^ (idx as u32);
        for b in token.bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(0x01000193);
        }
        sub_bits |= 1u32 << (h % 32);
        sub_bits |= 1u32 << ((h >> 8) % 32);
        sub_bits |= 1u32 << ((h >> 16) % 32);
    }
    sig |= (sub_bits as u64) << 32;
    sig
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
            creator_referral_reward: "Massive viral open-source adoption & zero hosting cost"
                .to_string(),
            deploy_url: "https://dash.cloudflare.com/?to=/:account/workers".to_string(),
            one_click_command: "cd hera-rs && npx wrangler deploy".to_string(),
        },
        CloudReferralOption {
            provider: "Railway.app (1-Click Container + TCP P2P)".to_string(),
            tier_type: "Trial + $5/mo Hobby".to_string(),
            user_free_bonus: "$5.00 Free Trial Credit".to_string(),
            creator_referral_reward: "Up to 25%-50% recurring monthly template kickback"
                .to_string(),
            deploy_url: "https://railway.app/template/hera-node?referralCode=HERA_REFERRAL"
                .to_string(),
            one_click_command: "railway up".to_string(),
        },
        CloudReferralOption {
            provider: "Vultr Cloud Compute (1 Gbps P2P Node)".to_string(),
            tier_type: "Micro VPS from $2.50-$5.00/mo".to_string(),
            user_free_bonus: "$100 to $300 Free Credit for new users".to_string(),
            creator_referral_reward: "$35 to $100 USD cash payout per referred active user"
                .to_string(),
            deploy_url: "https://www.vultr.com/?ref=HERA_VULTR_REF".to_string(),
            one_click_command: "curl -fsSL https://raw.githubusercontent.com/davidcastilloc/hera/main/deploy/install-node.sh | bash".to_string(),
        },
        CloudReferralOption {
            provider: "DigitalOcean Droplets".to_string(),
            tier_type: "Basic Droplet from $4.00/mo".to_string(),
            user_free_bonus: "$200 Free Credit for 60 days (2 months 100% free)".to_string(),
            creator_referral_reward: "$25 to $200 USD per referred user reaching $25 spend"
                .to_string(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareBetaEconomicsReport {
    pub cohort_djs_count: usize,
    pub r2_quota_gb_per_dj: f32,
    pub total_r2_storage_gb: f32,
    pub approx_lossless_flac_tracks_per_dj: usize,
    pub total_shared_p2p_tracks_in_pool: usize,
    pub cloudflare_r2_free_tier_gb: f32,
    pub max_zero_cost_djs_in_free_r2_tier: usize,
    pub billable_r2_gb: f32,
    pub monthly_r2_storage_cost_usd: f32,
    pub monthly_egress_bandwidth_cost_usd: f32,
    pub monthly_workers_d1_vectorize_cost_usd: f32,
    pub total_monthly_cohort_cost_usd: f32,
    pub cost_per_dj_usd: f32,
    pub subscription_price_per_dj_usd: f32,
    pub gross_monthly_revenue_usd: f32,
    pub net_monthly_profit_usd: f32,
    pub profit_margin_pct: f32,
}

/// Calculates exact monthly Cloudflare unit economics & SaaS profit margins for a cohort of DJs maxing out the app.
pub fn calculate_cloudflare_beta_economics(
    num_djs: usize,
    gb_per_dj: f32,
) -> CloudflareBetaEconomicsReport {
    calculate_cloudflare_saas_economics_with_price(num_djs, gb_per_dj, 5.99)
}

pub fn calculate_cloudflare_saas_economics_with_price(
    num_djs: usize,
    gb_per_dj: f32,
    price_per_dj_usd: f32,
) -> CloudflareBetaEconomicsReport {
    let djs = num_djs.max(1);
    let gb_dj = gb_per_dj.max(0.1);
    let total_gb = (djs as f32) * gb_dj;
    let free_r2_gb = 10.0f32;
    let max_free_djs = (free_r2_gb / gb_dj).floor() as usize;
    let billable_gb = (total_gb - free_r2_gb).max(0.0);

    // Cloudflare R2: $0.015 / GB-month, $0.00 egress
    let r2_cost = (billable_gb * 0.015 * 100.0).round() / 100.0;
    // Heavy compute + D1 + Vectorize 64D (~$0.02/mo per DJ when covered by free/base allowances)
    let compute_db_cost = if djs <= 25 {
        0.0 // 100% covered inside Cloudflare Free Tier (100k req/day, 5M D1 reads/day, 30M Vectorize dims)
    } else {
        (((djs - 25) as f32) * 0.02 * 100.0).round() / 100.0
    };

    let total_cost = ((r2_cost + compute_db_cost) * 100.0).round() / 100.0;
    let per_dj = ((total_cost / (djs as f32)) * 1000.0).round() / 1000.0;
    let tracks_per_dj = ((gb_dj * 1024.0) / 38.0).round() as usize;

    let price = price_per_dj_usd.max(0.0);
    let gross_rev = ((djs as f32) * price * 100.0).round() / 100.0;
    let net_profit = ((gross_rev - total_cost) * 100.0).round() / 100.0;
    let margin_pct = if gross_rev > 0.0 {
        (((net_profit / gross_rev) * 100.0) * 10.0).round() / 10.0
    } else {
        0.0
    };

    CloudflareBetaEconomicsReport {
        cohort_djs_count: djs,
        r2_quota_gb_per_dj: gb_dj,
        total_r2_storage_gb: total_gb,
        approx_lossless_flac_tracks_per_dj: tracks_per_dj,
        total_shared_p2p_tracks_in_pool: tracks_per_dj * djs,
        cloudflare_r2_free_tier_gb: free_r2_gb,
        max_zero_cost_djs_in_free_r2_tier: max_free_djs,
        billable_r2_gb: billable_gb,
        monthly_r2_storage_cost_usd: r2_cost,
        monthly_egress_bandwidth_cost_usd: 0.0,
        monthly_workers_d1_vectorize_cost_usd: compute_db_cost,
        total_monthly_cohort_cost_usd: total_cost,
        cost_per_dj_usd: per_dj,
        subscription_price_per_dj_usd: price,
        gross_monthly_revenue_usd: gross_rev,
        net_monthly_profit_usd: net_profit,
        profit_margin_pct: margin_pct,
    }
}

/// Generates a Google Drive direct binary download URL with `&confirm=t` virus-scan warning bypass
/// automatically appended for lossless audio files larger than 25 MB.
pub fn build_gdrive_large_file_bypass_url(file_id: &str, file_size_mb: f32) -> String {
    let base = build_gdrive_direct_download_url(file_id);
    if file_size_mb > 25.0 {
        format!("{base}&confirm=t")
    } else {
        base
    }
}
