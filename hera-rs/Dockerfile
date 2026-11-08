# Stage 1: Build pure-Rust static binary (Symphonia + rustfft + Axum)
FROM rust:1.85-slim AS builder
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release --bin hera-node

# Stage 2: Ultra-minimal runtime (~15 MB disk, ~10 MB RAM)
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/hera-node /usr/local/bin/hera-node
ENV PORT=8080
ENV HERA_NODE_ID=hera-cloud-node
EXPOSE 8080
CMD ["hera-node"]
