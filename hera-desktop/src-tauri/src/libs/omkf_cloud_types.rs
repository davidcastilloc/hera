//! Wire DTOs for OMKF v1 Swarm KV & DjAccountVaultDO cloud sync (`Milestone C4`).

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::super::omkf_cloud_client::*;
    use hera_node_rs::scanner::omkf_capsule::{
        OMKF_FLAG_IS_FAKE_320KBPS, OMKF_FLAG_IS_OWN_PRODUCTION, OMKF_SLOT_BYTES, OmkfHeader,
        OmkfTrackEntry, OmkfTrackSlot, encode_omkf_capsule,
    };

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

    #[test]
    fn test_build_crate_delta_and_swarm_batch_from_capsule_isolates_private_bounce_and_parses_ledger()
     {
        let pub_slot = OmkfTrackSlot {
            partial_hash_64kb: [0x11; 16],
            file_size_bytes: 8_000_000,
            scan_level: 2,
            bpm_x100: 12500,
            camelot_code: 8,
            hot_cues_ms: [1000, 32000, 64000, 96000, 0, 0, 0, 0],
            ..Default::default()
        };
        let own_bounce = OmkfTrackSlot {
            partial_hash_64kb: [0x22; 16],
            file_size_bytes: 12_000_000,
            scan_level: 2,
            bpm_x100: 12600,
            flags_bits: OMKF_FLAG_IS_OWN_PRODUCTION,
            ..Default::default()
        };
        let hdr = OmkfHeader {
            tree_seal_64: 0xCAFE_BABE,
            track_count: 2,
            ..Default::default()
        };
        let entries = vec![
            OmkfTrackEntry {
                slot: pub_slot,
                filename: "club.flac".into(),
                artist: "A".into(),
                title: "B".into(),
            },
            OmkfTrackEntry {
                slot: own_bounce,
                filename: "my_edit.wav".into(),
                artist: "Me".into(),
                title: "VIP".into(),
            },
        ];
        let bytes = encode_omkf_capsule(&hdr, &entries);
        let md = "---\nomkf: 1\n---\n# Intención y Reglas de Cabina\nSolo afro house.\n\n# Ledger Reversible\n- tx:org_77 | 2026-11-16T12:00Z | 2 moves\n";
        let (delta_req, swarm_req) = build_crate_delta_and_swarm_batch_from_capsule(
            "DaviDJ",
            "crate-01",
            "Afro House",
            &bytes,
            md,
            1_700_000_000_000,
        )
        .unwrap();
        assert_eq!(
            (
                delta_req.crates.len(),
                delta_req.crates[0].track_refs.len(),
                delta_req.crates[0].ledger_entries.len()
            ),
            (1, 2, 1)
        );
        assert_eq!(delta_req.crates[0].ledger_entries[0].tx_id, "org_77");
        assert_eq!(
            (
                swarm_req.items.len(),
                swarm_req.items[0].partial_hash_64kb.as_str()
            ),
            (1, &"11".repeat(16)[..])
        );
    }
}
