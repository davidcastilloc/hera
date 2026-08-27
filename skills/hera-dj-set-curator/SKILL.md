---
name: hera-dj-set-curator
description: >
  Autonomous end-to-end DJ crate curator and harmonic set orchestrator. Acquires studio-grade
  masters from federated P2P sources (Soulseek), performs FFT zero-trust acoustic audits (zero
  synthetic tones), extracts harmonic Camelot Wheel keys and BPM via Librosa DSP, injects native
  ID3v2.4/Vorbis metadata, sequences 60+ minute continuous crates (Side A / Side B), configures 800 Mbps
  community super-seeding, and pushes automated backups to Google Drive.
---

# 🎧 HERA DJ Set Curator & Harmonic Orchestrator

This skill allows agents to autonomously transform natural language music briefs (e.g., *"Build 8 French Touch and Electro sets from 2000-2005"*) into **verified, physical DJ crates ($\ge 60$ minutes each)** with harmonic sequencing, cloud backups, and community super-seeding.

```text
[ Natural Language DJ Intent ]
              │
              ▼
   [ hera-dj-set-curator ]
   ├── 1. Federated P2P Search with Anti-Flood Spacing (slskd)
   ├── 2. Zero-Trust Quarantine & FFT Spectral Flatness Validation
   ├── 3. Librosa Acoustic Feature Extraction (BPM, Camelot Key, LUFS)
   ├── 4. Mutagen Native Tag Injection (ID3v2.4 / FLAC Vorbis)
   ├── 5. Harmonic Crate Assembly & Transition Cue Sheet Export (>= 60 min)
   ├── 6. 800 Mbps P2P Super-Seed Distribution (Community Good Citizen)
   └── 7. 1-Click Multi-Cloud Backup (Google Drive via rclone)
```

---

## 🚨 Invariant Rules for AI Agents

1. **Zero Synthetic Audio**: Never accept pure sine wave placeholders or fake upscaled transcodes. Reject any track with $Peak\ Ratio > 0.35$ and $Spectral\ Flatness < 10^{-4}$.
2. **Hard 60+ Minute Rule**: Every generated crate must contain enough full-length tracks to total **$\ge 60.0$ minutes** of playtime. If a set is under 60 minutes, query the library or P2P network to reinforce it.
3. **Harmonic & Holistic Synergy Compatibility**: Order tracks sequentially using the Unified Synergy Engine combining adjacent Camelot Wheel transitions (e.g. `8A` ➔ `8A` ➔ `9A` ➔ `10A` ➔ `10B`), steady/ascending BPM tempo curves, transient punch matching (Crest Factor), and Discogs-EffNet style taxonomy affinity.
4. **Anti-Flood & Good Citizen Protocol**: Maintain at least 7.5 seconds between federated P2P search calls to avoid server rate-limit bans, and ensure 800 Mbps / 100 slots remain open for community sharing.
5. **Deterministic Output & Cue Guides**: Always generate `_00_SET_GUIDE.txt` in each crate directory with track numbers, BPM, Camelot Keys, and durations.

---

## 🛠️ CLI Helper Tool

The skill includes a dedicated helper script `scripts/set_curator_cli.py` that handles all phases:

```bash
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
1. **Analyze Brief**: Extract era, sub-genres, target energy, and number of sets (Side A / Side B).
2. **Scan & Acquire**: Search local canonical library first; if missing, query Soulseek P2P.
3. **Validate & Promote**: Quarantine ➔ FFT check ➔ Librosa DSP ➔ Tag injection ➔ `library/`.
4. **Assemble Crates**: Create physical directories under `sets/` with Camelot progression and verify $\ge 60$ minutes.
5. **Sync & Super-Seed**: Back up to Google Drive and verify active 800 Mbps sharing daemon.
