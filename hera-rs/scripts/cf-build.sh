#!/usr/bin/env bash
set -euo pipefail

CARGO_CACHE="$HOME/.npm/hera-cargo-home.tar.zst"
TARGET_CACHE="$HOME/.npm/hera-target-deps.tar.zst"
mkdir -p "$HOME/.npm"

if [ -f "$CARGO_CACHE" ] && command -v zstd >/dev/null 2>&1; then
  echo "[HERA Cache] Restaurando ~/.cargo desde caché .npm..."
  tar -I zstd -xf "$CARGO_CACHE" -C "$HOME" || true
fi

if [ ! -x "$HOME/.cargo/bin/cargo" ]; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable --target wasm32-unknown-unknown
fi
export PATH="$HOME/.cargo/bin:$PATH"
rustup target add wasm32-unknown-unknown

TARGET_DIR=$(cargo metadata --manifest-path Cargo.toml --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
mkdir -p "$TARGET_DIR"

if [ -f "$TARGET_CACHE" ] && command -v zstd >/dev/null 2>&1; then
  echo "[HERA Cache] Restaurando dependencias WASM compiladas en $TARGET_DIR..."
  tar -I zstd -xf "$TARGET_CACHE" -C "$TARGET_DIR" || true
fi

command -v worker-build >/dev/null 2>&1 || cargo install -q worker-build --locked --version 0.1.3
CARGO_INCREMENTAL=0 worker-build --release -- --locked --features cloudflare --no-default-features

if command -v zstd >/dev/null 2>&1; then
  echo "[HERA Cache] Guardando caché en $HOME/.npm..."
  tar -I "zstd -1 -T0" -cf "$CARGO_CACHE" -C "$HOME" .cargo/bin .cargo/registry/cache 2>/dev/null || true
  REL_PATHS=()
  for sub in release/deps release/build release/.fingerprint wasm32-unknown-unknown/release/deps wasm32-unknown-unknown/release/.fingerprint; do
    [ -e "$TARGET_DIR/$sub" ] && REL_PATHS+=("$sub")
  done
  if [ "${#REL_PATHS[@]}" -gt 0 ]; then
    tar -I "zstd -1 -T0" -cf "$TARGET_CACHE" -C "$TARGET_DIR" "${REL_PATHS[@]}" 2>/dev/null || true
  fi
fi
