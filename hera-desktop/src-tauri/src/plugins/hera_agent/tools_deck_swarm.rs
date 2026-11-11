use super::*;

pub(super) fn track_to_export_item(track: &Track) -> ExportTrackItem {
    let artist = track
        .artists
        .first()
        .cloned()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Artista Desconocido".to_string());
    let cues = track
        .hot_cues
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|pin| HotCuePoint {
            num: pin.num,
            name: pin.name,
            seconds: pin.seconds as f32,
            bar_index: 0,
            role: pin.role,
            confidence: 0.95,
        })
        .collect();
    ExportTrackItem {
        track_id: track.id.clone(),
        artist,
        title: track.title.clone(),
        version: None,
        file_path: track.path.clone(),
        bpm: track.bpm.unwrap_or(124.0) as f32,
        camelot: track.camelot.clone().unwrap_or_else(|| "8A".to_string()),
        duration_seconds: track.duration as f32,
        bitrate_kbps: 1411,
        energy: track.energy.unwrap_or(0.82) as f32,
        file_size_bytes: 0,
        is_video: false,
        is_own_production: false,
        hot_cues: cues,
    }
}

pub(super) async fn execute_deck_swarm_tool<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    tool_name: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    match tool_name {
        "control_audio_player" => Some(
            async {
                let action = args
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("play_track")
                    .to_string();
                let track_id = args
                    .get("track_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let seek_seconds = args.get("seek_seconds").and_then(|v| v.as_f64());

                let track_label = if let Some(ref tid) = track_id {
                    let tid_cl = tid.clone();
                    ctx.with_db(|db| {
                        Box::pin(async move {
                            Ok(db
                                .get_track(&tid_cl)
                                .await
                                .ok()
                                .flatten()
                                .map(|t| {
                                    format!(
                                        "{} — {}",
                                        t.artists.first().cloned().unwrap_or_default(),
                                        t.title
                                    )
                                })
                                .unwrap_or(tid_cl))
                        })
                    })
                    .await
                    .unwrap_or_else(|_| action.clone())
                } else {
                    action.clone()
                };

                ctx.emit_ui_sync(AgentUiSyncEvent {
                    kind: "player_control".to_string(),
                    action: Some(action.clone()),
                    track_id,
                    seek_seconds,
                    route: None,
                });
                Ok(json!({ "status": "dispatched", "action": action, "track_label": track_label }))
            }
            .await,
        ),
        "export_dj_crate_file" => Some(
            async {
                let crate_name = args
                    .get("crate_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("HERA Export")
                    .to_string();
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
                let formats: Vec<String> = args
                    .get("formats")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .filter(|v: &Vec<String>| !v.is_empty())
                    .unwrap_or_else(|| vec!["virtualdj".to_string(), "m3u8".to_string()]);
                let output_dir = args
                    .get("output_dir")
                    .and_then(|v| v.as_str())
                    .map(String::from);

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
                            db.get_all_tracks().await.map_err(|e| e.to_string())
                        })
                    })
                    .await?;
                let items: Vec<ExportTrackItem> = tracks.iter().map(track_to_export_item).collect();
                let res = tauri::async_runtime::spawn_blocking(move || {
                    export_crate_bundle(&ExportCrateRequest {
                        crate_name,
                        brief: "Exportado por Agente HERA".to_string(),
                        tracks: items,
                        formats,
                        output_dir,
                    })
                })
                .await
                .map_err(|e| format!("Fallo exportando crate: {e}"))?;
                serde_json::to_value(res).map_err(|e| e.to_string())
            }
            .await,
        ),
        "set_app_view_or_mini_mode" => Some(
            async {
                if let Some(wm) = args.get("window_mode").and_then(|v| v.as_str())
                    && let AgentHostBridge::Tauri(app) = &ctx.host
                {
                    let _ = set_mini_overlay_mode(
                        app.clone(),
                        SetMiniOverlayModeRequest {
                            mode: wm.to_string(),
                            pinned: wm != "full",
                        },
                    )
                    .await;
                }
                if let Some(route) = args.get("route").and_then(|v| v.as_str()) {
                    ctx.emit_ui_sync(AgentUiSyncEvent {
                        kind: "navigate".to_string(),
                        action: None,
                        track_id: None,
                        seek_seconds: None,
                        route: Some(route.to_string()),
                    });
                }
                Ok(json!({ "ok": true }))
            }
            .await,
        ),
        "download_swarm_track_to_library" => Some(
            async {
                let artist = args
                    .get("artist")
                    .and_then(|v| v.as_str())
                    .unwrap_or("DaviDJ")
                    .to_string();
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Swarm Master")
                    .to_string();
                let bpm = args.get("bpm").and_then(|v| v.as_f64()).unwrap_or(125.0) as f32;
                let camelot = args
                    .get("camelot")
                    .and_then(|v| v.as_str())
                    .unwrap_or("8A")
                    .to_string();
                let track_id = args
                    .get("track_id")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                let AgentHostBridge::Tauri(app) = &ctx.host else {
                    return Err("Descarga P2P requiere contexto de aplicación activa".to_string());
                };
                let dl = download_p2p_track(
                    app.clone(),
                    DownloadP2PTrackRequest {
                        track_id,
                        artist,
                        title,
                        bpm,
                        camelot,
                        hera_seal: "VERIFIED_STUDIO".to_string(),
                        peer_endpoint: None,
                        turn_relay_endpoint: None,
                        chunk_count: None,
                    },
                )
                .await?;

                let saved_path = PathBuf::from(&dl.saved_path);
                if let Ok(parsed) = get_track_from_file(&saved_path) {
                    let _ = ctx
                        .with_db(|db| {
                            Box::pin(async move {
                                db.insert_tracks(vec![parsed])
                                    .await
                                    .map_err(|e| e.to_string())
                            })
                        })
                        .await;
                    ctx.emit_ui_sync(AgentUiSyncEvent {
                        kind: "invalidate_library".to_string(),
                        action: None,
                        track_id: None,
                        seek_seconds: None,
                        route: None,
                    });
                }
                serde_json::to_value(dl).map_err(|e| e.to_string())
            }
            .await,
        ),
        _ => None,
    }
}

pub(super) async fn execute_tool_inner<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session_id: Option<&str>,
    tool_name: &str,
    args: &Value,
) -> Result<Value, String> {
    if let Some(res) = execute_library_tool(ctx, tool_name, args).await {
        return res;
    }
    if let Some(res) = execute_curation_feedback_tool(ctx, session_id, tool_name, args).await {
        return res;
    }
    if let Some(res) = execute_playlist_vdj_tool(ctx, session_id, tool_name, args).await {
        return res;
    }
    if let Some(res) = execute_deck_swarm_tool(ctx, tool_name, args).await {
        return res;
    }
    if let Some(res) = execute_host_os_tool(tool_name, args).await {
        return res;
    }
    let args_cl = args.clone();
    let name_cl = tool_name.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        dispatch_core_mcp_tool(&name_cl, args_cl, shared_desktop_p2p_state())
    })
    .await
    .map_err(|e| format!("Fallo en tool DSP/P2P: {e}"))?
}
