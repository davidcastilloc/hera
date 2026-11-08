# 🤖 HERA — Operational Guide for AI Agents & MCP Clients

**Package:** `hera` (`pyproject.toml` + `hera-rs/Cargo.toml`)  
**Protocol:** Model Context Protocol (MCP) `stdio` + HTTP + Code Mode  
**Harness:** `superpowers-odd` (`https://github.com/davidcastilloc/superpowers-odd`)  
**Philosophy:** *"Todo es posible si lo imaginas (If you can imagine it, you can build it)"*

---

## 🎯 Package Overview & Architecture

This repository contains the canonical Python + Rust implementation of **HERA**:
- `src/hera/cli.py`: Click CLI entry point (`hera = "hera.cli:main"`).
- `src/hera/contracts/`: Strict Pydantic v2 data models (`Track`, `Crate`, `Job`, `Candidate`).
- `src/hera/domain/`: SQLite database (`aiosqlite`), `HeraConfig`, community stats, and crate export logic.
- `src/hera/agent/`: Multi-backend conversational agent powered by Google Antigravity & GenAI SDKs.
- `src/hera/mcp/`: MCP protocol server and tool handlers.
- `src/hera/adapters/`: Cloud sync (`rclone`) and P2P daemon (`slskd`) bridges.
- `analyzers/`: Python DSP feature extractors (`librosa`, `ffmpeg`, `chromaprint`, `transients.py`, `style_embeddings.py`, `synergy_engine.py`).
- `hera-rs/`: Pure-Rust Universal Audio DSP, 2-Opt TSP Crate Curator & P2P Node (`hera-node-rs` v0.2.0, `axum`, `symphonia`, `rustfft`, features `native` and `cloudflare`).
- `.agent/skills/` & `.claude/skills/`: Multi-OS vendored `superpowers-odd` + `hera` skills (zero OS symlinks).
- `docs/superpowers/`: Multi-instance Git state (`specs/`, `plans/`, and `DECISIONS.md`).

---

## 🛠️ Canonical Build, Lint, Typecheck & Test Commands

Always use these exact commands verified against `pyproject.toml`, `.pre-commit-config.yaml`, and `hera-rs/Cargo.toml`:

```bash
# 1. Sync Python environment (Python >= 3.11)
uv sync

# 2. Linter & Formatter (Ruff — target py311, line-length 120)
uv run ruff check .
uv run ruff format --check .

# 3. Static Typecheck (Mypy)
uv run mypy src/

# 4. Python Unit & Integration Test Suite (Pytest — asyncio_mode = "auto")
uv run pytest

# 5. Rust DSP & P2P Node Verification (hera-rs)
cargo check --manifest-path hera-rs/Cargo.toml
cargo test --manifest-path hera-rs/Cargo.toml
```

---

## ⚡ Superpowers-ODD + RDD Mandatory Protocol (`superpowers-odd`)

Every AI agent working across **Windows**, **macOS**, or **Linux / Antigravity** MUST obey these rules:

1. **Zero Context Rot:**
   - Dispatch ephemeral subagents per task (`50–200` lines of code max per iteration).
   - Never drag stale session history between tasks or across sessions.
