# LumaDB Architecture Design

## Version 3.0.0 | December 2024

---

## 1. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Client Applications                              │
│     psql │ Grafana │ OpenTelemetry Collector │ Custom Apps              │
└────┬────────┬──────────────┬─────────────────────┬─────────────────────┘
     │        │              │                     │
     ▼        ▼              ▼                     ▼
┌─────────┬─────────┬─────────────────┬──────────────────────────────────┐
│ :5432   │ :9090   │     :4317       │           :8080                   │
│ Postgres│ Prom.   │     OTLP        │           HTTP                    │
│ Protocol│ API     │     gRPC        │           REST                    │
└────┬────┴────┬────┴────────┬────────┴───────────┬──────────────────────┘
     │         │             │                    │
     ▼         ▼             ▼                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        QueryExecutor                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  │
│  │    Parser    │  │   Planner    │  │   Executor   │                  │
│  │  (SQL/PromQL)│  │   (IR Tree)  │  │ (Vectorized) │                  │
│  └──────────────┘  └──────────────┘  └──────────────┘                  │
└───────────────────────────────┬────────────────────────────────────────┘
                                │
                                ▼
┌────────────────────────────────────────────────────────────────────────┐
│                      MultiTierStorage                                   │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                        Data Path                                 │   │
│  │  Ingest → WAL → Hot Tier → Warm Tier → Cold Tier                │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────────┐   │
│  │ HotTier  │  │ WarmTier │  │ ColdTier │  │ Specialized Storage  │   │
│  │(DashMap) │  │ (Disk)   │  │(ObjStore)│  │                      │   │
│  │Arc<Seg>  │  │ (Parquet)│  │(RustMinio)│ │ • MetricsStorage    │   │
│  └──────────┘  └──────────┘  └──────────┘  │ • TraceStorage      │   │
│                                             │ • LogStorage        │   │
│  ┌──────────────────────────────────────┐  │ • InvertedIndex     │   │
│  │           Write-Ahead Log            │  └──────────────────────┘   │
│  │  (Length-prefixed JSON entries)      │                             │
│  └──────────────────────────────────────┘                             │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Storage Tier Details

### 2.1 Hot Tier (In-Memory)

```rust
pub struct HotTier {
    segments: Arc<DashMap<SegmentId, Arc<Segment>>>,
}
```

- **Technology:** DashMap (lock-free concurrent HashMap)
- **Use Case:** Recent data, sub-microsecond access
- **Capacity:** Limited by RAM

### 2.2 Warm Tier (Disk)

- **Technology:** tokio::fs with Parquet format
- **Use Case:** Queryable historical data
- **Capacity:** Limited by SSD

### 2.3 Cold Tier (Object Store)

- **Technology:** LocalObjectStore (Rust-native S3-compatible)
- **Use Case:** Long-term archival
- **Capacity:** Unlimited

---

## 3. Compression Pipeline

```
Raw Data → Gorilla Encoder → Packed Bytes
              ↓
         Vec<u8> buffer
         bit_position: u8
              ↓
      XOR + Leading/Trailing Zeros
              ↓
         8x Memory Savings
```

### Gorilla Algorithm

1. First value: stored raw (64 bits)
2. Subsequent values: XOR with previous
3. If XOR = 0: write 1 bit (0)
4. Else: write leading zeros count (5 bits) + length (6 bits) + significant bits

---

## 4. Query Execution

```
SQL Query
    ↓
Parser (pest/nom)
    ↓
Abstract Syntax Tree
    ↓
Planner → QueryPlan
    ↓
Optimizer (predicate pushdown, projection)
    ↓
Executor → Arrow RecordBatch
    ↓
Protocol Formatter → Wire Response
```

### Vectorized Execution

- SIMD aggregations for SUM, AVG, COUNT
- Columnar iteration with Arrow arrays
- Parallel scan with Rayon (optional)

---

## 5. Protocol Handlers

### PostgreSQL Wire Protocol

```
Client → StartupMessage
Server → AuthenticationMD5Password + Salt
Client → PasswordMessage (md5 hash)
Server → AuthenticationOk → ReadyForQuery
Client → Query
Server → RowDescription → DataRow* → CommandComplete → ReadyForQuery
```

### OTLP gRPC

```
Client → ExportMetricsServiceRequest
Server → Route to MetricsStorage
Server → ExportMetricsServiceResponse
```

---

## 6. Security Architecture

```
┌─────────────────────────────────────────┐
│            Connection Manager            │
│  ┌───────────────────────────────────┐  │
│  │         Rate Limiter              │  │
│  │   IP → Token Bucket → Ban List    │  │
│  └───────────────────────────────────┘  │
│  ┌───────────────────────────────────┐  │
│  │      Semaphore Pool               │  │
│  │   Protocol → Max Connections      │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
                   ↓
┌─────────────────────────────────────────┐
│           AuthConfig                     │
│   username, password, require_auth      │
└─────────────────────────────────────────┘
```

---

## 7. Data Flow Diagrams

### Metrics Ingestion

```
Prometheus Scraper
      ↓ (pull)
Target Endpoint → Parse Text Format
      ↓
MetricsStorage.insert_sample()
      ↓
Shard Selection (hash labels)
      ↓
TimeSeries.chunk.append()
      ↓
Gorilla Compression
```

### Query Execution

```
PostgreSQL Query
      ↓
Protocol Parser
      ↓
QueryProcessor.process()
      ↓
QueryExecutor.execute()
      ↓
Storage Layer Scan
      ↓
Arrow RecordBatch
      ↓
Protocol Formatter
      ↓
Wire Response
```

---

*Last Updated: December 2024*
