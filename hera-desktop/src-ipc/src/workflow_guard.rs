//! Root Cargo Workspace Release Workflow Guardian (`ADR-024`).
//!
//! Guards `.github/workflows/release-binaries.yml` and `.github/workflows/release.yml`
//! against stale pre-workspace crate-local `target/` paths and lockfile-less `rust-cache` entries.

use std::{fs, path::Path};

const RELEASE_BINARIES_YML: &str = ".github/workflows/release-binaries.yml";
const RELEASE_YML: &str = ".github/workflows/release.yml";
const EXPECTED_ROOT_BIN_ARTIFACT: &str =
    "target/${{ matrix.target }}/release/${{ matrix.bin_name }}";

/// Verifies that GitHub Release workflows consume artifacts and Cargo caches from the root workspace.
pub fn verify_workspace_release_workflows(repo_root: &Path) -> Result<(), String> {
    let mut errs = Vec::new();

    let bins_path = repo_root.join(RELEASE_BINARIES_YML);
    match fs::read_to_string(&bins_path) {
        Err(e) => errs.push(format!("{RELEASE_BINARIES_YML}: {e}")),
        Ok(bins) => {
            for forbidden in ["hera-rs/target/", "src-tauri/target/"] {
                if bins.contains(forbidden) {
                    errs.push(format!(
                        "{RELEASE_BINARIES_YML}: contains stale sub-crate target path '{forbidden}'"
                    ));
                }
            }
            if !bins.contains(EXPECTED_ROOT_BIN_ARTIFACT) {
                errs.push(format!(
                    "{RELEASE_BINARIES_YML}: missing root workspace target path '{EXPECTED_ROOT_BIN_ARTIFACT}'"
                ));
            }
        }
    }

    let rel_path = repo_root.join(RELEASE_YML);
    match fs::read_to_string(&rel_path) {
        Err(e) => errs.push(format!("{RELEASE_YML}: {e}")),
        Ok(rel) => {
            for forbidden in [
                "hera-rs -> target",
                "hera-desktop/src-tauri -> target",
                "libgstreamer1.0-dev",
                "libgstreamer-plugins-base1.0-dev",
            ] {
                if rel.contains(forbidden) {
                    errs.push(format!(
                        "{RELEASE_YML}: contains forbidden/conflicting entry '{forbidden}'"
                    ));
                }
            }
            if !rel.contains("Swatinem/rust-cache@v2") {
                errs.push(format!(
                    "{RELEASE_YML}: missing 'Swatinem/rust-cache@v2' step"
                ));
            } else if rel.contains("workspaces:") && !rel.contains(". -> target") {
                errs.push(format!(
                    "{RELEASE_YML}: stale 'workspaces:' in Swatinem/rust-cache@v2 without root '. -> target'"
                ));
            }
        }
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join("\n"))
    }
}

fn major_minor(ver: &str) -> Option<&str> {
    let clean = ver.trim().trim_start_matches(['^', '~', '=']);
    let mut parts = clean.split('.');
    let major = parts.next()?;
    let minor = parts.next()?;
    let end = major.len() + 1 + minor.len();
    clean.get(..end)
}

fn extract_after<'a>(haystack: &'a str, needle: &str) -> Option<&'a str> {
    let rest = haystack.split_once(needle)?.1;
    rest.split_once('"').map(|(val, _)| val)
}

