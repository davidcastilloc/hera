#[cfg(not(target_arch = "wasm32"))]
use super::cli_runner::run_command_with_timeout;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
enum LocalHttpError {
    Unreachable(String),
    InvalidResponse,
}

#[cfg(not(target_arch = "wasm32"))]
fn normalize_loopback_host_port(host_port: &str) -> String {
    if let Some(port) = host_port.strip_prefix("localhost:") {
        format!("127.0.0.1:{port}")
    } else if host_port.eq_ignore_ascii_case("localhost") {
        "127.0.0.1:80".to_string()
    } else {
        host_port.to_string()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn http_get_json_local(
    host_port: &str,
    path: &str,
    api_key: Option<&str>,
) -> Result<serde_json::Value, LocalHttpError> {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let connect_target = normalize_loopback_host_port(host_port);
    let connect_timeout = Duration::from_millis(200);
    let rw_timeout = Duration::from_millis(500);
    let addr = connect_target
        .to_socket_addrs()
        .map_err(|e| LocalHttpError::Unreachable(format!("Invalid address `{host_port}`: {e}")))?
        .next()
        .ok_or_else(|| LocalHttpError::Unreachable(format!("Could not resolve `{host_port}`")))?;
    let mut stream = TcpStream::connect_timeout(&addr, connect_timeout).map_err(|e| {
        LocalHttpError::Unreachable(format!("Connect timeout/error to `{host_port}`: {e}"))
    })?;
    let _ = stream.set_read_timeout(Some(rw_timeout));
    let _ = stream.set_write_timeout(Some(rw_timeout));

    let auth_header = api_key
        .filter(|k| !k.trim().is_empty())
        .map(|k| format!("Authorization: Bearer {}\r\n", k.trim()))
        .unwrap_or_default();
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {host_port}\r\nAccept: application/json\r\n{auth_header}Connection: close\r\n\r\n"
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|_| LocalHttpError::InvalidResponse)?;

    let mut raw = Vec::new();
    let _ = stream.read_to_end(&mut raw);
    let text = String::from_utf8_lossy(&raw);
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b)
        .or_else(|| text.split_once("\n\n").map(|(_, b)| b))
        .unwrap_or(&text)
        .trim();
    let json_slice = match (body.find('{'), body.rfind('}')) {
        (Some(start), Some(end)) if end >= start => &body[start..=end],
        _ => body,
    };
    serde_json::from_str(json_slice).map_err(|_| LocalHttpError::InvalidResponse)
}

#[cfg(not(target_arch = "wasm32"))]
fn extract_models_from_payload(val: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arr) = val.get("data").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                if !id.to_ascii_lowercase().contains("embed") {
                    out.push(id.to_string());
                }
            }
        }
    } else if let Some(arr) = val.get("models").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(name) = item
                .get("name")
                .or_else(|| item.get("model"))
                .and_then(|v| v.as_str())
            {
                if !name.to_ascii_lowercase().contains("embed") {
                    out.push(name.to_string());
                }
            }
        }
    }
    out
}

