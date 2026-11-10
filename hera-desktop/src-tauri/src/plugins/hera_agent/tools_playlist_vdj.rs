use super::*;

pub(super) async fn execute_playlist_vdj_tool<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session_id: Option<&str>,
    tool_name: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    match tool_name {
        "list_playlists" => Some(
            async {
                let playlists = ctx
                    .with_db(|db| {
                        Box::pin(
                            async move { db.get_all_playlists().await.map_err(|e| e.to_string()) },
                        )
                    })
                    .await?;
                let out: Vec<Value> = playlists
                    .iter()
                    .map(|p| {
                        json!({
                            "id": p.id,
                            "name": p.name,
                            "track_count": p.tracks.len(),
                            "track_ids": p.tracks,
                        })
                    })
                    .collect();
                Ok(json!({ "total": out.len(), "playlists": out }))
            }
            .await,
        ),
        "create_playlist" => Some(
            async {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Set HERA")
                    .to_string();
                let sort_harmonic = args
                    .get("sort_harmonic")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let explicit_ids: Vec<String> = args
                    .get("track_ids")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let (playlist, first_track_id, key_flow) = ctx
                    .with_db(|db| {
                        Box::pin(async move {
                            let mut tracks = if explicit_ids.is_empty() {
                                db.get_all_tracks().await.map_err(|e| e.to_string())?
                            } else {
                                db.get_tracks(&explicit_ids)
                                    .await
                                    .map_err(|e| e.to_string())?
                            };
                            if sort_harmonic {
                                sort_tracks_harmonically(&mut tracks);
                            }
                            let mut keys_dedup: Vec<String> = Vec::new();
                            for t in &tracks {
                                let k = t.camelot.clone().unwrap_or_else(|| "8A".to_string());
                                if keys_dedup.last() != Some(&k) {
                                    keys_dedup.push(k);
                                }
                            }
                            let flow = if keys_dedup.is_empty() {
                                "vacía".to_string()
                            } else {
                                keys_dedup
                                    .into_iter()
                                    .take(4)
                                    .collect::<Vec<_>>()
                                    .join(" → ")
                            };
                            let first_id = tracks.first().map(|t| t.id.clone());
                            let final_ids: Vec<String> = tracks.into_iter().map(|t| t.id).collect();
                            let created = db
                                .create_playlist(name, final_ids, None)
                                .await
                                .map_err(|e| e.to_string())?;
                            Ok((created, first_id, flow))
                        })
                    })
                    .await?;

                ctx.emit_ui_sync(AgentUiSyncEvent {
                    kind: "invalidate_library".to_string(),
                    action: None,
                    track_id: None,
                    seek_seconds: None,
                    route: None,
                });

                if let Some(fid) = first_track_id {
                    record_followup_action(
                        ctx,
                        session_id,
                        AgentFollowupAction {
                            kind: "play_track".to_string(),
                            label: "Escuchar primera pista".to_string(),
                            target_id: fid,
                        },
                    )
                    .await;
                }
                record_followup_action(
                    ctx,
                    session_id,
                    AgentFollowupAction {
                        kind: "open_playlist".to_string(),
                        label: "Abrir playlist".to_string(),
                        target_id: playlist.id.clone(),
                    },
                )
                .await;

                Ok(json!({
                    "playlist_id": playlist.id,
                    "name": playlist.name,
                    "track_count": playlist.tracks.len(),
                    "track_ids": playlist.tracks,
                    "key_flow": key_flow,
                }))
            }
            .await,
        ),
        "set_playlist_tracks" => Some(
            async {
                let pid = args
                    .get("playlist_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'playlist_id'".to_string())?
                    .to_string();
                let ids: Vec<String> = args
                    .get("track_ids")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let updated = ctx
                    .with_db(|db| {
                        Box::pin(async move {
                            db.set_playlist_tracks(&pid, ids)
                                .await
                                .map_err(|e| e.to_string())
                        })
                    })
                    .await?;
                ctx.emit_ui_sync(AgentUiSyncEvent {
                    kind: "invalidate_library".to_string(),
                    action: None,
                    track_id: None,
                    seek_seconds: None,
                    route: None,
                });
                Ok(json!({ "playlist_id": updated.id, "track_count": updated.tracks.len() }))
            }
            .await,
        ),
        "rename_playlist" => Some(
            async {
                let pid = args
                    .get("playlist_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'playlist_id'".to_string())?
                    .to_string();
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'name'".to_string())?
                    .to_string();
                let updated = ctx
                    .with_db(|db| {
                        Box::pin(async move {
                            db.rename_playlist(&pid, name)
                                .await
                                .map_err(|e| e.to_string())
                        })
                    })
                    .await?;
                ctx.emit_ui_sync(AgentUiSyncEvent {
                    kind: "invalidate_library".to_string(),
                    action: None,
                    track_id: None,
                    seek_seconds: None,
                    route: None,
                });
                Ok(json!({ "playlist_id": updated.id, "name": updated.name }))
            }
            .await,
        ),
        "delete_playlist" => Some(
            async {
                let pid = args
                    .get("playlist_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'playlist_id'".to_string())?
                    .to_string();
                ctx.with_db(|db| {
                    Box::pin(
                        async move { db.delete_playlist(&pid).await.map_err(|e| e.to_string()) },
                    )
                })
                .await?;
                ctx.emit_ui_sync(AgentUiSyncEvent {
                    kind: "invalidate_library".to_string(),
                    action: None,
                    track_id: None,
                    seek_seconds: None,
                    route: None,
                });
                Ok(json!({ "deleted": true }))
            }
            .await,
        ),
        "push_virtualdj_live_crate" => Some(
            async {
                let crate_name = args
                    .get("crate_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("HERA En Vivo")
                    .to_string();
                let append = args
                    .get("append")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let playlist_id = args
                    .get("playlist_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let explicit_ids: Vec<String> = args
                    .get("track_ids")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let tracks = ctx
                    .with_db(|db| {
                        Box::pin(async move {
                            if let Some(pid) = playlist_id
                                && let Some(pl) =
                                    db.get_playlist(&pid).await.map_err(|e| e.to_string())?
                            {
                                return db.get_tracks(&pl.tracks).await.map_err(|e| e.to_string());
                            }
                            if !explicit_ids.is_empty() {
                                return db
                                    .get_tracks(&explicit_ids)
                                    .await
                                    .map_err(|e| e.to_string());
                            }
                            let mut all = db.get_all_tracks().await.map_err(|e| e.to_string())?;
                            sort_tracks_harmonically(&mut all);
                            Ok(all)
                        })
                    })
                    .await?;

                let export_items: Vec<ExportTrackItem> =
                    tracks.iter().map(track_to_export_item).collect();

                match &ctx.host {
                    AgentHostBridge::Tauri(app) => {
                        let res = push_virtualdj_live_crate(
                            app.clone(),
                            PushVirtualDjLiveCrateRequest {
                                tracks: export_items,
                                append,
                                crate_name: Some(crate_name),
                            },
                        )
                        .await?;
                        serde_json::to_value(res).map_err(|e| e.to_string())
                    }
                    AgentHostBridge::Standalone(_) => {
                        let mut target_dirs = Vec::new();
                        if let Ok(vdj_home) = std::env::var("HERA_VDJ_HOME")
                            && !vdj_home.trim().is_empty()
                        {
                            let folders = PathBuf::from(vdj_home).join("Folders");
                            if std::fs::create_dir_all(&folders).is_ok() {
                                target_dirs.push(folders);
                            }
                        }
                        if target_dirs.is_empty() {
                            let tmp = std::env::temp_dir().join("hera_agent_vdj_exports");
                            let _ = std::fs::create_dir_all(&tmp);
                            target_dirs.push(tmp);
                        }
                        let written = crate::plugins::hera_dj::write_virtualdj_live_crate_files(
                            &crate_name,
                            &export_items,
                            &target_dirs,
                        );
                        Ok(json!({
                            "crate_name": crate_name,
                            "total_tracks": export_items.len(),
                            "written_files": written,
                            "synced_to_virtualdj_home": true,
                        }))
                    }
                }
            }
            .await,
        ),
        "read_virtualdj_now_playing" => Some(
            async {
                let np = read_virtualdj_now_playing().await?;
                Ok(json!({ "now_playing": np }))
            }
            .await,
        ),
        _ => None,
    }
}
