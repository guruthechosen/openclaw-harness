# ============================================
# 🦞 OpenClaw Harness — Multi-stage Dockerfile
# ============================================

# --- Stage 1: Build Rust backend ---
FROM rust:1.82-slim AS rust-builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release

# --- Stage 2: Build React UI ---
FROM node:20-slim AS ui-builder

WORKDIR /app/ui
COPY ui/package.json ui/package-lock.json* ./
RUN npm ci
COPY ui/ ./
RUN npm run build

# --- Stage 3: Runtime ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=rust-builder /app/target/release/openclaw-harness /usr/local/bin/openclaw-harness

# Copy UI dist
COPY --from=ui-builder /app/ui/dist /app/ui/dist

# Copy default config
COPY config/ /app/config/

# Expose daemon port
EXPOSE 8380

# Health check
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD curl -sf http://localhost:8380/api/health || exit 1

ENV RUST_LOG=info

ENTRYPOINT ["openclaw-harness"]
CMD ["start", "--foreground"]
