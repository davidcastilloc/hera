//! Human-readable Spanish tool step summaries, prompt quote parsing, and session title helpers.

use serde_json::Value;
use std::path::Path;

pub fn human_tool_title(tool_name: &str) -> String {
    match tool_name {
        "list_library_tracks" => "Leer biblioteca".to_string(),
        "search_library_tracks" => "Filtrar biblioteca".to_string(),
        "get_track_details" => "Inspeccionar pista".to_string(),
        "import_audio_paths" => "Importar carpeta de audio".to_string(),
        "update_track_dj_metadata" => "Actualizar metadatos DJ".to_string(),
        "remove_library_tracks" => "Quitar pistas de biblioteca".to_string(),
        "list_playlists" => "Consultar playlists".to_string(),
        "get_playlist_tracks" => "Leer pistas de playlist".to_string(),
        "create_playlist" => "Crear playlist".to_string(),
        "update_playlist_tracks" => "Actualizar orden de playlist".to_string(),
        "rename_playlist" => "Renombrar playlist".to_string(),
        "delete_playlist" => "Eliminar playlist".to_string(),
        "control_audio_player" => "Controlar reproductor".to_string(),
        "set_app_view_or_mini_mode" => "Cambiar vista o Modo Mini".to_string(),
        "push_virtualdj_live_crate" => "Enviar a VirtualDJ".to_string(),
        "get_virtualdj_live_deck" => "Leer deck en VirtualDJ".to_string(),
        "download_p2p_swarm_track" => "Descargar del enjambre P2P".to_string(),
        "list_directory_entries" => "Inspeccionar carpeta".to_string(),
        "read_text_file" => "Leer archivo".to_string(),
        "write_text_file" => "Escribir archivo".to_string(),
        "move_or_rename_path" => "Mover o renombrar archivo".to_string(),
        "run_powershell_command" => "Ejecutar en PowerShell".to_string(),
        "open_system_target" => "Abrir en el sistema".to_string(),
        "audit_repertoire" => "Auditar repertorio".to_string(),
        "curate_dj_crate" => "Ordenar crate armónico".to_string(),
        "export_dj_crate" => "Exportar crate DJ".to_string(),
        "analyze_audio_file" => "Analizar audio".to_string(),
        "detect_track_cues" => "Detectar 4 Hot Cues".to_string(),
        "build_dj_style_dna" => "Calcular perfil musical".to_string(),
        "record_golden_pair" => "Guardar pareja de oro".to_string(),
        "exclude_track_curation" => "Excluir pista de curación".to_string(),
        "record_dj_feedback" => "Aprender regla DJ".to_string(),
        _ => "Ejecutar herramienta".to_string(),
    }
}

