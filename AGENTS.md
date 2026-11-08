# 🤖 HERA — Operational Guide for AI Agents & MCP Clients

**Package:** `outputs/hera`  
**Protocol:** Model Context Protocol (MCP) `stdio`  
**Philosophy:** *"Todo es posible si lo imaginas (If you can imagine it, you can build it)"*

---

## 🎯 Package Overview

This directory contains the canonical Python implementation of **HERA**:
- `src/hera/cli.py`: Click CLI entry point.
- `src/hera/agent/`: Multi-backend conversational agent powered by the Google Antigravity SDK.
- `src/hera/mcp/`: MCP protocol server and tool handlers.
- `src/hera/domain/`: SQLite database, configuration, and crate export logic.
- `src/hera/adapters/`: Cloud sync (`rclone`) and P2P daemon (`slskd`) bridges.
- `analyzers/`: DSP feature extractors (`librosa`, `ffmpeg`, `chromaprint`, `transients.py`, `style_embeddings.py`, `synergy_engine.py`).

---

## 🔌 Running the MCP Server

To expose HERA's tools to any MCP-compatible agent (Claude, Codex, Antigravity, Cursor):

```bash
uv run hera serve
```

### Supported MCP Tools:
1. `search_music(query, filters, providers)`
2. `get_track_candidates(search_id, limit)`
3. `download_track(candidate_id, authorization, idempotency_key)`
4. `download_status(job_id)`
5. `identify_track(asset_id)`
6. `analyze_track(track_id, profile, genre_hint)`
7. `analyze_batch(track_ids, profile, genre_hint, max_workers)`
8. `organize_track(track_id, template, collision_policy)`
9. `build_dj_crate(brief, duration_minutes, constraints, export)`
10. `audit_repertoire(sync_db, check_queries)`

---

## 🛡️ Key Invariants for AI Operators

- **Quarantine First:** Never write directly to `library/` without validation.
- **Explain Rankings:** Present score breakdown to users before recommending downloads.
- **Unified Holistic Synergy:** Always use the 4-layer synergy engine (Camelot harmonic flow, gradual BPM tempo curves, transient punch matching, and Discogs-EffNet style embeddings) when arranging crates.
- **Local Isolation:** Do not send audio files or library contents to external paid APIs.

---

## 🌐 Two-Repository Open-Core Ecosystem (`hera` ↔ `hera-cloud`)

The Hera ecosystem is divided into two complementary repositories that work together:

1. **`hera` (Public Repository — `c:\Users\David Castillo\Git-Repos\hera`)**:
   - **Role:** Open-Source Client, Local/Edge Pure-Rust DSP Engine (`hera-rs/`), MCP Server (`stdio` + HTTP + Code Mode), and Community P2P Node (`DaviDJ` ↔ `DJFoo`).
   - **Storage & Sharing Rule:** Connects Zero-OAuth Google Drive (`/embeddedfolderview`), Dropbox, OneDrive, or Local Disk P2P Seeder (`--music-dir`). **All uploaded tracks are shared on the P2P network by default (`share_on_p2p = true`).**
   - **Security Boundary:** Never place private SaaS unit economics (`/api/v1/cloud/economics`), payment webhooks, or unauthenticated paid tier upgrades (`/api/v1/dj/upgrade-tier`) inside this public repository.

2. **`hera-cloud` (Private Repository — `c:\Users\David Castillo\Git-Repos\hera-cloud`)**:
   - **Role:** Private Cloudflare SaaS Control Plane, Paid R2 CAS Vault, Internal P2P Data Broker, and Subscription Tier Gatekeeper.
   - **How `hera` talks to `hera-cloud`:**
     - Local `hera` nodes analyze audio locally on the DJ's CPU (`$0.00` cloud CPU cost) and announce verified metadata (`hera_seal`, 4 Hot Cues, `cas_fingerprint`, `style_simhash_u64`) to `hera-cloud`.
     - `hera-cloud` enforces the DJ's subscription tier (`beta_mvp` = `5 GB` R2 vs `pro_monthly` = `50 GB` R2), performs Global CAS Deduplication (`0 MB` extra R2 for duplicate tracks), awards the `+38 MB` Curator Quota Refund Bounty (`POST /api/v1/broker/claim-cloud-track`), runs Safe Atomic Hot/Cold R2 Eviction (`POST /api/v1/broker/optimize-tiering`), and unlocks the **Private Vault Toggle (`POST /api/v1/dj/p2p-sharing`, `share_on_p2p = false`)** exclusively for `pro_monthly` subscribers.

See the root [`AGENTS.md`](../../AGENTS.md) and [`MANIFESTO.md`](../../MANIFESTO.md) for full architectural guidelines.
