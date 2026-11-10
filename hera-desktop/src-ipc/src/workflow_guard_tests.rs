use crate::workflow_guard::{
    verify_documentation_assets_and_v060_alignment, verify_open_core_and_lean_ci,
    verify_qa_mcp_bridge_default, verify_tauri_version_parity, verify_workspace_release_workflows,
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
    fs::write(
        tmp.join("Cargo.lock"),
        "[[package]]\nname = \"tauri\"\nversion = \"2.12.5\"\n",
    )
    .unwrap();
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
