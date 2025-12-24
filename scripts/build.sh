#!/bin/bash
# LumaDB Build Script

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${BLUE}[STEP]${NC} $1"; }

# Parse arguments
BUILD_TYPE="${1:-release}"
TARGET="${2:-}"

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║              LumaDB Build System                          ║"
echo "║         100x Faster Streaming Database                    ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

cd "$(dirname "$0")/.."
ROOT_DIR=$(pwd)

# Check dependencies
log_step "Checking dependencies..."
command -v cargo >/dev/null 2>&1 || { log_error "Rust/Cargo not found. Install from https://rustup.rs"; exit 1; }
command -v rustc >/dev/null 2>&1 || { log_error "Rust compiler not found"; exit 1; }

RUST_VERSION=$(rustc --version)
log_info "Rust: $RUST_VERSION"

# Build configuration
CARGO_FLAGS=""
if [[ "$BUILD_TYPE" == "release" ]]; then
    CARGO_FLAGS="--release"
    log_info "Build type: Release (optimized)"
else
    log_info "Build type: Debug"
fi

if [[ -n "$TARGET" ]]; then
    CARGO_FLAGS="$CARGO_FLAGS --target $TARGET"
    log_info "Target: $TARGET"
fi

# Format check
log_step "Checking code formatting..."
(cd crates && cargo fmt --all -- --check) || {
    log_warn "Code formatting issues found. Run: cargo fmt --all"
}

# Clippy
log_step "Running Clippy lints..."
(cd crates && cargo clippy $CARGO_FLAGS --all-targets -- -D warnings) || {
    log_warn "Clippy warnings found"
}

# Build
log_step "Building LumaDB..."
(cd crates && cargo build $CARGO_FLAGS)

# Test
log_step "Running tests..."
(cd crates && cargo test $CARGO_FLAGS) || {
    log_warn "Some tests failed"
}

# Output path
if [[ "$BUILD_TYPE" == "release" ]]; then
    if [[ -n "$TARGET" ]]; then
        BINARY_PATH="crates/target/$TARGET/release/lumadb"
    else
        BINARY_PATH="crates/target/release/lumadb"
    fi
else
    if [[ -n "$TARGET" ]]; then
        BINARY_PATH="crates/target/$TARGET/debug/lumadb"
    else
        BINARY_PATH="crates/target/debug/lumadb"
    fi
fi

if [[ -f "$BINARY_PATH" ]]; then
    BINARY_SIZE=$(du -h "$BINARY_PATH" | cut -f1)
    log_info "Binary: $BINARY_PATH ($BINARY_SIZE)"
fi

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║              Build Complete!                              ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""
echo "Run LumaDB:"
echo "  $BINARY_PATH server --config configs/lumadb.production.yaml"
echo ""
