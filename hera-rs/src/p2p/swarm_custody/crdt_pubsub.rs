use super::super::*;

impl P2PMeshState {
    /// Evaluates active swarm tracks against a `SwarmTopicSubscription` (`hera://swarm/<camelot>/<bpm_band>`)
    /// and emits proactive `SwarmSemanticPubSubEvent` notifications with `style_simhash_u64` and `merkle_root_sha256`.
    pub fn match_tracks_for_subscription(
        &self,
        sub: &SwarmTopicSubscription,
    ) -> Vec<SwarmSemanticPubSubEvent> {
        let mut events = Vec::new();
        let sha_map = self
            .cas_audio_sha256
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        if let Ok(cat) = self.catalog.read() {
            for t in cat.iter() {
                if !t.share_on_p2p {
                    continue;
                }
                if t.bpm < sub.bpm_min || t.bpm > sub.bpm_max {
                    continue;
                }
                if !are_camelot_keys_harmonically_compatible(&sub.camelot_key, &t.camelot_key) {
                    continue;
                }
                if let Some(ref sg) = sub.subgenre_filter {
                    let sg_clean = sg.trim().to_ascii_lowercase();
                    if !sg_clean.is_empty()
                        && !t.top_subgenre.to_ascii_lowercase().contains(&sg_clean)
                    {
                        continue;
                    }
                }
                let cas_fp = compute_cas_fingerprint(&t.artist, &t.title, t.bpm);
                let audio_sha = sha_map.get(&cas_fp).cloned().unwrap_or_else(|| {
                    compute_canonical_audio_sha256(&cas_fp, &t.hera_seal, 38.0)
                });
                let opaque_blob_key = compute_opaque_shard_blob_key(&cas_fp, &audio_sha);
                let merkle_tree = compute_shard_merkle_tree(&opaque_blob_key, &audio_sha, 0);
                let simhash = compute_style_simhash_u64(t.bpm, &t.camelot_key, &t.top_subgenre);
                events.push(SwarmSemanticPubSubEvent {
                    event_id: format!("moq_evt_{}_{}", sub.subscriber_node_id, t.track_id),
                    topic_uri: sub.topic_uri.clone(),
                    cas_fingerprint: cas_fp,
                    track_id: t.track_id.clone(),
                    artist: t.artist.clone(),
                    title: t.title.clone(),
                    bpm: t.bpm,
                    camelot_key: t.camelot_key.clone(),
                    style_simhash_u64: simhash,
                    merkle_root_sha256: merkle_tree.merkle_root_sha256,
                    opaque_blob_key,
                    hot_cues_count: t.hot_cues.len(),
                    prefetch_recommended: true,
                });
            }
        }
        events
    }

    /// Registers an MoQ-style Harmonic Topic Subscription (`POST /api/v1/swarm/subscribe`,
    /// e.g. `hera://swarm/8A/120-128`) and returns immediate matching events for zero-wait pre-fetching.
    pub fn subscribe_semantic_topic(&self, req: SwarmSubscribeRequest) -> SwarmSubscribeResponse {
        let camelot = if req.camelot_key.trim().is_empty() {
            "8A".to_string()
        } else {
            req.camelot_key.trim().to_ascii_uppercase()
        };
        let bpm_min = req.bpm_min.unwrap_or(118.0).max(50.0);
        let bpm_max = req.bpm_max.unwrap_or(132.0).max(bpm_min);
        let topic_uri = build_swarm_topic_uri(
            &camelot,
            bpm_min,
            bpm_max,
            req.subgenre_filter.as_deref(),
        );
        let sub = SwarmTopicSubscription {
            subscriber_node_id: req.subscriber_node_id.trim().to_string(),
            subscriber_dj: req.subscriber_dj.trim().to_string(),
            topic_uri: topic_uri.clone(),
            camelot_key: camelot,
            bpm_min,
            bpm_max,
            subgenre_filter: req.subgenre_filter.clone(),
        };

        if let Ok(mut subs_guard) = self.topic_subscriptions.write() {
            if let Some(existing) = subs_guard.iter_mut().find(|s| {
                s.subscriber_node_id == sub.subscriber_node_id && s.topic_uri == sub.topic_uri
            }) {
                *existing = sub.clone();
            } else {
                subs_guard.push(sub.clone());
            }
        }

        let immediate = self.match_tracks_for_subscription(&sub);
        SwarmSubscribeResponse {
            status: "MOQ_SEMANTIC_TOPIC_SUBSCRIBED".to_string(),
            subscription: sub,
            immediate_matching_events: immediate,
        }
    }

