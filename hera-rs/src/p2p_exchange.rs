//! Federated R2 Edge Swarm & `Edge Guardian` Exchange Protocol (`src/p2p_exchange.rs`).
//!
//! Core Motto: *"Mientras más nodos de Hera, más fuerte es el sistema de enjambres."*
//!
//! This module provides the public entrypoints and types for the Federated R2 Edge Swarm:
//! - **Constrained Water-Filling (`compute_water_filling_allocations`)**: Allocates `4 MB` immutable
//!   shards across healthy community Cloudflare Workers while strictly enforcing `9,000 MB` R2
//!   (`CF_WORKER_SAFE_R2_CAP_MB`) and `85,000 req/day` (`CF_WORKER_SAFE_REQ_CAP`) safety ceilings.
//! - **Elastic Thermal Replication $K$ (`compute_elastic_replication_factor`)**: Scales shard
//!   replicas dynamically with active node count $N$ and track thermal popularity (`heat_score`).
//! - **Blind Catalog Sharding (`derive_blind_shard_key`)**: Derives opaque HMAC-SHA256 R2 keys
//!   (`shard-blobs/{hmac}.hbin`) and single-use `60s` `SwarmRedirectTicket` tokens (`X-Hera-Swarm-Ticket`).
//! - **64-Byte `Proof-of-Custody` (`issue_custody_challenge`)**: Zero-egress random range slice
//!   audits (`Range: bytes=offset..offset+63`) and double-sealed (`hera_seal` + `audio_sha256`)
//!   quarantine against Bait-and-Switch attacks.

pub use crate::p2p::{
    are_camelot_keys_harmonically_compatible, build_cgnat_relay_url, build_swarm_topic_uri,
    compute_canonical_audio_sha256, compute_crdt_state_digest_sha256,
    compute_elastic_replication_k, compute_hmac_sha256_hex, compute_merkle_parent_sha256,
    compute_node_p2c_composite_cost, compute_opaque_shard_blob_key, compute_sha256_bytes,
    compute_sha256_hex, compute_shard_leaf_hash_sha256, compute_shard_merkle_tree,
    compute_swarm_ticket_hmac, derive_node_endpoint_pubkey_id, generate_merkle_inclusion_proof,
    merge_swarm_crdt_deltas, synthesize_deterministic_64b_custody_slice,
    transform_p2p_boost_shard_bytes, verify_merkle_inclusion_proof, verify_swarm_redirect_ticket,
    CrdtLwwTrackEntry, CustodyAuditResult, CustodyAuditSubmission, CustodyChallenge,
    CustodyChallengeRequest, MerkleInclusionProof, OutboundPullTicket, P2PMeshState, P2PPeerNode,
    ProofOfDeliveryReceipt, QuarantinedNodeRecord, ShardMerkleTree, SwarmCrdtDelta,
    SwarmCrdtSyncResponse, SwarmHeartbeatRequest, SwarmHeartbeatResponse,
    SwarmParallelBlockAssignment, SwarmParallelManifestRequest, SwarmParallelManifestResponse,
    SwarmRedirectTicket, SwarmRouteRequest, SwarmRouteResponse, SwarmSemanticPubSubEvent,
    SwarmStrengthReport, SwarmSubscribeRequest, SwarmSubscribeResponse, SwarmTopicSubscription,
    WaterFillingNodeAllocation, CF_WORKER_FREE_R2_LIMIT_MB, CF_WORKER_FREE_REQ_LIMIT,
    CF_WORKER_SAFE_R2_CAP_MB, CF_WORKER_SAFE_REQ_CAP, CUSTODY_PROOF_SLICE_BYTES,
    SWARM_ALIGNED_BLOCK_BYTES, SWARM_CREDIT_MULTIPLIER, SWARM_MERKLE_LEAF_BYTES,
    SWARM_MERKLE_LEAF_COUNT, SWARM_MOTTO, SWARM_TICKET_TTL_SECS,
};

/// Canonical alias for the `1:1.5` (`1.5x`) Meritocratic Turbo Credit multiplier.
pub const SWARM_TURBO_CREDIT_MULTIPLIER: f32 = SWARM_CREDIT_MULTIPLIER;

/// Computes the Constrained Water-Filling shard allocations across all active, healthy
/// `Edge Guardian` nodes while respecting `CF_WORKER_SAFE_R2_CAP_MB` (`9,000 MB`) and
/// `CF_WORKER_SAFE_REQ_CAP` (`85,000 req/day`).
pub fn compute_water_filling_allocations(state: &P2PMeshState) -> Vec<WaterFillingNodeAllocation> {
    state.compute_swarm_strength().water_filling_allocations
}

/// Computes the elastic thermal replication factor $K \in [1, \min(N, 5)]$ as a function
/// of active healthy swarm nodes (`active_healthy_nodes`) and track popularity (`heat_score`).
pub fn compute_elastic_replication_factor(active_healthy_nodes: usize, heat_score: f32) -> u32 {
    compute_elastic_replication_k(active_healthy_nodes, heat_score)
}

/// Derives an opaque, privacy-preserving R2 object key (`shard-blobs/{hmac}.hbin`) from
/// `(cas_fingerprint, audio_sha256)` so community Worker buckets cannot be scraped or enumerated.
pub fn derive_blind_shard_key(cas_fingerprint: &str, audio_sha256: &str) -> String {
    compute_opaque_shard_blob_key(cas_fingerprint, audio_sha256)
}

/// Issues a deterministic 64-byte (`CUSTODY_PROOF_SLICE_BYTES`) `Range: bytes=offset..offset+63`
/// `Proof-of-Custody` challenge against a registered `Edge Guardian` node.
pub fn issue_custody_challenge(
    state: &P2PMeshState,
    req: CustodyChallengeRequest,
) -> Result<CustodyChallenge, String> {
    state.generate_custody_challenge(req)
}