2. **100% Git-Backed Multi-Instance Synchronization (`superpowers:multi-instance-handoff`):**
   - All specifications live in `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`.
   - All implementation plans live in `docs/superpowers/plans/YYYY-MM-DD-<feature>.md` with `- [ ]` / `- [x]` checkboxes.
   - All distilled architectural decisions and root-cause bug findings live in `docs/superpowers/DECISIONS.md`.
   - **Handoff & Git Author Rule (Maintainer vs. Community Contributors):**
     - **Core Maintainer (`@davidcastilloc`):** Uses `git config user.name "David Castillo"` and `git config user.email "vikruzdavid@gmail.com"` (auto-configured by `scripts/ensure_latest_hera.py` on maintainer instances).
     - **Community Contributors:** `scripts/ensure_latest_hera.py` preserves your own GitHub `user.name` and `user.email`. Work on a feature branch (`feat/*` or `fix/*`) in your fork and submit a Pull Request against `main` using `.github/PULL_REQUEST_TEMPLATE.md`.
     - **Atomic Checkpoints:** Upon finishing each subtask in `docs/superpowers/plans/`, mark `[x]`, update `docs/superpowers/DECISIONS.md` only if there was a critical technical finding, and create an atomic Semantic Release / Conventional Commit (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`).
   - **Zero OS-Specific Absolute Paths & Zero Symlinks:** Never commit paths containing `C:\`, `/Users/`, or `/home/`. Always use repository-relative paths and `pathlib.Path`. Keep skills as physical files in `.agent/skills/` and `.claude/skills/`.
3. **ODD (Organic-Driven Development) Ceremony Scaling:**
   - **Micro-changes / Bugfixes (`< 30` LOC):** Go straight to failing test + minimal code fix + terminal verification receipt + atomic commit.
   - **Features / Architectural Changes (`>= 30` LOC):** Follow `brainstorming` (`docs/superpowers/specs/`) -> `writing-plans` (`docs/superpowers/plans/`) -> `subagent-driven-development` -> `verification-before-completion`.
4. **RDD (Receipt-Driven Development) & Anti-Hallucination Gate:**
   - Never claim a task is complete without pasting real terminal output (`exit code 0`) from linters (`ruff`), typecheck (`mypy` / `cargo check`), and tests (`pytest` / `cargo test`).
   - **Forbidden:** Fake tests (mocks testing mocks), empty stubs (`TODO`, `pass`, `unimplemented!()`), or deleting existing tests to pass the suite.
5. **Performance Gate & Stack Respect:**
   - **Surgical Search First:** Always use `ripgrep` (`rg -n`) before reading full files. Never bulk-read ignored directories (`.venv`, `target`, `node_modules`, `.wrangler`, `quarantine`, `library`, `exports`, `logs`).
   - **Stack Respect:** Never introduce unrequested UI frameworks or external dependencies; strictly respect `pyproject.toml` and `hera-rs/Cargo.toml`.

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

1. **`hera` (Public Repository — `davidcastilloc/hera`, root `.`)**:
   - **Role:** Open-Source Client, Local/Edge Pure-Rust DSP Engine (`hera-rs/`), MCP Server (`stdio` + HTTP + Code Mode), and Community P2P / `Edge Guardian` Node (`DaviDJ` ↔ `DJFoo`).
   - **Dual-Runtime Self-Hosting (VPS + Custom Cloudflare Workers):** Developers in the community can deploy this public `hera` repo either on **their own VPS / Local Machine** (`cargo run --release --manifest-path hera-rs/Cargo.toml` / `--features native`) OR on **their own Cloudflare Workers instance** (`cd hera-rs && npx wrangler deploy` / `--features cloudflare --no-default-features`). Any self-hosted `hera` instance can federate with other nodes and with `hera-cloud` via `POST /api/v1/p2p/federate-sync` and `/api/v1/swarm/*`.
   - **Federated R2 Edge Swarm (`hera-rs/src/p2p_exchange.rs` & [`docs/FEDERATED_R2_SWARM.md`](docs/FEDERATED_R2_SWARM.md)):**
     - **Motto:** *"Mientras más nodos de Hera, más fuerte es el sistema de enjambres."*
     - **Constrained Water-Filling & P2C:** Dynamically allocates hot `4 MB` immutable shards (`shard-blobs/{hmac}.hbin`) across healthy community Cloudflare Workers while capping usage at `9,000 MB` R2 (`CF_WORKER_SAFE_R2_CAP_MB`) and `85,000 req/day` (`CF_WORKER_SAFE_REQ_CAP`) so community nodes never exceed Cloudflare's Free Tier.
     - **`Edge Guardian` Meritocracy & 64B `Proof-of-Custody`:** Nodes passing random 64-byte range challenges (`Range: bytes=offset..offset+63`) get unlimited R2 Fast Lane access while online + `1.5 MB` of Turbo Credit per `1.0 MB` served (`SWARM_TURBO_CREDIT_MULTIPLIER = 1.5`) via single-use `ProofOfDeliveryReceipt` headers, gated by `60s` HMAC-signed `X-Hera-Swarm-Ticket` tokens and double-sealed (`hera_seal` + `audio_sha256`).
   - **Storage & Sharing Rule:** Connects Zero-OAuth Google Drive (`/embeddedfolderview`), Dropbox, OneDrive, or Local Disk P2P Seeder (`--music-dir`). **All uploaded tracks are shared on the P2P network by default (`share_on_p2p = true`).**
   - **Security Boundary:** Never place private SaaS unit economics (`/api/v1/cloud/economics`), payment webhooks, or unauthenticated paid tier upgrades (`/api/v1/dj/upgrade-tier`) inside this public repository.

2. **`hera-cloud` (Private Repository — `davidcastilloc/hera-cloud`, companion repo `../hera-cloud`)**:
   - **Role:** Private Cloudflare SaaS Control Plane, Paid R2 CAS Vault, Internal P2P Data Broker, and Subscription Tier Gatekeeper.
   - **How `hera` talks to `hera-cloud`:**
     - Local `hera` nodes analyze audio locally on the DJ's CPU (`$0.00` cloud CPU cost) and announce verified metadata (`hera_seal`, 4 Hot Cues, `cas_fingerprint`, `style_simhash_u64`) to `hera-cloud`.
     - `hera-cloud` enforces the DJ's subscription tier (`beta_mvp` = `5 GB` R2 vs `pro_monthly` = `50 GB` R2), performs Global CAS Deduplication (`0 MB` extra R2 for duplicate tracks), awards the `+38 MB` Curator Quota Refund Bounty (`POST /api/v1/broker/claim-cloud-track`), runs Safe Atomic Hot/Cold R2 Eviction (`POST /api/v1/broker/optimize-tiering`), and unlocks the **Private Vault Toggle (`POST /api/v1/dj/p2p-sharing`, `share_on_p2p = false`)** exclusively for `pro_monthly` subscribers.

See [`MANIFESTO.md`](MANIFESTO.md) and [`docs/superpowers/DECISIONS.md`](docs/superpowers/DECISIONS.md) for full architectural guidelines.
