# Polyguard Backend API
# Multi-stage build for minimal production image

# Build stage
FROM rust:1.83-bookworm AS builder

WORKDIR /usr/src/polyguard

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY app/Cargo.toml ./app/

# Create dummy source for dependency compilation
RUN mkdir -p app/src && \
    echo 'fn main() {}' > app/src/main.rs && \
    cargo build --release --manifest-path app/Cargo.toml && \
    rm -rf app/src

# Copy actual source
COPY app/src ./app/src

# Build release binary
RUN touch app/src/main.rs && \
    cargo build --release --manifest-path app/Cargo.toml

# Build sqlx-cli for migrations
RUN cargo install sqlx-cli --no-default-features --features postgres

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash polyguard
USER polyguard
WORKDIR /home/polyguard

# Copy binary and sqlx-cli from builder
COPY --from=builder /usr/src/polyguard/target/release/polyguard-api ./
COPY --from=builder /usr/local/cargo/bin/sqlx ./

# Copy migrations
COPY migrations ./migrations

# Runtime configuration
ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=8080

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["./polyguard-api"]
