#[cfg(not(target_arch = "wasm32"))]
fn configure_background_command(cmd: &mut std::process::Command) {
    cmd.stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn run_command_with_timeout(
    mut cmd: std::process::Command,
    timeout: std::time::Duration,
) -> Result<std::process::Output, String> {
    use std::io::Read;
    use std::process::Stdio;
    use std::time::Instant;

    configure_background_command(&mut cmd);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let out_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut pipe) = stdout_pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut pipe) = stderr_pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "Tiempo de espera agotado ({:.1}s) esperando salida del proceso CLI",
                        timeout.as_secs_f32()
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Err(e) => {
                let _ = child.kill();
                return Err(format!("Fallo monitoreando proceso CLI: {e}"));
            }
        }
    };

    let stdout = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();
    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

pub fn check_binary_in_path(bin_name: &str) -> (bool, Option<String>) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = bin_name;
        (false, None)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let probe_cmd = if cfg!(windows) { "where" } else { "which" };
        let mut cmd = std::process::Command::new(probe_cmd);
        cmd.arg(bin_name);
        if let Ok(out) = run_command_with_timeout(cmd, std::time::Duration::from_millis(1200)) {
            if out.status.success() {
                let raw = String::from_utf8_lossy(&out.stdout);
                let lines: Vec<&str> = raw
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .collect();
                let preferred = if cfg!(windows) {
                    lines
                        .iter()
                        .copied()
                        .find(|l| {
                            let lower = l.to_ascii_lowercase();
                            lower.ends_with(".exe")
                                || lower.ends_with(".cmd")
                                || lower.ends_with(".bat")
                        })
                        .or_else(|| lines.first().copied())
                } else {
                    lines.first().copied()
                };
                if let Some(resolved) = preferred {
                    return (true, Some(resolved.to_string()));
                }
            }
        }
        (false, None)
    }
}

pub fn build_cli_agent_args(
    bin_name: &str,
    combined_prompt: &str,
    raw_prompt: &str,
) -> Vec<String> {
    match bin_name {
        "agy" => vec![
            "--disable-slash-commands".to_string(),
            "--effort".to_string(),
            "low".to_string(),
            "--print-timeout".to_string(),
            "25s".to_string(),
            "-p".to_string(),
            combined_prompt.to_string(),
        ],
        "claude" => vec!["-p".to_string(), combined_prompt.to_string()],
        "codex" => vec!["exec".to_string(), combined_prompt.to_string()],
        "gemini" => vec!["--prompt".to_string(), combined_prompt.to_string()],
        "ollama" => vec![
            "run".to_string(),
            "llama3.3".to_string(),
            combined_prompt.to_string(),
        ],
        _ => vec![raw_prompt.to_string()],
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn execute_cli_agent_turn(
    cli_id: &str,
    prompt: &str,
    system_prompt: &str,
) -> Result<String, String> {
    let bin_name = match cli_id {
        "cli_agy" => "agy",
        "cli_claude" => "claude",
        "cli_codex" => "codex",
        "cli_gemini" => "gemini",
        "cli_ollama" => "ollama",
        _ => return Err(format!("Unknown CLI agent ID `{cli_id}`")),
    };

    let (installed, path_hint) = check_binary_in_path(bin_name);
    if !installed {
        return Err(format!(
            "El binario `{bin_name}` no fue encontrado en el PATH del sistema operativo."
        ));
    }

    if cfg!(test) {
        return Err(format!(
            "Modo de pruebas unitarias offline activo para `{bin_name}`."
        ));
    }

    let exec_target = path_hint.as_deref().unwrap_or(bin_name);
    let mut cmd = std::process::Command::new(exec_target);
    let combined_prompt = format!("{system_prompt}\n\nUsuario: {prompt}");
    cmd.args(build_cli_agent_args(bin_name, &combined_prompt, prompt));

    let out = run_command_with_timeout(cmd, std::time::Duration::from_secs(26)).map_err(|e| {
        format!(
            "Fallo al ejecutar binario `{bin_name}` (path {:?}): {e}",
            path_hint
        )
    })?;

    if out.status.success() {
        let stdout_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !stdout_str.is_empty() {
            return Ok(stdout_str);
        }
    }

    let stderr_str = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if !stderr_str.is_empty() {
        Err(format!("Error desde `{bin_name}`: {stderr_str}"))
    } else {
        Err(format!("Salida vacía desde `{bin_name}`"))
    }
}
