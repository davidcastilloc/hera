# HERA v0.6.0 End-to-End QA Harness, Default Loopback MCP Bridge & Browser E2E Suite Design

**Date:** 2026-11-10
**Repository:** `davidcastilloc/hera` (`v0.6.0`)
**Status:** Approved

---

## 1. Problem Statement

1. **Release Binary Tauri MCP Bridge Gate:** `hera-desktop/src-tauri/src/main.rs` gates `tauri-plugin-mcp-bridge` (`127.0.0.1:9223`) behind `#[cfg(any(debug_assertions, feature = "mcp-bridge"))]`, while `hera-desktop/src-tauri/Cargo.toml` sets `default = []`. Consequently, production installers (`.exe`, `.dmg`, `.deb`, `.rpm`, `.AppImage`) and `cargo build --release` omit the loopback MCP bridge unless `--features mcp-bridge` is explicitly passed.
2. **Legacy E2E Tests in `hera-desktop/src/__tests__/`:** The Playwright/Vitest Browser E2E tests (`e2e-helpers.tsx`, `library.test-e2e.tsx`, `translations.test-e2e.tsx`) still reference pre-`v0.6.0` upstream Museeks selectors (`Main navigation`, `Library`, `Settings`, `Title`, `Artist` sort labels, and row text without `BPM` / `Camelot` columns) and lack coverage for HERA's `v0.6.0` routes (`/agent`, `/bridge`, `/dna`, and `Modo Mini`).

---

## 2. Design

### 2.1 Loopback-Only Tauri MCP Bridge Enabled by Default (`127.0.0.1:9223`)
- Enable `default = ["mcp-bridge"]` in `hera-desktop/src-tauri/Cargo.toml` while preserving `.bind_address("127.0.0.1")` in `hera-desktop/src-tauri/src/main.rs` so both debug and release builds can be inspected live by QA tools on localhost without exposing any external network interface.
- Add `verify_qa_mcp_bridge_default(repo_root: &Path)` in `hera-desktop/src-ipc/src/workflow_guard.rs` and unit test `test_qa_mcp_bridge_enabled_by_default_on_loopback`.

### 2.2 Browser E2E QA Suite (`bun run test:e2e`) & CI Integration
- Configure `hera-desktop/vitest.config.ts` to use the canonical `1180x720` booth viewport and optional `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH` env var for sandboxed/system Chromium environments.
- Align `e2e-helpers.tsx`, `library.test-e2e.tsx`, `player.test-e2e.tsx`, and `translations.test-e2e.tsx` with `v0.6.0` `Navigation.tsx` (`Secciones`, `Biblioteca`, `Ajustes`) and `TrackRow.tsx` (`BPM`, `Camelot Key`, `Puntos`).
- Add `hera-desktop/src/__tests__/hera-studio-qa.test-e2e.tsx` verifying `/library` (`HeraDjToolbar`), `/agent` (`Agente`), `/bridge` (`Transición`), `/dna` (`Mi Estilo`), and `MiniOverlay` (`Modo Mini` Card & Pill).
- Wire `bun run test:e2e` into `.github/workflows/ci.yml`.
