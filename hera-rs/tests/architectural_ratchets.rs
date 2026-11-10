use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Canonical mapping of Tauri v2 command-bearing internal plugins:
/// `(plugin_source_selector_in_plugins_dir, rust_module_ident, tauri_plugin_acl_name)`.
/// Selectors ending with `/` match directory plugins recursively (`plugins/<dir>/**/*.rs`).
const COMMAND_BEARING_PLUGINS: &[(&str, &str, &str)] = &[
    ("app_menu.rs", "app_menu", "app-menu"),
    ("config.rs", "config", "config"),
    ("cover.rs", "cover", "cover"),
    ("db.rs", "db", "database"),
    ("default_view.rs", "default_view", "default-view"),
    ("hera_dj/", "hera_dj", "hera-dj"),
    ("hera_agent/", "hera_agent", "hera-agent"),
    ("sleepblocker.rs", "sleepblocker", "sleepblocker"),
];

/// Non-command internal plugin files in `hera-desktop/src-tauri/src/plugins/`
/// verified explicitly to exist and contain `0` `#[tauri::command]` functions.
const ZERO_COMMAND_PLUGIN_FILES: &[&str] = &[
    "mod.rs",
    "app_close.rs",
    "debug.rs",
    "file_associations.rs",
    "stream_server.rs",
];

/// Exact count of `#[tauri::command]` functions across the 8 internal plugins in `v0.6.0`.
const EXPECTED_TAURI_COMMANDS_COUNT: usize = 57;

/// Strict line-count ceilings (`content.lines().count()`) for module facades,
/// pre-existing ADR-013 P2P submodules, route coordinators, and IPC bridges.
const FILE_LINE_CEILINGS: &[(&str, usize)] = &[
    ("hera-rs/src/p2p.rs", 349),
    ("hera-rs/src/p2p/swarm_custody.rs", 349),
    ("hera-rs/src/p2p/broker.rs", 481),
    ("hera-rs/src/p2p/cloud_links.rs", 272),
    ("hera-rs/src/p2p/crypto_merkle.rs", 628),
    ("hera-rs/src/p2p/nat_stun.rs", 863),
    ("hera-desktop/src/routes/bridge.tsx", 450),
    ("hera-desktop/src/components/MiniOverlay.tsx", 450),
    ("hera-desktop/src/lib/bridge-hera.ts", 300),
    ("hera-desktop/src/lib/bridge-agent.ts", 300),
    ("hera-desktop/src/lib/native-audio-transport.ts", 280),
    ("hera-desktop/src/lib/player.ts", 680),
];

/// Recursive line-count ceilings (`<= 349` lines) enforced on every `.rs` file inside
/// Leaf-First sliced Rust module directories across the `v0.6.0` workspace.
const SLICED_SUBMODULE_DIRS: &[(&str, usize)] = &[
    ("hera-rs/src/mcp", 349),
    ("hera-rs/src/code_mode", 349),
    ("hera-rs/src/dj_brain", 349),
    ("hera-rs/src/p2p/swarm_custody", 349),
    ("hera-rs/src/p2p", 349),
    ("hera-rs/src/audio_engine", 349),
    ("hera-desktop/src-ipc/src", 349),
    ("hera-desktop/src-tauri/src/plugins/hera_dj", 349),
    ("hera-desktop/src-tauri/src/plugins/hera_agent", 349),
];

/// Recursive line-count ceilings (`<= 450` lines) enforced on every `.ts` and `.tsx` file
/// inside Leaf-First sliced frontend component directories (`Transición` & `Modo Mini`).
const SLICED_FRONTEND_DIRS: &[(&str, usize)] = &[
    ("hera-desktop/src/components/transition", 450),
    ("hera-desktop/src/components/mini-overlay", 450),
];

/// Pre-existing ADR-013 (`[RESUELTO — OMITIR]`) submodules in `hera-rs/src/p2p` whose exact
/// line-count ceilings are locked individually in `FILE_LINE_CEILINGS`.
const ADR013_PREEXISTING_P2P_FILES: &[&str] = &[
    "hera-rs/src/p2p/broker.rs",
    "hera-rs/src/p2p/cloud_links.rs",
    "hera-rs/src/p2p/crypto_merkle.rs",
    "hera-rs/src/p2p/nat_stun.rs",
];

/// Exact ceiling of manual `export type Name =` / `export interface Name` declarations in
/// `hera-desktop/src/lib/bridge-hera.ts` (only pure local TS union/derived types).
const MAX_MANUAL_EXPORTED_TYPES_IN_BRIDGE_HERA: usize = 3;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn selector_matches_rel_path(selector: &str, rel_path: &str) -> bool {
    if selector.ends_with('/') {
        rel_path.starts_with(selector)
    } else {
        rel_path == selector
    }
}

