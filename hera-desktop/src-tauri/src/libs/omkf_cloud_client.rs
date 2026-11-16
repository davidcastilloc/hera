//! Asynchronous non-blocking OMKF v1 Swarm KV & DjAccountVaultDO cloud client (`Milestone C4`).
#![allow(dead_code)]

pub use super::omkf_cloud_types::*;
use hera_node_rs::scanner::omkf_capsule::{
    OMKF_FLAG_HAS_AUDIO_SKIPS, OMKF_FLAG_IS_FAKE_320KBPS, OMKF_FLAG_IS_OWN_PRODUCTION,
    OMKF_SLOT_BYTES, OmkfTrackSlot, decode_omkf_capsule,
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

const DEFAULT_CLOUD_URL: &str = "https://hera-cloud.pc911itservices.workers.dev";
const POISON_OR_PRIVATE_MASK: u16 =
    OMKF_FLAG_IS_FAKE_320KBPS | OMKF_FLAG_HAS_AUDIO_SKIPS | OMKF_FLAG_IS_OWN_PRODUCTION | (1 << 5);

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

pub fn build_crate_delta_and_swarm_batch_from_capsule(
    dj_handle: &str,
    crate_id: &str,
    folder_rel_path: &str,
    capsule_bytes: &[u8],
    crate_md: &str,
    now_ms: i64,
) -> Result<(CloudPushDeltaRequest, CloudSwarmPublishRequest), String> {
    let cap = decode_omkf_capsule(capsule_bytes)?;
    let mut track_refs = Vec::with_capacity(cap.tracks.len());
    let mut swarm_entries = Vec::with_capacity(cap.tracks.len());
    for (idx, t) in cap.tracks.iter().enumerate() {
        let hex32: String = t
            .slot
            .partial_hash_64kb
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        track_refs.push(CloudTrackRefDto {
            track_uuid: format!("{crate_id}:{idx}:{hex32}"),
            partial_hash_64kb: hex32.clone(),
            file_size_bytes: t.slot.file_size_bytes,
            rel_filename: t.filename.clone(),
            user_hot_cues_ms_json: serde_json::to_string(&t.slot.hot_cues_ms)
                .unwrap_or_else(|_| "[]".into()),
        });
        swarm_entries.push((hex32, t.slot.file_size_bytes, t.slot.clone()));
    }
    let ledger_entries: Vec<CloudLedgerEntryDto> = crate_md
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let rest = trimmed.split("tx:").nth(1)?;
            let tx_id = rest
                .trim_start_matches('#')
                .split([' ', '|'])
                .next()?
                .trim();
            if tx_id.is_empty() {
                return None;
            }
            Some(CloudLedgerEntryDto {
                tx_id: tx_id.to_string(),
                entry_line: trimmed.to_string(),
                created_at_unix_ms: now_ms,
            })
        })
        .collect();
    let mut fnv: u64 = 0xcbf29ce484222325;
    for b in crate_md.as_bytes() {
        fnv = (fnv ^ u64::from(*b)).wrapping_mul(0x100000001b3);
    }
    let delta = CloudPushDeltaRequest {
        dj_handle: dj_handle.trim().to_string(),
        subscription_tier: None,
        crates: vec![CloudCrateDeltaItem {
            crate_id: crate_id.trim().to_string(),
            folder_rel_path: folder_rel_path.to_string(),
            tree_seal_64: format!("{:016x}", cap.header.tree_seal_64),
            rules_sha256: format!("{fnv:016x}"),
            rules_markdown: crate_md.to_string(),
            updated_at_unix_ms: now_ms,
            requested_share_on_p2p: None,
            track_refs,
            ledger_entries,
        }],
    };
    let swarm = build_swarm_publish_batch(dj_handle, &swarm_entries, now_ms / 1000);
    Ok((delta, swarm))
}

pub fn spawn_background_omkf_folder_cloud_sync(
    dj_handle: String,
    crate_id: String,
    folder_path: std::path::PathBuf,
) {
    if cfg!(test)
        || std::env::var("HERA_DISABLE_CLOUD_SYNC")
            .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let Ok(bytes) = tokio::fs::read(folder_path.join(".hera/tracks.omkf")).await else {
            return;
        };
        let md = tokio::fs::read_to_string(folder_path.join(".hera/crate.md"))
            .await
            .unwrap_or_default();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        let rel = folder_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("crate")
            .to_string();
        if let Ok((delta, swarm)) = build_crate_delta_and_swarm_batch_from_capsule(
            &dj_handle, &crate_id, &rel, &bytes, &md, now_ms,
        ) {
            let _ = push_omkf_crate_deltas_async(&delta).await;
            if !swarm.items.is_empty() {
                let _ = publish_swarm_slots_async(&swarm).await;
            }
        }
    });
}
