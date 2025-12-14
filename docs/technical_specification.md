# LumaDB Technical Specification

## Document Version: 3.0.0 | December 2024

---

## 1. System Architecture

### 1.1 Component Overview

| Component | Language | Purpose |
|-----------|----------|---------|
| `luma-core` | Rust | Storage engine, query executor, compression |
| `luma-server` | Rust | Protocol handlers, connection management |
| `luma-postgres` | Rust | PostgreSQL wire protocol (v3) |
| `luma-prometheus` | Rust | Prometheus remote write/read |
| `luma-benchmark` | Rust | TPC-H benchmarking |

### 1.2 Directory Structure

```
lumadb-compat/
├── crates/
│   ├── luma-core/           # Core storage and query engine
│   │   ├── src/
│   │   │   ├── storage/     # Multi-tier storage (tiering.rs, wal.rs)
│   │   │   ├── query/       # Query executor (executor.rs, simd.rs)
│   │   │   ├── compression/ # Gorilla, ZSTD compression
│   │   │   ├── indexing/    # Inverted index, bitmap
│   │   │   ├── stream/      # Materialized views, windowing
│   │   │   └── ingestion/   # Prometheus, OTLP handlers
│   │   └── tests/           # Unit tests
│   ├── luma-server/         # Server binary
│   │   └── src/
│   │       ├── protocols/   # postgres.rs, otlp.rs, prometheus.rs
│   │       ├── connection.rs # Rate limiting
│   │       └── config.rs    # Configuration
│   └── luma-benchmark/      # Benchmarking
├── docs/                    # Documentation
├── Dockerfile               # Container build
└── release.sh               # Release build script
```

## 2. Storage Engine

### 2.1 Multi-Tier Architecture

```rust
pub struct MultiTierStorage {
    hot: HotTier,          // DashMap<SegmentId, Arc<Segment>>
    warm: WarmTier,        // Disk (tokio::fs)
    cold: ColdTier,        // LocalObjectStore
    wal: Arc<WalManager>,  // Write-Ahead Log
    metrics: Arc<MetricsStorage>,   // Gorilla compression
    traces: Arc<TraceStorage>,      // Span storage
    logs: Arc<LogStorage>,          // Log entries
    text_index: Arc<InvertedIndex>, // Full-text search
}
```

### 2.2 Write Path

1. Client → Protocol Handler (PostgreSQL/Prometheus/OTLP)
2. QueryExecutor.process() → Parse, Plan, Execute
3. MultiTierStorage.ingest(segment)
   - WAL.append_segment() → Durability
   - HotTier.insert(Arc<Segment>) → Visibility
   - MaterializedViews.on_insert() → Stream Processing

### 2.3 Read Path

1. QueryExecutor.execute(plan)
2. HotTier.get(id) → O(1) DashMap lookup
3. If miss → WarmTier/ColdTier
4. Return Arrow RecordBatch

## 3. Compression

### 3.1 Gorilla Compression (Time-Series)

```rust
pub struct GorillaEncoder {
    buffer: Vec<u8>,      // Packed bits (8x efficient vs Vec<bool>)
    bit_position: u8,     // Current bit offset
    last_value: u64,      // Previous value for XOR
    leading_zeros: u8,    // Block compression state
    trailing_zeros: u8,
}
```

**Compression Ratio:** ~1.37 bits/value for typical time-series

### 3.2 Delta-of-Delta (Timestamps)

**Compression Ratio:** ~1.3 bits/timestamp for monotonic series

## 4. Security

### 4.1 Authentication

```rust
pub struct AuthConfig {
    pub username: String,
    pub password: String,
    pub require_auth: bool,
}
```

**PostgreSQL MD5 Flow:**
1. Server sends AuthenticationMD5Password + salt
2. Client sends `md5(md5(password + username) + salt)`
3. Server validates and sends AuthenticationOk

### 4.2 Rate Limiting

```rust
pub struct RateLimiter {
    config: RateLimitConfig,   // max_requests, window, ban_duration
    buckets: RwLock<HashMap<IpAddr, IpBucket>>,
}
```

**Default Configuration:**
- Max requests: 100/minute
- Ban duration: 5 minutes

## 5. Observability Ingestion

### 5.1 Prometheus Scraper

```rust
pub struct PrometheusScraper {
    config: ScraperConfig,
    metrics_storage: Arc<MetricsStorage>,
}
```

Pulls metrics from configured targets at `global_interval`.

### 5.2 OTLP Receiver

Implements `tonic` gRPC services:
- `MetricsService` → MetricsStorage
- `LogsService` → LogStorage
- `TraceService` → TraceStorage

## 6. Query Execution

### 6.1 Operations

| Operation | Description |
|-----------|-------------|
| `Scan` | Full table scan with optional filter |
| `Aggregate` | SUM, COUNT, AVG with SIMD acceleration |
| `VectorSearch` | k-NN with HNSW index |
| `TextSearch` | Boolean search with RoaringBitmap |

### 6.2 SIMD Aggregation

```rust
pub struct SimdAggregates;
impl SimdAggregates {
    pub fn sum_f64(data: &[f64]) -> f64;  // Auto-vectorized
}
```

## 7. Protocol Compatibility

### 7.1 PostgreSQL Wire Protocol (v3)

| Message | Direction | Implemented |
|---------|-----------|-------------|
| StartupMessage | C→S | ✅ |
| AuthenticationMD5Password | S→C | ✅ |
| PasswordMessage | C→S | ✅ |
| AuthenticationOk | S→C | ✅ |
| Query | C→S | ✅ |
| RowDescription | S→C | ✅ |
| DataRow | S→C | ✅ |
| CommandComplete | S→C | ✅ |
| ReadyForQuery | S→C | ✅ |
| ErrorResponse | S→C | ✅ |

## 8. Configuration

```toml
[server]
host = "127.0.0.1"
port = 8080

[metrics]
enabled = true
host = "127.0.0.1"
port = 9091
path = "/metrics"

[general]
data_dir = "./data"
log_level = "info"
```

## 9. Performance Characteristics

| Metric | Value |
|--------|-------|
| Binary Size | 7.7 MB |
| Startup Time | < 500ms |
| Write Latency (p50) | < 100μs |
| Read Latency (p50) | < 50μs |
| Memory per Series | ~200 bytes |
| Compression Ratio | 8x for time-series |

## 10. Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1.35 | Async runtime |
| `tonic` | 0.9 | gRPC |
| `prost` | 0.11 | Protobuf |
| `arrow` | 50.0 | Columnar format |
| `dashmap` | 5.5 | Concurrent HashMap |
| `roaring` | 0.10 | Bitmap index |
| `opentelemetry-proto` | 0.4 | OTLP types |

---

*Last Updated: December 2024*