    /// Lists all nodes quarantined for Bait-and-Switch or forged custody proofs.
    pub fn list_quarantined_nodes(&self) -> Vec<QuarantinedNodeRecord> {
        let mut list: Vec<QuarantinedNodeRecord> = self
            .quarantined_nodes
            .read()
            .map(|q| q.values().cloned().collect())
            .unwrap_or_default();
        list.sort_by(|a, b| a.node_id.cmp(&b.node_id));
        list
    }

    /// Synchronizes an incoming `SwarmCrdtDelta` (`POST /api/v1/swarm/crdt-sync`) using commutative,
    /// associative, and idempotent `GCounter` + `LWW-Element-Set` lattice joins.
    pub fn sync_crdt_delta(&self, incoming: SwarmCrdtDelta) -> SwarmCrdtSyncResponse {
        let merged = if let Ok(mut guard) = self.crdt_state.write() {
            let mut next = merge_swarm_crdt_deltas(&guard, &incoming);
            next.origin_node_id = self.node_id.clone();
            next.origin_endpoint_pubkey_id = derive_node_endpoint_pubkey_id("DaviDJ", &self.node_id);
            *guard = next.clone();
            next
        } else {
            incoming
        };

        let mut tracks_upserted = 0usize;
        if let Ok(mut cat_guard) = self.catalog.write() {
            for entry in merged.lww_track_entries.values() {
                if entry.tombstone {
                    cat_guard.retain(|t| {
                        compute_cas_fingerprint(&t.artist, &t.title, t.bpm)
                            != entry.cas_fingerprint
                    });
                    continue;
                }
                let expected_seal = compute_hera_seal(
                    &entry.track.track_id,
                    entry.track.bpm,
                    &entry.track.camelot_key,
                    entry.track.quality_score,
                    &entry.track.forensic_verdict,
                );
                if entry.track.hera_seal == expected_seal {
                    if let Some(existing) = cat_guard.iter_mut().find(|t| {
                        t.track_id == entry.track.track_id
                            || compute_cas_fingerprint(&t.artist, &t.title, t.bpm)
                                == entry.cas_fingerprint
                    }) {
                        *existing = entry.track.clone();
                    } else {
                        cat_guard.push(entry.track.clone());
                        tracks_upserted += 1;
                    }
                }
            }
        }

        if let Ok(mut heat_guard) = self.cas_heat_scores.write() {
            for (cas_fp, node_counts) in &merged.gcounter_cas_play_counts {
                let total_plays: u64 = node_counts.values().copied().sum();
                let new_heat = (1.0 + (total_plays as f32) * 0.5).max(1.0);
                let e = heat_guard.entry(cas_fp.clone()).or_insert(1.0);
                if new_heat > *e {
                    *e = new_heat;
                }
            }
        }

        let total_served_mb = merged
            .gcounter_node_bytes_served_mb
            .values()
            .copied()
            .sum::<f32>();
        let rounded_total = (total_served_mb * 100.0).round() / 100.0;
        let digest = compute_crdt_state_digest_sha256(&merged);

        SwarmCrdtSyncResponse {
            status: "CRDT_DELTA_CONVERGED".to_string(),
            node_id: self.node_id.clone(),
            converged_state_digest_sha256: digest,
            tracks_upserted,
            gcounter_total_swarm_served_mb: rounded_total,
            merged_delta: merged,
        }
    }
}