/// Strips Rust line comments (`// ...`) and nested block comments (`/* ... */`),
/// replacing stripped characters with spaces while preserving `\n` line boundaries.
/// When `strip_strings` is `true`, string and character literals are also blanked to spaces
/// while keeping Rust lifetimes (`'a`, `'_`, `'static`) intact.
fn strip_rust_comments(src: &str, strip_strings: bool) -> String {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        // 1. Line comment `//`
        if src[i..].starts_with("//") {
            while i < len {
                let ch = src[i..].chars().next().expect("valid utf8");
                i += ch.len_utf8();
                if ch == '\n' {
                    out.push('\n');
                    break;
                }
                out.push(' ');
            }
            continue;
        }

        // 2. Nested block comment `/* ... */`
        if src[i..].starts_with("/*") {
            let mut depth = 1usize;
            out.push_str("  ");
            i += 2;
            while i < len && depth > 0 {
                if src[i..].starts_with("/*") {
                    depth += 1;
                    out.push_str("  ");
                    i += 2;
                } else if src[i..].starts_with("*/") {
                    depth -= 1;
                    out.push_str("  ");
                    i += 2;
                } else {
                    let ch = src[i..].chars().next().expect("valid utf8");
                    out.push(if ch == '\n' { '\n' } else { ' ' });
                    i += ch.len_utf8();
                }
            }
            continue;
        }

        // 3. Raw string literal `r"..."`, `r#"..."#`, `br#"..."#`, `cr#"..."#`
        if bytes[i] == b'r' {
            let valid_raw_prefix = i == 0
                || !is_ident_continue(bytes[i - 1])
                || ((bytes[i - 1] == b'b' || bytes[i - 1] == b'c')
                    && (i == 1 || !is_ident_continue(bytes[i - 2])));
            if valid_raw_prefix {
                let mut hashes = 0usize;
                while i + 1 + hashes < len && bytes[i + 1 + hashes] == b'#' {
                    hashes += 1;
                }
                if i + 1 + hashes < len && bytes[i + 1 + hashes] == b'"' {
                    let open_len = 1 + hashes + 1;
                    if strip_strings {
                        for _ in 0..open_len {
                            out.push(' ');
                        }
                    } else {
                        out.push_str(&src[i..i + open_len]);
                    }
                    i += open_len;

                    while i < len {
                        if bytes[i] == b'"'
                            && i + 1 + hashes <= len
                            && bytes[i + 1..i + 1 + hashes].iter().all(|&b| b == b'#')
                        {
                            let close_len = 1 + hashes;
                            if strip_strings {
                                for _ in 0..close_len {
                                    out.push(' ');
                                }
                            } else {
                                out.push_str(&src[i..i + close_len]);
                            }
                            i += close_len;
                            break;
                        }
                        let ch = src[i..].chars().next().expect("valid utf8");
                        if strip_strings {
                            out.push(if ch == '\n' { '\n' } else { ' ' });
                        } else {
                            out.push(ch);
                        }
                        i += ch.len_utf8();
                    }
                    continue;
                }
            }
        }

        // 4. Character literal (`'c'` or `'\...'`) vs lifetime (`'a`, `'_`)
        if bytes[i] == b'\'' && i + 1 < len {
            if bytes[i + 1] == b'\\' {
                let mut end = i + 2;
                let mut found_closing = None;
                while end < len && end - i <= 12 && bytes[end] != b'\n' {
                    if bytes[end] == b'\'' {
                        found_closing = Some(end + 1);
                        break;
                    }
                    end += 1;
                }
                if let Some(next_i) = found_closing {
                    if strip_strings {
                        for _ in 0..src[i..next_i].chars().count() {
                            out.push(' ');
                        }
                    } else {
                        out.push_str(&src[i..next_i]);
                    }
                    i = next_i;
                    continue;
                }
            } else if let Some(ch) = src[i + 1..].chars().next() {
                let after_ch = i + 1 + ch.len_utf8();
                if ch != '\''
                    && ch != '\\'
                    && ch != '\n'
                    && after_ch < len
                    && bytes[after_ch] == b'\''
                {
                    let next_i = after_ch + 1;
                    if strip_strings {
                        out.push_str("   ");
                    } else {
                        out.push_str(&src[i..next_i]);
                    }
                    i = next_i;
                    continue;
                }
            }
        }

        // 5. Standard string literal `"..."`
        if bytes[i] == b'"' {
            if strip_strings {
                out.push(' ');
            } else {
                out.push('"');
            }
            i += 1;
            while i < len {
                if bytes[i] == b'\\' && i + 1 < len {
                    let escaped = src[i + 1..].chars().next().expect("valid utf8");
                    if strip_strings {
                        out.push(' ');
                        out.push(if escaped == '\n' { '\n' } else { ' ' });
                    } else {
                        out.push('\\');
                        out.push(escaped);
                    }
                    i += 1 + escaped.len_utf8();
                } else if bytes[i] == b'"' {
                    if strip_strings {
                        out.push(' ');
                    } else {
                        out.push('"');
                    }
                    i += 1;
                    break;
                } else {
                    let ch = src[i..].chars().next().expect("valid utf8");
                    if strip_strings {
                        out.push(if ch == '\n' { '\n' } else { ' ' });
                    } else {
                        out.push(ch);
                    }
                    i += ch.len_utf8();
                }
            }
            continue;
        }

        let ch = src[i..].chars().next().expect("valid utf8");
        out.push(ch);
        i += ch.len_utf8();
    }

    out
}

fn find_matching_bracket_in_clean(
    src: &str,
    open_idx: usize,
    open: u8,
    close: u8,
) -> Result<usize, String> {
    let bytes = src.as_bytes();
    if bytes.get(open_idx) != Some(&open) {
        return Err(format!("Expected '{}' at byte {open_idx}", open as char));
    }
    let mut depth = 1usize;
    let mut i = open_idx + 1;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    let ch = src[i..].chars().next().expect("valid utf8");
                    i += ch.len_utf8();
                }
            }
            continue;
        }
        if bytes[i] == open {
            depth += 1;
        } else if bytes[i] == close {
            depth -= 1;
            if depth == 0 {
                return Ok(i);
            }
        }
        let ch = src[i..].chars().next().expect("valid utf8");
        i += ch.len_utf8();
    }
    Err(format!(
        "Unclosed delimiter '{}' opened at byte {open_idx}",
        open as char
    ))
}

