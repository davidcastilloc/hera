#!/bin/bash
set -euo pipefail

if [ ! -f ./package.json ]; then
    echo "You should run HERA Desktop scripts from the hera-desktop directory, exiting..."
    exit 1
fi

# 1. Generate core Museeks/Tauri types when platform WebView headers are available
if [ "$(uname -s)" != "Linux" ] || pkg-config --exists libsoup-3.0 2>/dev/null; then
    cargo test --manifest-path src-tauri/Cargo.toml export_bindings
fi

# 2. Generate canonical HERA & Agent IPC types via pure-Rust hera-ipc (zero WebView headers required)
cargo test -p hera-ipc
