# HERA v0.6.0 QA Harness & Browser E2E Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable out-of-the-box live QA on release builds via loopback Tauri MCP Bridge (`127.0.0.1:9223`) and full Playwright Chromium E2E QA (`bun run test:e2e`) across all `v0.6.0` HERA Desktop routes (`Biblioteca`, `Agente`, `Transición`, `Mi Estilo`, and `Modo Mini`).

**Architecture:** Enable `mcp-bridge` in default `src-tauri` features (guarded by `hera-ipc`), align Vitest Browser Playwright E2E helpers with `v0.6.0` Spanish booth navigation and `BPM`/`Camelot` columns, and add an end-to-end studio QA test suite enforced in CI.

**Tech Stack:** Rust (`hera-ipc`, `tauri-plugin-mcp-bridge 0.13.2`), TypeScript, React 19, Vite+ (`vp test run --project e2e`), Playwright Chromium.

**Spec:** `docs/superpowers/specs/2026-11-10-v060-qa-harness-and-e2e-suite-design.md`

---

### Task 1: Default Loopback Tauri MCP Bridge (`127.0.0.1:9223`) & `hera-ipc` Guard

**Files:**
- Modify: `hera-desktop/src-ipc/src/workflow_guard.rs`
- Modify: `hera-desktop/src-tauri/Cargo.toml`

- [x] **Step 1: Add failing test `test_qa_mcp_bridge_enabled_by_default_on_loopback` in `hera-desktop/src-ipc/src/workflow_guard.rs`**
  Assert `hera-desktop/src-tauri/Cargo.toml` contains `default = ["mcp-bridge"]` and `hera-desktop/src-tauri/src/main.rs` binds `tauri_plugin_mcp_bridge` to `"127.0.0.1"`.
- [x] **Step 2: Run `cargo test -p hera-ipc` to capture RED receipt**
- [x] **Step 3: Enable `default = ["mcp-bridge"]` in `hera-desktop/src-tauri/Cargo.toml` and verify GREEN `cargo test -p hera-ipc`**

---

### Task 2: `v0.6.0` Local Browser E2E QA Suite (`bun run test:e2e`)

**Files:**
- Modify: `hera-desktop/vitest.config.ts`
- Modify: `hera-desktop/src/__tests__/e2e-helpers.tsx`
- Modify: `hera-desktop/src/__tests__/library.test-e2e.tsx`
- Modify: `hera-desktop/src/__tests__/translations.test-e2e.tsx`
- Create: `hera-desktop/src/__tests__/hera-studio-qa.test-e2e.tsx`
- Modify: `docs/superpowers/DECISIONS.md`

- [x] **Step 1: Update `vitest.config.ts` to support `1180x720` viewport and optional `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH`**
- [x] **Step 2: Align `e2e-helpers.tsx`, `library.test-e2e.tsx`, and `translations.test-e2e.tsx` with `v0.6.0` navigation & `BPM`/`Camelot` columns**
- [x] **Step 3: Create `hera-desktop/src/__tests__/hera-studio-qa.test-e2e.tsx` covering `/library`, `/agent`, `/bridge`, `/dna`, and `Modo Mini`**
- [x] **Step 4: Run `bun run check`, `bun run test`, and `bun run test:e2e` locally (`0` errors, all unit + E2E tests passing)**
- [x] **Step 5: Record `ADR-026` in `DECISIONS.md`, keep `.github/workflows/ci.yml` free of E2E runner overhead, commit atomically, and push `main`**