/// Extracts every `fn <cmd_name>` annotated with `#[tauri::command]` or `#[tauri::command(...)]`,
/// cleanly handling stacked attributes, visibility (`pub`, `pub(crate)`), and `async`/`unsafe`/`const`
/// modifiers across multiple lines while ignoring comments and string literals.
fn extract_tauri_commands(source: &str) -> Result<Vec<String>, String> {
    let cleaned = strip_rust_comments(source, true);
    let bytes = cleaned.as_bytes();
    let len = bytes.len();
    let mut commands = Vec::new();
    let mut pos = 0usize;

    while let Some(rel) = cleaned[pos..].find("#[") {
        let attr_start = pos + rel;
        let attr_end = find_matching_bracket_in_clean(&cleaned, attr_start + 1, b'[', b']')?;
        let attr_inner = &cleaned[attr_start + 2..attr_end];
        let compact: String = attr_inner.chars().filter(|c| !c.is_whitespace()).collect();
        pos = attr_end + 1;

        let is_tauri_cmd =
            compact == "tauri::command" || compact.starts_with("tauri::command(");
        if !is_tauri_cmd {
            continue;
        }

        loop {
            while pos < len && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if pos >= len {
                return Err("Hit EOF after #[tauri::command] before `fn <name>`".to_string());
            }
            if cleaned[pos..].starts_with("#[") {
                let next_end = find_matching_bracket_in_clean(&cleaned, pos + 1, b'[', b']')?;
                pos = next_end + 1;
                continue;
            }
            if !is_ident_start(bytes[pos]) {
                return Err(format!(
                    "Unexpected syntax near `{}` after #[tauri::command]",
                    cleaned[pos..].chars().take(24).collect::<String>().trim()
                ));
            }
            let tok_start = pos;
            while pos < len && is_ident_continue(bytes[pos]) {
                pos += 1;
            }
            let tok = &cleaned[tok_start..pos];
            match tok {
                "pub" => {
                    while pos < len && bytes[pos].is_ascii_whitespace() {
                        pos += 1;
                    }
                    if pos < len && bytes[pos] == b'(' {
                        let close_vis =
                            find_matching_bracket_in_clean(&cleaned, pos, b'(', b')')?;
                        pos = close_vis + 1;
                    }
                }
                "async" | "unsafe" | "const" | "extern" => {}
                "fn" => {
                    while pos < len && bytes[pos].is_ascii_whitespace() {
                        pos += 1;
                    }
                    if cleaned[pos..].starts_with("r#") {
                        pos += 2;
                    }
                    if pos >= len || !is_ident_start(bytes[pos]) {
                        return Err("Expected function identifier after `fn`".to_string());
                    }
                    let fn_start = pos;
                    while pos < len && is_ident_continue(bytes[pos]) {
                        pos += 1;
                    }
                    commands.push(cleaned[fn_start..pos].to_string());
                    break;
                }
                other => {
                    return Err(format!(
                        "#[tauri::command] must annotate a `fn`, found `{other}` instead"
                    ));
                }
            }
        }
    }

    Ok(commands)
}

fn extract_string_literals(slice: &str) -> Result<Vec<String>, String> {
    let bytes = slice.as_bytes();
    let len = bytes.len();
    let mut literals = Vec::new();
    let mut i = 0usize;

    while i < len {
        if bytes[i] == b'"' {
            i += 1;
            let mut value = String::new();
            let mut closed = false;
            while i < len {
                if bytes[i] == b'\\' && i + 1 < len {
                    let ch = slice[i + 1..].chars().next().expect("valid utf8");
                    value.push(ch);
                    i += 1 + ch.len_utf8();
                } else if bytes[i] == b'"' {
                    i += 1;
                    closed = true;
                    break;
                } else {
                    let ch = slice[i..].chars().next().expect("valid utf8");
                    value.push(ch);
                    i += ch.len_utf8();
                }
            }
            if !closed {
                return Err("Unclosed string literal while parsing build.rs".to_string());
            }
            literals.push(value);
        } else {
            let ch = slice[i..].chars().next().expect("valid utf8");
            i += ch.len_utf8();
        }
    }

    Ok(literals)
}

/// Parses `.plugin("<plugin-name>", tauri_build::InlinedPlugin::new().commands(&[ ... ]))`
/// blocks from `hera-desktop/src-tauri/build.rs`.
fn parse_build_rs_inlined_plugins(
    build_rs_source: &str,
) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let cleaned = strip_rust_comments(build_rs_source, false);
    let bytes = cleaned.as_bytes();
    let len = bytes.len();
    let mut plugins = BTreeMap::new();
    let mut pos = 0usize;

    while let Some(rel) = cleaned[pos..].find(".plugin") {
        let dot_idx = pos + rel;
        let mut cursor = dot_idx + ".plugin".len();
        if cursor < len && is_ident_continue(bytes[cursor]) {
            pos = cursor;
            continue;
        }
        while cursor < len && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= len || bytes[cursor] != b'(' {
            pos = cursor;
            continue;
        }

        let close_paren = find_matching_bracket_in_clean(&cleaned, cursor, b'(', b')')?;
        let plugin_args = &cleaned[cursor + 1..close_paren];
        pos = close_paren + 1;

        let first_quote = plugin_args
            .find('"')
            .ok_or_else(|| "Expected plugin name string literal in `.plugin(...)`".to_string())?;
        let first_lits = extract_string_literals(&plugin_args[first_quote..])?;
        let plugin_name = first_lits
            .first()
            .cloned()
            .ok_or_else(|| "Missing plugin name literal in `.plugin(...)`".to_string())?;

        let compact_args: String = plugin_args.chars().filter(|c| !c.is_whitespace()).collect();
        if !compact_args.contains("InlinedPlugin::new()") {
            return Err(format!(
                "Plugin `{plugin_name}` in build.rs does not construct `InlinedPlugin::new()`"
            ));
        }

        let cmd_call_idx = plugin_args.find(".commands").ok_or_else(|| {
            format!("Plugin `{plugin_name}` in build.rs is missing `.commands(&[...])`")
        })?;
        let mut cmd_paren = cmd_call_idx + ".commands".len();
        while cmd_paren < plugin_args.len()
            && plugin_args.as_bytes()[cmd_paren].is_ascii_whitespace()
        {
            cmd_paren += 1;
        }
        let cmd_close = find_matching_bracket_in_clean(plugin_args, cmd_paren, b'(', b')')?;
        let cmd_list = extract_string_literals(&plugin_args[cmd_paren + 1..cmd_close])?;

        let mut cmd_set = BTreeSet::new();
        for cmd in cmd_list {
            if !cmd_set.insert(cmd.clone()) {
                return Err(format!(
                    "Duplicate command `{cmd}` listed for plugin `{plugin_name}` in build.rs"
                ));
            }
        }

        if plugins.insert(plugin_name.clone(), cmd_set).is_some() {
            return Err(format!(
                "Duplicate `.plugin(\"{plugin_name}\", ...)` registration in build.rs"
            ));
        }
    }

    Ok(plugins)
}

