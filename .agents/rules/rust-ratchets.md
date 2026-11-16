---
trigger: glob
globs: "**/*.rs"
description: Enforce HERA Rust <= 349 LOC submodule ratchets, forbid(unsafe_code) in audio_engine, Rust 2024 unsafe extern FFI, and golden_masters parity.
---
# Rust Architectural Ratchets
- Every `.rs` file inside `SLICED_SUBMODULE_DIRS` (`bpm_engine`, `scanner`, `genre_intelligence`, `shazam_id`, `mcp`, `code_mode`, `dj_brain`, `p2p`, `audio_engine`, `hera-ipc/src`, `plugins/hera_dj`, `plugins/hera_agent`) MUST be `<= 349` lines post-`cargo fmt`.
- Preserve `#![forbid(unsafe_code)]` in `hera-rs/src/audio_engine/` and 100% safe Rust in `hera-rs/src/scanner/omkf_capsule.rs`.
- In `hera-desktop/src-tauri` (`edition = "2024"`), any OS FFI block MUST use `unsafe extern "system"` or `unsafe extern "C"`.
- Preserve bit-identical snapshots in `hera-rs/tests/golden_masters.rs`.