pub fn probe_backend_models(base_url: &str, api_key: Option<&str>) -> Result<Vec<String>, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (base_url, api_key);
        Err("Local TCP probing is only supported on native target".to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let trimmed = base_url.trim().trim_end_matches('/');
        let without_scheme = trimmed
            .strip_prefix("http://")
            .or_else(|| trimmed.strip_prefix("https://"))
            .unwrap_or(trimmed);
        let (host_port_raw, base_path) = match without_scheme.split_once('/') {
            Some((h, p)) => (h, format!("/{p}")),
            None => (without_scheme, String::new()),
        };
        let host_port = if host_port_raw.contains(':') {
            host_port_raw.to_string()
        } else {
            format!("{host_port_raw}:80")
        };
        let models_path = if base_path.ends_with("/models") {
            base_path
        } else if base_path.ends_with("/v1") {
            format!("{base_path}/models")
        } else if base_path.is_empty() {
            "/v1/models".to_string()
        } else {
            format!("{base_path}/v1/models")
        };

        match http_get_json_local(&host_port, &models_path, api_key) {
            Ok(json_val) => {
                let models = extract_models_from_payload(&json_val);
                if !models.is_empty() {
                    return Ok(models);
                }
            }
            Err(LocalHttpError::Unreachable(err)) => {
                return Err(err);
            }
            Err(LocalHttpError::InvalidResponse) => {}
        }

        if let Ok(json_val) = http_get_json_local(&host_port, "/api/tags", api_key) {
            let models = extract_models_from_payload(&json_val);
            if !models.is_empty() {
                return Ok(models);
            }
        }
        Err(format!("No active models discovered at `{base_url}`"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn http_post_chat_completions(
    base_url: &str,
    api_key: Option<&str>,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let trimmed = base_url.trim().trim_end_matches('/');
    let chat_url = if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/chat/completions")
    };
    let body_str = serde_json::to_string(payload).map_err(|e| e.to_string())?;

    if let Some(without_http) = chat_url.strip_prefix("http://") {
        let (host_port_raw, path) = match without_http.split_once('/') {
            Some((h, p)) => (h, format!("/{p}")),
            None => (without_http, "/v1/chat/completions".to_string()),
        };
        let host_port = if host_port_raw.contains(':') {
            host_port_raw.to_string()
        } else {
            format!("{host_port_raw}:80")
        };
        let connect_target = normalize_loopback_host_port(&host_port);
        let is_loopback = connect_target.starts_with("127.0.0.1:");
        let connect_timeout = if is_loopback {
            Duration::from_millis(500)
        } else {
            Duration::from_millis(1500)
        };
        let addr = connect_target
            .to_socket_addrs()
            .map_err(|e| format!("Invalid address `{host_port}`: {e}"))?
            .next()
            .ok_or_else(|| format!("Could not resolve `{host_port}`"))?;
        let mut stream = TcpStream::connect_timeout(&addr, connect_timeout)
            .map_err(|e| format!("Connect error to `{host_port}`: {e}"))?;
        let _ = stream.set_read_timeout(Some(Duration::from_secs(12)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(12)));

        let auth_header = api_key
            .filter(|k| !k.trim().is_empty())
            .map(|k| format!("Authorization: Bearer {}\r\n", k.trim()))
            .unwrap_or_default();
        let req = format!(
            "POST {path} HTTP/1.1\r\nHost: {host_port}\r\nContent-Type: application/json\r\nAccept: application/json\r\n{auth_header}Content-Length: {}\r\nConnection: close\r\n\r\n{body_str}",
            body_str.len()
        );
        stream
            .write_all(req.as_bytes())
            .map_err(|e| format!("Write failed to `{host_port}`: {e}"))?;

        let mut raw = Vec::new();
        let mut chunk = [0u8; 4096];
        let mut expected_total = None;
        while let Ok(n) = stream.read(&mut chunk) {
            if n == 0 {
                break;
            }
            raw.extend_from_slice(&chunk[..n]);
            if expected_total.is_none() {
                let s = String::from_utf8_lossy(&raw);
                if let Some(hdr_end) = s.find("\r\n\r\n") {
                    if let Some(cl) = s[..hdr_end].lines().find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse::<usize>().ok())
                    }) {
                        expected_total = Some(hdr_end + 4 + cl);
                    }
                }
            }
            if let Some(tot) = expected_total {
                if raw.len() >= tot {
                    break;
                }
            }
        }
        let text = String::from_utf8_lossy(&raw);
        let body = text
            .split_once("\r\n\r\n")
            .map(|(_, b)| b)
            .or_else(|| text.split_once("\n\n").map(|(_, b)| b))
            .unwrap_or(&text)
            .trim();
        let json_slice = match (body.find('{'), body.rfind('}')) {
            (Some(s), Some(e)) if e >= s => &body[s..=e],
            _ => body,
        };
        return serde_json::from_str(json_slice)
            .map_err(|e| format!("Invalid JSON from `{chat_url}`: {e}"));
    }

    let mut cmd = std::process::Command::new("curl");
    cmd.args([
        "-sS",
        "--max-time",
        "12",
        "-X",
        "POST",
        &chat_url,
        "-H",
        "Content-Type: application/json",
    ]);
    if let Some(k) = api_key.filter(|k| !k.trim().is_empty()) {
        cmd.args(["-H", &format!("Authorization: Bearer {}", k.trim())]);
    }
    cmd.args(["-d", &body_str]);
    let out = run_command_with_timeout(cmd, std::time::Duration::from_secs(14))
        .map_err(|e| format!("curl execution failed: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "HTTP POST failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("Invalid JSON response: {e}"))
}
