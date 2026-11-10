use super::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::time::timeout;

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

    /// Obtains or initializes the cached `iroh::Endpoint` for this adapter instance.
    pub async fn get_or_init_endpoint(&self, accept_shards: bool) -> Result<&iroh::Endpoint, String> {
        self.endpoint
            .get_or_try_init(|| async { self.bind_local_endpoint(accept_shards).await })
            .await
    }

    /// Encodes an `iroh::EndpointAddr` into a portable `"iroh://<json>"` URI ticket.
    pub fn encode_endpoint_ticket(addr: &iroh::EndpointAddr) -> Result<String, String> {
        let json = serde_json::to_string(addr).map_err(|e| format!("Failed to serialize EndpointAddr: {e}"))?;
        Ok(format!("iroh://{json}"))
    }

    /// Decodes an `iroh::EndpointAddr` from an `"iroh://<json>"` URI or direct JSON string.
    pub fn decode_endpoint_ticket(ticket: &str) -> Result<iroh::EndpointAddr, String> {
        let trimmed = ticket.trim();
        let json_str = trimmed.strip_prefix("iroh://").unwrap_or(trimmed);
        serde_json::from_str(json_str).map_err(|e| format!("Failed to parse EndpointAddr ticket: {e}"))
    }

    /// Spawns a background shard provider task serving shards from `store` over `HERA_IROH_SHARD_ALPN`.
    pub async fn spawn_shard_provider(
        &self,
        store: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    ) -> Result<iroh::EndpointAddr, String> {
        let endpoint = self.get_or_init_endpoint(true).await?.clone();
        let sock = endpoint
            .bound_sockets()
            .into_iter()
            .find(|a| a.is_ipv4())
            .ok_or_else(|| "Provider iroh endpoint has no IPv4 bound socket".to_string())?;
        let addr = iroh::EndpointAddr::new(endpoint.id()).with_ip_addr(sock);

        let ep_clone = endpoint.clone();
        tokio::spawn(async move {
            while let Ok(Some(incoming)) = timeout(QUIC_STEP_TIMEOUT, ep_clone.accept()).await {
                let conn = match timeout(QUIC_STEP_TIMEOUT, incoming).await {
                    Ok(Ok(c)) => c,
                    _ => continue,
                };
                let store_clone = Arc::clone(&store);
                tokio::spawn(async move {
                    let (mut send_stream, mut recv_stream) = match timeout(QUIC_STEP_TIMEOUT, conn.accept_bi()).await {
                        Ok(Ok(streams)) => streams,
                        _ => return,
                    };
                    let req_bytes = match timeout(QUIC_STEP_TIMEOUT, recv_stream.read_to_end(4096)).await {
                        Ok(Ok(b)) => b,
                        _ => return,
                    };
                    let req: QuicShardRequest = match serde_json::from_slice(&req_bytes) {
                        Ok(r) => r,
                        Err(_) => return,
                    };
                    let payload_opt = {
                        let map = match store_clone.read() {
                            Ok(m) => m,
                            Err(_) => return,
                        };
                        map.get(&req.shard_id).cloned()
                    };
                    if let Some(payload) = payload_opt {
                        if timeout(QUIC_STEP_TIMEOUT, send_stream.write_all(&payload)).await.is_ok()
                            && send_stream.finish().is_ok()
                        {
                            let _ = timeout(QUIC_STEP_TIMEOUT, send_stream.stopped()).await;
                        }
                    }
                });
            }
        });

        Ok(addr)
    }

    /// Fetches and verifies a shard from a remote `iroh::EndpointAddr`.
    pub async fn fetch_verified_shard(
        &self,
        remote_addr: iroh::EndpointAddr,
        shard_id: &str,
        expected_blake3_or_sha256: Option<&str>,
    ) -> Result<QuicShardTransferReceipt, String> {
        let ep = self.get_or_init_endpoint(false).await?.clone();
        let conn = timeout(QUIC_STEP_TIMEOUT, ep.connect(remote_addr.clone(), HERA_IROH_SHARD_ALPN))
            .await
            .map_err(|_| "QUIC connect timed out after 5s".to_string())?
            .map_err(|e| format!("QUIC connect failed: {e}"))?;

        let (mut send_stream, mut recv_stream) = timeout(QUIC_STEP_TIMEOUT, conn.open_bi())
            .await
            .map_err(|_| "QUIC open_bi timed out after 5s".to_string())?
            .map_err(|e| format!("QUIC open_bi failed: {e}"))?;

        let req = QuicShardRequest {
            shard_id: shard_id.to_string(),
            expected_sha256: expected_blake3_or_sha256.unwrap_or("").to_string(),
            requesting_node_id: self.node_id.clone(),
        };
        let req_json = serde_json::to_vec(&req).map_err(|e| format!("Failed to serialize request: {e}"))?;

        timeout(QUIC_STEP_TIMEOUT, send_stream.write_all(&req_json))
            .await
            .map_err(|_| "QUIC write_all timed out after 5s".to_string())?
            .map_err(|e| format!("QUIC write_all failed: {e}"))?;

        send_stream
            .finish()
            .map_err(|e| format!("QUIC finish failed: {e}"))?;

        let received = timeout(QUIC_STEP_TIMEOUT, recv_stream.read_to_end(MAX_QUIC_SHARD_STREAM_BYTES))
            .await
            .map_err(|_| "QUIC read_to_end timed out after 5s".to_string())?
            .map_err(|e| format!("QUIC read_to_end failed: {e}"))?;

        conn.close(0u8.into(), b"shard-verified");

        let verified_sha256 = if let Some(expected) = expected_blake3_or_sha256 {
            self.verify_shard_digest(&received, expected)?
        } else {
            compute_sha256_hex(&received)
        };

        Ok(QuicShardTransferReceipt {
            shard_id: shard_id.to_string(),
            provider_endpoint_id: remote_addr.id.to_string(),
            leecher_endpoint_id: ep.id().to_string(),
            bytes_transferred: received.len(),
            verified_sha256,
            alpn_protocol: String::from_utf8_lossy(HERA_IROH_SHARD_ALPN).into_owned(),
            payload: received,
        })
    }

    /// Fetches and verifies a shard given a ticket (`"iroh://<json>"` or direct json).
    pub async fn fetch_verified_shard_from_ticket(
        &self,
        endpoint_ticket: &str,
        shard_id: &str,
        expected_sha256: Option<&str>,
    ) -> Result<QuicShardTransferReceipt, String> {
        let remote_addr = Self::decode_endpoint_ticket(endpoint_ticket)?;
        self.fetch_verified_shard(remote_addr, shard_id, expected_sha256).await
    }

    /// Loopback shard exchange leveraging `spawn_shard_provider` + `fetch_verified_shard`.
    pub async fn transfer_verified_shard_loopback(
        &self,
        shard_id: &str,
        shard_bytes: &[u8],
    ) -> Result<QuicShardTransferReceipt, String> {
        let expected_sha256 = compute_sha256_hex(shard_bytes);
        let mut map = HashMap::new();
        map.insert(shard_id.to_string(), shard_bytes.to_vec());
        let store = Arc::new(RwLock::new(map));

        let provider_adapter = IrohQuicAdapter::new(format!("{}-provider", self.node_id), self.bind_loopback_only);
        let provider_addr = provider_adapter.spawn_shard_provider(store).await?;

        self.fetch_verified_shard(provider_addr, shard_id, Some(&expected_sha256)).await
    }
}
