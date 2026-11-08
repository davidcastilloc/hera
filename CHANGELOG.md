# Changelog

All notable changes to **HERA (`davidcastilloc/hera`)** are documented in this file following [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.3.0] - 2026-11-08

### Added
- **`HERA Desktop` (`hera-desktop/` — Tauri v2 + React 19 Compiler + StyleX):**
  - Cloned `martpie/museeks` (`v0.23.4`, MIT) and adapted it into a `~8 MB` 100% Pure-Rust + React 19 DJ pre-gig command center (`59a4da4`, `a05b217`, `d876eb7`).
  - Added top `TrackProgress.tsx` bar with **4 clickable acoustic Hot Cue pins** (`🟢 1:Intro`, `🔵 2:Breakdown`, `🔴 3:Drop`, `🟡 4:Outro`) and virtualized DJ columns (`BPM`, `Camelot Key`, `4 Hot Cues`) in `TrackListHeader.tsx` and `TrackRow.tsx`.
  - Added deterministic offline toolbar (`HeraDjToolbar.tsx` + `bridge-hera.ts`) for 1-click export to **VirtualDJ (`.vdjfolder`)**, **Rekordbox (`.xml`)**, **Traktor Pro (`.nml`)**, and **M3U8**.
  - Added `/dna` view (`dna.tsx`) with the full 24-key Camelot Wheel (`1A..12A` and `1B..12B`), DJ software selector, and Golden Rules switches backed by `hera-rs/src/dj_brain.rs`.
  - Added `/bridge` view (`bridge.tsx`) with Deck A ➔ Deck B P2P Harmonic Bridge resolution, 15-second transition preview (`Cue 4 Mix Out ➔ Cue 1 Intro`), and **Cloudflare Workers AI Free (`10,000 Neurons/day` at `$0.00`)** copilot.
- **Constrained `2-Opt TSP` Set Spine Builder (`hera-rs/src/synergy.rs`):**
  - Added `SetSpineConfig`, `EnergyAnchor`, `PinnedTrackConstraint`, and `build_set_spine` (`POST /api/v1/crates/set-spine`, MCP tool `build_set_spine`, and Code Mode `hera.buildSetSpine()`), raising the native Rust MCP server to **22 tools**.
- **Pure-Rust Native DJ Exporters & `DJ Style DNA` Engine (`hera-rs/src/export.rs`, `hera-rs/src/dj_brain.rs`):**
  - Exposed `export_dj_crate`, `compute_dj_style_dna`, and `chat_with_dj_copilot` over HTTP REST, MCP (`stdio` + HTTP), Tauri v2 IPC (`plugin:hera-dj`), and TypeScript Code Mode.
- **Live Playwright UI/UX QA Gallery (`docs/superpowers/assets/hera_live_*.png` & `hera-desktop/README.md`):**
  - Versioned 4 production screenshots (`1100x680` and `900x550`) and the 5 DJ Curator Superpowers guide (`39e920f`).

### Fixed
- **Root `.gitignore` Anchoring & `HERA Desktop` UI QA (`ADR-007`, `d876eb7`, `627079a`, `dcf404e`):**
  - Anchored `/lib/` to the root `.gitignore` so `hera-desktop/src/lib/` is tracked in Git.
  - Enforced `1:1` column order and flex widths between `TrackListHeader.tsx` and `TrackRow.tsx`, high-contrast `camelotBadgeSelected`, and default DJ-booth dark mode.
  - Registered `hera-dj` commands across the Tauri v2 ACL triad (`main.rs`, `build.rs`, `capabilities/main.json`).
  - Repositioned `Toasts.tsx` (`top: 56px, right: 12px`) with click-to-dismiss so playback alerts never overlap the `🤖 Consultar Copiloto` button in `/bridge`, and synchronized `useLibraryStore.setState({ tracks })` on library load.

### Removed
- **BREAKING (`c08677f`):** Purged legacy Python runtime and Streamlit dual-deck scripts in favor of the 100% Pure-Rust (`hera-rs`) + Tauri v2 (`hera-desktop`) architecture (`~8 MB` native binary with zero Python dependency).

---

## [0.2.0] - 2026-11-08

### Added
- **5-Layer Post-P2P Federated R2 Swarm Orchestrator (`hera-rs/src/p2p.rs`, `hera-rs/src/p2p_exchange.rs`, `ADR-003`, `ADR-004`, `ADR-005`):**
  - Constrained Water-Filling shard allocation respecting Cloudflare Free Tier safety caps (`9,000 MB` R2, `85,000 req/day`).
  - Latency-Aware Power-of-Two-Choices (`P2C`), single-use `60s` HMAC-signed `X-Hera-Swarm-Ticket` tokens, `1 : 1.5` Turbo Credit rewards, and 64-byte `Proof-of-Custody` range audits.
  - Streaming Merkle Inclusion Proofs (`16 x 256 KiB = 4 MB`), `ed25519` endpoint identity roaming (`Wi-Fi -> 5G`), Multi-Node `HTTP/3-QUIC` parallel striping, `Delta-CRDT` sync, and `MoQ` Semantic Harmonic Pub/Sub.
