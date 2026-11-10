use super::*;

fn camelot_sort_tuple(camelot: &str) -> (i32, u8) {
    let trimmed = camelot.trim().to_ascii_uppercase();
    let mode = match trimmed.chars().last() {
        Some('A') => 0,
        Some('B') => 1,
        _ => 2,
    };
    let num = trimmed
        .trim_end_matches(['A', 'B'])
        .parse::<i32>()
        .unwrap_or(99);
    (num, mode)
}

pub(super) fn sort_tracks_harmonically(tracks: &mut [Track]) {
    tracks.sort_by(|a, b| {
        let ca = camelot_sort_tuple(a.camelot.as_deref().unwrap_or("8A"));
        let cb = camelot_sort_tuple(b.camelot.as_deref().unwrap_or("8A"));
        ca.cmp(&cb).then_with(|| {
            let ba = a.bpm.unwrap_or(124.0);
            let bb = b.bpm.unwrap_or(124.0);
            ba.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
        })
    });
}

pub(super) async fn execute_library_tool<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    tool_name: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    match tool_name {
        "list_library_tracks" => Some(
            async {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100)
                    .clamp(1, 500) as usize;
                let all = ctx
                    .with_db(|db| {
                        Box::pin(
                            async move { db.get_all_tracks().await.map_err(|e| e.to_string()) },
                        )
                    })
                    .await?;
                let total = all.len();
                let items: Vec<Value> = all
                    .into_iter()
                    .take(limit)
                    .map(|t| {
                        json!({
                            "id": t.id,
                            "title": t.title,
                            "artists": t.artists,
                            "bpm": t.bpm.unwrap_or(124.0),
                            "camelot": t.camelot.unwrap_or_else(|| "8A".to_string()),
                            "duration": t.duration,
                            "path": t.path,
                        })
                    })
                    .collect();
                Ok(json!({ "total": total, "tracks": items }))
            }
            .await,
        ),
        "search_library_tracks" => Some(
            async {
                let q = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_lowercase())
                    .unwrap_or_default();
                let bpm_min = args.get("bpm_min").and_then(|v| v.as_f64());
                let bpm_max = args.get("bpm_max").and_then(|v| v.as_f64());
                let camelot = args
                    .get("camelot")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_ascii_uppercase())
                    .filter(|s| !s.is_empty());

                let all = ctx
                    .with_db(|db| {
                        Box::pin(
                            async move { db.get_all_tracks().await.map_err(|e| e.to_string()) },
                        )
                    })
                    .await?;
                let mut matched: Vec<Track> = all
                    .into_iter()
                    .filter(|t| {
                        if !q.is_empty() {
                            let hay = format!(
                                "{} {} {} {}",
                                t.title.to_lowercase(),
                                t.artists.join(" ").to_lowercase(),
                                t.album.to_lowercase(),
                                t.genres.join(" ").to_lowercase()
                            );
                            if !hay.contains(&q) {
                                return false;
                            }
                        }
                        let bpm = t.bpm.unwrap_or(124.0);
                        if let Some(min_b) = bpm_min
                            && bpm < min_b
                        {
                            return false;
                        }
                        if let Some(max_b) = bpm_max
                            && bpm > max_b
                        {
                            return false;
                        }
                        if let Some(ref cam) = camelot {
                            let tc = t.camelot.as_deref().unwrap_or("8A").to_ascii_uppercase();
                            if &tc != cam {
                                return false;
                            }
                        }
                        true
                    })
                    .collect();
                sort_tracks_harmonically(&mut matched);

                let items: Vec<Value> = matched
                    .iter()
                    .map(|t| {
                        json!({
                            "id": t.id,
                            "title": t.title,
                            "artists": t.artists,
                            "bpm": t.bpm.unwrap_or(124.0),
                            "camelot": t.camelot.clone().unwrap_or_else(|| "8A".to_string()),
                            "path": t.path,
                        })
                    })
                    .collect();
                Ok(json!({ "matched": items.len(), "tracks": items }))
            }
            .await,
        ),
        "import_audio_paths" => Some(
            async {
                let raw_paths: Vec<PathBuf> = args
                    .get("paths")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(PathBuf::from))
                            .collect()
                    })
                    .unwrap_or_default();
                if raw_paths.is_empty() {
                    return Err("Se requiere al menos una ruta en 'paths'".to_string());
                }

                let existing = ctx
                    .with_db(|db| {
                        Box::pin(async move {
                            db.get_all_track_paths().await.map_err(|e| e.to_string())
                        })
                    })
                    .await?;
                let existing_set: std::collections::HashSet<PathBuf> =
                    existing.into_iter().map(PathBuf::from).collect();

                let (scanned_files, parsed_tracks) =
                    tauri::async_runtime::spawn_blocking(move || {
                        let mut files = Vec::new();
                        for p in &raw_paths {
                            if p.is_file() {
                                files.push(p.clone());
                            } else if p.is_dir() {
                                files.extend(scan_dirs(
                                    std::slice::from_ref(p),
                                    &SUPPORTED_TRACKS_EXTENSIONS,
                                ));
                            }
                        }
                        let total_found = files.len();
                        files.retain(|f| !existing_set.contains(f));
                        let tracks: Vec<Track> = files
                            .iter()
                            .filter_map(|f| get_track_from_file(f).ok())
                            .collect();
                        (total_found, tracks)
                    })
                    .await
                    .map_err(|e| format!("Fallo escaneando audio: {e}"))?;

                let imported_count = parsed_tracks.len();
                let imported_ids: Vec<String> =
                    parsed_tracks.iter().map(|t| t.id.clone()).collect();
                if !parsed_tracks.is_empty() {
                    ctx.with_db(|db| {
                        Box::pin(async move {
                            db.insert_tracks(parsed_tracks)
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
                }

                Ok(json!({
                    "scanned_files": scanned_files,
                    "imported_count": imported_count,
                    "track_ids": imported_ids,
                }))
            }
            .await,
        ),
        "update_track_metadata" => Some(
            async {
                let track_id = args
                    .get("track_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'track_id'".to_string())?
                    .to_string();
                let title_opt = args.get("title").and_then(|v| v.as_str()).map(String::from);
                let artist_opt = args
                    .get("artist")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let bpm_opt = args.get("bpm").and_then(|v| v.as_f64());
                let cam_opt = args
                    .get("camelot")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let updated = ctx
                    .with_db(|db| {
                        Box::pin(async move {
                            let Some(mut track) =
                                db.get_track(&track_id).await.map_err(|e| e.to_string())?
                            else {
                                return Err(format!("Pista '{track_id}' no encontrada"));
                            };
                            if let Some(t) = title_opt {
                                track.title = t;
                            }
                            if let Some(a) = artist_opt {
                                track.artists = vec![a.clone()];
                                track.album_artist = a;
                            }
                            if let Some(b) = bpm_opt {
                                track.bpm = Some(b);
                            }
                            if let Some(c) = cam_opt {
                                track.camelot = Some(c);
                            }
                            db.update_track(track).await.map_err(|e| e.to_string())
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
                Ok(json!({ "updated": true, "track_id": updated.id, "title": updated.title }))
            }
            .await,
        ),
        "remove_library_tracks" => Some(
            async {
                let ids: Vec<String> = args
                    .get("track_ids")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let removed = ids.len();
                ctx.with_db(|db| {
                    Box::pin(async move { db.remove_tracks(&ids).await.map_err(|e| e.to_string()) })
                })
                .await?;
                ctx.emit_ui_sync(AgentUiSyncEvent {
                    kind: "invalidate_library".to_string(),
                    action: None,
                    track_id: None,
                    seek_seconds: None,
                    route: None,
                });
                Ok(json!({ "removed_count": removed }))
            }
            .await,
        ),
        _ => None,
    }
}
