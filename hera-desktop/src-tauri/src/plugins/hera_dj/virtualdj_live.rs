use super::*;

fn live_vdj_crate_slot() -> &'static Mutex<Vec<ExportTrackItem>> {
    static CRATE: OnceLock<Mutex<Vec<ExportTrackItem>>> = OnceLock::new();
    CRATE.get_or_init(|| Mutex::new(Vec::new()))
}

fn candidate_virtualdj_homes() -> Vec<std::path::PathBuf> {
    let mut homes = Vec::new();
    if let Ok(override_dir) = std::env::var("HERA_VDJ_HOME")
        && !override_dir.trim().is_empty()
    {
        homes.push(std::path::PathBuf::from(override_dir));
    }
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        homes.push(
            std::path::PathBuf::from(&user_profile)
                .join("Documents")
                .join("VirtualDJ"),
        );
    }
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        homes.push(std::path::PathBuf::from(local_app_data).join("VirtualDJ"));
    }
    if let Ok(home) = std::env::var("HOME") {
        homes.push(
            std::path::PathBuf::from(home)
                .join("Documents")
                .join("VirtualDJ"),
        );
    }
    homes
}

pub(crate) fn parse_virtualdj_history_content(raw: &str) -> Option<(String, String)> {
    for line in raw.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("#EXTM3U") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#EXTINF:") {
            let after_comma = rest.split_once(',').map(|(_, t)| t.trim()).unwrap_or(rest);
            if let Some((artist, title)) = after_comma
                .split_once(" - ")
                .or_else(|| after_comma.split_once(" — "))
            {
                let a = artist.trim();
                let t = title.trim();
                if !a.is_empty() && !t.is_empty() {
                    return Some((a.to_string(), t.to_string()));
                }
            }
            continue;
        }
        if trimmed.starts_with('#')
            || trimmed.contains(":\\")
            || trimmed.starts_with('/')
            || trimmed.ends_with(".mp3")
            || trimmed.ends_with(".wav")
            || trimmed.ends_with(".flac")
        {
            continue;
        }
        let cleaned = if trimmed.len() >= 7
            && trimmed.as_bytes().get(2) == Some(&b':')
            && let Some((_, after_time)) = trimmed.split_once(" : ")
        {
            after_time.trim()
        } else if trimmed.len() >= 6
            && trimmed.as_bytes().get(2) == Some(&b':')
            && let Some((_, after_time)) = trimmed.split_once(' ')
        {
            after_time.trim_start_matches(':').trim()
        } else {
            trimmed
        };

        if let Some((artist, title)) = cleaned
            .split_once(" - ")
            .or_else(|| cleaned.split_once(" — "))
        {
            let a = artist.trim();
            let t = title.trim();
            if !a.is_empty() && !t.is_empty() {
                return Some((a.to_string(), t.to_string()));
            }
        }
    }
    None
}

pub(crate) fn write_virtualdj_live_crate_files(
    crate_name: &str,
    tracks: &[ExportTrackItem],
    target_dirs: &[std::path::PathBuf],
) -> Vec<String> {
    let mut all_written = Vec::new();
    for dir in target_dirs {
        let req = ExportCrateRequest {
            crate_name: crate_name.to_string(),
            brief: "HERA Live Overlay Crate".to_string(),
            tracks: tracks.to_vec(),
            formats: vec!["virtualdj".to_string(), "m3u8".to_string()],
            output_dir: Some(dir.to_string_lossy().to_string()),
        };
        let res = export_crate_bundle(&req);
        all_written.extend(res.written_files);
    }
    all_written
}

#[tauri::command]
pub async fn push_virtualdj_live_crate<R: Runtime>(
    app: tauri::AppHandle<R>,
    request: PushVirtualDjLiveCrateRequest,
) -> Result<PushVirtualDjLiveCrateResponse, String> {
    let fallback_data_dir = app.path().app_data_dir().ok();
    tauri::async_runtime::spawn_blocking(move || {
        let crate_name = request
            .crate_name
            .unwrap_or_else(|| "HERA En Vivo".to_string());

        let merged_tracks = {
            let mut guard = live_vdj_crate_slot()
                .lock()
                .map_err(|_| "No se pudo bloquear el estado de VirtualDJ en vivo".to_string())?;
            if !request.append {
                guard.clear();
            }
            for item in request.tracks {
                if let Some(existing) = guard.iter_mut().find(|t| {
                    t.track_id == item.track_id
                        || (!item.file_path.is_empty() && t.file_path == item.file_path)
                }) {
                    *existing = item;
                } else {
                    guard.push(item);
                }
            }
            guard.clone()
        };

        let mut target_dirs = Vec::new();
        let mut synced_to_virtualdj_home = false;

        for vdj_home in candidate_virtualdj_homes() {
            if vdj_home.exists() {
                let folders_dir = vdj_home.join("Folders");
                if std::fs::create_dir_all(&folders_dir).is_ok() {
                    target_dirs.push(folders_dir);
                    synced_to_virtualdj_home = true;
                }
            }
        }

        let cwd_exports = std::env::current_dir()
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("exports")
            .join("VirtualDJ");
        if std::fs::create_dir_all(&cwd_exports).is_ok() {
            target_dirs.push(cwd_exports);
        } else if let Some(app_dir) = fallback_data_dir {
            let fallback_exports = app_dir.join("exports").join("VirtualDJ");
            if std::fs::create_dir_all(&fallback_exports).is_ok() {
                target_dirs.push(fallback_exports);
            }
        }

        let written_files =
            write_virtualdj_live_crate_files(&crate_name, &merged_tracks, &target_dirs);

        Ok(PushVirtualDjLiveCrateResponse {
            crate_name,
            total_tracks: merged_tracks.len(),
            written_files,
            synced_to_virtualdj_home,
        })
    })
    .await
    .map_err(|e| format!("Fallo en tarea push_virtualdj_live_crate: {e}"))?
}

#[tauri::command]
pub async fn read_virtualdj_now_playing() -> Result<Option<VirtualDjNowPlaying>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        for vdj_home in candidate_virtualdj_homes() {
            let history_dir = vdj_home.join("History");
            if !history_dir.exists() {
                continue;
            }
            let tracklist_txt = history_dir.join("tracklist.txt");
            if tracklist_txt.is_file()
                && let Ok(raw) = std::fs::read_to_string(&tracklist_txt)
                && let Some((artist, title)) = parse_virtualdj_history_content(&raw)
            {
                return Ok(Some(VirtualDjNowPlaying {
                    artist,
                    title,
                    source_path: tracklist_txt.to_string_lossy().to_string(),
                }));
            }
        }
        Ok(None)
    })
    .await
    .map_err(|e| format!("Fallo en tarea read_virtualdj_now_playing: {e}"))?
}

/// Generates live System Tray tooltip text matching the format:
/// `HERA BETA · Sonando: <Artista> - <Título>` or `HERA BETA · Cabina lista`
pub fn format_now_playing_tray_tooltip(now_playing: Option<&VirtualDjNowPlaying>) -> String {
    match now_playing {
        Some(track) if !track.artist.is_empty() && !track.title.is_empty() => {
            format!("HERA BETA · Sonando: {} - {}", track.artist, track.title)
        }
        _ => "HERA BETA · Cabina lista".to_string(),
    }
}

