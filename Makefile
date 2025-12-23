# LumaDB Makefile
# Ultra-fast unified database platform

.PHONY: all build release debug test lint fmt clean docker help

# Default target
all: build

# Build targets
build: release

release:
	@echo "Building LumaDB (release)..."
	cd crates && cargo build --release

debug:
	@echo "Building LumaDB (debug)..."
	cd crates && cargo build

# Test targets
test:
	@echo "Running tests..."
	cd crates && cargo test

test-all: test
	@echo "Running all tests including integration..."
	cd crates && cargo test --all-features

# Code quality
lint:
	@echo "Running Clippy..."
	cd crates && cargo clippy --all-targets -- -D warnings

fmt:
	@echo "Formatting code..."
	cd crates && cargo fmt --all

fmt-check:
	@echo "Checking code format..."
	cd crates && cargo fmt --all -- --check

# Security
audit:
	@echo "Running security audit..."
	cd crates && cargo audit

security-scan: audit
	@echo "Security scan complete"

# Documentation
docs:
	@echo "Building documentation..."
	cd crates && cargo doc --no-deps --open

# Docker
docker-build:
	@echo "Building Docker image..."
	docker build -t ghcr.io/abiolaogu/lumadb:latest -f deploy/docker/Dockerfile .

docker-run:
	@echo "Running LumaDB in Docker..."
	docker-compose -f deploy/docker/docker-compose.yml up -d

docker-stop:
	@echo "Stopping LumaDB Docker containers..."
	docker-compose -f deploy/docker/docker-compose.yml down

docker-logs:
	docker-compose -f deploy/docker/docker-compose.yml logs -f

# Cluster
cluster-up:
	@echo "Starting 3-node cluster..."
	docker-compose -f deploy/docker/docker-compose.yml --profile cluster up -d

cluster-down:
	docker-compose -f deploy/docker/docker-compose.yml --profile cluster down

# Installation
install: release
	@echo "Installing LumaDB..."
	sudo cp crates/target/release/lumadb /usr/local/bin/
	sudo chmod +x /usr/local/bin/lumadb
	@echo "LumaDB installed to /usr/local/bin/lumadb"

uninstall:
	@echo "Uninstalling LumaDB..."
	sudo rm -f /usr/local/bin/lumadb

# Clean
clean:
	@echo "Cleaning build artifacts..."
	cd crates && cargo clean
	rm -rf dist/

# Development
dev:
	@echo "Starting development server..."
	cd crates && RUST_BACKTRACE=1 cargo run -- server --config ../configs/lumadb.production.yaml

watch:
	@echo "Starting watch mode..."
	cd crates && cargo watch -x "run -- server"

# Benchmarks
bench:
	@echo "Running benchmarks..."
	cd crates && cargo bench

# Release packaging
package: release
	@echo "Packaging release..."
	mkdir -p dist
	cp crates/target/release/lumadb dist/
	cp configs/lumadb.production.yaml dist/lumadb.yaml
	cp README.md dist/
	tar -czvf dist/lumadb-$(shell uname -s | tr '[:upper:]' '[:lower:]')-$(shell uname -m).tar.gz -C dist lumadb lumadb.yaml README.md
	@echo "Package created: dist/lumadb-*.tar.gz"

# Help
help:
	@echo "LumaDB Build System"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  build       Build release binary (default)"
	@echo "  release     Build optimized release binary"
	@echo "  debug       Build debug binary"
	@echo "  test        Run unit tests"
	@echo "  test-all    Run all tests"
	@echo "  lint        Run Clippy lints"
	@echo "  fmt         Format code"
	@echo "  fmt-check   Check code formatting"
	@echo "  audit       Run security audit"
	@echo "  docs        Build and open documentation"
	@echo ""
	@echo "  docker-build   Build Docker image"
	@echo "  docker-run     Start Docker container"
	@echo "  docker-stop    Stop Docker container"
	@echo "  docker-logs    View Docker logs"
	@echo ""
	@echo "  cluster-up     Start 3-node cluster"
	@echo "  cluster-down   Stop cluster"
	@echo ""
	@echo "  install     Install to /usr/local/bin"
	@echo "  uninstall   Remove installation"
	@echo "  clean       Clean build artifacts"
	@echo "  package     Create release package"
	@echo ""
	@echo "  dev         Start development server"
	@echo "  watch       Start watch mode"
	@echo "  bench       Run benchmarks"
	@echo ""
	@echo "  help        Show this help"
