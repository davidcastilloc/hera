#!/usr/bin/env bash
set -euo pipefail

echo "⚡ Installing Hera Universal Rust Audio DSP & P2P Node..."

if command -v docker >/dev/null 2>&1; then
  echo "🐳 Docker detected — building & launching hera-node container (~10 MB RAM)..."
  git clone --depth 1 https://github.com/davidcastilloc/hera.git /opt/hera || (cd /opt/hera && git pull)
  cd /opt/hera
  docker build -t hera-node-rs:latest -f hera-rs/Dockerfile .
  docker rm -f hera-node 2>/dev/null || true
  docker run -d --name hera-node --restart unless-stopped \
    -p 8080:8080 \
    -e HERA_NODE_ID="hera-vps-$(hostname)" \
    hera-node-rs:latest
  echo "✅ Hera Node live at http://$(hostname -I | awk '{print $1}'):8080/health"
else
  echo "🦀 Verifying Rust toolchain to build native binary..."
  if ! command -v cargo >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  fi
  if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
  fi
  export PATH="$HOME/.cargo/bin:$PATH"

  git clone --depth 1 https://github.com/davidcastilloc/hera.git "$HOME/hera" 2>/dev/null || (cd "$HOME/hera" && git pull --rebase)
  cd "$HOME/hera"

  RAM_MB=$(awk '/MemTotal/ {printf "%.0f", $2/1024}' /proc/meminfo 2>/dev/null || sysctl -n hw.memsize 2>/dev/null | awk '{printf "%.0f", $1/1048576}' || echo 4096)
  TARGET_DIR=$(cargo metadata --manifest-path hera-rs/Cargo.toml --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')

  if [ "${RAM_MB:-4096}" -lt 2500 ]; then
    echo "[*] Host con RAM limitada (${RAM_MB} MB): compilando con perfil release-lowmem..."
    CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 cargo build --manifest-path hera-rs/Cargo.toml --profile release-lowmem --locked --bin hera-node
    mkdir -p "$HOME/hera/bin"
    cp "${TARGET_DIR}/release-lowmem/hera-node" "$HOME/hera/bin/hera-node"
    echo "✅ Built binary at $HOME/hera/bin/hera-node"
  else
    echo "[*] Compilando hera-node optimizado con SIMD nativo (-C target-cpu=native)..."
    CARGO_INCREMENTAL=0 RUSTFLAGS="-C target-cpu=native" cargo build --manifest-path hera-rs/Cargo.toml --release --locked --bin hera-node
    mkdir -p "$HOME/hera/bin"
    cp "${TARGET_DIR}/release/hera-node" "$HOME/hera/bin/hera-node"
    echo "✅ Built binary at $HOME/hera/bin/hera-node"
  fi
fi
