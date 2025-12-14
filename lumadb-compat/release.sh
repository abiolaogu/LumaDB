
#!/bin/bash
set -e

echo "Building LumaDB v3.0 Release..."

# Ensure clean state
cargo clean -p luma-server

# Build optimized binary
# RUSTFLAGS: Native CPU optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release --package luma-server

echo "Build Complete: target/release/luma-server"
ls -lh target/release/luma-server
