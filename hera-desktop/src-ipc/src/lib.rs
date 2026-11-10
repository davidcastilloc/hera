//! Pure-Rust Hexagonal IPC Contracts (`ts-rs`) and Tauri v2 ACL Triad Guardian for HERA Desktop.
//!
//! Decouples Desktop IPC DTO generation, stream-JSON parsing, unified MCP tool catalog schemas,
//! and compile-time plugin/capability verification from platform WebView headers (`webkit2gtk-4.1`
//! / `WebView2`) so verification runs in seconds across Windows, macOS, and headless Linux.

pub mod acl_guard;
pub mod agent_catalog_schemas;
pub mod agent_catalog_summary;
pub mod agent_types;
pub mod agy_stream;
pub mod audio_types;
pub mod dj_types;
pub mod typegen;
pub mod workflow_guard;

/// Canonical workspace crate version exposed by `hera-ipc`.
pub const HERA_IPC_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::HERA_IPC_VERSION;

    #[test]
    fn test_workspace_ipc_crate_ready() {
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.6.0");
        assert_eq!(HERA_IPC_VERSION, "0.6.0");
    }
}
