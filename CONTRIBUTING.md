# 🤝 Contributing to Hera

Thank you for your interest in contributing to **Hera**! We welcome contributions from developers, DJs, audio engineers, and music enthusiasts.

Hera is an open-source, local-first super-agent designed to make audio discovery, acoustic DSP analysis, and DJ crate curation seamless, intelligent, and human-friendly.

---

## 📑 Table of Contents
- [Code of Conduct & Core Principles](#-code-of-conduct--core-principles)
- [Development Setup](#-development-setup)
- [Codebase Architecture](#-codebase-architecture)
- [Coding Standards](#-coding-standards)
- [Running Tests & Linting](#-running-tests--linting)
- [Pull Request Workflow](#-pull-request-workflow)

---

## 🎯 Code of Conduct & Core Principles

When contributing to Hera, please adhere to our core project tenets:

1. **100% Real Audio — Zero Placeholders:** Never commit or generate synthetic sine waves or beep placeholders in user-facing crates. Every track promoted to a library must be a genuine, validated audio master.
2. **Local-First & Privacy-Preserving:** Hera runs on the user's local machine, NAS, VPS, or Docker container with SQLite and stdio-based MCP. Avoid introducing mandatory dependencies on proprietary cloud APIs or cloud databases.
3. **Cross-Platform Portability:** All features must operate seamlessly across **Linux**, **macOS**, and **Windows**. Never hardcode OS-specific path separators; always use `pathlib.Path`.
4. **Human-Centric UX:** Keep CLI commands, error messages, and workflows intuitive and friendly for real human DJs working in fast-paced booth and studio environments.

---

## 🚀 Development Setup

Hera uses [`uv`](https://github.com/astral-sh/uv) for fast, deterministic dependency management.

### 1. Prerequisites
- **Python 3.11+**
- **Git**
- **FFmpeg & FFprobe** (installed and available on your system `PATH`)
- [`uv`](https://github.com/astral-sh/uv)

### 2. Clone and Install
```bash
git clone https://github.com/davidcastilloc/hera.git
cd hera

# Install virtual environment and dependencies
uv sync
```

### 3. Initialize Auxiliary Binaries & Database
Run the turnkey setup command to download standalone engine binaries (`slskd`, `fpcalc`, `rclone`) and initialize the SQLite schema:
```bash
uv run hera setup
uv run hera doctor
```

---

## 🏗️ Codebase Architecture

The project is structured into clean domain boundaries:

```text
src/hera/
├── contracts/          # Strict Pydantic models (Track, Crate, Job, Candidate)
├── domain/             # Core business rules (HeraConfig, Database, CrateExporter)
├── adapters/           # External tool integrations (RcloneStorageAdapter, etc.)
├── analyzers/          # DSP acoustic analyzers (FFmpegValidator, AudioFeatureAnalyzer)
├── jobs/               # Background task queue & idempotency runner
├── mcp/                # Model Context Protocol (MCP) server & tool handlers
└── cli.py              # Click command-line interface
```

### Key Subsystems:
* **Acoustic Engine (`analyzers/`):** Extracts BPM, musical key (Camelot Wheel notation), and loudness (LUFS) using `librosa` and `ffmpeg`.
* **Storage Adapter (`adapters/storage/rclone.py`):** Cross-platform cloud sync orchestrator supporting Google Drive, S3, R2, and Dropbox via `rclone`.
* **Tagging Engine (`mutagen`):** Injects ID3v2.4 and Vorbis Comments directly into audio headers.

---

## 📐 Coding Standards

* **Type Annotations:** All functions and methods must include explicit type hints (`str`, `Path`, `list[Track]`, `dict[str, Any]`).
* **Pydantic Validation:** All external data and configuration inputs must be validated through Pydantic v2 models.
* **Docstrings:** Maintain clear, concise docstrings for public classes and methods.
* **Async/Await:** Use asynchronous I/O (`asyncio`, `httpx`, `aiosqlite`) for network, database, and process execution tasks.

---

## 🧪 Running Tests & Linting

### Running Unit & Integration Tests
```bash
uv run pytest
```

### Code Formatting & Linting
We use [`ruff`](https://github.com/astral-sh/ruff) for blazing-fast linting and formatting:
```bash
# Format code
uv run ruff format .

# Check for lint errors
uv run ruff check .
```

### Type Checking
```bash
uv run mypy src/
```

---

## 🔄 Pull Request Workflow

1. **Fork the Repository:** Create a personal fork on GitHub.
2. **Create a Feature Branch:**
   ```bash
   git checkout -b feat/your-feature-name
   # or
   git checkout -b fix/your-bugfix-name
   ```
3. **Commit Your Changes:** Follow conventional commit guidelines:
   - `feat: add automatic DJ cue sheet generator`
   - `fix: resolve permission issue during active Soulseek download`
   - `docs: update cloud sync guide in README`
   - `refactor: optimize Camelot key detection pipeline`
4. **Push to Your Fork:**
   ```bash
   git push origin feat/your-feature-name
   ```
5. **Open a Pull Request:** Submit a PR against the `main` branch of `davidcastilloc/hera`. Provide a clear description of the problem solved, testing steps taken, and any relevant issue numbers.

---

## 💬 Questions & Community

Feel free to open an [Issue](https://github.com/davidcastilloc/hera/issues) or start a [Discussion](https://github.com/davidcastilloc/hera/discussions) on GitHub for feature requests, architectural ideas, or questions.

Thank you for helping make Hera the best AI super-agent for music curation! 🎧
