//! Asynchronous non-blocking OMKF v1 Swarm KV & DjAccountVaultDO cloud client (`Milestone C4`).
#![allow(dead_code)]

use hera_node_rs::scanner::omkf_capsule::{
    OMKF_FLAG_HAS_AUDIO_SKIPS, OMKF_FLAG_IS_FAKE_320KBPS, OMKF_FLAG_IS_OWN_PRODUCTION,
    OMKF_SLOT_BYTES, OmkfTrackSlot,
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

const DEFAULT_CLOUD_URL: &str = "https://hera-node.pc911itservices.workers.dev";
const POISON_OR_PRIVATE_MASK: u16 =
    OMKF_FLAG_IS_FAKE_320KBPS | OMKF_FLAG_HAS_AUDIO_SKIPS | OMKF_FLAG_IS_OWN_PRODUCTION | (1 << 5);

macro_rules! cloud_dto {
    ($(#[$m:meta])* $name:ident => $($f:ident : $t:ty),* $(,)?) => {
        $(#[$m])* #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name { $(pub $f: $t),* }
    };
}

cloud_dto!(CloudKvMetadata => v: u8, bpm: u16, ebpm: u16, xbpm: u16, key: u8, lufs: i16, rg: i16, gf: u8, era: u16, dur: u16, sr: u16, flags: u16, conf: u8, ts: i64);
cloud_dto!(CloudSwarmLookupKey => partial_hash_64kb: String, file_size_bytes: u64);
cloud_dto!(CloudSwarmLookupRequest => keys: Vec<CloudSwarmLookupKey>);
cloud_dto!(CloudSwarmPublishItem => partial_hash_64kb: String, file_size_bytes: u64, slot_base64_or_hex: String, meta: CloudKvMetadata);
cloud_dto!(CloudSwarmPublishRequest => dj_handle: String, items: Vec<CloudSwarmPublishItem>);
cloud_dto!(#[derive(Default)] CloudSwarmPublishReport => accepted: usize, rejected_poisoned: usize, skipped_rate_guard: usize, d1_batches_executed: usize);
cloud_dto!(CloudCrateDigest => crate_id: String, tree_seal_64: String, rules_sha256: String, updated_at_unix_ms: i64);
cloud_dto!(CloudTreeDiffRequest => dj_handle: String, crates: Vec<CloudCrateDigest>);
cloud_dto!(#[derive(Default)] CloudTreeDiffResponse => need_push_to_cloud: Vec<String>, need_pull_from_cloud: Vec<String>, in_sync: Vec<String>);
cloud_dto!(CloudTrackRefDto => track_uuid: String, partial_hash_64kb: String, file_size_bytes: u64, rel_filename: String, user_hot_cues_ms_json: String);
cloud_dto!(CloudLedgerEntryDto => tx_id: String, entry_line: String, created_at_unix_ms: i64);
cloud_dto!(CloudCrateDeltaItem => crate_id: String, folder_rel_path: String, tree_seal_64: String, rules_sha256: String, rules_markdown: String, updated_at_unix_ms: i64, requested_share_on_p2p: Option<bool>, track_refs: Vec<CloudTrackRefDto>, ledger_entries: Vec<CloudLedgerEntryDto>);
cloud_dto!(CloudPushDeltaRequest => dj_handle: String, subscription_tier: Option<String>, crates: Vec<CloudCrateDeltaItem>);
cloud_dto!(CloudPulledCrate => crate_id: String, folder_rel_path: String, tree_seal_64: String, rules_sha256: String, rules_markdown: String, share_on_p2p: bool, updated_at_unix_ms: i64, track_refs: Vec<CloudTrackRefDto>, ledger_entries: Vec<CloudLedgerEntryDto>);

pub fn resolve_hera_cloud_base_url() -> String {
    std::env::var("HERA_CLOUD_API_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_CLOUD_URL.to_string())
}

pub fn build_swarm_publish_batch(
    dj_handle: &str,
    entries: &[(String, u64, OmkfTrackSlot)],
    now_sec: i64,
) -> CloudSwarmPublishRequest {
    let mut items = Vec::with_capacity(entries.len().min(200));
    for (hash_hex, size, raw_slot) in entries {
        if items.len() >= 200
            || raw_slot.scan_level != 2
            || !(4500..=22_000).contains(&raw_slot.bpm_x100)
            || (raw_slot.flags_bits & POISON_OR_PRIVATE_MASK) != 0
        {
            continue;
        }
        let mut s = raw_slot.clone();
        (s.mtime_ns, s.name_offset, s.name_len) = (0, 0, 0);
        s.flags_bits &= !((1 << 5) | OMKF_FLAG_IS_OWN_PRODUCTION);
        let hex: String = s.to_bytes().iter().map(|b| format!("{b:02x}")).collect();
        let meta = CloudKvMetadata {
            v: 1,
            bpm: s.bpm_x100,
            ebpm: s.entry_bpm_x100,
            xbpm: s.exit_bpm_x100,
            key: s.camelot_code,
            lufs: s.lufs_x100,
            rg: s.replay_gain_x100,
            gf: s.genre_family_code,
            era: s.inferred_era_year,
            dur: s.duration_sec_u16,
            sr: s.sample_rate_div100,
            flags: s.flags_bits,
            conf: 1,
            ts: now_sec,
        };
        items.push(CloudSwarmPublishItem {
            partial_hash_64kb: hash_hex.trim().to_ascii_lowercase(),
            file_size_bytes: *size,
            slot_base64_or_hex: hex,
            meta,
        });
    }
    CloudSwarmPublishRequest {
        dj_handle: dj_handle.trim().to_string(),
        items,
    }
}

pub fn parse_swarm_lookup_binary_response(bytes: &[u8]) -> Vec<(u32, OmkfTrackSlot)> {
    bytes
        .chunks_exact(4 + OMKF_SLOT_BYTES)
        .filter_map(|frame| {
            let idx = u32::from_le_bytes(frame[..4].try_into().ok()?);
            OmkfTrackSlot::from_bytes(&frame[4..])
                .ok()
                .map(|s| (idx, s))
        })
        .collect()
}

async fn http_cloud_bytes(method: &str, url: &str, body: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let mut cmd = tokio::process::Command::new("curl");
    cmd.args([
        "--silent",
        "--show-error",
        "--fail-with-body",
        "--max-time",
        "4",
        "-X",
        method,
    ]);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    if body.is_some() {
        cmd.args([
            "-H",
            "Content-Type: application/json",
            "--data-binary",
            "@-",
        ]);
        cmd.stdin(std::process::Stdio::piped());
    }
    cmd.arg(url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    if let (Some(payload), Some(mut stdin)) = (body, child.stdin.take()) {
        stdin.write_all(payload).await.map_err(|e| e.to_string())?;
        let _ = stdin.shutdown().await;
    }
    let out = child.wait_with_output().await.map_err(|e| e.to_string())?;
    out.status
        .success()
        .then_some(out.stdout)
        .ok_or_else(|| String::from_utf8_lossy(&out.stderr).into_owned())
}

async fn post_json<B: Serialize, R: for<'a> Deserialize<'a>>(
    path: &str,
    body: &B,
) -> Result<R, String> {
    let raw = serde_json::to_vec(body).map_err(|e| e.to_string())?;
    let bytes = http_cloud_bytes(
        "POST",
        &format!("{}{path}", resolve_hera_cloud_base_url()),
        Some(&raw),
    )
    .await?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub async fn lookup_swarm_slots_async(
    req: &CloudSwarmLookupRequest,
) -> Option<Vec<(u32, OmkfTrackSlot)>> {
    let raw = serde_json::to_vec(req).ok()?;
    let bytes = http_cloud_bytes(
        "POST",
        &format!(
            "{}/api/v1/edge/omkf-swarm/lookup",
            resolve_hera_cloud_base_url()
        ),
        Some(&raw),
    )
    .await
    .ok()?;
    Some(parse_swarm_lookup_binary_response(&bytes))
}
pub async fn publish_swarm_slots_async(
    req: &CloudSwarmPublishRequest,
) -> Option<CloudSwarmPublishReport> {
    post_json("/api/v1/edge/omkf-swarm/publish", req).await.ok()
}
pub async fn sync_omkf_tree_diff_async(
    req: &CloudTreeDiffRequest,
) -> Option<CloudTreeDiffResponse> {
    post_json("/api/v1/omkf/crates/tree-diff", req).await.ok()
}
pub async fn push_omkf_crate_deltas_async(req: &CloudPushDeltaRequest) -> Option<usize> {
    let v: serde_json::Value = post_json("/api/v1/omkf/crates/push-delta", req)
        .await
        .ok()?;
    v["updated_crates"].as_u64().map(|n| n as usize)
}
pub async fn pull_omkf_crate_async(dj_handle: &str, crate_id: &str) -> Option<CloudPulledCrate> {
    let url = format!(
        "{}/api/v1/omkf/crates/pull/{}?dj_handle={}",
        resolve_hera_cloud_base_url(),
        crate_id.trim(),
        dj_handle.trim()
    );
    let bytes = http_cloud_bytes("GET", &url, None).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_batch_scrubs_pii_filters_poison_and_parses_lookup_frames() {
        let mut clean = OmkfTrackSlot {
            scan_level: 2,
            bpm_x100: 12600,
            camelot_code: 13,
            mtime_ns: 999,
            name_offset: 7,
            name_len: 12,
            ..Default::default()
        };
        let own = OmkfTrackSlot {
            scan_level: 2,
            bpm_x100: 12800,
            flags_bits: OMKF_FLAG_IS_OWN_PRODUCTION,
            ..Default::default()
        };
        let fake = OmkfTrackSlot {
            scan_level: 2,
            bpm_x100: 12800,
            flags_bits: OMKF_FLAG_IS_FAKE_320KBPS,
            ..Default::default()
        };
        let batch = build_swarm_publish_batch(
            "DaviDJ",
            &[
                ("AA".repeat(16), 42, clean.clone()),
                ("BB".repeat(16), 43, own),
                ("CC".repeat(16), 44, fake),
            ],
            1700,
        );
        assert_eq!(batch.items.len(), 1);
        let raw: Vec<u8> = (0..OMKF_SLOT_BYTES)
            .map(|i| {
                u8::from_str_radix(&batch.items[0].slot_base64_or_hex[i * 2..i * 2 + 2], 16)
                    .unwrap()
            })
            .collect();
        assert_eq!((&raw[24..32], &raw[80..86]), (&[0u8; 8][..], &[0u8; 6][..]));
        clean.mtime_ns = 0;
        let mut frame = 2u32.to_le_bytes().to_vec();
        frame.extend_from_slice(&clean.to_bytes());
        let decoded = parse_swarm_lookup_binary_response(&frame);
        assert_eq!(
            (decoded.len(), decoded[0].0, decoded[0].1.bpm_x100),
            (1, 2, 12600)
        );
    }
}
