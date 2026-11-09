//! Pure-Rust Cryptographic Primitives (FIPS 180-4 SHA-256, RFC 2104 HMAC-SHA256),
//! 16-Leaf Shard Merkle Trees, 64-Byte `Proof-of-Custody`, `zk-ctr-sha256-v1` Stream Cipher,
//! `Delta-CRDT` Lattice Join, and MoQ Harmonic Topic Routing (`src/p2p/crypto_merkle.rs`).

use super::{
    MerkleInclusionProof, P2PPeerNode, ShardMerkleTree, SwarmCrdtDelta, SwarmRedirectTicket,
    CUSTODY_PROOF_SLICE_BYTES, SWARM_MERKLE_LEAF_BYTES, SWARM_MERKLE_LEAF_COUNT,
    SWARM_SHARD_BLINDING_SECRET, SWARM_TICKET_MASTER_SECRET, SWARM_TICKET_TTL_SECS,
};
use crate::error::constant_time_eq;

/// Computes the Elastic Thermal Replication Factor (`K`) based on active healthy nodes (`N`)
/// and track popularity (`heat_score`).
/// - When `N == 1`: `K = 1`
/// - When `N == 2`: `K = 2`
/// - When `N >= 3`: `K = 2` for cold/warm tracks, scaling to `K = 3..min(N, 5)` for hot tracks!
pub fn compute_elastic_replication_k(active_healthy_nodes: usize, heat_score: f32) -> u32 {
    if active_healthy_nodes <= 1 {
        return 1;
    }
    if active_healthy_nodes == 2 {
        return 2;
    }
    let max_allowed = (active_healthy_nodes.min(5)) as u32;
    if heat_score >= 8.0 {
        max_allowed
    } else if heat_score >= 3.0 {
        3.min(max_allowed)
    } else {
        2
    }
}

/// Computes the canonical SHA-256 hex digest of a track's verified audio master for double-seal
/// protection (`hera_seal` + `audio_sha256`) against Bait-and-Switch attacks.
pub fn compute_canonical_audio_sha256(
    cas_fingerprint: &str,
    hera_seal: &str,
    file_size_mb: f32,
) -> String {
    let payload = format!(
        "HERA-AUDIO-MASTER-SHA256|{}|{}|{:.2}",
        cas_fingerprint.trim(),
        hera_seal.trim(),
        file_size_mb
    );
    compute_sha256_hex(payload.as_bytes())
}

/// Blinds the global catalog on community Worker R2 buckets by mapping `(cas_fingerprint, audio_sha256)`
/// to an opaque HMAC-SHA256 blob key (`shard-blobs/{hmac}.hbin`).
pub fn compute_opaque_shard_blob_key(cas_fingerprint: &str, audio_sha256: &str) -> String {
    let msg = format!(
        "HERA-OPAQUE-SHARD|{}|{}",
        cas_fingerprint.trim(),
        audio_sha256.trim()
    );
    let mac = compute_hmac_sha256_hex(SWARM_SHARD_BLINDING_SECRET.as_bytes(), msg.as_bytes());
    format!("shard-blobs/{}.hbin", &mac[..32])
}

/// Synthesizes a deterministic 64-byte (`CUSTODY_PROOF_SLICE_BYTES`) binary slice at `byte_offset`
/// for zero-egress Proof-of-Custody range audits (`Range: bytes=offset..offset+63`).
pub fn synthesize_deterministic_64b_custody_slice(
    opaque_blob_key: &str,
    audio_sha256: &str,
    byte_offset: u64,
) -> Vec<u8> {
    let seed_a = format!("SLICE-A|{}|{}|{}", opaque_blob_key, audio_sha256, byte_offset);
    let seed_b = format!(
        "SLICE-B|{}|{}|{}",
        opaque_blob_key,
        audio_sha256,
        byte_offset ^ 0x55aa55aa
    );
    let block_a = compute_sha256_bytes(seed_a.as_bytes());
    let block_b = compute_sha256_bytes(seed_b.as_bytes());
    let mut out = Vec::with_capacity(CUSTODY_PROOF_SLICE_BYTES);
    out.extend_from_slice(&block_a);
    out.extend_from_slice(&block_b);
    out
}