pub fn summarize_tool_execution(
    tool_name: &str,
    args: &Value,
    result: Result<&Value, &str>,
) -> (String, String, String) {
    let title = human_tool_title(tool_name);
    match result {
        Err(err_msg) => (
            title,
            format!("Error: {}", truncate_chars(err_msg, 78)),
            format!("argumentos: {}\nerror: {}", args, err_msg),
        ),
        Ok(val) => match tool_name {
            "list_library_tracks" => {
                let count = val.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                (
                    title,
                    format!("{count} pistas en biblioteca local"),
                    format!("total: {count} pistas en hera.db"),
                )
            }
            "search_library_tracks" => {
                let count = val.get("matched").and_then(|v| v.as_u64()).unwrap_or(0);
                let q = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("catálogo completo");
                (
                    title,
                    format!("{count} pistas coincidentes ({q})"),
                    format!("criterio: {}\ncoincidencias: {count}", args),
                )
            }
            "import_audio_paths" => {
                let imported = val
                    .get("imported_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let scanned = val
                    .get("scanned_files")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                (
                    title,
                    format!("{imported} pistas con BPM, tono y 4 Hot Cues"),
                    format!("archivos escaneados: {scanned}\nnuevas pistas en SQLite: {imported}"),
                )
            }
            "create_playlist" => {
                let name = val
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Playlist");
                let count = val.get("track_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let keys = val
                    .get("key_flow")
                    .and_then(|v| v.as_str())
                    .unwrap_or("orden armónico");
                (
                    title,
                    format!("«{name}» ({count} pistas · {keys})"),
                    format!("playlist: {name}\npistas: {count}\nflujo: {keys}"),
                )
            }
            "push_virtualdj_live_crate" => {
                let crate_name = val
                    .get("crate_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("HERA En Vivo");
                let total = val
                    .get("total_tracks")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let first_file = val
                    .get("written_files")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or("HERA En Vivo.vdjfolder");
                (
                    title,
                    format!("{crate_name}.vdjfolder ({total} pistas)"),
                    format!(
                        "ruta: {first_file}\nestado: {total} pistas · {} Hot Cues",
                        total * 4
                    ),
                )
            }
            "control_audio_player" => {
                let act = args
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("play_track");
                let label = val
                    .get("track_label")
                    .and_then(|v| v.as_str())
                    .unwrap_or(act);
                (
                    title,
                    label.to_string(),
                    format!("acción: {act}\ndetalle: {label}"),
                )
            }
            "list_directory_entries" => {
                let count = val
                    .get("total_entries")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let audio = val.get("audio_files").and_then(|v| v.as_u64()).unwrap_or(0);
                let p = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                (
                    title,
                    format!(
                        "{} ({count} elementos · {audio} audios)",
                        short_path_label(p)
                    ),
                    format!("ruta: {p}\nelementos: {count} (archivos de audio: {audio})"),
                )
            }
            "run_powershell_command" => {
                let code = val.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0);
                let cmd_str = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
                let out_str = val.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
                (
                    title,
                    format!("{} (código {code})", truncate_chars(cmd_str, 46)),
                    format!(
                        "comando: {cmd_str}\nsalida:\n{}",
                        truncate_chars(
                            if out_str.is_empty() {
                                "(sin salida)"
                            } else {
                                out_str
                            },
                            420
                        )
                    ),
                )
            }
            "record_golden_pair" => {
                let from = val.get("from_track").and_then(|v| v.as_str()).unwrap_or("Pista A");
                let to = val.get("to_track").and_then(|v| v.as_str()).unwrap_or("Pista B");
                let bonus = val.get("bonus").and_then(|v| v.as_f64()).unwrap_or(0.38);
                (
                    title,
                    format!("Pareja guardada (+{bonus:.2})"),
                    format!("transición: {from} -> {to}\nbonus de sinergia: +{bonus:.2}"),
                )
            }
            "exclude_track_curation" => {
                let pat = val
                    .get("title_pattern")
                    .and_then(|v| v.as_str())
                    .or_else(|| val.get("track_id").and_then(|v| v.as_str()))
                    .unwrap_or("Pista");
                let reason = val.get("reason").and_then(|v| v.as_str()).unwrap_or("Exclusión DJ");
                (
                    title,
                    format!("Excluida: {pat}"),
                    format!("criterio: {pat}\nmotivo: {reason}"),
                )
            }
            "record_dj_feedback" => {
                let cat = val.get("category").and_then(|v| v.as_str()).unwrap_or("general");
                let rule = val.get("rule_text").and_then(|v| v.as_str()).unwrap_or("");
                (
                    title,
                    format!("Regla [{cat}]: {}", truncate_chars(rule, 36)),
                    format!("categoría: {cat}\nregla: {rule}"),
                )
            }
            _ => {
                let raw = val.to_string();
                (
                    title,
                    tool_name.replace('_', " "),
                    truncate_chars(&raw, 360),
                )
            }
        },
    }
}

pub fn short_path_label(p: &str) -> String {
    Path::new(p)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| truncate_chars(p, 32))
}

pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max_chars).collect::<String>())
    }
}

pub fn extract_quoted_name(text: &str) -> Option<String> {
    for (open, close) in [('«', '»'), ('"', '"'), ('\'', '\'')] {
        if let Some(start) = text.find(open) {
            let rest = &text[start + open.len_utf8()..];
            if let Some(end) = rest.find(close) {
                let candidate = rest[..end].trim();
                if !candidate.is_empty() && candidate.chars().count() <= 48 {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    for marker in ["llamada ", "llamado ", "nombre "] {
        if let Some(idx) = text.to_lowercase().find(marker) {
            let after = text[idx + marker.len()..].trim();
            let words: Vec<&str> = after
                .split_whitespace()
                .take_while(|w| {
                    let lw = w.to_lowercase();
                    !matches!(
                        lw.as_str(),
                        "con" | "y" | "en" | "para" | "ordenada" | "ordenado"
                    )
                })
                .take(4)
                .collect();
            let joined = words
                .join(" ")
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_string();
            if !joined.is_empty() {
                return Some(joined);
            }
        }
    }
    None
}

pub fn derive_session_title(message: &str) -> String {
    let cleaned = message
        .trim()
        .trim_start_matches('/')
        .replace(['\n', '\r'], " ");
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "Nueva sesión".to_string()
    } else {
        truncate_chars(trimmed, 36)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_human_tool_title_and_summarize_tool_execution() {
        assert_eq!(human_tool_title("create_playlist"), "Crear playlist");
        assert_eq!(
            human_tool_title("run_powershell_command"),
            "Ejecutar en PowerShell"
        );
        let ok_val = json!({ "name": "Warmup", "track_count": 4, "key_flow": "8A → 9A" });
        let (title, summary, receipt) =
            summarize_tool_execution("create_playlist", &json!({}), Ok(&ok_val));
        assert_eq!(title, "Crear playlist");
        assert!(summary.contains("Warmup") && receipt.contains("8A → 9A"));

        let (_, err_summary, _) =
            summarize_tool_execution("read_text_file", &json!({}), Err("404"));
        assert!(err_summary.contains("Error: 404"));
        assert_eq!(
            extract_quoted_name("Arma «Set Cabina»"),
            Some("Set Cabina".to_string())
        );
        assert_eq!(derive_session_title("/set Afro House"), "set Afro House");
    }
}
