//! Tauri v2 ACL Triad Guardian (`ADR-007`).
//! Verifies every `#[tauri::command]` across `src/plugins/**/*.rs` is wired in `src/main.rs`,
//! `build.rs` (`tauri_build::InlinedPlugin`), and `capabilities/main.json`.

use std::collections::{BTreeMap, BTreeSet};
use std::{fs, path::Path, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AclParityReport {
    pub plugins_checked: usize,
    pub commands_checked: usize,
}

fn skip_balanced(s: &str, open: u8, close: u8) -> usize {
    let (b, mut i, mut depth, mut in_str) = (s.as_bytes(), 1usize, 1usize, false);
    while i < b.len() && depth > 0 {
        if b[i] == b'"' {
            in_str = !in_str;
        } else if !in_str && (b[i] == open || b[i] == close) {
            depth = if b[i] == open { depth + 1 } else { depth - 1 };
        }
        i += 1;
    }
    i
}

fn quoted(s: &str) -> Vec<String> {
    s.split('"').skip(1).step_by(2).map(str::to_owned).collect()
}

fn strip_comments(src: &str) -> String {
    let (b, n, mut i, mut in_str, mut out) = (src.as_bytes(), src.len(), 0, false, String::new());
    while i < n {
        if !in_str && b[i..].starts_with(b"//") {
            i = src[i..].find('\n').map_or(n, |d| i + d);
        } else if !in_str && b[i..].starts_with(b"/*") {
            i = src[i + 2..].find("*/").map_or(n, |d| i + 4 + d);
            out.push(' ');
        } else if !in_str && b[i..].starts_with(b"r#\"") {
            i = src[i + 3..].find("\"#").map_or(n, |d| i + 5 + d);
            out.push_str("\"\"");
        } else if in_str && b[i] == b'\\' {
            i += 1 + src[i + 1..].chars().next().map_or(0, char::len_utf8);
        } else {
            if b[i] == b'"' && (i == 0 || b[i - 1] != b'\'') {
                in_str = !in_str;
            }
            let ch = src[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn sorted_entries(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|r| r.ok().map(|e| e.path()))
        .collect();
    v.sort();
    Ok(v)
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for p in sorted_entries(dir)? {
        if p.is_dir() {
            collect_rs(&p, out)?;
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(p);
        }
    }
    Ok(())
}

pub(crate) fn discover_plugins(
    plugins_dir: &Path,
) -> Result<BTreeMap<String, (String, BTreeSet<String>)>, String> {
    let mut mod_files: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for p in sorted_entries(plugins_dir)? {
        if p.is_dir() {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            collect_rs(&p, mod_files.entry(name).or_default())?;
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs")
            && p.file_name().and_then(|s| s.to_str()) != Some("mod.rs")
        {
            let stem = p.file_stem().unwrap().to_string_lossy().into_owned();
            mod_files.entry(stem).or_default().push(p);
        }
    }
    let mut plugins = BTreeMap::new();
    for (mod_name, files) in mod_files {
        let (mut builder_id, mut cmds) = (None, BTreeSet::new());
        for file in files {
            let clean = strip_comments(&fs::read_to_string(&file).map_err(|e| format!("{e}"))?);
            let flat: String = clean.chars().filter(|c| !c.is_whitespace()).collect();
            for (idx, _) in flat.match_indices("Builder::") {
                if idx == 0 || !flat.as_bytes()[idx - 1].is_ascii_alphanumeric() {
                    let r = &flat[idx + 9..];
                    let r = if r.starts_with('<') {
                        &r[skip_balanced(r, b'<', b'>') + 2..]
                    } else {
                        r
                    };
                    if let Some((id, _)) =
                        r.strip_prefix("new(\"").and_then(|s| s.split_once("\")"))
                    {
                        builder_id = Some(id.to_string());
                    }
                }
            }
            for part in clean.split("#[tauri::command").skip(1) {
                let boxed = format!("[{part}");
                let mut r = boxed[skip_balanced(&boxed, b'[', b']')..].trim_start();
                while r.starts_with("#[") {
                    r = r[1 + skip_balanced(&r[1..], b'[', b']')..].trim_start();
                }
                let mut toks = r
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .filter(|t| !t.is_empty());
                if toks.any(|t| t == "fn") {
                    if let Some(fn_name) = toks.next() {
                        cmds.insert(fn_name.to_string());
                    }
                }
            }
        }
        let id = builder_id.unwrap_or_else(|| mod_name.replace('_', "-"));
        plugins.insert(mod_name, (id, cmds));
    }
    Ok(plugins)
}

fn parse_build_rs(src: &str) -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::<String, Vec<String>>::new();
    for part in strip_comments(src).split(".plugin(").skip(1) {
        let wrapped = format!("({part}");
        let block = &wrapped[1..skip_balanced(&wrapped, b'(', b')') - 1];
        let qs = quoted(block);
        if block.contains("InlinedPlugin") && !qs.is_empty() {
            map.entry(qs[0].clone())
                .or_default()
                .extend_from_slice(&qs[1..]);
        }
    }
    map
}

pub fn verify_tauri_v2_acl_triad(src_tauri_dir: &Path) -> Result<AclParityReport, String> {
    let plugins = discover_plugins(&src_tauri_dir.join("src/plugins"))?;
    let rd =
        |rel: &str| fs::read_to_string(src_tauri_dir.join(rel)).map_err(|e| format!("{rel}: {e}"));
    let main_rs: String = strip_comments(&rd("src/main.rs")?)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let build_map = parse_build_rs(&rd("build.rs")?);
    let cap_val: serde_json::Value = serde_json::from_str(&rd("capabilities/main.json")?)
        .map_err(|e| format!("capabilities/main.json: {e}"))?;
    let cap_perms: BTreeSet<String> = cap_val["permissions"]
        .as_array()
        .ok_or("capabilities/main.json: missing permissions")?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();

    let (mut errs, mut plugins_checked, mut commands_checked) = (Vec::new(), 0, 0);
    let known: BTreeSet<&str> = plugins.values().map(|(id, _)| id.as_str()).collect();
    for (id, cmds) in build_map
        .iter()
        .filter(|(id, _)| !known.contains(id.as_str()))
    {
        errs.push(format!("build.rs: ghost plugin '{id}' ({cmds:?})"));
    }
    for (m, (id, cmds)) in &plugins {
        if cmds.is_empty() && !build_map.contains_key(id) {
            continue;
        }
        plugins_checked += 1;
        commands_checked += cmds.len();
        if !main_rs.contains(&format!(".plugin(plugins::{m}::init(")) {
            errs.push(format!("src/main.rs: '{id}' (plugins::{m}::init) missing"));
        }
        match build_map.get(id) {
            None => errs.push(format!("build.rs: '{id}' missing {cmds:?}")),
            Some(b_cmds) => {
                for c in cmds.iter().filter(|c| !b_cmds.contains(c)) {
                    errs.push(format!("build.rs: '{id}' missing '{c}'"));
                }
                for c in b_cmds.iter().filter(|c| !cmds.contains(*c)) {
                    errs.push(format!("build.rs: '{id}' stale '{c}'"));
                }
            }
        }
        let allow = |c: &str| format!("{id}:allow-{}", c.replace('_', "-"));
        let expected: BTreeSet<String> = cmds.iter().map(|c| allow(c)).collect();
        for c in cmds.iter().filter(|c| !cap_perms.contains(&allow(c))) {
            let req = allow(c);
            errs.push(format!(
                "capabilities/main.json: '{id}' '{c}' missing '{req}'"
            ));
        }
        let pfx = format!("{id}:allow-");
        for p in cap_perms
            .iter()
            .filter(|s| s.starts_with(&pfx) && !expected.contains(*s))
        {
            errs.push(format!("capabilities/main.json: '{id}' stale '{p}'"));
        }
    }
    if errs.is_empty() {
        Ok(AclParityReport {
            plugins_checked,
            commands_checked,
        })
    } else {
        Err(errs.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::{discover_plugins, verify_tauri_v2_acl_triad};
    use std::{collections::BTreeMap, fs, path::Path};

    #[test]
    fn test_tauri_v2_acl_triad_parity() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src-tauri");
        let by_id: BTreeMap<String, usize> = discover_plugins(&dir.join("src/plugins"))
            .unwrap()
            .into_values()
            .filter(|(_, c)| !c.is_empty())
            .map(|(id, c)| (id, c.len()))
            .collect();
        assert_eq!(by_id["app-menu"], 3);
        assert_eq!(by_id["config"], 3);
        assert_eq!(by_id["cover"], 1);
        assert_eq!(by_id["database"], 18);
        assert_eq!(by_id["default-view"], 1);
        assert_eq!(by_id["hera-agent"], 9);
        assert_eq!(by_id["hera-dj"], 22);
        assert_eq!(by_id["sleepblocker"], 2);
        let r = verify_tauri_v2_acl_triad(&dir).expect("ACL triad parity");
        assert_eq!((r.plugins_checked, r.commands_checked), (8, 59));
        assert!(r.commands_checked >= 45);
    }

    #[test]
    fn test_acl_guard_detects_missing_build_rs_and_capability_and_nested_mod_files() {
        let tmp = std::env::temp_dir().join(format!("hera_acl_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src/plugins/hera_dj")).unwrap();
        fs::create_dir_all(tmp.join("capabilities")).unwrap();
        let wr = |r: &str, s: &str| fs::write(tmp.join(r), s).unwrap();
        let bld = |c: &str| {
            format!("fn main(){{x.plugin(\"hera-dj\",InlinedPlugin::new().commands(&[{c}]));}}")
        };
        let cap = |c: &str| format!("{{\"permissions\":[{c}]}}");
        wr("src/plugins/hera_dj/mod.rs", "#[tauri::command] pub fn base_cmd() {}\npub fn init() { Builder::<R>::new(\"hera-dj\"); }");
        wr(
            "src/plugins/hera_dj/submod.rs",
            "#[tauri::command]\npub async fn orphan_cmd<R: Runtime>() {}",
        );
        wr(
            "src/main.rs",
            "fn main() { x.plugin(plugins::hera_dj::init()); }",
        );
        wr("build.rs", &bld("\"base_cmd\""));
        wr("capabilities/main.json", &cap("\"hera-dj:allow-base-cmd\""));

        let e = verify_tauri_v2_acl_triad(&tmp).unwrap_err();
        assert!(
            e.contains("build.rs") && e.contains("capabilities/main.json"),
            "{e}"
        );
        assert!(e.contains("hera-dj:allow-orphan-cmd"), "{e}");

        wr("build.rs", &bld("\"base_cmd\",\"orphan_cmd\""));
        let allow_both = "\"hera-dj:allow-base-cmd\",\"hera-dj:allow-orphan-cmd\"";
        wr("capabilities/main.json", &cap(allow_both));
        let ok = verify_tauri_v2_acl_triad(&tmp).unwrap();
        assert_eq!((ok.plugins_checked, ok.commands_checked), (1, 2));

        wr(
            "build.rs",
            &bld("\"base_cmd\",\"orphan_cmd\",\"ghost_cmd\""),
        );
        let g = verify_tauri_v2_acl_triad(&tmp).unwrap_err();
        assert!(g.contains("build.rs") && g.contains("ghost_cmd"), "{g}");
        wr("src/main.rs", "fn main() {}");
        assert!(verify_tauri_v2_acl_triad(&tmp)
            .unwrap_err()
            .contains("src/main.rs"));
        let _ = fs::remove_dir_all(&tmp);
    }
}