fn parse_capabilities_permissions(capabilities_json_source: &str) -> Result<Vec<String>, String> {
    let parsed: Value = serde_json::from_str(capabilities_json_source)
        .map_err(|err| format!("Invalid JSON in capabilities/main.json: {err}"))?;
    let arr = parsed
        .get("permissions")
        .and_then(Value::as_array)
        .ok_or_else(|| "capabilities/main.json is missing a `permissions` array".to_string())?;

    let mut permissions = Vec::with_capacity(arr.len());
    let mut seen = BTreeSet::new();
    for entry in arr {
        let perm = entry
            .as_str()
            .ok_or_else(|| {
                "Non-string entry in capabilities/main.json `permissions`".to_string()
            })?;
        if !seen.insert(perm.to_string()) {
            return Err(format!(
                "Duplicate permission `{perm}` in capabilities/main.json"
            ));
        }
        permissions.push(perm.to_string());
    }
    Ok(permissions)
}

/// Audits exact 1:1:1 Tauri v2 ACL triad parity across:
/// 1. `src-tauri/src/plugins/**/*.rs` (both single-file and directory plugins, `#[tauri::command]`)
/// 2. `src-tauri/build.rs` (`InlinedPlugin::new().commands(&[...])`)
/// 3. `src-tauri/capabilities/main.json` (`"<plugin>:allow-<kebab-cmd>"`)
/// 4. `src-tauri/src/main.rs` (`.plugin(plugins::<rust_mod>::init(...)`)
fn audit_tauri_acl_triad(
    plugin_sources: &BTreeMap<String, String>,
    build_rs_source: &str,
    capabilities_json_source: &str,
    main_rs_source: &str,
) -> Result<usize, Vec<String>> {
    let mut violations = Vec::new();

    let expected_plugin_names: BTreeSet<String> = COMMAND_BEARING_PLUGINS
        .iter()
        .map(|(_, _, name)| (*name).to_string())
        .collect();

    // 1. Audit non-triad `.rs` files in `src/plugins/` so no unmapped file defines `#[tauri::command]`.
    for (rel_file, source) in plugin_sources {
        let mapped = COMMAND_BEARING_PLUGINS
            .iter()
            .any(|(selector, _, _)| selector_matches_rel_path(selector, rel_file));
        if !mapped {
            match extract_tauri_commands(source) {
                Ok(cmds) if !cmds.is_empty() => {
                    violations.push(format!(
                        "Unmapped plugin file `hera-desktop/src-tauri/src/plugins/{rel_file}` defines \
                         #[tauri::command] functions {cmds:?} without being registered in COMMAND_BEARING_PLUGINS"
                    ));
                }
                Err(err) => {
                    violations.push(format!(
                        "Failed to parse `hera-desktop/src-tauri/src/plugins/{rel_file}`: {err}"
                    ));
                }
                Ok(_) => {}
            }
        }
    }

    // 2. Parse `build.rs` and `capabilities/main.json`.
    let build_plugins = match parse_build_rs_inlined_plugins(build_rs_source) {
        Ok(map) => map,
        Err(err) => {
            violations.push(err);
            BTreeMap::new()
        }
    };
    for build_plugin_name in build_plugins.keys() {
        if !expected_plugin_names.contains(build_plugin_name) {
            violations.push(format!(
                "build.rs registers unknown inlined plugin `{build_plugin_name}` not present in COMMAND_BEARING_PLUGINS"
            ));
        }
    }

    let capabilities_perms = match parse_capabilities_permissions(capabilities_json_source) {
        Ok(perms) => perms,
        Err(err) => {
            violations.push(err);
            Vec::new()
        }
    };

    let main_cleaned = strip_rust_comments(main_rs_source, true);
    let main_compact: String = main_cleaned.chars().filter(|c| !c.is_whitespace()).collect();

    let mut total_commands = 0usize;

    // 3. Verify 1:1 parity per command-bearing plugin (single-file or directory plugin).
    for &(selector, rust_mod, plugin_name) in COMMAND_BEARING_PLUGINS {
        let matching_sources: Vec<(&String, &String)> = plugin_sources
            .iter()
            .filter(|(rel, _)| selector_matches_rel_path(selector, rel))
            .collect();

        if matching_sources.is_empty() {
            violations.push(format!(
                "Missing expected plugin source matching `hera-desktop/src-tauri/src/plugins/{selector}`"
            ));
            continue;
        }

        let mut rust_cmd_set = BTreeSet::new();
        for (rel_file, src) in matching_sources {
            match extract_tauri_commands(src) {
                Ok(cmds) => {
                    for cmd in cmds {
                        if !rust_cmd_set.insert(cmd.clone()) {
                            violations.push(format!(
                                "Duplicate #[tauri::command] `{cmd}` in `{rel_file}` (`{selector}`)"
                            ));
                        }
                    }
                }
                Err(err) => {
                    violations.push(format!("Failed to parse `{rel_file}`: {err}"));
                }
            }
        }

        if rust_cmd_set.is_empty() {
            violations.push(format!(
                "Expected at least 1 #[tauri::command] in `{selector}`, found 0"
            ));
        }
        total_commands += rust_cmd_set.len();

        // Leg A: `build.rs` InlinedPlugin::new().commands(&[...])
        match build_plugins.get(plugin_name) {
            Some(build_cmds) => {
                if &rust_cmd_set != build_cmds {
                    let missing_in_build: Vec<_> = rust_cmd_set.difference(build_cmds).collect();
                    let orphan_in_build: Vec<_> = build_cmds.difference(&rust_cmd_set).collect();
                    violations.push(format!(
                        "build.rs triad drift for plugin `{plugin_name}` (`{selector}`): \
                         missing_in_build_rs={missing_in_build:?}, stale_in_build_rs={orphan_in_build:?}"
                    ));
                }
            }
            None => {
                violations.push(format!(
                    "build.rs is missing `.plugin(\"{plugin_name}\", InlinedPlugin::new().commands(...))`"
                ));
            }
        }

        // Leg B: `capabilities/main.json` "<plugin-name>:allow-<kebab-cmd>"
        let expected_caps: BTreeSet<String> = rust_cmd_set
            .iter()
            .map(|cmd| format!("{plugin_name}:allow-{}", cmd.replace('_', "-")))
            .collect();
        let prefix = format!("{plugin_name}:");
        let actual_caps: BTreeSet<String> = capabilities_perms
            .iter()
            .filter(|perm| perm.starts_with(&prefix))
            .cloned()
            .collect();
        if expected_caps != actual_caps {
            let missing_in_caps: Vec<_> = expected_caps.difference(&actual_caps).collect();
            let orphan_in_caps: Vec<_> = actual_caps.difference(&expected_caps).collect();
            violations.push(format!(
                "capabilities/main.json triad drift for plugin `{plugin_name}` (`{selector}`): \
                 missing_permissions={missing_in_caps:?}, stale_or_unmapped_permissions={orphan_in_caps:?}"
            ));
        }

        // Leg C: `src/main.rs` `.plugin(plugins::<rust_mod>::init(`
        let init_needle = format!(".plugin(plugins::{rust_mod}::init(");
        if !main_compact.contains(&init_needle) {
            violations.push(format!(
                "hera-desktop/src-tauri/src/main.rs is missing `{init_needle}...`"
            ));
        }
    }

    if violations.is_empty() {
        Ok(total_commands)
    } else {
        Err(violations)
    }
}

