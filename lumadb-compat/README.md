# LumaDB

> **The AI-Native, Multi-Model Observability Platform**

[![Build Status](https://img.shields.io/github/actions/workflow/status/lumadb/lumadb/ci.yml?branch=main)](https://github.com/lumadb/lumadb/actions)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-3.0.0-green.svg)](CHANGELOG.md)

---

## 🚀 What is LumaDB?

LumaDB is a unified observability database that replaces your entire monitoring stack with a single **7.7 MB binary**. It speaks PostgreSQL, Prometheus, and OpenTelemetry protocols natively while storing all data in a high-performance columnar engine.

### Key Features

| Feature | Description |
|---------|-------------|
| 📊 **Multi-Protocol** | PostgreSQL wire, Prometheus API, OTLP gRPC |
| ⚡ **High Performance** | 2.5M+ ops/sec, 8x compression |
| 🔒 **Production Security** | MD5 auth, rate limiting, RBAC |
| 🗃️ **Multi-Tier Storage** | Hot (RAM) → Warm (SSD) → Cold (Object Store) |
| 🔍 **Full-Text Search** | RoaringBitmap inverted index |
| 🤖 **AI-Ready** | Vector search, embeddings support |

---

## 📦 Quick Start

### Binary

```bash
curl -LO https://github.com/lumadb/releases/latest/luma-server
chmod +x luma-server
./luma-server --config config.toml
```

### Docker

```bash
docker run -p 5432:5432 -p 9090:9090 -p 4317:4317 lumadb/lumadb:latest
```

### Connect

```bash
psql -h localhost -p 5432 -U lumadb -d default
# Password: lumadb
```

---

## 📖 Documentation

| Document | Description |
|----------|-------------|
| [Quick Start](docs/tutorials/quickstart.md) | Get running in 5 minutes |
| [User Manual](docs/user_manual.md) | Configuration and usage |
| [API Reference](docs/api-reference/api_reference.md) | All endpoints |
| [Architecture](docs/architecture_design.md) | System design |
| [Technical Spec](docs/technical_specification.md) | Implementation details |
| [Training Manual](docs/training_manual.md) | Learning resources |
| [Hardware Requirements](docs/hardware_requirements.md) | Sizing guide |

---

## 🏗️ Architecture

```
┌────────────┬────────────┬────────────┬────────────┐
│  :5432     │   :9090    │   :4317    │   :8080    │
│ PostgreSQL │ Prometheus │   OTLP     │   HTTP     │
└─────┬──────┴─────┬──────┴─────┬──────┴─────┬──────┘
      │            │            │            │
      └────────────┴─────┬──────┴────────────┘
                         ▼
               ┌───────────────────┐
               │   QueryExecutor   │
               │  (Arrow + SIMD)   │
               └─────────┬─────────┘
                         ▼
          ┌──────────────┴──────────────┐
          │      MultiTierStorage       │
          │  Hot → Warm → Cold + WAL    │
          └─────────────────────────────┘
```

---

## 🔧 Configuration

```toml
[general]
data_dir = "./data"
log_level = "info"

[postgres]
enabled = true
port = 5432
max_connections = 100

[metrics]
enabled = true
port = 9091

[prometheus]
enabled = true
port = 9090
```

---

## 🧪 Testing

```bash
# Run unit tests
cargo test -p luma-protocol-core

# Run all tests
cargo test --workspace
```

---

## 🔒 Security

- **Authentication:** PostgreSQL MD5 (SCRAM-SHA-256 planned)
- **Rate Limiting:** 100 requests/min per IP (configurable)
- **TLS:** Planned for v3.1

---

## 📊 Performance

| Metric | Value |
|--------|-------|
| Binary Size | 7.7 MB |
| Write Throughput | 2.5M ops/sec |
| Read Latency (p50) | < 50μs |
| Compression Ratio | 8x (Gorilla) |
| Startup Time | < 500ms |

---

## 🛣️ Roadmap

- [x] v3.0 - Observability Platform, Security, Performance
- [ ] v3.1 - TLS/SSL, SCRAM-SHA-256, Query Caching
- [ ] v3.2 - Distributed Mode, Raft Consensus
- [ ] v4.0 - PromptQL AI Queries

---

## 🤝 Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## 📄 License

Apache 2.0 - See [LICENSE](LICENSE).

---

**LumaDB - One Binary, All Your Observability Data**