/// Verifies that Rust `tauri` (`Cargo.lock`) and NPM `@tauri-apps/{api,cli}` (`package.json`, `bun.lock`)
/// share the exact same `major.minor` release required by `tauri build` pre-flight checks.
pub fn verify_tauri_version_parity(repo_root: &Path) -> Result<(), String> {
    let cargo_lock = fs::read_to_string(repo_root.join("Cargo.lock"))
        .map_err(|e| format!("Cargo.lock: {e}"))?;
    let rust_ver = extract_after(&cargo_lock, "name = \"tauri\"\nversion = \"")
        .or_else(|| extract_after(&cargo_lock, "name = \"tauri\"\r\nversion = \""))
        .ok_or_else(|| "Cargo.lock: missing `tauri` package entry".to_string())?;
    let rust_mm = major_minor(rust_ver)
        .ok_or_else(|| format!("Cargo.lock: invalid tauri semver '{rust_ver}'"))?;

    let pkg_json = fs::read_to_string(repo_root.join("hera-desktop/package.json"))
        .map_err(|e| format!("hera-desktop/package.json: {e}"))?;
    let bun_lock = fs::read_to_string(repo_root.join("hera-desktop/bun.lock"))
        .map_err(|e| format!("hera-desktop/bun.lock: {e}"))?;

    let mut errs = Vec::new();
    for pkg in ["@tauri-apps/api", "@tauri-apps/cli"] {
        let pkg_needle = format!("\"{pkg}\": \"");
        match extract_after(&pkg_json, &pkg_needle).and_then(major_minor) {
            Some(mm) if mm == rust_mm => {}
            Some(mm) => errs.push(format!(
                "hera-desktop/package.json: {pkg} ({mm}.x) mismatches Rust tauri ({rust_ver})"
            )),
            None => errs.push(format!("hera-desktop/package.json: missing {pkg}")),
        }
        let lock_needle = format!("\"{pkg}@");
        match extract_after(&bun_lock, &lock_needle).and_then(major_minor) {
            Some(mm) if mm == rust_mm => {}
            Some(mm) => errs.push(format!(
                "hera-desktop/bun.lock: {pkg} ({mm}.x) mismatches Rust tauri ({rust_ver})"
            )),
            None => errs.push(format!("hera-desktop/bun.lock: missing resolved {pkg}")),
        }
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join("\n"))
    }
}

/// Verifies that `hera-desktop/src-tauri` enables `mcp-bridge` by default and binds strictly to loopback `127.0.0.1`.
pub fn verify_qa_mcp_bridge_default(repo_root: &Path) -> Result<(), String> {
    let cargo_toml = fs::read_to_string(repo_root.join("hera-desktop/src-tauri/Cargo.toml"))
        .map_err(|e| format!("hera-desktop/src-tauri/Cargo.toml: {e}"))?;
    let main_rs = fs::read_to_string(repo_root.join("hera-desktop/src-tauri/src/main.rs"))
        .map_err(|e| format!("hera-desktop/src-tauri/src/main.rs: {e}"))?;

    let mut errs = Vec::new();
    if !cargo_toml.contains("default = [\"mcp-bridge\"]") {
        errs.push("hera-desktop/src-tauri/Cargo.toml: missing `default = [\"mcp-bridge\"]` for release QA".to_string());
    }
    if !main_rs.contains("tauri_plugin_mcp_bridge::Builder::new()")
        || !main_rs.contains(".bind_address(\"127.0.0.1\")")
    {
        errs.push("hera-desktop/src-tauri/src/main.rs: missing loopback 127.0.0.1 tauri_plugin_mcp_bridge binding".to_string());
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join("\n"))
    }
}

