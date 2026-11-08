---
name: hera-dj-set-curator
description: >
  Autonomous end-to-end DJ crate curator and harmonic set orchestrator. Automatically verifies
  and updates Hera skills and binaries (hera, hera-node, slskd, rclone, ffmpeg) on startup,
  acquires studio-grade masters from federated P2P sources (Soulseek), performs FFT zero-trust
  acoustic audits (zero synthetic tones), extracts harmonic Camelot Wheel keys and BPM via
  Librosa/Rust DSP, injects native ID3v2.4/Vorbis metadata, sequences 60+ minute continuous
  crates (Side A / Side B), configures 800 Mbps community super-seeding, and pushes automated
  backups to Google Drive.
---

# ⚡ Step 0 (MANDATORY FIRST STEP): Auto-Verify & Auto-Update Skill and Binaries

**STRICT BOOTSTRAP RULE:** Before running any repertoire scan, P2P search, acoustic audit, or crate assembly, you **MUST** verify that Hera, its skills (`hera`, `hera-dj-set-curator`), and all required binaries (`hera`, `hera-node`, `slskd`, `rclone`, `ffmpeg`, `fpcalc`) are installed and updated to the latest version:

1. **Via MCP Tool (Preferred when MCP server is active):**
   Call `ensure_hera_updated(auto_compile_rust=True, sync_skills=True)` as your very first action.
2. **Via CLI Bootstrapper (Windows / Linux / macOS / Raspberry Pi):**
   ```bash
   python scripts/ensure_latest_hera.py
   ```
   - Automatically checks `git fetch/pull` (preserving any local uncommitted work), compiles the native Rust binary `hera-node` (`cargo build --release`) into `bin/hera-node(.exe)` whenever `hera-rs/src/*.rs` has new changes, verifies `.venv`, checks companion binaries, and syncs `skills/hera` and `skills/hera-dj-set-curator` into `~/.gemini/config/skills/`, `~/.claude/skills/`, and `~/.agents/skills/`.
   - If Hera is not installed yet on the system, run `./install.sh` (Linux/macOS/Pi) or `powershell -ExecutionPolicy Bypass -File .\install.ps1` (Windows).

---

# 🎧 HERA DJ Set Curator & Harmonic Orchestrator

This skill allows agents to autonomously transform natural language music briefs (e.g., *"Build 8 French Touch and Electro sets from 2000-2005"*) into **verified, physical DJ crates ($\ge 60$ minutes each)** with harmonic sequencing, cloud backups, and community super-seeding.

```text
[ Natural Language DJ Intent ]
              │
              ▼
   [ hera-dj-set-curator ]
   ├── 00. Mandatory Auto-Verification & Self-Update of Skills & Binaries (ensure_hera_updated / ensure_latest_hera.py)
   ├── 0.  Phase 1 Anti-Duplicate Audit & Repertoire Intelligence (audit_repertoire >= 85% fuzzy match + Global CAS SHA-256)
   ├── 1.  Federated P2P Search with Anti-Flood Spacing (slskd) & Video-DJ HD (mp4/webm/mkv)
   ├── 2.  Zero-Trust Quarantine & FFT Spectral Flatness Validation (in-place promotion for local tracks)
   ├── 3.  Librosa + Rust Symphonia/FFT + ITU-R BS.1770 Acoustic Extraction (DAW .als ground-truth BPM, Camelot, LUFS, dBTP)
   ├── 4.  Mutagen Native Tag Injection (ID3v2.4 / FLAC Vorbis) & Smart Filename Parsing
   ├── 5.  Harmonic Crate Assembly: 2-Opt TSP + Rekordbox XML (4 acoustic Hot Cues), VirtualDJ Crates, M3U8 & Ableton Live
   ├── 6.  800 Mbps P2P Super-Seed Distribution & Internal P2P Data Broker (64-bit POPCNT SimHash)
   └── 7.  1-Click Multi-Cloud Backup & Zero-Tech DJ Drive Onboarding (Google Drive / Cloudflare R2)
```