fn collect_files_recursively(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("Failed to read directory `{}`: {err}", dir.display()));
    let mut paths: Vec<PathBuf> = entries
        .map(|entry| entry.expect("valid dir entry").path())
        .collect();
    paths.sort();

    for path in paths {
        if path.is_dir() {
            collect_files_recursively(&path, extensions, out);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if extensions.contains(&ext) {
                out.push(path);
            }
        }
    }
}

/// Counts manual exported type/interface definitions (`export type Name = ...` or
/// `export interface Name ...`), excluding `export type * from ...` and
/// `export type { ... } from ...` re-exports.
fn count_manual_exported_types(ts_source: &str) -> usize {
    ts_source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("export interface ") {
                rest.as_bytes().first().copied().is_some_and(is_ident_start)
            } else if let Some(rest) = trimmed.strip_prefix("export type ") {
                rest.as_bytes().first().copied().is_some_and(is_ident_start)
            } else {
                false
            }
        })
        .count()
}

/// Scans text line-by-line for concrete OS user-directory paths:
/// - POSIX user roots (`/<prefix>/<username>/` where `<prefix>` is `home` or `Users`
///   and `<username>` is a concrete ASCII identifier followed by `/`)
/// - Windows user roots (`<Drive>:\Users\` or JSON-escaped `<Drive>:\\Users\\` or `<Drive>:/Users/`)
///
/// Bare generic directory tokens used in governance prose (without a concrete username segment)
/// are intentionally ignored so documentation rules remain expressible.
fn find_os_user_path_violations(content: &str) -> Vec<(usize, String)> {
    let mut violations = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        if let Some(snippet) = detect_os_user_path_in_line(line) {
            violations.push((line_no, snippet));
        }
    }

    violations
}

