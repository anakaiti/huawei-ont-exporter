# syntax=docker/dockerfile:1

# Build stage with Cargo cache mount
FROM rust:1-slim-bookworm AS builder

WORKDIR /app

# Install dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# OCI Labels (build stage)
LABEL org.opencontainers.image.title="Huawei ONT Exporter" \
      org.opencontainers.image.description="Prometheus exporter for Huawei ONT optical metrics" \
      org.opencontainers.image.authors="Yahya H" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.source="https://github.com/anakaiti/huawei-ont-exporter"

# Copy manifest files first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir -p src && \
    echo 'fn main() {}' > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Now copy the real source code
COPY src ./src

# Build release binary - only recompiles changed files
RUN cargo build --release

# Runtime stage - using distroless nonroot
FROM gcr.io/distroless/cc-debian12:nonroot

# OCI Labels (runtime stage)
LABEL org.opencontainers.image.title="Huawei ONT Exporter" \
      org.opencontainers.image.description="Prometheus exporter for Huawei ONT optical metrics" \
      org.opencontainers.image.authors="Yahya H" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.source="https://github.com/anakaiti/huawei-ont-exporter"

# Copy the binary from builder
COPY --from=builder /app/target/release/huawei_ont_exporter /usr/local/bin/huawei_ont_exporter

# Use nonroot user (uid 65532 in distroless)
USER nonroot:nonroot

# Expose the metrics port
EXPOSE 8000

# Set readonly root filesystem (handled at runtime with securityContext in k8s or docker run --read-only)
# Here we just ensure the binary is executable
ENTRYPOINT ["/usr/local/bin/huawei_ont_exporter"]