---

## 🚨 Invariant Rules for AI Agents

0. **Mandatory Bootstrap & Self-Update**: Always run `ensure_hera_updated` or `python scripts/ensure_latest_hera.py` first so the user always has the latest skill definitions and compiled binaries (`hera`, `hera-node`).
1. **Zero Synthetic Audio**: Never accept pure sine wave placeholders or fake upscaled transcodes. Reject any track with $Peak\ Ratio > 0.35$ and $Spectral\ Flatness < 10^{-4}$.
2. **Hard 60+ Minute Rule**: Every generated crate must contain enough full-length tracks to total **$\ge 60.0$ minutes** of playtime. If a set is under 60 minutes, query the library or P2P network to reinforce it.
3. **Harmonic & Holistic Synergy Compatibility**: Order tracks sequentially using the Unified Synergy Engine combining adjacent Camelot Wheel transitions (e.g. `8A` ➔ `8A` ➔ `9A` ➔ `10A` ➔ `10B`), steady/ascending BPM tempo curves (`(bpm // 3, camelot_wheel, bpm)`), transient punch matching (Crest Factor), and 64D style embedding affinity.
4. **Anti-Flood & Good Citizen Protocol**: Maintain at least 7.5 seconds between federated P2P search calls to avoid server rate-limit bans, and ensure 800 Mbps / 100 slots remain open for community sharing.
5. **Deterministic Output & Cue Guides**: Always generate `_00_SET_GUIDE.txt` in each crate directory with track numbers, BPM, Camelot Keys, and durations.

---

## 🛠️ CLI Helper Tool

The skill includes a dedicated helper script `scripts/set_curator_cli.py` (plus `scripts/ensure_latest_hera.py` for Step 0) that handles all phases:

```bash
# 0. Verify & auto-update Hera repo, Rust binary (hera-node), Python venv, and global agent skills
python scripts/ensure_latest_hera.py

# 1. Acquire tracks from Soulseek P2P with anti-flood spacing
uv run python skills/hera-dj-set-curator/scripts/set_curator_cli.py acquire \
  --queries "Daft Punk One More Time" "Modjo Lady" \
  --output acquisitions.json

# 2. Audit acoustic purity (FFT & Spectral Flatness)
uv run python skills/hera-dj-set-curator/scripts/set_curator_cli.py audit \
  --directory quarantine \
  --output audit_report.json

# 3. Assemble and enforce 60+ minute crates with Camelot sequencing
uv run python skills/hera-dj-set-curator/scripts/set_curator_cli.py build \
  --min-duration-mins 60 \
  --output crate_manifest.json

# 4. Configure 800 Mbps P2P Super-Seed sharing
uv run python skills/hera-dj-set-curator/scripts/set_curator_cli.py share \
  --speed-kbps 819200 \
  --slots 100 \
  --output sharing_status.json

# 5. Push completed crates to Google Drive
uv run python skills/hera-dj-set-curator/scripts/set_curator_cli.py sync \
  --remote "gdrive:Hera_Music/sets" \
  --output sync_report.json
```

---

## 📖 Standard Step-by-Step Agent Workflow

When the human asks for DJ set curation:
0. **Auto-Verify & Update**: Run `ensure_hera_updated` or `python scripts/ensure_latest_hera.py` to ensure the latest skill and binaries are active.
1. **Analyze Brief**: Extract era, sub-genres, target energy, and number of sets (Side A / Side B).
2. **Scan & Acquire**: Search local canonical library first (`audit_repertoire`); if missing, query Soulseek P2P.
3. **Validate & Promote**: Quarantine ➔ FFT check ➔ Librosa/Rust DSP ➔ Tag injection ➔ `library/`.
4. **Assemble Crates**: Create physical directories under `sets/` with global ascending BPM & Camelot progression and verify $\ge 60$ minutes.
5. **Sync & Super-Seed**: Back up to Google Drive and verify active 800 Mbps sharing daemon.