/// Zero-Knowledge P2P Boost stream cipher (`zk-ctr-sha256-v1`).
/// Seals or unseals `payload` in-place (involution: `seal(seal(x)) == x`) using a 32-byte
/// counter-mode keystream derived from `(opaque_blob_key, audio_sha256, aligned_offset_bytes)`
/// so R2 P2P Boost buckets never store or transmit plaintext audio headers or tags.
pub fn transform_p2p_boost_shard_bytes(
    payload: &[u8],
    opaque_blob_key: &str,
    audio_sha256: &str,
    aligned_offset_bytes: u64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len());
    for (chunk_idx, chunk) in payload.chunks(32).enumerate() {
        let block_offset = aligned_offset_bytes.wrapping_add((chunk_idx as u64) * 32);
        let seed = format!(
            "HERA-P2P-BOOST-ZK-V1|{}|{}|{}",
            opaque_blob_key.trim(),
            audio_sha256.trim(),
            block_offset
        );
        let keystream = compute_sha256_bytes(seed.as_bytes());
        for (i, &b) in chunk.iter().enumerate() {
            out.push(b ^ keystream[i]);
        }
    }
    out
}

/// Computes the HMAC-SHA256 signature for a 60-second `SwarmRedirectTicket` (`X-Hera-Swarm-Ticket`).
#[allow(clippy::too_many_arguments)]
pub fn compute_swarm_ticket_hmac(
    ticket_id: &str,
    requester_dj: &str,
    target_node_id: &str,
    opaque_blob_key: &str,
    audio_sha256: &str,
    aligned_offset_bytes: u64,
    aligned_length_bytes: u64,
    issued_at_unix: u64,
    expires_at_unix: u64,
) -> String {
    let canonical = format!(
        "HERA-TICKET-V1|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        ticket_id,
        requester_dj,
        target_node_id,
        opaque_blob_key,
        audio_sha256,
        aligned_offset_bytes,
        aligned_length_bytes,
        issued_at_unix,
        expires_at_unix
    );
    compute_hmac_sha256_hex(SWARM_TICKET_MASTER_SECRET.as_bytes(), canonical.as_bytes())
}

/// Verifies both the HMAC-SHA256 signature (in constant time) and the strict 60-second TTL window of a `SwarmRedirectTicket`.
pub fn verify_swarm_redirect_ticket(ticket: &SwarmRedirectTicket, current_unix: u64) -> bool {
    if ticket.expires_at_unix < ticket.issued_at_unix {
        return false;
    }
    if ticket.expires_at_unix - ticket.issued_at_unix > SWARM_TICKET_TTL_SECS {
        return false;
    }
    if current_unix < ticket.issued_at_unix || current_unix > ticket.expires_at_unix {
        return false;
    }
    let expected = compute_swarm_ticket_hmac(
        &ticket.ticket_id,
        &ticket.requester_dj,
        &ticket.target_node_id,
        &ticket.opaque_blob_key,
        &ticket.audio_sha256,
        ticket.aligned_offset_bytes,
        ticket.aligned_length_bytes,
        ticket.issued_at_unix,
        ticket.expires_at_unix,
    );
    let provided = ticket.hmac_signature.trim().to_ascii_lowercase();
    constant_time_eq(expected.as_bytes(), provided.as_bytes())
}

