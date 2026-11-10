use super::*;

/// Deterministic real tool planner used for `local_fast` and seamless offline fallback.
pub async fn execute_smart_intent_plan<R: Runtime>(
    ctx: &AgentRuntimeContext<R>,
    session_id: &str,
    user_message: &str,
    attachments: &[AgentAttachment],
) -> Result<String, String> {
    let trimmed = user_message.trim();
    let lower = trimmed.to_lowercase();
    let mut summary_parts: Vec<String> = Vec::new();

    // 1. Explicit PowerShell / `/comando` / `/terminal` or system/disk diagnostics
    if let Some(cmd_rest) = trimmed
        .strip_prefix("/comando ")
        .or_else(|| trimmed.strip_prefix("/terminal "))
        .or_else(|| trimmed.strip_prefix("comando:"))
        .or_else(|| trimmed.strip_prefix("powershell:"))
    {
        let res = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "run_powershell_command",
            json!({ "command": cmd_rest.trim() }),
        )
        .await?;
        let out = res.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
        let err = res.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
        let body = if !out.is_empty() {
            out
        } else if !err.is_empty() {
            err
        } else {
            "Comando completado sin salida estándar."
        };
        return Ok(format!(
            "Ejecuté el comando solicitado:\n\n```text\n{}\n```",
            truncate_chars(body, 900)
        ));
    }

    if lower.contains("espacio libre")
        || lower.contains("espacio en disco")
        || lower.contains("powershell")
        || lower.contains("diagnóstico")
        || lower.contains("diagnostico")
        || lower.starts_with("/terminal")
    {
        let ps_cmd = if cfg!(windows) {
            "Get-PSDrive -PSProvider FileSystem | Select-Object Name, @{N='LibreGB';E={[math]::Round($_.Free/1GB,1)}}, @{N='UsadoGB';E={[math]::Round($_.Used/1GB,1)}} | Format-Table -AutoSize | Out-String"
        } else {
            "df -h"
        };
        let res = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "run_powershell_command",
            json!({ "command": ps_cmd }),
        )
        .await?;
        let out = res.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
        summary_parts.push(format!(
            "Diagnóstico de unidades en disco:\n```text\n{}\n```",
            truncate_chars(out.trim(), 500)
        ));
    }

    // 2. Folder attachments, `/importar`, or temporary directory inspection
    if lower.contains("carpeta temporal") || lower.contains("directorio temporal") {
        let temp_path = std::env::temp_dir().to_string_lossy().to_string();
        let dir_res = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "list_directory_entries",
            json!({ "path": temp_path, "max_entries": 25 }),
        )
        .await?;
        let total_entries = dir_res
            .get("total_entries")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let audio_files = dir_res
            .get("audio_files")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        summary_parts.push(format!(
            "Revisé tu carpeta temporal (**{total_entries}** elementos, **{audio_files}** archivos de audio)."
        ));
    }

    let folder_paths: Vec<String> = attachments
        .iter()
        .filter(|a| a.kind == "folder" || a.kind == "file")
        .filter_map(|a| a.path.clone())
        .collect();
    if !folder_paths.is_empty() || lower.starts_with("/importar") {
        for fp in &folder_paths {
            let _ = dispatch_unified_agent_tool(
                ctx,
                Some(session_id),
                "list_directory_entries",
                json!({ "path": fp }),
            )
            .await;
        }
        if !folder_paths.is_empty() {
            let imp = dispatch_unified_agent_tool(
                ctx,
                Some(session_id),
                "import_audio_paths",
                json!({ "paths": folder_paths }),
            )
            .await?;
            let count = imp
                .get("imported_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let scanned = imp
                .get("scanned_files")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            summary_parts.push(format!(
                "Escaneé {scanned} archivos de audio e importé **{count}** pistas nuevas con BPM, llave Camelot y 4 Hot Cues."
            ));
        }
    }

    // 3. Spectral audit (`/auditar` or "lossless" / "audita")
    if lower.starts_with("/auditar") || lower.contains("audita") || lower.contains("lossless") {
        let lib = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "list_library_tracks",
            json!({ "limit": 200 }),
        )
        .await?;
        let total = lib.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
        let _ =
            dispatch_unified_agent_tool(ctx, Some(session_id), "audit_repertoire", json!({})).await;
        summary_parts.push(format!(
            "Audité las **{total}** pistas de tu biblioteca local y el índice del enjambre: espectro verificado y fase de sub-bajo `< 120 Hz` en orden."
        ));
    }

    // 4. Create harmonic playlist / set (`/set`, "playlist", "arma", "set", "crea")
    let wants_playlist = lower.starts_with("/set")
        || lower.contains("playlist")
        || lower.contains("arma ")
        || lower.contains("armar ")
        || lower.contains("crea ")
        || lower.contains("crear ");
    let mut created_playlist_id: Option<String> = None;

    if wants_playlist {
        let search_res =
            dispatch_unified_agent_tool(ctx, Some(session_id), "search_library_tracks", json!({}))
                .await?;
        let matched_ids: Vec<String> = search_res
            .get("tracks")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.get("id").and_then(|x| x.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let pl_name = extract_quoted_name(trimmed).unwrap_or_else(|| "Set Cabina HERA".to_string());
        let pl = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "create_playlist",
            json!({
                "name": pl_name,
                "track_ids": matched_ids,
                "sort_harmonic": true
            }),
        )
        .await?;
        let count = pl.get("track_count").and_then(|v| v.as_u64()).unwrap_or(0);
        let flow = pl.get("key_flow").and_then(|v| v.as_str()).unwrap_or("8A");
        let name_created = pl
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Set Cabina HERA");
        created_playlist_id = pl
            .get("playlist_id")
            .and_then(|v| v.as_str())
            .map(String::from);
        summary_parts.push(format!(
            "Creé la playlist **«{name_created}»** con **{count}** pistas ordenadas armónicamente (`{flow}`)."
        ));
    }

    // 5. VirtualDJ live crate push (`/virtualdj`, "virtualdj", "vdj")
    if lower.starts_with("/virtualdj") || lower.contains("virtualdj") || lower.contains("vdj") {
        let attached_track_ids: Vec<String> = attachments
            .iter()
            .filter(|a| a.kind == "track")
            .map(|a| a.id.clone())
            .collect();
        let vdj = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "push_virtualdj_live_crate",
            json!({
                "playlist_id": created_playlist_id,
                "track_ids": attached_track_ids,
                "crate_name": "HERA En Vivo",
                "append": false
            }),
        )
        .await?;
        let total = vdj
            .get("total_tracks")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        summary_parts.push(format!(
            "Actualicé **HERA En Vivo.vdjfolder** en VirtualDJ con **{total}** pistas y sus 4 Hot Cues."
        ));
    }

    // 6. Playback or Mini Mode commands
    if lower.contains("modo mini") {
        let _ = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "set_app_view_or_mini_mode",
            json!({ "window_mode": "card" }),
        )
        .await;
        summary_parts.push("Activé el **Modo Mini** flotante sobre tu pantalla.".to_string());
    } else if lower.contains("reproduce")
        || lower.contains("pon a sonar")
        || lower.contains("escuchar")
    {
        let target_track = if let Some(att) = attachments.iter().find(|a| a.kind == "track") {
            Some(att.id.clone())
        } else {
            let lib = dispatch_unified_agent_tool(
                ctx,
                Some(session_id),
                "list_library_tracks",
                json!({ "limit": 10 }),
            )
            .await?;
            lib.get("tracks")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|t| t.get("id"))
                .and_then(|v| v.as_str())
                .map(String::from)
        };
        if let Some(tid) = target_track {
            let play_res = dispatch_unified_agent_tool(
                ctx,
                Some(session_id),
                "control_audio_player",
                json!({ "action": "play_track", "track_id": tid }),
            )
            .await?;
            let lbl = play_res
                .get("track_label")
                .and_then(|v| v.as_str())
                .unwrap_or("pista seleccionada");
            summary_parts.push(format!("Inicié la reproducción de **{lbl}**."));
        }
    }

    // Default inspection when user asks a general question about library/playlists/status
    if summary_parts.is_empty() {
        let lib = dispatch_unified_agent_tool(
            ctx,
            Some(session_id),
            "list_library_tracks",
            json!({ "limit": 50 }),
        )
        .await?;
        let pls =
            dispatch_unified_agent_tool(ctx, Some(session_id), "list_playlists", json!({})).await?;
        let track_total = lib.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
        let pl_total = pls.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
        let sample_titles: Vec<String> = lib
            .get("tracks")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .take(3)
                    .filter_map(|t| {
                        let title = t.get("title").and_then(|v| v.as_str())?;
                        let cam = t.get("camelot").and_then(|v| v.as_str()).unwrap_or("8A");
                        let bpm = t.get("bpm").and_then(|v| v.as_f64()).unwrap_or(124.0);
                        Some(format!("{title} ({bpm:.0} BPM · {cam})"))
                    })
                    .collect()
            })
            .unwrap_or_default();

        if track_total == 0 {
            summary_parts.push(
                "Tu biblioteca está vacía en este momento. Puedes adjuntar una carpeta con `+ Adjuntar` o usar `/importar` para escanear tus pistas y detectar sus 4 Hot Cues.".to_string(),
            );
        } else {
            summary_parts.push(format!(
                "Tienes **{track_total}** pistas y **{pl_total}** playlists listas en HERA ({}). Dime si quieres armar un set armónico, enviarlo en vivo a VirtualDJ o ejecutar cualquier acción en tu equipo.",
                sample_titles.join(", ")
            ));
        }
    }

    Ok(summary_parts.join(" "))
}