/// Verifies that `v0.6.0` visual assets exist, are embedded in `README.md` and `hera-desktop/README.md`,
/// and that `docs/` guides no longer contain legacy Python/Streamlit (`localhost:8501`, `.venv/bin/hera`) strings.
pub fn verify_documentation_assets_and_v060_alignment(repo_root: &Path) -> Result<(), String> {
    let mut errs = Vec::new();

    for asset in [
        "assets/hera_banner_y2k.jpg",
        "assets/hera_banner.jpg",
        "assets/hera_swarm_concept.jpg",
        "docs/superpowers/assets/hera_v060_1_library_1180x720.png",
        "docs/superpowers/assets/hera_v060_2_agent_1180x720.png",
        "docs/superpowers/assets/hera_v060_3_transition_1180x720.png",
        "docs/superpowers/assets/hera_v060_4_dna_1180x720.png",
        "docs/superpowers/assets/hera_v060_5_mini_overlay_1180x720.png",
    ] {
        match fs::metadata(repo_root.join(asset)) {
            Ok(meta) if meta.len() > 10_000 => {}
            Ok(meta) => errs.push(format!("{asset}: file too small ({} bytes)", meta.len())),
            Err(e) => errs.push(format!("{asset}: missing ({e})")),
        }
    }

    for (doc, required_refs) in [
        (
            "README.md",
            &[
                "assets/hera_swarm_concept.jpg",
                "docs/superpowers/assets/hera_v060_1_library_1180x720.png",
                "docs/superpowers/assets/hera_v060_2_agent_1180x720.png",
                "docs/superpowers/assets/hera_v060_3_transition_1180x720.png",
                "docs/superpowers/assets/hera_v060_4_dna_1180x720.png",
                "docs/superpowers/assets/hera_v060_5_mini_overlay_1180x720.png",
            ][..],
        ),
        (
            "hera-desktop/README.md",
            &[
                "../assets/hera_swarm_concept.jpg",
                "../docs/superpowers/assets/hera_v060_1_library_1180x720.png",
                "../docs/superpowers/assets/hera_v060_2_agent_1180x720.png",
                "../docs/superpowers/assets/hera_v060_3_transition_1180x720.png",
                "../docs/superpowers/assets/hera_v060_4_dna_1180x720.png",
                "../docs/superpowers/assets/hera_v060_5_mini_overlay_1180x720.png",
            ][..],
        ),
    ] {
        match fs::read_to_string(repo_root.join(doc)) {
            Err(e) => errs.push(format!("{doc}: {e}")),
            Ok(text) => {
                for req in required_refs {
                    if !text.contains(req) {
                        errs.push(format!("{doc}: missing visual reference '{req}'"));
                    }
                }
            }
        }
    }

    for doc in [
        "MANIFESTO.md",
        "docs/README.md",
        "docs/MODOS_DE_USO.md",
        "docs/VPS_REMOTE_GUIDE.md",
        "docs/Hera_Guia_para_Agentes_AI.md",
    ] {
        match fs::read_to_string(repo_root.join(doc)) {
            Err(e) => errs.push(format!("{doc}: {e}")),
            Ok(text) => {
                for forbidden in ["localhost:8501", ".venv/bin/hera", "Python 3.11+"] {
                    if text.contains(forbidden) {
                        errs.push(format!("{doc}: contains stale legacy string '{forbidden}'"));
                    }
                }
            }
        }
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join("\n"))
    }
}

/// Verifies locally (`< 50ms`) that:
/// 1. `.agent/skills` and `.claude/skills` contain zero OS symlinks.
/// 2. Public `hera-rs/src` and `hera-desktop/src` contain zero private `hera-cloud` SaaS leaks.
/// 3. `.github/workflows/ci.yml` stays lean (no Playwright `test:e2e` and no heavy `libwebkit2gtk-4.1-dev` apt installs).
pub fn verify_open_core_and_lean_ci(repo_root: &Path) -> Result<(), String> {
    let mut errs = Vec::new();

    for skills_dir in [".agent/skills", ".claude/skills"] {
        let root = repo_root.join(skills_dir);
        if root.exists() {
            check_no_symlinks_recursive(&root, &mut errs);
        }
    }

    for src_dir in ["hera-rs/src", "hera-desktop/src"] {
        let root = repo_root.join(src_dir);
        if root.exists() {
            check_no_private_saas_leaks_recursive(&root, &mut errs);
        }
    }

    let ci_path = repo_root.join(".github/workflows/ci.yml");
    match fs::read_to_string(&ci_path) {
        Err(e) => errs.push(format!(".github/workflows/ci.yml: {e}")),
        Ok(ci) => {
            for forbidden in ["test:e2e", "playwright install", "libwebkit2gtk-4.1-dev"] {
                if ci.contains(forbidden) {
                    errs.push(format!(
                        ".github/workflows/ci.yml: contains expensive non-release step '{forbidden}' (must run locally or in release.yml only)"
                    ));
                }
            }
        }
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join("\n"))
    }
}

fn check_no_symlinks_recursive(dir: &Path, errs: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(ft) = entry.file_type() {
                if ft.is_symlink() {
                    errs.push(format!("OS symlink detected: {}", path.display()));
                } else if ft.is_dir() {
                    check_no_symlinks_recursive(&path, errs);
                }
            }
        }
    }
}

