use super::*;

pub(super) async fn execute_host_os_tool(
    tool_name: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    match tool_name {
        "run_powershell_command" => Some(
            async {
                let command_str = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'command'".to_string())?
                    .to_string();
                let cwd_opt = args.get("cwd").and_then(|v| v.as_str()).map(PathBuf::from);
                let timeout_sec = args
                    .get("timeout_sec")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30)
                    .clamp(1, 120);

                let mut cmd = if cfg!(windows) {
                    let mut c = tokio::process::Command::new("powershell");
                    c.args([
                        "-NoProfile",
                        "-NonInteractive",
                        "-Command",
                        &format!(
                            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; {}",
                            command_str
                        ),
                    ]);
                    #[cfg(windows)]
                    {
                        c.creation_flags(0x08000000); // CREATE_NO_WINDOW
                    }
                    c
                } else {
                    let mut c = tokio::process::Command::new("sh");
                    c.args(["-c", &command_str]);
                    c
                };

                if let Some(cwd) = cwd_opt
                    && cwd.exists()
                {
                    cmd.current_dir(cwd);
                }
                cmd.stdin(std::process::Stdio::null());

                let output = tokio::time::timeout(Duration::from_secs(timeout_sec), cmd.output())
                    .await
                    .map_err(|_| format!("Tiempo de espera agotado ({timeout_sec}s)"))?
                    .map_err(|e| format!("Fallo lanzando shell: {e}"))?;

                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                Ok(json!({
                    "exit_code": exit_code,
                    "stdout": truncate_chars(&stdout, 12000),
                    "stderr": truncate_chars(&stderr, 4000),
                }))
            }
            .await,
        ),
        "list_directory_entries" => Some(
            async {
                let path_str = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'path'".to_string())?
                    .to_string();
                let max_entries = args
                    .get("max_entries")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(200)
                    .clamp(1, 1000) as usize;

                tauri::async_runtime::spawn_blocking(move || {
                    let dir = PathBuf::from(&path_str);
                    if !dir.exists() {
                        return Err(format!("La ruta '{}' no existe", path_str));
                    }
                    let read_dir = std::fs::read_dir(&dir)
                        .map_err(|e| format!("No se pudo leer '{}': {e}", path_str))?;
                    let mut entries = Vec::new();
                    let mut total_entries = 0usize;
                    let mut audio_files = 0usize;

                    for entry_res in read_dir.flatten() {
                        total_entries += 1;
                        let p = entry_res.path();
                        let is_dir = p.is_dir();
                        let ext = p
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        let is_audio =
                            !is_dir && SUPPORTED_TRACKS_EXTENSIONS.contains(&ext.as_str());
                        if is_audio {
                            audio_files += 1;
                        }
                        if entries.len() < max_entries {
                            let size = entry_res.metadata().map(|m| m.len()).unwrap_or(0);
                            entries.push(json!({
                                "name": entry_res.file_name().to_string_lossy(),
                                "path": p.to_string_lossy(),
                                "is_dir": is_dir,
                                "is_audio": is_audio,
                                "size_bytes": size,
                            }));
                        }
                    }
                    Ok(json!({
                        "path": path_str,
                        "total_entries": total_entries,
                        "audio_files": audio_files,
                        "entries": entries,
                    }))
                })
                .await
                .map_err(|e| format!("Fallo listando directorio: {e}"))?
            }
            .await,
        ),
        "read_text_file" => Some(
            async {
                let path_str = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'path'".to_string())?
                    .to_string();
                let max_bytes = args
                    .get("max_bytes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(32768)
                    .clamp(256, 262144) as usize;
                tauri::async_runtime::spawn_blocking(move || {
                    let bytes = std::fs::read(&path_str)
                        .map_err(|e| format!("No se pudo leer '{}': {e}", path_str))?;
                    let truncated = bytes.len() > max_bytes;
                    let slice = if truncated {
                        &bytes[..max_bytes]
                    } else {
                        &bytes[..]
                    };
                    Ok(json!({
                        "path": path_str,
                        "truncated": truncated,
                        "content": String::from_utf8_lossy(slice),
                    }))
                })
                .await
                .map_err(|e| format!("Fallo leyendo archivo: {e}"))?
            }
            .await,
        ),
        "write_text_file" => Some(
            async {
                let path_str = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'path'".to_string())?
                    .to_string();
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'content'".to_string())?
                    .to_string();
                tauri::async_runtime::spawn_blocking(move || {
                    let target = PathBuf::from(&path_str);
                    if let Some(parent) = target.parent()
                        && !parent.as_os_str().is_empty()
                    {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| format!("No se pudo crear carpeta padre: {e}"))?;
                    }
                    let bytes_len = content.len();
                    std::fs::write(&target, content)
                        .map_err(|e| format!("No se pudo escribir '{}': {e}", path_str))?;
                    Ok(json!({ "path": path_str, "bytes_written": bytes_len }))
                })
                .await
                .map_err(|e| format!("Fallo escribiendo archivo: {e}"))?
            }
            .await,
        ),
        "move_or_rename_path" => Some(
            async {
                let from_path = args
                    .get("from_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'from_path'".to_string())?
                    .to_string();
                let to_path = args
                    .get("to_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'to_path'".to_string())?
                    .to_string();
                tauri::async_runtime::spawn_blocking(move || {
                    let dst = PathBuf::from(&to_path);
                    if let Some(parent) = dst.parent()
                        && !parent.as_os_str().is_empty()
                    {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    std::fs::rename(&from_path, &dst).map_err(|e| {
                        format!("No se pudo mover '{from_path}' a '{to_path}': {e}")
                    })?;
                    Ok(json!({ "from_path": from_path, "to_path": to_path, "moved": true }))
                })
                .await
                .map_err(|e| format!("Fallo moviendo ruta: {e}"))?
            }
            .await,
        ),
        "open_system_target" => Some(
            async {
                let target = args
                    .get("target")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Falta 'target'".to_string())?
                    .to_string();
                tauri::async_runtime::spawn_blocking(move || {
                    let mut cmd = if cfg!(windows) {
                        let mut c = std::process::Command::new("explorer");
                        c.arg(&target);
                        c
                    } else if cfg!(target_os = "macos") {
                        let mut c = std::process::Command::new("open");
                        c.arg(&target);
                        c
                    } else {
                        let mut c = std::process::Command::new("xdg-open");
                        c.arg(&target);
                        c
                    };
                    cmd.spawn()
                        .map_err(|e| format!("No se pudo abrir '{target}': {e}"))?;
                    Ok(json!({ "opened": target }))
                })
                .await
                .map_err(|e| format!("Fallo abriendo destino: {e}"))?
            }
            .await,
        ),
        _ => None,
    }
}
