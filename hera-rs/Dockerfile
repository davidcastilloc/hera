# Stage 1: Build pure-Rust static binary (Symphonia + rustfft + Axum) from workspace root
FROM rust:1.85-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY hera-rs ./hera-rs
COPY hera-desktop/src-ipc ./hera-desktop/src-ipc
RUN sed -i '/"hera-desktop\/src-tauri",/d' Cargo.toml \
    && cargo build --release -p hera-node-rs --bin hera-node

# Stage 2: Ultra-minimal runtime (~15 MB disk, ~10 MB RAM)
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/hera-node /usr/local/bin/hera-node
ENV PORT=8080
ENV HERA_NODE_ID=hera-cloud-node
EXPOSE 8080
CMD ["hera-node"]
