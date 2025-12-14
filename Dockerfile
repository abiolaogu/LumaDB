FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests first for layer caching
COPY lumadb-compat/Cargo.toml lumadb-compat/Cargo.lock ./lumadb-compat/
COPY lumadb-compat/crates ./lumadb-compat/crates
COPY Cargo.toml ./

# Build release binary
WORKDIR /app/lumadb-compat
RUN cargo build --release -p luma-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create lumadb user
RUN useradd -r -s /bin/false lumadb

# Create directories
RUN mkdir -p /var/lib/lumadb/data /var/lib/lumadb/wal /etc/lumadb \
    && chown -R lumadb:lumadb /var/lib/lumadb

# Copy binary
COPY --from=builder /app/lumadb-compat/target/release/luma-server /usr/local/bin/

# Copy default config
COPY lumadb-compat/config.toml /etc/lumadb/

# Set user
USER lumadb

# Expose all protocol ports
EXPOSE 5432 3306 6379 9200 9042 27017 8123 8082 9090 4317

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:9200/_cluster/health || exit 1

# Entry point
ENTRYPOINT ["luma-server"]
CMD ["--config", "/etc/lumadb/config.toml"]
