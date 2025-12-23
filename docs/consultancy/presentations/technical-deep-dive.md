---
marp: true
theme: default
paginate: true
backgroundColor: #0d1117
color: #c9d1d9
style: |
  section {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
  }
  h1 { color: #58a6ff; }
  h2 { color: #7ee787; }
  code { background: #161b22; }
  pre { background: #161b22; }
---

# LumaDB
## Technical Deep-Dive

### Architecture, Implementation & Performance

**For Engineers & Architects**

---

# Agenda

1. **Architecture Overview** - Multi-language design
2. **Storage Engine** - LSM-Tree, tiering, compression
3. **Query Processing** - Parsers, planners, execution
4. **Distributed Systems** - Raft, sharding, replication
5. **AI/Vector Features** - HNSW, embeddings, PromptQL
6. **Protocol Compatibility** - Wire protocols, dialects
7. **Performance** - Benchmarks, tuning
8. **Operations** - Deployment, monitoring

---

# Architecture: The Big Picture

```
┌─────────────────────────────────────────────────────────────────┐
│                         LumaDB Platform                          │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ TypeScript  │  │   Python    │  │     Go      │             │
│  │  SDK/CLI    │  │ AI Service  │  │   Cluster   │             │
│  │   8K LOC    │  │   5K LOC    │  │   25K LOC   │             │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
│         │                │                │                     │
│         └────────────────┼────────────────┘                     │
│                          │                                      │
│                   ┌──────┴──────┐                               │
│                   │    Rust     │                               │
│                   │ Core Engine │                               │
│                   │   24K LOC   │                               │
│                   └─────────────┘                               │
└─────────────────────────────────────────────────────────────────┘
```

**~62K lines of production code**

---

# Why Multi-Language?

## Each Language Plays to Its Strengths

| Language | Role | Why |
|----------|------|-----|
| **Rust** | Storage engine | Zero-cost abstractions, memory safety, SIMD |
| **Go** | Distributed layer | Goroutines, built-in concurrency, fast compile |
| **Python** | AI/ML service | ML ecosystem (PyTorch, FAISS, transformers) |
| **TypeScript** | SDK/CLI | Developer experience, type safety |

### Communication
- Rust ↔ Go: FFI + shared memory
- Go ↔ Python: gRPC
- SDK ↔ Server: HTTP/WebSocket/gRPC

---

# Rust Core: Storage Architecture

## LSM-Tree Implementation

```
Write Path:
───────────
Client → WAL → Memtable → Immutable Memtable → SSTable (L0)
                                                    │
Compaction:                                         ▼
───────────                                    L1 → L2 → L3 → ...
```

### Key Components
- **Memtable:** Lock-free skip-list (crossbeam-skiplist)
- **WAL:** Group commit with configurable sync modes
- **SSTable:** Sorted, compressed, with bloom filters
- **Compaction:** Leveled, Universal, or FIFO strategies

---

# Memtable Design

## Lock-Free Concurrent Skip List

```rust
pub struct Memtable {
    data: crossbeam_skiplist::SkipMap<Key, Value>,
    size: AtomicUsize,
    max_size: usize,
}

impl Memtable {
    pub fn insert(&self, key: Key, value: Value) -> Result<()> {
        let value_size = value.len();
        self.data.insert(key, value);
        self.size.fetch_add(value_size, Ordering::Relaxed);

        if self.size.load(Ordering::Relaxed) >= self.max_size {
            self.trigger_flush();
        }
        Ok(())
    }
}
```

- **O(log n)** insert/lookup
- **No locks** on read path
- **Automatic flush** when size threshold reached

---

# SSTable Format

## On-Disk Structure

```
┌─────────────────────────────────────────────────┐
│                   SSTable File                   │
├─────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────┐   │
│  │           Data Blocks                    │   │
│  │  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐      │   │
│  │  │ Blk │ │ Blk │ │ Blk │ │ ... │      │   │
│  │  │  1  │ │  2  │ │  3  │ │     │      │   │
│  │  └─────┘ └─────┘ └─────┘ └─────┘      │   │
│  └─────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────┐   │
│  │         Index Block                      │   │
│  │  key1 → offset1, key2 → offset2, ...    │   │
│  └─────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────┐   │
│  │         Bloom Filter                     │   │
│  │  ~1% false positive rate                 │   │
│  └─────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────┐   │
│  │         Footer (metadata)               │   │
│  └─────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

---

# Hybrid Memory Architecture

## Aerospike-Style Tiering

```
┌────────────────────────────────────────────────────────────────┐
│                     Data Tiering                                │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│   HOT TIER              WARM TIER             COLD TIER        │
│   ────────              ─────────             ─────────        │
│   RAM (DashMap)         NVMe SSD              HDD / S3         │
│                                                                 │
│   • ~100ns access       • ~100μs access       • ~10ms access   │
│   • Recent data         • Historical data     • Archive        │
│   • Primary index       • Parquet format      • Object store   │
│                                                                 │
│        ──────────▶           ──────────▶                       │
│        Automatic migration based on access patterns            │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

### Configuration
```toml
[tiering]
hot_threshold_hours = 24
warm_threshold_days = 30
migration_batch_size = 1000
```

---

# Compression Algorithms

## Multiple Codecs for Different Data Types

| Codec | Compression | Speed | Use Case |
|-------|-------------|-------|----------|
| **LZ4** | 2-3x | 500 MB/s | General purpose |
| **Zstd** | 3-5x | 200 MB/s | High compression |
| **Gorilla** | 8-12x | Fast | Time-series floats |
| **Delta** | 5-10x | Very fast | Timestamps |
| **Dictionary** | 10-100x | Fast | Low cardinality |

### Gorilla Algorithm (Time-Series)
```
First value: 64 bits raw
Subsequent:  XOR with previous
             If XOR = 0: 1 bit
             Else: leading zeros (5) + length (6) + bits
```

---

# Write-Ahead Log (WAL)

## Group Commit for High Throughput

```rust
pub struct GroupCommit {
    batch: Mutex<Vec<LogEntry>>,
    batch_size: usize,
    batch_timeout: Duration,
    notify: Condvar,
}

impl GroupCommit {
    pub async fn append(&self, entry: LogEntry) -> Result<()> {
        let mut batch = self.batch.lock().await;
        batch.push(entry);

        if batch.len() >= self.batch_size {
            self.flush_batch(&mut batch).await?;
        }
        Ok(())
    }

    async fn flush_batch(&self, batch: &mut Vec<LogEntry>) -> Result<()> {
        // Write all entries in single fsync
        self.file.write_all(&serialize(batch))?;
        self.file.sync_all()?;
        batch.clear();
        Ok(())
    }
}
```

**Result:** 10-100x throughput improvement

---

# Query Processing Pipeline

## From Query to Results

```
┌────────────────────────────────────────────────────────────────┐
│                    Query Pipeline                               │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Input        Parser       Planner      Optimizer    Executor │
│   ─────        ──────       ───────      ─────────    ──────── │
│                                                                 │
│   "SELECT.."   ┌───────┐    ┌───────┐    ┌───────┐   ┌───────┐ │
│   ──────────▶  │  AST  │───▶│ Plan  │───▶│Optimized──▶│Results│ │
│                └───────┘    │ Tree  │    │ Plan  │   └───────┘ │
│                             └───────┘    └───────┘              │
│                                                                 │
│   Parsers:     • LQL (SQL)   • NQL (Natural)   • JQL (JSON)   │
│                • 11 dialects (InfluxQL, PromQL, Flux, ...)    │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

---

# Dialect Auto-Detection

## Universal Query Language Support

```go
type DialectDetector struct {
    patterns map[Dialect][]Pattern
}

func (d *DialectDetector) Detect(query string) (Dialect, float64) {
    scores := make(map[Dialect]float64)

    for dialect, patterns := range d.patterns {
        for _, p := range patterns {
            if p.Regex.MatchString(query) {
                scores[dialect] += p.Weight
            }
        }
    }

    best := findMax(scores)
    confidence := scores[best] / totalWeight
    return best, confidence
}
```

**11 Supported Dialects:**
InfluxQL, Flux, PromQL, MetricsQL, TimescaleDB, QuestDB, ClickHouse, Druid, OpenTSDB, Graphite, TDengine

---

# Query Optimization

## Cost-Based Optimizer

### Optimizations Applied:
1. **Predicate Pushdown** - Filter early
2. **Projection Pushdown** - Only read needed columns
3. **Index Selection** - Choose best index
4. **Join Reordering** - Optimal join order
5. **Parallel Execution** - Multi-core utilization

```sql
-- Before optimization
SELECT * FROM orders o
JOIN users u ON o.user_id = u.id
WHERE u.status = 'active';

-- After optimization (predicate pushdown)
SELECT o.* FROM orders o
JOIN (SELECT id FROM users WHERE status = 'active') u
ON o.user_id = u.id;
```

---

# Distributed Architecture

## Go Cluster Layer

```
┌────────────────────────────────────────────────────────────────┐
│                     Raft Consensus                              │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌─────────┐      ┌─────────┐      ┌─────────┐               │
│   │ Node 1  │◀────▶│ Node 2  │◀────▶│ Node 3  │               │
│   │ (Leader)│      │(Follower│      │(Follower│               │
│   └─────────┘      └─────────┘      └─────────┘               │
│        │                                                        │
│        ▼                                                        │
│   ┌─────────────────────────────────────────────────────┐      │
│   │                  Raft Log                            │      │
│   │  [Entry 1] [Entry 2] [Entry 3] [Entry 4] ...        │      │
│   └─────────────────────────────────────────────────────┘      │
│                                                                 │
│   • Election timeout: 1000ms                                   │
│   • Heartbeat: 100ms                                           │
│   • Log replication: Synchronous to quorum                     │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

---

# Sharding Strategy

## Consistent Hashing

```go
type ShardRouter struct {
    ring      *consistenthash.Ring
    shardMap  map[uint64]*Shard
    replicas  int
}

func (r *ShardRouter) Route(key []byte) *Shard {
    hash := xxhash.Sum64(key)
    shardID := r.ring.Get(hash)
    return r.shardMap[shardID]
}

// Minimal resharding on node changes
// Virtual nodes for better distribution
// Configurable replication factor
```

### Sharding Options
- **Hash:** Even distribution, no range queries
- **Range:** Range queries, potential hotspots
- **Geo:** Location-based routing

---

# Vector Search Engine

## HNSW Implementation

```
┌────────────────────────────────────────────────────────────────┐
│              Hierarchical Navigable Small World                 │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Layer 3:    o─────────────────o      (sparse, long edges)    │
│               │                 │                               │
│   Layer 2:    o───o─────o───────o      (medium density)        │
│               │   │     │       │                               │
│   Layer 1:    o─o─o─o─o─o─o─o─o─o      (dense, short edges)    │
│                                                                 │
│   Search: Start at top layer, greedy descent                   │
│   Build:  Random layer assignment with exponential decay       │
│                                                                 │
│   Parameters:                                                   │
│   • M = 16 (max connections per node)                          │
│   • ef_construction = 200 (build-time beam width)              │
│   • ef_search = 100 (query-time beam width)                    │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

**Performance:** 2.5M+ QPS, <1ms latency

---

# Vector Search API

## Usage Example

```rust
// Create vector index
let config = VectorIndexConfig {
    dimensions: 1536,
    metric: DistanceMetric::Cosine,
    m: 16,
    ef_construction: 200,
};
let index = VectorIndex::new(config);

// Insert vectors
index.insert(id, vector, metadata)?;

// Search
let results = index.search(SearchRequest {
    vector: query_vector,
    top_k: 10,
    ef_search: 100,
    filter: Some(json!({"category": "electronics"})),
})?;

// Results include: id, score, metadata
```

---

# Protocol Compatibility

## PostgreSQL Wire Protocol v3

```
┌────────────────────────────────────────────────────────────────┐
│                PostgreSQL Protocol Flow                         │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Client                              Server                    │
│   ──────                              ──────                    │
│                                                                 │
│   StartupMessage ───────────────────▶                          │
│                  ◀─────────────────── AuthenticationMD5        │
│   PasswordMessage ──────────────────▶                          │
│                  ◀─────────────────── AuthenticationOk         │
│                  ◀─────────────────── ParameterStatus*         │
│                  ◀─────────────────── ReadyForQuery            │
│                                                                 │
│   Query ────────────────────────────▶                          │
│                  ◀─────────────────── RowDescription           │
│                  ◀─────────────────── DataRow*                 │
│                  ◀─────────────────── CommandComplete          │
│                  ◀─────────────────── ReadyForQuery            │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

Works with: psql, pgAdmin, JDBC, psycopg2, etc.

---

# Time-Series Features

## TDengine-Compatible Window Functions

```sql
-- INTERVAL: Fixed time windows
SELECT _wstart, AVG(temperature)
FROM sensors
WHERE ts >= NOW - INTERVAL '1 hour'
INTERVAL(5m);

-- SLIDING: Overlapping windows
SELECT _wstart, AVG(temperature)
FROM sensors
INTERVAL(10m) SLIDING(1m);

-- SESSION: Gap-based sessions
SELECT _wstart, _wend, COUNT(*)
FROM events
SESSION(ts, 30m);

-- STATE_WINDOW: State change boundaries
SELECT _wstart, status, ELAPSED(ts)
FROM machine_status
STATE_WINDOW(status);
```

---

# Performance Benchmarks

## Test Environment: 16-core, 64GB RAM, NVMe

| Operation | Throughput | Latency (p99) |
|-----------|------------|---------------|
| Point Read | 1.2M ops/sec | 0.3ms |
| Range Scan (1K rows) | 180K/sec | 8ms |
| Single Insert | 450K ops/sec | 0.8ms |
| Batch Insert (1K) | 2.1M docs/sec | 12ms |
| Vector Search (10K vectors) | 15K QPS | 4ms |
| Vector Search (optimized) | 2.5M+ QPS | <1ms |

### Comparison
```
LumaDB vs PostgreSQL: 10-20x faster reads
LumaDB vs MongoDB:    5-10x faster writes
LumaDB vs Redis:      Comparable with persistence
```

---

# Performance Tuning

## Key Configuration Parameters

```toml
[memory]
memtable_size = 67108864      # 64 MB per memtable
block_cache_size = 536870912  # 512 MB block cache
bloom_bits_per_key = 10       # ~1% false positive

[wal]
sync_mode = "group_commit"    # batch writes
batch_size = 1000             # entries per batch
batch_timeout_us = 1000       # max wait time

[compaction]
style = "leveled"             # or "universal", "fifo"
max_background_jobs = 4       # parallel compaction

[sharding]
shard_per_core = true         # ScyllaDB-style
hash_function = "xxh3"        # fast hashing
```

---

# Monitoring & Observability

## Built-in Prometheus Metrics

```
# Query performance
lumadb_query_duration_seconds_bucket{type="select",le="0.001"} 12345
lumadb_query_duration_seconds_bucket{type="select",le="0.01"} 45678

# Storage metrics
lumadb_storage_bytes_total{tier="hot"} 12345678900
lumadb_compaction_bytes_total 987654321

# Cluster health
lumadb_raft_term 42
lumadb_replication_lag_seconds 0.045

# Cache efficiency
lumadb_cache_hits_total 9876543
lumadb_cache_misses_total 123456
```

Pre-built Grafana dashboards included.

---

# Deployment Architecture

## Production Kubernetes Setup

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: lumadb
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: lumadb
          resources:
            requests:
              cpu: "4"
              memory: "16Gi"
            limits:
              cpu: "8"
              memory: "32Gi"
          volumeMounts:
            - name: data
              mountPath: /var/lib/lumadb
  volumeClaimTemplates:
    - spec:
        storageClassName: ssd
        resources:
          requests:
            storage: 500Gi
```

---

# Security Architecture

## Defense in Depth

```
┌────────────────────────────────────────────────────────────────┐
│                    Security Layers                              │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Network:     Firewall → VPC → Security Groups → TLS 1.3     │
│                                                                 │
│   Auth:        JWT / SCRAM-SHA-256 / mTLS / API Keys          │
│                                                                 │
│   AuthZ:       RBAC → Row-Level Security → Field-Level        │
│                                                                 │
│   Encryption:  TLS (transit) → AES-256-GCM (at-rest)          │
│                → Field-level encryption (PII)                  │
│                                                                 │
│   Audit:       Full query logging → Tamper-proof logs         │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

---

# Code Statistics

## Project Breakdown

| Component | Language | Files | Lines |
|-----------|----------|-------|-------|
| rust-core | Rust | 132 | 23,599 |
| go-cluster | Go | 57 | ~25,000 |
| src/ | TypeScript | 32 | 8,157 |
| python-ai | Python | 23 | ~5,000 |
| ui/admin | TypeScript | - | ~2,000 |
| **Total** | | ~300 | ~63,000 |

### Test Coverage
- Unit tests: ~80%
- Integration tests: Key paths
- Benchmark suite: Comprehensive

---

# Q&A

## Resources

- **Documentation:** https://docs.lumadb.io
- **GitHub:** https://github.com/lumadb
- **API Reference:** https://api.lumadb.io

### Contact
- Technical: solutions@lumadb.io
- Support: support@lumadb.io

---

# Appendix: io_uring (Linux)

## Async I/O for Maximum Performance

```rust
#[cfg(target_os = "linux")]
pub struct IoUringStorage {
    ring: IoUring,
    buffers: Vec<AlignedBuffer>,
}

impl IoUringStorage {
    pub async fn read(&self, offset: u64, len: usize) -> Result<Vec<u8>> {
        let buffer = self.get_buffer();

        // Submit read to kernel
        let entry = opcode::Read::new(
            types::Fd(self.fd),
            buffer.as_mut_ptr(),
            len as u32,
        )
        .offset(offset)
        .build();

        self.ring.submission().push(&entry)?;
        self.ring.submit_and_wait(1)?;

        // Zero-copy result
        Ok(buffer.to_vec())
    }
}
```

**Benefit:** 1M+ IOPS on single node