fn detect_os_user_path_in_line(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let len = bytes.len();

    // 1. POSIX concrete user directories: `/home/<ident>/` or `/Users/<ident>/`
    for prefix in ["home", "Users"] {
        let needle = format!("/{prefix}/");
        let mut search_from = 0usize;
        while let Some(rel) = line[search_from..].find(&needle) {
            let start = search_from + rel;
            let user_start = start + needle.len();
            search_from = user_start;

            let valid_left_boundary = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
            if !valid_left_boundary || user_start >= len {
                continue;
            }
            if !(bytes[user_start].is_ascii_alphanumeric() || bytes[user_start] == b'_') {
                continue;
            }
            let mut user_end = user_start + 1;
            while user_end < len
                && (bytes[user_end].is_ascii_alphanumeric()
                    || bytes[user_end] == b'_'
                    || bytes[user_end] == b'-'
                    || bytes[user_end] == b'.')
            {
                user_end += 1;
            }
            if user_end < len && bytes[user_end] == b'/' {
                return Some(line[start..=user_end].to_string());
            }
        }
    }

    // 2. Windows user directories: `<Drive>:\Users\` (including `\\` and `/`)
    let mut i = 0usize;
    while i + 8 <= len {
        let valid_left = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
        if valid_left && bytes[i].is_ascii_alphabetic() && bytes[i + 1] == b':' {
            let mut sep_end = i + 2;
            while sep_end < len && (bytes[sep_end] == b'\\' || bytes[sep_end] == b'/') {
                sep_end += 1;
            }
            if sep_end > i + 2 && sep_end + 5 < len {
                let candidate = &line[sep_end..sep_end + 5];
                let trailing = bytes[sep_end + 5];
                if candidate.eq_ignore_ascii_case("Users")
                    && (trailing == b'\\' || trailing == b'/')
                {
                    return Some(line[i..=sep_end + 5].to_string());
                }
            }
        }
        let ch = line[i..].chars().next().expect("valid utf8");
        i += ch.len_utf8();
    }

    None
}

#[test]
fn ratchet_tauri_v2_acl_triad_parity() {
    let root = workspace_root();
    let plugins_dir = root.join("hera-desktop/src-tauri/src/plugins");

    let mut rs_files = Vec::new();
    collect_files_recursively(&plugins_dir, &["rs"], &mut rs_files);
    assert!(
        !rs_files.is_empty(),
        "Expected `.rs` files under `{}`",
        plugins_dir.display()
    );

    let mut plugin_sources = BTreeMap::new();
    for path in rs_files {
        let rel = path
            .strip_prefix(&plugins_dir)
            .expect("plugin relative path")
            .to_string_lossy()
            .replace('\\', "/");
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("Failed to read `{}`: {err}", path.display()));
        plugin_sources.insert(rel, content);
    }

    for &zero_cmd_file in ZERO_COMMAND_PLUGIN_FILES {
        let src = plugin_sources.get(zero_cmd_file).unwrap_or_else(|| {
            panic!("Expected non-command plugin file `hera-desktop/src-tauri/src/plugins/{zero_cmd_file}`")
        });
        let cmds = extract_tauri_commands(src)
            .unwrap_or_else(|err| panic!("Failed to parse `{zero_cmd_file}`: {err}"));
        assert!(
            cmds.is_empty(),
            "Expected 0 #[tauri::command] functions in `{zero_cmd_file}`, found {cmds:?}"
        );
    }

    let build_rs_source = fs::read_to_string(root.join("hera-desktop/src-tauri/build.rs"))
        .expect("read hera-desktop/src-tauri/build.rs");
    let capabilities_json_source =
        fs::read_to_string(root.join("hera-desktop/src-tauri/capabilities/main.json"))
            .expect("read hera-desktop/src-tauri/capabilities/main.json");
    let main_rs_source = fs::read_to_string(root.join("hera-desktop/src-tauri/src/main.rs"))
        .expect("read hera-desktop/src-tauri/src/main.rs");

    match audit_tauri_acl_triad(
        &plugin_sources,
        &build_rs_source,
        &capabilities_json_source,
        &main_rs_source,
    ) {
        Ok(total_commands) => {
            assert_eq!(
                total_commands, EXPECTED_TAURI_COMMANDS_COUNT,
                "Expected exact parity of {EXPECTED_TAURI_COMMANDS_COUNT} Tauri commands across the 8 internal plugins, found {total_commands}"
            );
        }
        Err(violations) => {
            panic!(
                "Tauri v2 ACL Triad Parity Ratchet failed ({} violation(s)):\n  - {}",
                violations.len(),
                violations.join("\n  - ")
            );
        }
    }
}

