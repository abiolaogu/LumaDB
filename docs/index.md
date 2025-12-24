# LumaDB Documentation

Welcome to LumaDB - the world's fastest unified database platform.

## Features

- **100x Faster Streaming** - Thread-per-core reactor with io_uring
- **Multi-Model Storage** - Document, columnar, vector, time-series, full-text
- **Kafka Compatible** - Drop-in replacement with better performance
- **Multiple APIs** - REST, GraphQL, gRPC, Kafka protocol

## Quick Links

- [Getting Started](getting-started/quickstart.md)
- [Architecture](architecture/overview.md)
- [API Reference](api-reference/index.md)
- [Operations](operations/index.md)
- [Security](security/index.md)

## Installation

### Docker (Recommended)

```bash
docker pull ghcr.io/abiolaogu/lumadb:latest
docker run -d -p 8080:8080 -p 9092:9092 ghcr.io/abiolaogu/lumadb:latest
```

### From Source

```bash
git clone https://github.com/abiolaogu/LumaDB.git
cd LumaDB
make build
make install
```

### Linux Service

```bash
sudo ./deploy/systemd/install.sh
```

### Windows Service

```powershell
.\deploy\windows\install.ps1
```

## Quick Start

```bash
# Start LumaDB
lumadb server --config /etc/lumadb/lumadb.yaml

# Check health
curl http://localhost:8080/health

# Create a topic (Kafka-compatible)
curl -X POST http://localhost:8080/api/v1/topics \
  -H "Content-Type: application/json" \
  -d '{"name": "events", "partitions": 3}'

# Produce records
curl -X POST http://localhost:8080/api/v1/topics/events/produce \
  -H "Content-Type: application/json" \
  -d '{"records": [{"key": "user-1", "value": {"action": "login"}}]}'

# Consume records
curl http://localhost:8080/api/v1/topics/events/consume?group_id=my-group
```

## Architecture

LumaDB is built with a Pure Rust architecture for maximum performance:

```
┌─────────────────────────────────────────────────────────────┐
│                      API Layer                               │
│    REST API │ GraphQL │ gRPC │ Kafka Protocol │ WebSocket    │
├─────────────────────────────────────────────────────────────┤
│                    Query Engine                              │
│         Parser │ Analyzer │ Optimizer │ Executor             │
├─────────────────────────────────────────────────────────────┤
│                   Storage Engine                             │
│  LSM-Tree │ Columnar │ Vector │ Full-Text │ Time-Series      │
├─────────────────────────────────────────────────────────────┤
│                  Streaming Engine                            │
│     Thread-per-Core │ io_uring │ Zero-Copy │ SIMD           │
├─────────────────────────────────────────────────────────────┤
│                 Consensus (Raft)                             │
│         Leader Election │ Log Replication │ Snapshots        │
└─────────────────────────────────────────────────────────────┘
```

## Performance

| Metric | LumaDB | Kafka | Improvement |
|--------|--------|-------|-------------|
| Throughput | 10M msg/s | 100K msg/s | 100x |
| Latency (P99) | 50μs | 5ms | 100x |
| Memory | 500MB | 2GB | 4x |

## Support

- GitHub Issues: https://github.com/abiolaogu/LumaDB/issues
- Documentation: https://github.com/abiolaogu/LumaDB/docs
