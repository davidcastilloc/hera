use super::compute_sha256_hex;
use serde::{Deserialize, Serialize};

/// Canonical ALPN negotiated by HERA Swarm nodes over `iroh` QUIC streams.
pub const HERA_IROH_SHARD_ALPN: &[u8] = b"hera-swarm-shard/1";

/// Maximum shard size (`4 MiB` + header margin) permitted over a single QUIC bidirectional stream.
pub const MAX_QUIC_SHARD_STREAM_BYTES: usize = 4 * 1024 * 1024 + 4096;

/// Timeout applied to individual QUIC handshake, accept, open_bi, write, and read steps.
pub const QUIC_STEP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Wire request sent by a leecher over a bidirectional QUIC stream (`open_bi`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuicShardRequest {
    pub shard_id: String,
    pub expected_sha256: String,
    pub requesting_node_id: String,
}

/// Receipt returned after a verified QUIC shard exchange between two Swarm endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuicShardTransferReceipt {
    pub shard_id: String,
    pub provider_endpoint_id: String,
    pub leecher_endpoint_id: String,
    pub bytes_transferred: usize,
    pub verified_sha256: String,
    pub alpn_protocol: String,
    pub payload: Vec<u8>,
}

/// Canonical alias matching both receipt naming styles.
pub type QuicShardReceipt = QuicShardTransferReceipt;

/// Hexagonal port for verified P2P shard exchange over QUIC (`iroh`).
pub trait SwarmQuicTransport {
    fn transport_protocol_name(&self) -> &'static str;
    fn verify_shard_digest(&self, payload: &[u8], expected_digest: &str) -> Result<String, String> {
        let actual = compute_sha256_hex(payload);
        if !actual.eq_ignore_ascii_case(expected_digest.trim()) {
            return Err(format!(
                "QUIC shard digest mismatch: expected {}, got {}",
                expected_digest, actual
            ));
        }
        Ok(actual)
    }
}

/// Adapter for `iroh` QUIC Endpoint P2P Swarm shard transport (`HERA_IROH_SHARD_ALPN`).
#[derive(Debug, Clone)]
pub struct IrohQuicAdapter {
    pub node_id: String,
    pub bind_loopback_only: bool,
    #[cfg(feature = "native")]
    pub endpoint: std::sync::Arc<tokio::sync::OnceCell<iroh::Endpoint>>,
}

impl IrohQuicAdapter {
    pub fn new(node_id: impl Into<String>, bind_loopback_only: bool) -> Self {
        Self {
            node_id: node_id.into(),
            bind_loopback_only,
            #[cfg(feature = "native")]
            endpoint: std::sync::Arc::new(tokio::sync::OnceCell::new()),
        }
    }
}

impl SwarmQuicTransport for IrohQuicAdapter {
    fn transport_protocol_name(&self) -> &'static str {
        "iroh-quic-v1"
    }
}

#[cfg(feature = "native")]
mod peer_transfer;