#[test]
fn ratchet_monolith_max_lines_ceiling() {
    let root = workspace_root();
    let mut violations = Vec::new();

    for &(rel_path, ceiling) in FILE_LINE_CEILINGS {
        let full_path = root.join(rel_path);
        let content = fs::read_to_string(&full_path)
            .unwrap_or_else(|err| panic!("Failed to read `{rel_path}`: {err}"));
        let actual_lines = content.lines().count();
        if actual_lines > ceiling {
            violations.push(format!(
                "`{rel_path}` has {actual_lines} lines (ceiling: {ceiling}, excess: +{}). \
                 Per Law 4 (Ratcheting) & Tidy First, slice leaf submodules before adding lines.",
                actual_lines - ceiling
            ));
        }
    }

    let preexisting_p2p: BTreeSet<&str> = ADR013_PREEXISTING_P2P_FILES.iter().copied().collect();
    for &(rel_dir, ceiling) in SLICED_SUBMODULE_DIRS {
        let dir_path = root.join(rel_dir);
        let mut rs_files = Vec::new();
        collect_files_recursively(&dir_path, &["rs"], &mut rs_files);
        assert!(
            !rs_files.is_empty(),
            "Expected `.rs` files inside sliced submodule directory `{rel_dir}`"
        );
        for file_path in rs_files {
            let rel_path = file_path
                .strip_prefix(&root)
                .expect("repo-relative path")
                .to_string_lossy()
                .replace('\\', "/");
            if preexisting_p2p.contains(rel_path.as_str()) {
                continue;
            }
            let content = fs::read_to_string(&file_path)
                .unwrap_or_else(|err| panic!("Failed to read `{rel_path}`: {err}"));
            let actual_lines = content.lines().count();
            if actual_lines > ceiling {
                violations.push(format!(
                    "`{rel_path}` has {actual_lines} lines (ceiling: {ceiling}, excess: +{}). \
                     Every `.rs` file under `{rel_dir}` must stay <= {ceiling} lines.",
                    actual_lines - ceiling
                ));
            }
        }
    }

    for &(rel_dir, ceiling) in SLICED_FRONTEND_DIRS {
        let dir_path = root.join(rel_dir);
        let mut ts_files = Vec::new();
        collect_files_recursively(&dir_path, &["ts", "tsx"], &mut ts_files);
        assert!(
            !ts_files.is_empty(),
            "Expected `.ts`/`.tsx` files inside sliced frontend directory `{rel_dir}`"
        );
        for file_path in ts_files {
            let rel_path = file_path
                .strip_prefix(&root)
                .expect("repo-relative path")
                .to_string_lossy()
                .replace('\\', "/");
            let content = fs::read_to_string(&file_path)
                .unwrap_or_else(|err| panic!("Failed to read `{rel_path}`: {err}"));
            let actual_lines = content.lines().count();
            if actual_lines > ceiling {
                violations.push(format!(
                    "`{rel_path}` has {actual_lines} lines (ceiling: {ceiling}, excess: +{}). \
                     Every `.ts`/`.tsx` module under `{rel_dir}` must stay <= {ceiling} lines.",
                    actual_lines - ceiling
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Monolith Max-Lines Ratchet violated:\n  - {}",
        violations.join("\n  - ")
    );
}

#[test]
fn ratchet_bridge_hera_manual_ipc_types_ceiling() {
    let root = workspace_root();
    let rel_path = "hera-desktop/src/lib/bridge-hera.ts";
    let content = fs::read_to_string(root.join(rel_path))
        .unwrap_or_else(|err| panic!("Failed to read `{rel_path}`: {err}"));

    let manual_count = count_manual_exported_types(&content);
    assert!(
        manual_count <= MAX_MANUAL_EXPORTED_TYPES_IN_BRIDGE_HERA,
        "`{rel_path}` defines {manual_count} manual `export type Name =` / `export interface Name` blocks \
         (ceiling: {MAX_MANUAL_EXPORTED_TYPES_IN_BRIDGE_HERA}). \
         Per Law 4 (Ratcheting) and Milestone M3.1 (`hera-ipc`), generate IPC contracts from Rust via `ts-rs` \
         into `hera-desktop/src/generated/hera-typings.ts` instead of adding manual duplicate types."
    );
}

#[test]
fn ratchet_player_ts_zero_html_audio_element() {
    let root = workspace_root();
    let rel_path = "hera-desktop/src/lib/player.ts";
    let content = fs::read_to_string(root.join(rel_path))
        .unwrap_or_else(|err| panic!("Failed to read `{rel_path}`: {err}"));

    for forbidden in ["HTMLAudioElement", "new Audio("] {
        assert!(
            !content.contains(forbidden),
            "`{rel_path}` must not reference `{forbidden}` after Milestone M4.2 migration to `NativeRustAudioTransport` (`AudioBackend`)"
        );
    }
}

#[test]
fn ratchet_zero_os_absolute_paths_in_docs_and_plans() {
    let root = workspace_root();
    let mut target_files = Vec::new();
    collect_files_recursively(&root.join("docs/superpowers"), &["md"], &mut target_files);
    collect_files_recursively(
        &root.join("hera-rs/tests"),
        &["rs", "snap"],
        &mut target_files,
    );

    assert!(
        !target_files.is_empty(),
        "Expected markdown/test files under `docs/superpowers` and `hera-rs/tests`"
    );

    let mut violations = Vec::new();
    for path in target_files {
        let rel = path
            .strip_prefix(&root)
            .expect("repo-relative path")
            .to_string_lossy()
            .replace('\\', "/");
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("Failed to read `{rel}`: {err}"));
        for (line_no, snippet) in find_os_user_path_violations(&content) {
            violations.push(format!("{rel}:{line_no} -> `{snippet}`"));
        }
    }

    assert!(
        violations.is_empty(),
        "Zero OS-Specific Absolute Paths Ratchet violated ({} match(es)):\n  - {}",
        violations.len(),
        violations.join("\n  - ")
    );
}

#[test]
fn ratchet_parsers_detect_simulated_violations() {
    // 1. Verify `extract_tauri_commands` handles multiline attributes/lifetimes and ignores comments/strings.
    let synthetic_plugin = r##"
        // #[tauri::command]
        // pub fn commented_ghost() {}
        /* #[tauri::command] pub async fn block_comment_ghost() {} */
        const DOC: &str = "#[tauri::command] pub fn string_literal_ghost() {}";

        #[tauri::command]
        #[allow(clippy::too_many_arguments)]
        pub(crate) async fn valid_alpha<'a>(label: &'a str) -> Result<(), String> {
            let quote_char = '"';
            let _ = (label, quote_char);
            Ok(())
        }

        #[tauri::command(rename_all = "snake_case")]
        pub fn valid_beta() {}
    "##;
    let cmds = extract_tauri_commands(synthetic_plugin).expect("parse synthetic plugin");
    assert_eq!(cmds, vec!["valid_alpha".to_string(), "valid_beta".to_string()]);

    // 2. Verify triad detector handles both single-file and directory plugins and catches drift.
    let mut base_plugins = BTreeMap::new();
    let mut build_entries = String::new();
    let mut cap_entries = Vec::new();
    let mut main_entries = String::new();

    for &(selector, rust_mod, plugin_name) in COMMAND_BEARING_PLUGINS {
        let cmd_name = format!("ping_{rust_mod}");
        let rel_key = if selector.ends_with('/') {
            format!("{selector}commands.rs")
        } else {
            selector.to_string()
        };
        base_plugins.insert(
            rel_key,
            format!("#[tauri::command]\npub async fn {cmd_name}() {{}}\n"),
        );
        build_entries.push_str(&format!(
            ".plugin(\"{plugin_name}\", tauri_build::InlinedPlugin::new().commands(&[\"{cmd_name}\"]))\n"
        ));
        cap_entries.push(format!(
            "\"{plugin_name}:allow-{}\"",
            cmd_name.replace('_', "-")
        ));
        main_entries.push_str(&format!(".plugin(plugins::{rust_mod}::init())\n"));
    }
    base_plugins.insert("debug.rs".to_string(), "pub fn init() {}\n".to_string());

    let valid_build = format!("fn main() {{ tauri_build::Attributes::new()\n{build_entries}; }}");
    let valid_caps = format!(
        "{{ \"permissions\": [\"core:app:default\", {}] }}",
        cap_entries.join(", ")
    );
    let valid_main = format!("fn main() {{ tauri::Builder::default()\n{main_entries}; }}");

    assert_eq!(
        audit_tauri_acl_triad(&base_plugins, &valid_build, &valid_caps, &valid_main),
        Ok(COMMAND_BEARING_PLUGINS.len())
    );

    // 2a. Rogue `#[tauri::command]` inside an unmapped plugin (`debug.rs`) must fail.
    let mut rogue_plugins = base_plugins.clone();
    rogue_plugins.insert(
        "debug.rs".to_string(),
        "#[tauri::command]\npub fn secret_backdoor() {}\n".to_string(),
    );
    let err_rogue = audit_tauri_acl_triad(&rogue_plugins, &valid_build, &valid_caps, &valid_main)
        .expect_err("unmapped plugin command must fail");
    assert!(err_rogue.iter().any(|msg| msg.contains("debug.rs")));

    // 2b. Extra command in a directory plugin submodule (`hera_dj/submod.rs`) missing from `build.rs` must fail.
    let mut nested_drift_plugins = base_plugins.clone();
    nested_drift_plugins.insert(
        "hera_dj/submod.rs".to_string(),
        "#[tauri::command]\npub async fn extra_dj_cmd() {}\n".to_string(),
    );
    let err_nested =
        audit_tauri_acl_triad(&nested_drift_plugins, &valid_build, &valid_caps, &valid_main)
            .expect_err("directory plugin drift must fail");
    assert!(err_nested.iter().any(|msg| msg.contains("hera-dj")));

    // 2c. Missing command in `build.rs` must fail.
    let drifted_build = valid_build.replace("\"ping_hera_dj\"", "\"stale_hera_dj\"");
    let err_build = audit_tauri_acl_triad(&base_plugins, &drifted_build, &valid_caps, &valid_main)
        .expect_err("build.rs drift must fail");
    assert!(err_build.iter().any(|msg| msg.contains("hera-dj")));

    // 2d. Missing/orphan permission in `capabilities/main.json` must fail.
    let drifted_caps = valid_caps.replace("hera-dj:allow-ping-hera-dj", "hera-dj:allow-orphan");
    let err_caps = audit_tauri_acl_triad(&base_plugins, &valid_build, &drifted_caps, &valid_main)
        .expect_err("capabilities drift must fail");
    assert!(err_caps
        .iter()
        .any(|msg| msg.contains("capabilities/main.json")));

    // 2e. Commented-out `.plugin(plugins::hera_agent::init())` in `main.rs` must fail.
    let drifted_main = valid_main.replace(
        ".plugin(plugins::hera_agent::init())",
        "// .plugin(plugins::hera_agent::init())",
    );
    let err_main = audit_tauri_acl_triad(&base_plugins, &valid_build, &valid_caps, &drifted_main)
        .expect_err("missing main.rs plugin init must fail");
    assert!(err_main.iter().any(|msg| msg.contains("hera_agent")));

    // 3. Verify manual TS type detector ignores `export type *` and `export type { ... }` re-exports.
    let ts_sample = "\
export type * from '../generated/typings';
export type { Foo, Bar } from './other';
export type LocalUnion = 'a' | 'b';
export interface LocalShape { ok: boolean }
type InternalOnly = string;
";
    assert_eq!(count_manual_exported_types(ts_sample), 2);

    // 4. Verify OS user-path detector flags synthetic Linux, macOS, and Windows paths
    //    (constructed dynamically via `format!` so zero OS-path literals exist in source)
    //    while allowing generic rule prose tokens.
    let linux_violation = format!("path = \"{}/{}/{}/src/lib.rs\"", "", "home", "dj_linux");
    let mac_violation = format!(
        "uri = \"file://{}/{}/{}/Music/track.flac\"",
        "", "Users", "dj_mac"
    );
    let win_violation = format!("dir = \"{}:\\{}\\dj_win\\Music\"", 'C', "Users");
    let win_json_violation = format!("\"dir\": \"{}:\\\\{}\\\\dj_win\\\\Music\"", 'D', "Users");

    assert_eq!(find_os_user_path_violations(&linux_violation).len(), 1);
    assert_eq!(find_os_user_path_violations(&mac_violation).len(), 1);
    assert_eq!(find_os_user_path_violations(&win_violation).len(), 1);
    assert_eq!(find_os_user_path_violations(&win_json_violation).len(), 1);

    let allowed_rule_prose = format!(
        "Never commit bare `{}:\\`, `/{}/`, `/{}/`, or placeholder `/{}/<user>` tokens.",
        'C', "Users", "home", "home"
    );
    assert!(
        find_os_user_path_violations(&allowed_rule_prose).is_empty(),
        "Generic governance prose tokens must not trigger false positives"
    );
}