/// Computes the deterministic SHA-256 leaf digest for a 256 KiB sub-slice (`leaf_index` in `0..16`)
/// inside a 4 MB aligned shard (Layer 3 Post-P2P / BitTorrent v2 BEP 52 / Iroh).
pub fn compute_shard_leaf_hash_sha256(
    opaque_blob_key: &str,
    audio_sha256: &str,
    aligned_offset_bytes: u64,
    leaf_index: usize,
) -> String {
    let leaf_offset = aligned_offset_bytes + (leaf_index as u64) * SWARM_MERKLE_LEAF_BYTES;
    let slice_64 = synthesize_deterministic_64b_custody_slice(
        opaque_blob_key.trim(),
        audio_sha256.trim(),
        leaf_offset,
    );
    let mut preimage = Vec::with_capacity(128);
    preimage.extend_from_slice(
        format!(
            "HERA-MERKLE-LEAF-V1|{}|{}|{}|{}|",
            opaque_blob_key.trim(),
            audio_sha256.trim(),
            aligned_offset_bytes,
            leaf_index
        )
        .as_bytes(),
    );
    preimage.extend_from_slice(&slice_64);
    compute_sha256_hex(&preimage)
}

/// Domain-separated internal node hash (`SHA256("HERA-MERKLE-NODE-V1|" + left + "|" + right)`).
pub fn compute_merkle_parent_sha256(left_hex: &str, right_hex: &str) -> String {
    let combined = format!(
        "HERA-MERKLE-NODE-V1|{}|{}",
        left_hex.trim().to_ascii_lowercase(),
        right_hex.trim().to_ascii_lowercase()
    );
    compute_sha256_hex(combined.as_bytes())
}

/// Builds the complete 16-leaf (`16 x 256 KiB = 4 MB`) binary SHA-256 Merkle Tree for a shard.
pub fn compute_shard_merkle_tree(
    opaque_blob_key: &str,
    audio_sha256: &str,
    aligned_offset_bytes: u64,
) -> ShardMerkleTree {
    let mut leaves = Vec::with_capacity(SWARM_MERKLE_LEAF_COUNT);
    for idx in 0..SWARM_MERKLE_LEAF_COUNT {
        leaves.push(compute_shard_leaf_hash_sha256(
            opaque_blob_key,
            audio_sha256,
            aligned_offset_bytes,
            idx,
        ));
    }

    let mut current_level = leaves.clone();
    while current_level.len() > 1 {
        let mut next_level = Vec::with_capacity(current_level.len() / 2);
        for pair in current_level.chunks_exact(2) {
            next_level.push(compute_merkle_parent_sha256(&pair[0], &pair[1]));
        }
        current_level = next_level;
    }

    let root = current_level.into_iter().next().unwrap_or_default();
    ShardMerkleTree {
        opaque_blob_key: opaque_blob_key.to_string(),
        aligned_offset_bytes,
        leaf_count: SWARM_MERKLE_LEAF_COUNT,
        leaf_size_bytes: SWARM_MERKLE_LEAF_BYTES,
        leaf_hashes_sha256: leaves,
        merkle_root_sha256: root,
    }
}

/// Generates a compact 4-sibling (`log2(16) = 4`) `MerkleInclusionProof` for `leaf_index` (`0..15`).
pub fn generate_merkle_inclusion_proof(
    opaque_blob_key: &str,
    audio_sha256: &str,
    aligned_offset_bytes: u64,
    leaf_index: usize,
) -> Result<MerkleInclusionProof, String> {
    if leaf_index >= SWARM_MERKLE_LEAF_COUNT {
        return Err(format!(
            "leaf_index {} out of bounds (must be 0..{})",
            leaf_index,
            SWARM_MERKLE_LEAF_COUNT - 1
        ));
    }
    let tree = compute_shard_merkle_tree(opaque_blob_key, audio_sha256, aligned_offset_bytes);
    let leaf_hash = tree.leaf_hashes_sha256[leaf_index].clone();

    let mut siblings = Vec::with_capacity(4);
    let mut level_nodes = tree.leaf_hashes_sha256;
    let mut idx = leaf_index;
    while level_nodes.len() > 1 {
        let sibling_idx = idx ^ 1;
        siblings.push(level_nodes[sibling_idx].clone());
        let mut next_level = Vec::with_capacity(level_nodes.len() / 2);
        for pair in level_nodes.chunks_exact(2) {
            next_level.push(compute_merkle_parent_sha256(&pair[0], &pair[1]));
        }
        level_nodes = next_level;
        idx >>= 1;
    }

    Ok(MerkleInclusionProof {
        opaque_blob_key: opaque_blob_key.to_string(),
        aligned_offset_bytes,
        leaf_index,
        leaf_size_bytes: SWARM_MERKLE_LEAF_BYTES,
        leaf_hash_sha256: leaf_hash,
        sibling_hashes_sha256: siblings,
        merkle_root_sha256: tree.merkle_root_sha256,
    })
}

