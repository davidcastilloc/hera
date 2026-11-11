# Stage 1: Build pure-Rust static binary (Symphonia + rustfft + Axum) from workspace root
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

# Precompile dependencies in a cached layer
COPY Cargo.toml Cargo.lock ./
COPY hera-rs/Cargo.toml ./hera-rs/Cargo.toml
COPY hera-desktop/src-ipc/Cargo.toml ./hera-desktop/src-ipc/Cargo.toml

# Create dummy source files to cache dependency compilation
RUN mkdir -p hera-rs/src hera-desktop/src-ipc/src \
    && echo "fn main() {}" > hera-rs/src/main.rs \
    && echo "pub fn lib() {}" > hera-rs/src/lib.rs \
    && echo "pub fn lib() {}" > hera-desktop/src-ipc/src/lib.rs \
    && sed -i '/"hera-desktop\/src-tauri",/d' Cargo.toml \
    && cargo build --release --locked -p hera-node-rs --bin hera-node || true \
    && rm -rf hera-rs/src hera-desktop/src-ipc/src

# Copy real sources and compile final binary
COPY hera-rs ./hera-rs
COPY hera-desktop/src-ipc ./hera-desktop/src-ipc
RUN sed -i '/"hera-desktop\/src-tauri",/d' Cargo.toml \
    && cargo build --release --locked -p hera-node-rs --bin hera-node

# Stage 2: Ultra-minimal runtime (~15 MB disk, ~10 MB RAM)
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/hera-node /usr/local/bin/hera-node
ENV PORT=8080
ENV HERA_NODE_ID=hera-cloud-node
EXPOSE 8080
CMD ["hera-node"]
