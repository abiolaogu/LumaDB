# LumaDB Complete Tech Stack Reference

## Comprehensive Technology Documentation

**Version:** 3.0.0 | **Last Updated:** December 2024

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Rust Core Engine](#2-rust-core-engine)
3. [Go Cluster Layer](#3-go-cluster-layer)
4. [Python AI Service](#4-python-ai-service)
5. [TypeScript SDK & CLI](#5-typescript-sdk--cli)
6. [Admin UI](#6-admin-ui)
7. [Infrastructure & DevOps](#7-infrastructure--devops)
8. [Dependencies Reference](#8-dependencies-reference)

---

## 1. Architecture Overview

### Multi-Language Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           LumaDB Architecture                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │  TypeScript  │  │    Python    │  │      Go      │  │    Admin     │    │
│  │  SDK & CLI   │  │  AI Service  │  │   Cluster    │  │   UI (Web)   │    │
│  │              │  │              │  │              │  │              │    │
│  │ • Query API  │  │ • Vector     │  │ • Raft       │  │ • Dashboard  │    │
│  │ • Type-safe  │  │ • Embeddings │  │ • Sharding   │  │ • Explorer   │    │
│  │ • REPL       │  │ • PromptQL   │  │ • Routing    │  │ • GraphiQL   │    │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘    │
│         │                 │                 │                 │            │
│         └─────────────────┴────────┬────────┴─────────────────┘            │
│                                    │                                        │
│                          ┌─────────┴─────────┐                              │
│                          │   Protocol Layer   │                              │
│                          │                    │                              │
│                          │ PostgreSQL │ MySQL │                              │
│                          │ MongoDB │ Redis   │                              │
│                          │ InfluxDB │ OTLP   │                              │
│                          │ GraphQL │ REST    │                              │
│                          └─────────┬─────────┘                              │
│                                    │                                        │
│                          ┌─────────┴─────────┐                              │
│                          │                    │                              │
│                          │   RUST CORE       │                              │
│                          │   ENGINE          │                              │
│                          │                    │                              │
│                          │ • LSM-Tree        │                              │
│                          │ • SIMD Columnar   │                              │
│                          │ • Vector Search   │                              │
│                          │ • Hybrid Tiering  │                              │
│                          │ • io_uring I/O    │                              │
│                          │                    │                              │
│                          └────────────────────┘                              │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Language Responsibilities

| Language | Component | Lines of Code | Responsibility |
|----------|-----------|---------------|----------------|
| **Rust** | Core Engine | ~23,600 | Storage, indexing, query execution, performance |
| **Go** | Cluster Layer | ~25,000 | Distribution, consensus, routing, platform |
| **Python** | AI Service | ~5,000 | ML inference, embeddings, NLP |
| **TypeScript** | SDK & CLI | ~8,200 | Developer experience, type safety |
| **TypeScript** | Admin UI | ~2,000 | Web administration interface |

---

## 2. Rust Core Engine

### 2.1 Project Structure

```
rust-core/
├── src/
│   ├── storage/           # LSM-Tree storage engine
│   │   ├── engine.rs      # Main storage engine (564 LOC)
│   │   ├── sstable.rs     # Sorted String Tables (330 LOC)
│   │   └── mod.rs
│   │
│   ├── memory/            # Memory management
│   │   ├── memtable.rs    # In-memory write buffer
│   │   ├── cache.rs       # LRU block/row cache
│   │   ├── arena.rs       # Arena allocator
│   │   └── hot_cache.rs   # Hot data cache tier
│   │
│   ├── wal/               # Write-Ahead Logging
│   │   ├── optimized_wal.rs   # High-performance WAL (275 LOC)
│   │   └── group_commit.rs    # Batch commit (153 LOC)
│   │
│   ├── index/             # Indexing subsystem
│   │   ├── btree.rs       # B-Tree index
│   │   ├── hash.rs        # Hash index
│   │   └── mod.rs
│   │
│   ├── vector/            # Vector search engine
│   │   └── ultra_engine.rs    # HNSW implementation (754 LOC)
│   │
│   ├── columnar/          # Columnar storage
│   │   ├── simd.rs        # SIMD operations
│   │   └── compression.rs # Compression codecs
│   │
│   ├── tsdb/              # Time-series database
│   │   ├── core.rs        # TSDB core (397 LOC)
│   │   └── gorilla.rs     # Gorilla compression (478 LOC)
│   │
│   ├── tdengine/          # TDengine compatibility
│   │   ├── window.rs      # Window functions (740 LOC)
│   │   ├── aggregation.rs # Aggregations (467 LOC)
│   │   └── parser.rs      # SQL parser (463 LOC)
│   │
│   ├── hybrid/            # Hybrid tiering
│   │   ├── mod.rs         # Tier management
│   │   ├── tier.rs        # Tier definitions
│   │   ├── migration.rs   # Data migration
│   │   └── prefetch.rs    # Predictive prefetch
│   │
│   ├── dialects/          # Multi-dialect support
│   │   ├── influxql.rs    # InfluxQL parser
│   │   ├── promql.rs      # PromQL parser
│   │   ├── flux.rs        # Flux parser
│   │   └── ...            # Other dialects
│   │
│   ├── server/            # Protocol servers
│   │   ├── mod.rs         # Server coordinator
│   │   ├── prometheus.rs  # Prometheus API
│   │   ├── influxdb.rs    # InfluxDB wire protocol
│   │   └── kdb.rs         # KDB+ compatibility
│   │
│   ├── graphql/           # GraphQL engine
│   │   └── schema_generator.rs
│   │
│   ├── meilisearch/       # Full-text search
│   │   └── engine.rs
│   │
│   ├── security/          # Security subsystem
│   │   ├── rbac.rs        # Role-based access
│   │   └── rate_limit.rs  # Rate limiting
│   │
│   ├── observability/     # Observability
│   │   └── otel.rs        # OpenTelemetry
│   │
│   ├── io/                # I/O subsystem
│   │   └── uring.rs       # io_uring (Linux)
│   │
│   ├── ffi/               # FFI bindings
│   │   └── mod.rs         # C-compatible interface
│   │
│   ├── compaction/        # Background compaction
│   │   └── mod.rs
│   │
│   ├── config.rs          # Configuration
│   └── lib.rs             # Library entry
│
├── benches/
│   └── vector_bench.rs    # Benchmarks
│
└── Cargo.toml
```

### 2.2 Key Dependencies

```toml
[dependencies]
# Async Runtime
tokio = { version = "1.34", features = ["full"] }

# Concurrent Data Structures
dashmap = "5.5"                    # Lock-free HashMap
parking_lot = "0.12"               # Fast mutexes
crossbeam = "0.8"                  # Concurrent utilities
crossbeam-skiplist = "0.1"         # Lock-free skip list

# Serialization
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"                    # Binary serialization
rmp-serde = "1.1"                  # MessagePack
bson = "2.7"                       # BSON for MongoDB compat

# Compression
lz4_flex = "0.11"                  # LZ4 compression
zstd = "0.13"                      # Zstandard compression

# Storage
memmap2 = "0.9"                    # Memory-mapped files
crc32fast = "1.3"                  # CRC32 checksums

# Metrics & Tracing
prometheus = "0.13"                # Metrics export
tracing = "0.1"                    # Structured logging

# Utilities
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
xxhash-rust = { version = "0.8", features = ["xxh3"] }
regex = "1.10"

# Protocol Support
sqlparser = "0.40"                 # SQL parsing
tokio-util = "0.7"                 # Async utilities
byteorder = "1.5"                  # Byte order handling
hex = "0.4"                        # Hex encoding
sha2 = "0.10"                      # SHA-256
md5 = "0.7"                        # MD5 (PostgreSQL auth)

# Linux-specific
[target.'cfg(target_os = "linux")'.dependencies]
io-uring = "0.6"                   # io_uring support
```

### 2.3 Performance Characteristics

| Operation | Throughput | Latency (p99) | Notes |
|-----------|------------|---------------|-------|
| Single Insert | 450K ops/sec | 0.8ms | With WAL |
| Batch Insert (1K) | 2.1M docs/sec | 12ms | Group commit |
| Point Lookup | 1.2M ops/sec | 0.3ms | Hot cache |
| Range Scan | 180K scans/sec | 8ms | 1000 docs |
| Vector Search | 2.5M+ QPS | <1ms | Target |

---

## 3. Go Cluster Layer

### 3.1 Project Structure

```
go-cluster/
├── cmd/
│   └── server/
│       └── main.go            # Entry point
│
├── pkg/
│   ├── cluster/               # Raft consensus
│   │   ├── raft.go           # Raft implementation
│   │   ├── fsm.go            # Finite state machine
│   │   └── snapshot.go       # Snapshots
│   │
│   ├── api/                   # API layer
│   │   ├── server.go         # HTTP server
│   │   ├── grpc.go           # gRPC server
│   │   └── pb/               # Protocol Buffers
│   │       └── luma.pb.go
│   │
│   ├── platform/              # Platform server
│   │   ├── server.go         # Main platform server
│   │   │
│   │   ├── graphql/          # GraphQL engine
│   │   │   ├── engine.go     # Query execution
│   │   │   ├── schema.go     # Schema generation
│   │   │   └── metadata.go   # Metadata management
│   │   │
│   │   ├── auth/             # Authentication
│   │   │   ├── engine.go     # Auth engine
│   │   │   ├── jwt.go        # JWT handling
│   │   │   └── rbac.go       # Role-based access
│   │   │
│   │   ├── events/           # Event system
│   │   │   ├── triggers.go   # Event triggers
│   │   │   └── webhooks.go   # Webhook delivery
│   │   │
│   │   ├── federation/       # Data federation
│   │   │   └── registry.go   # Source registry
│   │   │
│   │   ├── cron/             # Task scheduling
│   │   │   └── scheduler.go  # Cron scheduler
│   │   │
│   │   ├── rest/             # REST API
│   │   │   └── generator.go  # Auto-generated API
│   │   │
│   │   └── mcp/              # Model Context Protocol
│   │       └── server.go     # MCP server
│   │
│   ├── query/                 # Query processing
│   │   ├── parser.go         # Query parser
│   │   ├── planner.go        # Query planner
│   │   ├── executor.go       # Query executor
│   │   └── hybrid_executor.go # Rust+Go hybrid
│   │
│   ├── router/                # Request routing
│   │   └── router.go         # Load balancer
│   │
│   ├── pool/                  # Connection pooling
│   │   └── connection_pool.go
│   │
│   ├── dialects/              # Multi-dialect
│   │   ├── router.go         # Dialect routing
│   │   └── detector.go       # Auto-detection
│   │
│   ├── tdengine/              # TDengine compat
│   │   └── engine.go         # TDengine API
│   │
│   └── meilisearch/           # Full-text search
│       └── api.go            # Meilisearch API
│
├── proto/
│   └── luma.proto            # gRPC definitions
│
└── go.mod
```

### 3.2 Key Dependencies

```go
module github.com/lumadb/go-cluster

go 1.24

require (
    // Raft Consensus
    github.com/hashicorp/raft v1.6.0
    github.com/hashicorp/raft-boltdb v0.0.0

    // HTTP Framework
    github.com/valyala/fasthttp v1.51.0
    github.com/fasthttp/router v1.4.0

    // gRPC
    google.golang.org/grpc v1.60.0
    google.golang.org/protobuf v1.32.0

    // GraphQL
    github.com/graphql-go/graphql v0.8.1

    // Message Queue
    github.com/twmb/franz-go v1.15.0    // Kafka client

    // Serialization
    github.com/vmihailenco/msgpack/v5 v5.4.0

    // Parsing
    github.com/alecthomas/participle/v2 v2.1.0

    // Kubernetes
    k8s.io/api v0.29.0
    k8s.io/client-go v0.29.0
    sigs.k8s.io/controller-runtime v0.16.0

    // Configuration
    github.com/spf13/viper v1.18.0

    // Scheduling
    github.com/robfig/cron/v3 v3.0.1

    // Authentication
    github.com/golang-jwt/jwt/v5 v5.2.0

    // Logging
    go.uber.org/zap v1.26.0

    // MCP
    github.com/mark3labs/mcp-go v0.6.0
)
```

### 3.3 Platform Features

| Feature | Implementation | Notes |
|---------|---------------|-------|
| GraphQL Engine | Auto-generated schemas | CRUD + subscriptions |
| REST Generator | OpenAPI compatible | Auto-generated endpoints |
| JWT Auth | HS256 tokens | Role-based |
| Event Triggers | INSERT/UPDATE/DELETE | Webhook delivery |
| Kafka Integration | Franz-go client | High throughput |
| MCP Server | Model Context Protocol | LLM integration |

---

## 4. Python AI Service

### 4.1 Project Structure

```
python-ai/
├── lumaai/
│   ├── main.py              # FastAPI server
│   ├── vector.py            # FAISS vector search
│   ├── nlp.py               # NLP processing
│   ├── inference.py         # Model inference
│   ├── llm.py               # LLM integration
│   ├── bindings.py          # Rust FFI
│   │
│   └── promptql/            # PromptQL engine
│       ├── engine.py        # Core engine
│       ├── planner.py       # Query planning
│       ├── executor.py      # Execution
│       ├── optimizer.py     # Optimization
│       ├── context.py       # Execution context
│       ├── reasoner.py      # Semantic reasoning
│       └── llm.py           # LLM interface
│
├── tdbai/                   # Legacy AI service
│   ├── vector_service.py    # Vector indexing
│   ├── embedding_onnx.py    # ONNX embeddings
│   ├── vector_gpu.py        # GPU acceleration
│   ├── promptql.py          # PromptQL
│   └── ingestion.py         # Data ingestion
│
├── tests/                   # Test suite
│
└── pyproject.toml
```

### 4.2 Key Dependencies

```toml
[project]
name = "lumaai"
version = "3.0.0"
requires-python = ">=3.10"

dependencies = [
    # Web Framework
    "fastapi>=0.104.0",
    "uvicorn>=0.24.0",

    # ML/AI
    "torch>=2.0.0",
    "transformers>=4.35.0",
    "sentence-transformers>=2.2.0",

    # Vector Search
    "faiss-cpu>=1.7.4",        # or faiss-gpu

    # Data
    "numpy>=1.24.0",

    # HTTP Client
    "httpx>=0.25.0",

    # Cache
    "redis>=5.0.0",

    # Validation
    "pydantic>=2.5.0",

    # Metrics
    "prometheus-client>=0.19.0",
]
```

### 4.3 AI Capabilities

| Feature | Technology | Performance |
|---------|------------|-------------|
| Vector Search | FAISS (IVF-PQ) | 10K+ QPS |
| Embeddings | Sentence Transformers | 100+ docs/sec |
| LLM Integration | OpenAI/Anthropic/Local | Configurable |
| PromptQL | Multi-step reasoning | Context-aware |
| GPU Acceleration | CUDA (optional) | 10x speedup |

---

## 5. TypeScript SDK & CLI

### 5.1 Project Structure

```
src/
├── core/                    # Core classes
│   ├── Database.ts         # Database class (434 LOC)
│   ├── Collection.ts       # Collection class (598 LOC)
│   ├── Transaction.ts      # ACID transactions (351 LOC)
│   └── Document.ts         # Document handling
│
├── query/                   # Query engine
│   ├── QueryEngine.ts      # Main engine (535 LOC)
│   └── parsers/
│       ├── LQLParser.ts    # SQL-like parser (717 LOC)
│       ├── NQLParser.ts    # Natural language (739 LOC)
│       └── JQLParser.ts    # JSON query (528 LOC)
│
├── storage/                 # Storage backends
│   ├── MemoryStorage.ts    # In-memory
│   └── FileStorage.ts      # Persistent (313 LOC)
│
├── indexing/                # Index implementations
│   ├── BTreeIndex.ts       # B-Tree (370 LOC)
│   ├── HashIndex.ts        # Hash index
│   └── FullTextIndex.ts    # Full-text (417 LOC)
│
├── llm/                     # LLM integrations
│   ├── OpenAIProvider.ts
│   ├── AnthropicProvider.ts
│   ├── GeminiProvider.ts
│   ├── DeepSeekProvider.ts
│   └── LlamaProvider.ts
│
├── promptql/                # PromptQL
│   └── PromptQLParser.ts
│
├── server/                  # HTTP server
│   └── index.ts            # REST API (291 LOC)
│
├── client/                  # Remote client
│   └── index.ts            # Client library (477 LOC)
│
├── cli/                     # CLI interface
│   └── index.ts            # REPL (488 LOC)
│
└── types/                   # Type definitions
    └── index.ts            # All types (346 LOC)
```

### 5.2 Key Dependencies

```json
{
  "dependencies": {
    "express": "^4.18.2",
    "commander": "^11.1.0",
    "uuid": "^9.0.0",
    "ws": "^8.14.0",
    "chalk": "^4.1.2"
  },
  "devDependencies": {
    "typescript": "^5.2.0",
    "jest": "^29.7.0",
    "ts-jest": "^29.1.0",
    "ts-node": "^10.9.0",
    "@types/node": "^20.0.0",
    "@types/express": "^4.17.0",
    "@types/ws": "^8.5.0",
    "@types/uuid": "^9.0.0",
    "eslint": "^8.50.0",
    "prettier": "^3.0.0"
  }
}
```

---

## 6. Admin UI

### 6.1 Project Structure

```
ui/admin/
├── src/
│   ├── app/                 # Next.js app router
│   │   ├── layout.tsx
│   │   ├── page.tsx
│   │   ├── collections/
│   │   ├── queries/
│   │   ├── graphql/
│   │   ├── events/
│   │   └── settings/
│   │
│   ├── components/          # React components
│   │   ├── Sidebar.tsx
│   │   ├── QueryEditor.tsx
│   │   ├── DataTable.tsx
│   │   └── ...
│   │
│   └── lib/                 # Utilities
│
├── package.json
└── tsconfig.json
```

### 6.2 Tech Stack

```json
{
  "dependencies": {
    "next": "^16.0.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "tailwindcss": "^3.4.0",
    "@tanstack/react-query": "^5.0.0",
    "zustand": "^4.4.0",
    "monaco-editor": "^0.45.0",
    "graphiql": "^3.0.0"
  }
}
```

---

## 7. Infrastructure & DevOps

### 7.1 Docker Configuration

**Main Dockerfile:**
```dockerfile
# Multi-stage build
FROM rust:1.75 as rust-builder
WORKDIR /app
COPY rust-core/ ./rust-core/
RUN cd rust-core && cargo build --release

FROM golang:1.24 as go-builder
WORKDIR /app
COPY go-cluster/ ./go-cluster/
RUN cd go-cluster && go build -o server ./cmd/server

FROM python:3.11-slim as python-builder
WORKDIR /app
COPY python-ai/ ./python-ai/
RUN pip install --no-cache-dir ./python-ai

FROM debian:bookworm-slim
# Copy all components
COPY --from=rust-builder /app/rust-core/target/release/libluma_core.so /usr/lib/
COPY --from=go-builder /app/go-cluster/server /usr/local/bin/
COPY --from=python-builder /usr/local/lib/python3.11 /usr/local/lib/python3.11

EXPOSE 8080 50051 10000 9090 4317
CMD ["server"]
```

### 7.2 Docker Compose

```yaml
version: '3.8'
services:
  lumadb:
    build: .
    ports:
      - "8080:8080"    # HTTP/GraphQL
      - "5432:5432"    # PostgreSQL
      - "3306:3306"    # MySQL
      - "27017:27017"  # MongoDB
      - "6379:6379"    # Redis
      - "9090:9090"    # Prometheus
      - "4317:4317"    # OTLP
    volumes:
      - lumadb-data:/var/lib/lumadb
    deploy:
      resources:
        limits:
          cpus: '8'
          memory: 16G
    environment:
      - LUMADB_DATA_DIR=/var/lib/lumadb
      - LUMADB_LOG_LEVEL=info

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9091:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    volumes:
      - ./monitoring/grafana:/var/lib/grafana

volumes:
  lumadb-data:
```

### 7.3 Kubernetes

**StatefulSet:**
```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: lumadb
spec:
  serviceName: lumadb
  replicas: 3
  selector:
    matchLabels:
      app: lumadb
  template:
    metadata:
      labels:
        app: lumadb
    spec:
      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
            - labelSelector:
                matchLabels:
                  app: lumadb
              topologyKey: kubernetes.io/hostname
      containers:
        - name: lumadb
          image: lumadb:latest
          ports:
            - containerPort: 8080
            - containerPort: 50051
            - containerPort: 10000
          resources:
            requests:
              cpu: "4"
              memory: "8Gi"
            limits:
              cpu: "8"
              memory: "16Gi"
          volumeMounts:
            - name: data
              mountPath: /var/lib/lumadb
  volumeClaimTemplates:
    - metadata:
        name: data
      spec:
        accessModes: ["ReadWriteOnce"]
        storageClassName: ssd
        resources:
          requests:
            storage: 100Gi
```

### 7.4 CI/CD Pipeline

```yaml
# .github/workflows/ci.yml
name: CI/CD

on: [push, pull_request]

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo build --release
      - run: cargo test

  go:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-go@v5
        with:
          go-version: '1.24'
      - run: go vet ./...
      - run: go test ./...
      - run: go build ./cmd/server

  typescript:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm ci
      - run: npm run lint
      - run: npm test -- --coverage
      - run: npm run build

  python:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.11'
      - run: pip install -e ./python-ai[dev]
      - run: black --check .
      - run: ruff check .
      - run: pytest

  docker:
    needs: [rust, go, typescript, python]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: docker/build-push-action@v5
        with:
          push: ${{ github.ref == 'refs/heads/main' }}
          tags: lumadb:${{ github.sha }}
```

---

## 8. Dependencies Reference

### 8.1 Complete Dependency Matrix

| Component | Language | Key Dependencies | Version |
|-----------|----------|------------------|---------|
| **Storage** | Rust | tokio, dashmap, lz4_flex | 1.34, 5.5, 0.11 |
| **Cluster** | Go | hashicorp/raft, fasthttp | 1.6.0, 1.51.0 |
| **AI** | Python | torch, faiss-cpu, fastapi | 2.0+, 1.7.4+, 0.104+ |
| **SDK** | TypeScript | express, commander | 4.18, 11.1 |
| **UI** | TypeScript | next, react, tailwind | 16, 19, 3.4 |

### 8.2 System Requirements

| Resource | Minimum | Recommended | Production |
|----------|---------|-------------|------------|
| CPU | 4 cores | 8 cores | 16+ cores |
| RAM | 8 GB | 16 GB | 64+ GB |
| Storage | 50 GB SSD | 200 GB NVMe | 1+ TB NVMe |
| Network | 1 Gbps | 10 Gbps | 25+ Gbps |
| OS | Linux 5.4+ | Linux 6.1+ | Linux 6.1+ |

### 8.3 Supported Platforms

| Platform | Status | Notes |
|----------|--------|-------|
| Linux (x86_64) | ✅ Full Support | io_uring, SIMD |
| Linux (ARM64) | ✅ Full Support | NEON SIMD |
| macOS (x86_64) | ✅ Supported | No io_uring |
| macOS (ARM64) | ✅ Supported | No io_uring |
| Windows | ⚠️ Limited | WSL2 recommended |

---

*Document Version: 3.0.0*
*Last Updated: December 2024*
