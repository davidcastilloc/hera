#!/usr/bin/env bash
set -euo pipefail

echo "⚡ Installing Hera Universal Rust Audio DSP & P2P Node..."

if command -v docker >/dev/null 2>&1; then
  echo "🐳 Docker detected — building & launching hera-node container (~10 MB RAM)..."
  git clone --depth 1 https://github.com/davidcastilloc/hera.git /opt/hera || (cd /opt/hera && git pull)
  cd /opt/hera/hera-rs
  docker build -t hera-node-rs:latest .
  docker rm -f hera-node 2>/dev/null || true
  docker run -d --name hera-node --restart unless-stopped \
    -p 8080:8080 \
    -e HERA_NODE_ID="hera-vps-$(hostname)" \
    hera-node-rs:latest
  echo "✅ Hera Node live at http://$(hostname -I | awk '{print $1}'):8080/health"
else
  echo "🦀 Installing Rust toolchain to build native binary..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
  git clone --depth 1 https://github.com/davidcastilloc/hera.git "$HOME/hera" || (cd "$HOME/hera" && git pull)
  cd "$HOME/hera/hera-rs"
  cargo build --release --bin hera-node
  echo "✅ Built binary at $HOME/hera/hera-rs/target/release/hera-node"
fi
