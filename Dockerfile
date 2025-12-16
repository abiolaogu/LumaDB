FROM rust:1.75-bookworm as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    clang \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy source code
COPY rust-core ./rust-core
COPY lumadb-compat ./lumadb-compat
# Create a workspace Cargo.toml if it's missing, or we can build from within directories
# Since the user lacks a root Cargo.toml, we'll try to build by overriding the dependency path in lumadb-compat
# or assuming we can adjust paths.
# Best approach: Copy the fixed rust-core to replace the one in lumadb-compat if that's the intention,
# OR assume rust-core is the primary source.
# Given the user's workspace, 'rust-core' seems to be the main dev artifact.
# Let's replace the internal crate with our fixed one to ensure fixes propagate.
RUN rm -rf lumadb-compat/crates/luma-core && cp -r rust-core lumadb-compat/crates/luma-core

# Build release binary
WORKDIR /app/lumadb-compat
RUN cargo build --release -p luma-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
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