fn check_no_private_saas_leaks_recursive(dir: &Path, errs: &mut Vec<String>) {
    const FORBIDDEN_MARKERS: [&str; 4] = [
        "/api/v1/cloud/economics",
        "/api/v1/dj/upgrade-tier",
        "X-Hera-Admin-Secret",
        "calculate_cloudflare_beta_economics",
    ];
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                check_no_private_saas_leaks_recursive(&path, errs);
            } else if let Ok(text) = fs::read_to_string(&path) {
                for marker in FORBIDDEN_MARKERS {
                    if text.contains(marker) {
                        errs.push(format!(
                            "Private SaaS leak '{marker}' found in {}",
                            path.display()
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        verify_documentation_assets_and_v060_alignment, verify_open_core_and_lean_ci,
        verify_qa_mcp_bridge_default, verify_tauri_version_parity,
        verify_workspace_release_workflows,
    };
    use std::{fs, path::Path};

    #[test]
    fn test_open_core_and_lean_ci_hygiene() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(verify_open_core_and_lean_ci(&repo_root), Ok(()));
    }

    #[test]
    fn test_workspace_release_workflows_target_paths() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(verify_workspace_release_workflows(&repo_root), Ok(()));
    }

    #[test]
    fn test_documentation_assets_and_v060_alignment() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(
            verify_documentation_assets_and_v060_alignment(&repo_root),
            Ok(())
        );
    }

    #[test]
    fn test_tauri_rust_and_npm_minor_versions_match() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(verify_tauri_version_parity(&repo_root), Ok(()));
    }

    #[test]
    fn test_qa_mcp_bridge_enabled_by_default_on_loopback() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(verify_qa_mcp_bridge_default(&repo_root), Ok(()));
    }

    #[test]
    fn test_tauri_version_parity_rejects_minor_mismatch() {
        let tmp = std::env::temp_dir().join(format!("hera_tauri_ver_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("hera-desktop")).unwrap();
        fs::write(tmp.join("Cargo.lock"), "[[package]]\nname = \"tauri\"\nversion = \"2.12.5\"\n").unwrap();
        fs::write(
            tmp.join("hera-desktop/package.json"),
            "{\"dependencies\":{\"@tauri-apps/api\":\"2.11.0\"},\"devDependencies\":{\"@tauri-apps/cli\":\"2.11.2\"}}",
        )
        .unwrap();
        fs::write(
            tmp.join("hera-desktop/bun.lock"),
            "[\"@tauri-apps/api@2.11.0\", \"@tauri-apps/cli@2.11.2\"]",
        )
        .unwrap();
        let err = verify_tauri_version_parity(&tmp).unwrap_err();
        assert!(err.contains("@tauri-apps/api (2.11.x)"), "{err}");
        assert!(err.contains("@tauri-apps/cli (2.11.x)"), "{err}");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_workflow_guard_rejects_stale_subcrate_target_paths() {
        let tmp = std::env::temp_dir().join(format!("hera_wf_guard_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let wf_dir = tmp.join(".github/workflows");
        fs::create_dir_all(&wf_dir).unwrap();

        fs::write(
            wf_dir.join("release-binaries.yml"),
            "run: cp \"hera-rs/target/${{ matrix.target }}/release/${{ matrix.bin_name }}\" dist/out\n",
        )
        .unwrap();
        fs::write(
            wf_dir.join("release.yml"),
            "uses: Swatinem/rust-cache@v2\nwith:\n  workspaces: |\n    hera-rs -> target\n    hera-desktop/src-tauri -> target\n",
        )
        .unwrap();

        let err = verify_workspace_release_workflows(&tmp).unwrap_err();
        assert!(err.contains("release-binaries.yml"), "{err}");
        assert!(err.contains("hera-rs/target/"), "{err}");
        assert!(err.contains("release.yml"), "{err}");
        assert!(err.contains("hera-rs -> target"), "{err}");

        fs::write(
            wf_dir.join("release-binaries.yml"),
            "run: cp \"target/${{ matrix.target }}/release/${{ matrix.bin_name }}\" dist/out\n",
        )
        .unwrap();
        fs::write(
            wf_dir.join("release.yml"),
            "uses: Swatinem/rust-cache@v2\nwith:\n  workspaces: . -> target\n",
        )
        .unwrap();
        assert_eq!(verify_workspace_release_workflows(&tmp), Ok(()));

        let _ = fs::remove_dir_all(&tmp);
    }
}

