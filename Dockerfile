# Multi-stage build for minimal final image
FROM rust:1.75-slim as builder

WORKDIR /usr/src/app

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release

# Runtime stage with minimal base image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false mad

# Create directories
RUN mkdir -p /opt/mad/{config,logs} && \
    chown -R mad:mad /opt/mad

# Copy binary from builder stage
COPY --from=builder /usr/src/app/target/release/mad /opt/mad/bin/mad

# Switch to app user
USER mad
WORKDIR /opt/mad

# Expose ports
EXPOSE 7090 7091

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:7090/ping || exit 1

# Default command
ENTRYPOINT ["/opt/mad/bin/mad"]
CMD ["config/config.conf"]