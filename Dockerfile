# syntax=docker/dockerfile:1

# Build stage
FROM rust:1-slim-bookworm AS builder

WORKDIR /app

# Install dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifest files first for better caching
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release

# Runtime stage - using distroless nonroot
FROM gcr.io/distroless/cc-debian12:nonroot

# Copy the binary from builder
COPY --from=builder /app/target/release/huawei_ont_exporter /usr/local/bin/huawei_ont_exporter

# Use nonroot user (uid 65532 in distroless)
USER nonroot:nonroot

# Expose the metrics port
EXPOSE 8000

# Set readonly root filesystem (handled at runtime with securityContext in k8s or docker run --read-only)
# Here we just ensure the binary is executable
ENTRYPOINT ["/usr/local/bin/huawei_ont_exporter"]
