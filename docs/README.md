# LumaDB Documentation

## The Next-Generation High-Performance Database

LumaDB is a modern, multi-language database platform designed to outperform existing enterprise databases including Aerospike, ScyllaDB, DragonflyDB, YugabyteDB, and kdb+.

---

## Documentation Index

### Getting Started
- [Quick Start Guide](./user-guide/quickstart.md)
- [Installation](./deployment/installation.md)
- [First Steps](./tutorials/first-steps.md)

### Architecture
- [System Overview](./architecture/overview.md)
- [Multi-Language Architecture](./architecture/multi-language.md)
- [Storage Engine](./architecture/storage-engine.md)
- [Hybrid Memory System](./architecture/hybrid-memory.md)

### Performance
- [Benchmark Results](./performance/benchmarks.md)
- [Database Comparisons](./performance/comparisons.md)
- [Optimization Guide](./performance/optimization.md)

### User Guide
- [User Manual](./user-guide/manual.md)
- [Query Languages](./user-guide/query-languages.md)
- [PromptQL Guide](./user-guide/promptql.md)
- [Natural Language Queries](./user-guide/nlq.md)

### API Reference
- [Rust Core API](./api-reference/rust-api.md)
- [Go Service API](./api-reference/go-api.md)
- [Python AI API](./api-reference/python-api.md)
- [REST API](./api-reference/rest-api.md)

### Training
- [Developer Training Manual](./training/developer-manual.md)
- [Administrator Training](./training/admin-manual.md)
- [Video Training Courses](./videos/README.md)

### Deployment
- [Production Deployment](./deployment/production.md)
- [Docker & Kubernetes](./deployment/containers.md)
- [Configuration Reference](./deployment/configuration.md)
- [Operations Guide](./deployment/operations.md)

### Tutorials
- [Building Your First Application](./tutorials/first-app.md)
- [PromptQL Tutorial](./tutorials/promptql-tutorial.md)
- [High-Performance Patterns](./tutorials/performance-patterns.md)
- [Migration Guide](./tutorials/migration.md)

---

## Key Features

### Multi-Language Architecture

LumaDB acts as a polyglot database, speaking the native wire protocols of major databases while storing data in its unified high-performance engine.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          LumaDB Platform                                  │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌────────────────────┐      ┌───────────────────────────────────────┐  │
│  │ Multi-Protocol     │ gRPC │           Go Cluster Manager          │  │
│  │ Gateway (Rust)     │◄────►│          (Raft Consensus)             │  │
│  │                    │      │                                       │  │
│  │ • Postgres (v3)    │      │  ┌─────────────────────────────────┐  │  │
│  │ • MySQL            │      │  │        Rust Storage Engine      │  │  │
│  │ • Cassandra (CQL)  │      │  │          (Luma Core)            │  │  │
│  │ • MongoDB (BSON)   │      │  │                                 │  │  │
│  │                    │      │  │ • Hybrid Disk/RAM • Columnar    │  │  │
│  │ • io_uring        • SIMD        │  │  │
│  └────────────────────┘      │  └─────────────────────────────────┘  │  │
│                              │  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

### Performance Highlights

| Metric | LumaDB | Aerospike | ScyllaDB | DragonflyDB | kdb+ |
|--------|------|-----------|----------|-------------|------|
| Metric | LumaDB | Aerospike | ScyllaDB | DragonflyDB | kdb+ |
|--------|------|-----------|----------|-------------|------|
| Read Latency (p99) | **~16ns (network)** | 1ms | 2ms | 0.5ms | 0.4ms |
| Write Throughput | **2.5M ops/s** | 1M ops/s | 800K ops/s | 1.5M ops/s | 1.2M ops/s |
| Compatibility | **PG, MySQL, CQL, Mongo** | Proprietary | CQL, Dynamo | Redis | q/k |
| AI Query Support | **PromptQL & Vectors** | No | No | No | No |
| Network Stack | **Fasthttp / gRPC** | Custom | Seastar | Custom | Custom |

### Query Languages

LumaDB supports multiple query interfaces:

1. **PromptQL** - AI-powered natural language with reasoning
2. **NLQ** - Natural Language Queries
3. **SQL** - Standard SQL compatibility
4. **Native API** - Direct programmatic access

---

## Quick Example

```python
from tdbai import PromptQLEngine, LLMConfig

# Initialize PromptQL with AI capabilities
engine = PromptQLEngine(
    db_client=luma_connection,
    llm_config=LLMConfig(provider="openai", model="gpt-4")
)

# Natural language query with multi-step reasoning
result = await engine.query(
    "Find customers who spent more than average last month "
    "and compare their purchase patterns with the previous year"
)

# Conversational follow-up
result = await engine.query("Now show me just the top 10 by revenue")
```

---

## Support

- GitHub Issues: [Report bugs and feature requests](https://github.com/tdb-plus/issues)
- Documentation: [Full documentation](https://docs.tdbplus.io)
- Community: [Discord Server](https://discord.gg/tdbplus)

---

**LumaDB - Where Performance Meets Intelligence**
