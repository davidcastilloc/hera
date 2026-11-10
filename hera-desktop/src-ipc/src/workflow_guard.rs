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
            for forbidden in ["hera-rs -> target", "hera-desktop/src-tauri -> target"] {
                if rel.contains(forbidden) {
                    errs.push(format!(
                        "{RELEASE_YML}: contains stale sub-crate workspaces cache entry '{forbidden}'"
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

#[cfg(test)]
mod tests {
    use super::verify_workspace_release_workflows;
    use std::{fs, path::Path};

    #[test]
    fn test_workspace_release_workflows_target_paths() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(verify_workspace_release_workflows(&repo_root), Ok(()));
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