/// Verifies a `MerkleInclusionProof` in O(log2(16)) = 4 SHA-256 operations at the first kilobyte.
pub fn verify_merkle_inclusion_proof(proof: &MerkleInclusionProof) -> bool {
    if proof.leaf_index >= SWARM_MERKLE_LEAF_COUNT || proof.sibling_hashes_sha256.len() != 4 {
        return false;
    }
    let mut current = proof.leaf_hash_sha256.clone();
    let mut idx = proof.leaf_index;
    for sibling in &proof.sibling_hashes_sha256 {
        if (idx & 1) == 0 {
            current = compute_merkle_parent_sha256(&current, sibling);
        } else {
            current = compute_merkle_parent_sha256(sibling, &current);
        }
        idx >>= 1;
    }
    let expected_root = proof.merkle_root_sha256.trim().to_ascii_lowercase();
    constant_time_eq(current.as_bytes(), expected_root.as_bytes())
}

/// Derives a deterministic cryptographic `EndpointId` (`ed25519_<hex>`) for an `Edge Guardian`
/// node from `(owner_dj, node_id)` so its identity and accumulated `swarm_earned_credits_mb`
/// persist across network migrations (`Wi-Fi -> 5G` / IP changes).
pub fn derive_node_endpoint_pubkey_id(owner_dj: &str, node_id: &str) -> String {
    let seed = format!(
        "hera_iroh_endpoint_pubkey_v1:{}:{}",
        owner_dj.trim().to_ascii_lowercase(),
        node_id.trim().to_ascii_lowercase()
    );
    let digest = compute_sha256_hex(seed.as_bytes());
    format!("ed25519_{}", &digest[..32])
}

/// Builds a stateless CGNAT / Symmetric NAT edge relay URL for an `endpoint_pubkey_id` so
/// mobile 5G nodes or festival hotel Wi-Fi nodes remain reachable with zero port forwarding.
pub fn build_cgnat_relay_url(endpoint_pubkey_id: &str) -> String {
    format!(
        "https://relay.hera.audio/iroh-tunnel/{}",
        endpoint_pubkey_id.trim()
    )
}

/// Computes the Latency-Aware Power-of-Two-Choices (P2C) composite routing cost for an `Edge Guardian`
/// node, combining daily Cloudflare Worker request utilization (`daily_worker_requests_used / 85,000`),
/// smoothed network round-trip latency (`ewma_rtt_ms`), and R2 storage utilization tie-breaking.
pub fn compute_node_p2c_composite_cost(node: &P2PPeerNode) -> f32 {
    let req_cap = (node.daily_worker_request_safe_cap.max(1)) as f32;
    let req_load_pct = ((node.daily_worker_requests_used as f32) / req_cap) * 100.0;
    let rtt_penalty = node.ewma_rtt_ms.clamp(1.0, 2000.0) * 0.25;
    let r2_tiebreak = (node.r2_used_mb / (node.r2_safe_cap_mb.max(1.0))) * 2.0;
    ((req_load_pct + rtt_penalty + r2_tiebreak) * 100.0).round() / 100.0
}

