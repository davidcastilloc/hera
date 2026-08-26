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
- `analyzers/`: DSP feature extractors (`librosa`, `ffmpeg`, `chromaprint`).

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
6. `analyze_track(track_id, profile)`
7. `organize_track(track_id, template, collision_policy)`
8. `build_dj_crate(brief, duration_minutes, constraints, export)`

---

## 🛡️ Key Invariants for AI Operators

- **Quarantine First:** Never write directly to `library/` without validation.
- **Explain Rankings:** Present score breakdown to users before recommending downloads.
- **Camelot Key Flow:** Use harmonic mix transitions (+1, -1, or relative major/minor) when arranging crates.
- **Local Isolation:** Do not send audio files or library contents to external paid APIs.

See the root [`AGENTS.md`](../../AGENTS.md) and [`MANIFESTO.md`](../../MANIFESTO.md) for full architectural guidelines.
