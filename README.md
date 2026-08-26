<div align="center">

# 🎧 HERA
### *The Autonomous, Local-First AI Super-Agent for DJs & Music Curators*

[![Python 3.11+](https://img.shields.io/badge/python-3.11+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Platform: Linux | macOS | Windows](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg)](https://github.com/davidcastilloc/hera)
[![Protocol: MCP](https://img.shields.io/badge/protocol-MCP%20stdio-orange.svg)](https://modelcontextprotocol.io/)
[![Cloud: rclone Powered](https://img.shields.io/badge/cloud-Google%20Drive%20%7C%20S3%20%7C%20R2-brightgreen.svg)](https://rclone.org/)

<p align="center">
  <b>Hera</b> is an intelligent, cross-platform audio curation and asset orchestrator designed for real human DJs.<br/>
  It discovers studio-grade audio across authorized networks, executes deep acoustic & harmonic analysis (BPM, Camelot Wheel, LUFS), embeds native metadata tags, organizes physical DJ crates, and syncs seamlessly with Google Drive and cloud storage.
</p>

---

</div>

## 📑 Table of Contents
- [✨ Core Philosophy](#-core-philosophy)
- [⚡ Key Features](#-key-features)
- [🏗️ System Architecture](#️-system-architecture)
- [🚀 Quickstart & Installation](#-quickstart--installation)
- [🎛️ CLI Reference](#️-cli-reference)
- [☁️ Cloud & Google Drive Sync](#️-cloud--google-drive-sync)
- [📋 Wishlist & Roadmap Board (DJ & Human-Centric)](#-wishlist--roadmap-board-dj--human-centric)
- [🤝 Contributing](#-contributing)
- [📄 License](#-license)

---

## ✨ Core Philosophy

* **100% Real Audio — Zero Synthetic Placeholders:** No synthetic tone generators or beep placeholders. Hera only promotes verified, full-length lossless FLAC and 320kbps MP3 master files to your library.
* **Human-Centric & DJ-First:** Built for the booth and the studio. All songs are organized into clean, human-readable folders with rich metadata (BPM, Key, Camelot) embedded directly inside the audio files.
* **Local-First & Cloud-Agnostic:** Runs entirely on your local machine, VPS, or Docker container with SQLite and stdio-based Model Context Protocol (MCP). No mandatory paid third-party AI subscriptions.
* **Cross-Platform Portability:** Operates identically on **Linux (x86_64, ARM64)**, **macOS**, and **Windows**.

---

## ⚡ Key Features

| Capability | Description |
|---|---|
| **P2P Federated Discovery** | Automated headless acquisition via Soulseek (`slskd`) with slot-availability prioritization. |
| **Acoustic & Harmonic DSP** | Precise extraction of tempo (**BPM**), harmonic key (**Camelot Wheel** notation, e.g. `8A`, `11B`), and loudness (**LUFS**) using `librosa` and `ffmpeg`. |
| **Native Tag Injection** | Automatically embeds standard **ID3v2.4** (MP3) and **Vorbis Comments** (FLAC) containing Title, Artist, Set/Album, BPM, InitialKey, and Track Numbers. |
| **Simple Crate Management** | Generates standalone set folders ready for any USB stick, media player, or DJ software without locked proprietary database formats. |
| **1-Click Multi-Cloud Sync** | Direct headless OAuth synchronization with **Google Drive**, **Cloudflare R2**, **Amazon S3**, and **Dropbox** powered by `rclone`. |
| **MCP AI Server Integration** | Exposes a full Model Context Protocol (MCP) server over `stdio` for seamless pair-programming with AI agents (Claude, Codex, Antigravity, Cursor). |

---

## 🏗️ System Architecture

```text
 outputs/hera/
 ├── bin/                    # Standalone auto-downloaded binaries (slskd, rclone, fpcalc)
 ├── config/                 # Configuration files (hera.toml, slskd.yml)
 ├── library/                # Canonical organized artist archive
 ├── sets/                   # Clean, human-friendly DJ Set folders
 │   ├── Set 1 - French Touch & Vocal House (2000-2005)/
 │   ├── Set 2 - Electro House & Peak-Time (2002-2005)/
 │   ├── Set 3 - Eurodance & Trance Anthems (2000-2005)/
 │   ├── Set 4 - House Divas & Club Classics/
 │   └── Sensation White Megamixes (2002-2006)/
 ├── src/hera/
 │   ├── adapters/           # Cloud storage (rclone), P2P (slskd)
 │   ├── contracts/          # Pydantic schemas (Track, Crate, Job)
 │   ├── domain/             # Business logic (Config, Database, Export)
 │   ├── jobs/               # Asynchronous resilient job queue
 │   ├── mcp/                # MCP protocol tools & resource providers
 │   └── cli.py              # Turnkey Click CLI interface
 ├── tests/                  # Automated pytest suite
 └── hera.db                 # Lightweight local SQLite metadata store
```

---

## 🚀 Quickstart & Installation

Hera requires **Python 3.11+** and uses [`uv`](https://github.com/astral-sh/uv) for blazing-fast dependency resolution.

### 1. Clone Repository
```bash
git clone https://github.com/davidcastilloc/hera.git
cd hera
```

### 2. Turnkey Setup
Run the automated setup command. It will detect your operating system (**Linux**, **macOS**, or **Windows**), download the necessary standalone binaries (`slskd`, `fpcalc`, `rclone`), and initialize the SQLite database:
```bash
uv run hera setup
```

### 3. Verify System Health
```bash
uv run hera doctor
```
```text
[*] Running Hera health diagnostic...
[OK] Operating System: Linux / Windows / macOS
[OK] Python: 3.11+
[OK] SQLite Database: hera.db accessible
[OK] Binary ffmpeg: available
[OK] Binary ffprobe: available
[OK] Binary fpcalc: installed in bin/
[OK] Daemon Soulseek (slskd): installed in bin/
[OK] Cloud Engine (rclone): available (v1.69.1)
[OK] DSP Acoustic Engine (librosa): installed
[OK] Tagging Engine (mutagen): installed
[OK] MCP Python SDK: installed
[SUCCESS] All critical systems and tools are ready.
```

---

## 🎛️ CLI Reference

```bash
# General Setup and Diagnostics
uv run hera setup                  # Initialize directories, DB, and auto-download binaries
uv run hera doctor                 # Check health of all tools, engines, and remotes

# P2P Soulseek Daemon
uv run hera slskd                  # Start the local Soulseek daemon at http://localhost:5030

# Cloud & Google Drive Sync
uv run hera sync login             # 1-Click Direct OAuth web login for Google Drive
uv run hera sync status            # Check connection status and configured remotes
uv run hera sync push              # Upload & sync local sets/ to Google Drive (gdrive:Hera_Music/sets)
uv run hera sync pull              # Download sets from cloud to local machine
uv run hera sync config            # Advanced interactive assistant (S3, Cloudflare R2, Dropbox)

# AI Agent Server
uv run hera serve                  # Start Model Context Protocol (MCP) stdio server
```

---

## ☁️ Cloud & Google Drive Sync

Connecting your Google Drive is designed to be **100% human-friendly** without tedious manual JSON credentials or multi-step question prompts.

### 1-Click Direct OAuth
```bash
uv run hera sync login
```
1. Hera automatically opens your default web browser to the Google OAuth consent screen.
2. Select your Google account and click **"Allow"**.
3. The terminal instantly confirms authorization.

### Sync Local Sets to the Cloud
```bash
uv run hera sync push
```
All curated folders in `sets/` will be uploaded to `Hera_Music/sets` on your Google Drive, complete with all embedded BPM, Key, and Camelot tags.

---

## 📋 Wishlist & Roadmap Board (DJ & Human-Centric)

This board tracks upcoming features designed specifically to streamline live DJ performance, mobile accessibility, and team collaboration:

```text
========================================================================================
                                 HERA WISHLIST BOARD
========================================================================================

[ 📋 ] DJ Cue Sheet Generator (_00_SET_GUIDE.txt)
       Automatically generate an easy-to-read ASCII overview file in each set folder:
       - Displays track sequence, BPM progression, Camelot harmonic mix path, and energy.
       - Optimized for instant viewing on mobile phones in the DJ booth.

[ 📱 ] Terminal Mobile QR Code Fast-Access
       Upon completing `hera sync push`, display a scannable ASCII QR code in the terminal
       for instant opening of the Google Drive set folder on your phone or tablet.

[ 💾 ] 1-Click "Gig USB" Exporter (`hera sync to-usb E:`)
       Export selected sets directly to FAT32/exFAT USB drives with clean filesystem naming,
       omitting hidden OS metadata files that can cause playback stutter on Pioneer CDJs.

[ 🔄 ] Silent Background Auto-Sync (`auto_sync: true`)
       Automatically sync new sets to Google Drive in the background as soon as DSP
       validation finishes, sending a lightweight OS notification when ready.

[ 🤝 ] B2B Shared Crate Collaboration (`hera sync b2b <drive-folder-link>`)
       Allow two DJs playing Back-to-Back to drop tracks into a shared Google Drive folder;
       Hera downloads, analyzes, and arranges them into an optimal harmonic sequence.

[ 🎧 ] Lightweight Mobile Preview Streamer
       Optionally transcode sets to low-bitrate MP3 previews in a `_previews/` folder
       for quick, data-saving track previewing on mobile connections.
========================================================================================
```

---

## 🤝 Contributing

We welcome contributions from developers, DJs, and audio engineers! Please review [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines on:
- Setting up your local development environment with `uv`.
- Coding conventions, type annotations, and Pydantic domain models.
- Running automated test suites and linters (`pytest`, `ruff`, `mypy`).
- Submitting Pull Requests and Git commit conventions.

---

## 📄 License

Hera is licensed under the **MIT License**. See the [LICENSE](LICENSE) file for details.

<div align="center">
  <sub>Crafted for DJs, producers, and audio curators worldwide.</sub>
</div>