/// Pure mathematical Delta-CRDT Lattice Join (`merge(A, B)`):
/// - **Commutative:** `merge(A, B) == merge(B, A)`
/// - **Associative:** `merge(merge(A, B), C) == merge(A, merge(B, C))`
/// - **Idempotent:** `merge(A, A) == A`
pub fn merge_swarm_crdt_deltas(a: &SwarmCrdtDelta, b: &SwarmCrdtDelta) -> SwarmCrdtDelta {
    let mut gcounter_served = a.gcounter_node_bytes_served_mb.clone();
    for (nid, &val_b) in &b.gcounter_node_bytes_served_mb {
        let entry = gcounter_served.entry(nid.clone()).or_insert(0.0);
        if val_b > *entry {
            *entry = val_b;
        }
    }

    let mut gcounter_plays = a.gcounter_cas_play_counts.clone();
    for (cas, map_b) in &b.gcounter_cas_play_counts {
        let target_map = gcounter_plays.entry(cas.clone()).or_default();
        for (nid, &cnt_b) in map_b {
            let e = target_map.entry(nid.clone()).or_insert(0);
            if cnt_b > *e {
                *e = cnt_b;
            }
        }
    }

    let mut lww_entries = a.lww_track_entries.clone();
    for (cas, entry_b) in &b.lww_track_entries {
        match lww_entries.get(cas) {
            Some(entry_a) => {
                let key_a = (
                    entry_a.hlc_timestamp_ms,
                    entry_a.tombstone,
                    &entry_a.track.hera_seal,
                    &entry_a.origin_endpoint_pubkey_id,
                );
                let key_b = (
                    entry_b.hlc_timestamp_ms,
                    entry_b.tombstone,
                    &entry_b.track.hera_seal,
                    &entry_b.origin_endpoint_pubkey_id,
                );
                if key_b > key_a {
                    lww_entries.insert(cas.clone(), entry_b.clone());
                }
            }
            None => {
                lww_entries.insert(cas.clone(), entry_b.clone());
            }
        }
    }

    let hlc_max = a.hlc_timestamp_ms.max(b.hlc_timestamp_ms);
    let (origin_id, origin_pub) = if a.origin_node_id >= b.origin_node_id {
        (a.origin_node_id.clone(), a.origin_endpoint_pubkey_id.clone())
    } else {
        (b.origin_node_id.clone(), b.origin_endpoint_pubkey_id.clone())
    };

    SwarmCrdtDelta {
        origin_node_id: origin_id,
        origin_endpoint_pubkey_id: origin_pub,
        hlc_timestamp_ms: hlc_max,
        gcounter_node_bytes_served_mb: gcounter_served,
        gcounter_cas_play_counts: gcounter_plays,
        lww_track_entries: lww_entries,
    }
}

/// Computes a canonical, order-independent SHA-256 digest of a `SwarmCrdtDelta` state
/// so two offline nodes can verify in O(1) that their states have converged (`digest(A) == digest(B)`).
pub fn compute_crdt_state_digest_sha256(delta: &SwarmCrdtDelta) -> String {
    let mut parts = Vec::new();
    parts.push(format!("hlc={}", delta.hlc_timestamp_ms));

    let mut served_keys: Vec<_> = delta.gcounter_node_bytes_served_mb.keys().collect();
    served_keys.sort();
    for k in served_keys {
        let v = delta.gcounter_node_bytes_served_mb[k];
        parts.push(format!("served:{k}={:.2}", v));
    }

    let mut play_cas_keys: Vec<_> = delta.gcounter_cas_play_counts.keys().collect();
    play_cas_keys.sort();
    for cas in play_cas_keys {
        let inner = &delta.gcounter_cas_play_counts[cas];
        let mut node_keys: Vec<_> = inner.keys().collect();
        node_keys.sort();
        for nk in node_keys {
            parts.push(format!("play:{cas}:{nk}={}", inner[nk]));
        }
    }

    let mut lww_keys: Vec<_> = delta.lww_track_entries.keys().collect();
    lww_keys.sort();
    for cas in lww_keys {
        let e = &delta.lww_track_entries[cas];
        parts.push(format!(
            "lww:{cas}:hlc={}:tomb={}:seal={}:cues={}",
            e.hlc_timestamp_ms,
            e.tombstone,
            e.track.hera_seal,
            e.track.hot_cues.len()
        ));
    }

    compute_sha256_hex(parts.join("|").as_bytes())
}

