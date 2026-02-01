# Build stage
FROM rust:slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml ./
COPY common ./common
COPY gateway ./gateway
COPY server ./server

# Build the server from workspace root
RUN cargo build --release --bin wyldlands-server

# Runtime stage
FROM debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/wyldlands-server /app/wyldlands-server

# Copy configuration
COPY --from=builder /app/server/config.yaml /app/config.yaml

# Run the server
CMD ["/app/wyldlands-server", "--config", "/app/config.yaml"]