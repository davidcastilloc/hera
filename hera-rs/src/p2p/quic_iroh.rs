use super::compute_sha256_hex;
use serde::{Deserialize, Serialize};

/// Canonical ALPN negotiated by HERA Swarm nodes over `iroh` QUIC streams.
pub const HERA_IROH_SHARD_ALPN: &[u8] = b"hera-swarm-shard/1";

/// Maximum shard size (`4 MiB` + header margin) permitted over a single QUIC bidirectional stream.
pub const MAX_QUIC_SHARD_STREAM_BYTES: usize = 4 * 1024 * 1024 + 4096;

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

/// Hexagonal port for verified P2P shard exchange over QUIC (`iroh`).
pub trait SwarmQuicTransport {
    fn transport_protocol_name(&self) -> &'static str;
    fn verify_shard_digest(&self, payload: &[u8], expected_sha256: &str) -> Result<String, String> {
        let actual = compute_sha256_hex(payload);
        if !actual.eq_ignore_ascii_case(expected_sha256.trim()) {
            return Err(format!(
                "QUIC shard SHA-256 mismatch: expected {}, got {}",
                expected_sha256, actual
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
}

impl IrohQuicAdapter {
    pub fn new(node_id: impl Into<String>, bind_loopback_only: bool) -> Self {
        Self {
            node_id: node_id.into(),
            bind_loopback_only,
        }
    }
}

impl SwarmQuicTransport for IrohQuicAdapter {
    fn transport_protocol_name(&self) -> &'static str {
        "iroh-quic-v1"
    }
}

#[cfg(feature = "native")]
impl IrohQuicAdapter {
    /// Binds a local `iroh::Endpoint` configured with `RelayMode::Disabled` and
    /// `PortmapperConfig::Disabled` so local LAN/loopback transfers never block on external relays.
    pub async fn bind_local_endpoint(&self, accept_shards: bool) -> Result<iroh::Endpoint, String> {
        use iroh::{
            endpoint::{presets, BindOpts, PortmapperConfig},
            Endpoint, RelayMode,
        };

        let mut builder = Endpoint::builder(presets::Minimal)
            .relay_mode(RelayMode::Disabled)
            .portmapper_config(PortmapperConfig::Disabled)
            .clear_address_lookup();

        if self.bind_loopback_only {
            builder = builder
                .clear_ip_transports()
                .bind_addr_with_opts("127.0.0.1:0", BindOpts::default())
                .map_err(|e| format!("iroh bind_addr 127.0.0.1:0 failed: {e}"))?;
        }

        if accept_shards {
            builder = builder.alpns(vec![HERA_IROH_SHARD_ALPN.to_vec()]);
        }

        builder
            .bind()
            .await
            .map_err(|e| format!("iroh Endpoint::bind failed: {e}"))
    }

    /// Exchanges a verified shard between a provider `iroh::Endpoint` and a leecher `iroh::Endpoint`
    /// over a real QUIC connection (`HERA_IROH_SHARD_ALPN`) using `open_bi` / `accept_bi`.
    pub async fn transfer_verified_shard_loopback(
        &self,
        shard_id: &str,
        shard_bytes: &[u8],
    ) -> Result<QuicShardTransferReceipt, String> {
        let expected_sha256 = compute_sha256_hex(shard_bytes);
        let provider_ep = self.bind_local_endpoint(true).await?;
        let leecher_ep = self.bind_local_endpoint(false).await?;

        let provider_id = provider_ep.id().to_string();
        let leecher_id = leecher_ep.id().to_string();

        let provider_sock = provider_ep
            .bound_sockets()
            .into_iter()
            .find(|a| a.is_ipv4())
            .ok_or_else(|| "Provider iroh endpoint has no IPv4 bound socket".to_string())?;
        let provider_addr =
            iroh::EndpointAddr::new(provider_ep.id()).with_ip_addr(provider_sock);

        let shard_owned = shard_bytes.to_vec();
        let expected_id_owned = shard_id.to_string();
        let provider_ep_task = provider_ep.clone();

        let server_task = tokio::spawn(async move {
            let incoming = provider_ep_task
                .accept()
                .await
                .ok_or_else(|| "Provider endpoint closed before incoming QUIC conn".to_string())?;
            let conn = incoming
                .await
                .map_err(|e| format!("Provider QUIC handshake failed: {e}"))?;
            let (mut send_stream, mut recv_stream) = conn
                .accept_bi()
                .await
                .map_err(|e| format!("Provider accept_bi failed: {e}"))?;

            let req_bytes = recv_stream
                .read_to_end(4096)
                .await
                .map_err(|e| format!("Provider read_to_end failed: {e}"))?;
            let req: QuicShardRequest = serde_json::from_slice(&req_bytes)
                .map_err(|e| format!("Invalid QuicShardRequest JSON: {e}"))?;
            if req.shard_id != expected_id_owned {
                return Err(format!(
                    "Unexpected shard_id requested: {} != {}",
                    req.shard_id, expected_id_owned
                ));
            }

            send_stream
                .write_all(&shard_owned)
                .await
                .map_err(|e| format!("Provider write_all failed: {e}"))?;
            send_stream
                .finish()
                .map_err(|e| format!("Provider stream finish failed: {e}"))?;
            conn.closed().await;
            Ok::<(), String>(())
        });

        let conn = leecher_ep
            .connect(provider_addr, HERA_IROH_SHARD_ALPN)
            .await
            .map_err(|e| format!("Leecher QUIC connect failed: {e}"))?;
        let (mut send_stream, mut recv_stream) = conn
            .open_bi()
            .await
            .map_err(|e| format!("Leecher open_bi failed: {e}"))?;

        let req = QuicShardRequest {
            shard_id: shard_id.to_string(),
            expected_sha256: expected_sha256.clone(),
            requesting_node_id: self.node_id.clone(),
        };
        let req_json =
            serde_json::to_vec(&req).map_err(|e| format!("Serialize request failed: {e}"))?;
        send_stream
            .write_all(&req_json)
            .await
            .map_err(|e| format!("Leecher write request failed: {e}"))?;
        send_stream
            .finish()
            .map_err(|e| format!("Leecher finish request stream failed: {e}"))?;

        let received = recv_stream
            .read_to_end(MAX_QUIC_SHARD_STREAM_BYTES)
            .await
            .map_err(|e| format!("Leecher read shard stream failed: {e}"))?;

        conn.close(0u8.into(), b"shard-verified");
        let srv_res = server_task
            .await
            .map_err(|e| format!("Provider task join error: {e}"))?;
        srv_res?;

        provider_ep.close().await;
        leecher_ep.close().await;

        let verified_sha256 = self.verify_shard_digest(&received, &expected_sha256)?;

        Ok(QuicShardTransferReceipt {
            shard_id: shard_id.to_string(),
            provider_endpoint_id: provider_id,
            leecher_endpoint_id: leecher_id,
            bytes_transferred: received.len(),
            verified_sha256,
            alpn_protocol: String::from_utf8_lossy(HERA_IROH_SHARD_ALPN).into_owned(),
            payload: received,
        })
    }
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_iroh_quic_loopback_verified_shard_transfer() {
        let adapter = IrohQuicAdapter::new("hera-quic-test-node", true);
        assert_eq!(adapter.transport_protocol_name(), "iroh-quic-v1");

        // Synthesize a deterministic 16 KB audio shard payload
        let payload: Vec<u8> = (0..16_384)
            .map(|i| ((i * 73 + 19) & 0xFF) as u8)
            .collect();
        let expected_digest = compute_sha256_hex(&payload);

        let receipt = adapter
            .transfer_verified_shard_loopback("shard-blobs/test_01.hbin", &payload)
            .await
            .expect("QUIC loopback shard transfer should succeed");

        assert_eq!(receipt.shard_id, "shard-blobs/test_01.hbin");
        assert_eq!(receipt.bytes_transferred, payload.len());
        assert_eq!(receipt.verified_sha256, expected_digest);
        assert_eq!(receipt.alpn_protocol, "hera-swarm-shard/1");
        assert_eq!(receipt.payload, payload);
        assert_ne!(receipt.provider_endpoint_id, receipt.leecher_endpoint_id);
    }
}