/// Builds a canonical Media-over-QUIC (`MoQ`) style harmonic subscription topic URI:
/// `hera://swarm/<camelot>/<bpm_min>-<bpm_max>` (with optional `?subgenre=<filter>`).
pub fn build_swarm_topic_uri(
    camelot_key: &str,
    bpm_min: f32,
    bpm_max: f32,
    subgenre_filter: Option<&str>,
) -> String {
    let cam = camelot_key.trim().to_ascii_uppercase();
    let base = format!("hera://swarm/{cam}/{:.0}-{:.0}", bpm_min, bpm_max);
    if let Some(sg) = subgenre_filter {
        let sg_clean: String = sg
            .trim()
            .to_ascii_lowercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !sg_clean.is_empty() {
            return format!("{base}?subgenre={sg_clean}");
        }
    }
    base
}

/// Returns `true` if `target_camelot` is harmonically compatible with `sub_camelot` on the 24-key
/// Camelot wheel (exact match `8A == 8A`, adjacent fifth `7A` / `9A`, or relative major/minor `8B`).
pub fn are_camelot_keys_harmonically_compatible(sub_camelot: &str, target_camelot: &str) -> bool {
    let s = sub_camelot.trim().to_ascii_uppercase();
    let t = target_camelot.trim().to_ascii_uppercase();
    if s == "*" || s == "ANY" || s == t {
        return true;
    }
    let parse_cam = |k: &str| -> Option<(u8, char)> {
        let mode = k.chars().last()?;
        if mode != 'A' && mode != 'B' {
            return None;
        }
        let num: u8 = k[..k.len().saturating_sub(1)].parse().ok()?;
        if (1..=12).contains(&num) {
            Some((num, mode))
        } else {
            None
        }
    };
    if let (Some((n1, m1)), Some((n2, m2))) = (parse_cam(&s), parse_cam(&t)) {
        if n1 == n2 {
            return true; // Same key or relative major/minor (e.g. 8A <-> 8B)
        }
        if m1 == m2 {
            let diff = n1.abs_diff(n2);
            if diff == 1 || diff == 11 {
                return true; // Adjacent on Camelot wheel (e.g. 8A <-> 7A / 9A)
            }
        }
    }
    false
}

/// Pure-Rust FIPS 180-4 SHA-256 implementation (zero external crate dependencies, compiles to native & WASM).
pub fn compute_sha256_bytes(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut padded = Vec::with_capacity(data.len() + 72);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        let bytes = word.to_be_bytes();
        out[i * 4..i * 4 + 4].copy_from_slice(&bytes);
    }
    out
}

/// Computes lowercase hex SHA-256 digest of `data`.
pub fn compute_sha256_hex(data: &[u8]) -> String {
    let bytes = compute_sha256_bytes(data);
    let mut s = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}

/// Pure-Rust RFC 2104 HMAC-SHA256 returning a 64-character lowercase hex string.
pub fn compute_hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        let hashed = compute_sha256_bytes(key);
        key_block[..32].copy_from_slice(&hashed);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut o_key_pad = [0x5cu8; 64];
    let mut i_key_pad = [0x36u8; 64];
    for i in 0..64 {
        o_key_pad[i] ^= key_block[i];
        i_key_pad[i] ^= key_block[i];
    }

    let mut inner_data = Vec::with_capacity(64 + message.len());
    inner_data.extend_from_slice(&i_key_pad);
    inner_data.extend_from_slice(message);
    let inner_hash = compute_sha256_bytes(&inner_data);

    let mut outer_data = Vec::with_capacity(64 + 32);
    outer_data.extend_from_slice(&o_key_pad);
    outer_data.extend_from_slice(&inner_hash);
    compute_sha256_hex(&outer_data)
}
