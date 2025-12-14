# LumaDB API Reference

## Version 3.0.0 | December 2024

---

## 1. PostgreSQL Wire Protocol

### Connection
```bash
psql -h localhost -p 5432 -U lumadb -d default
# Password: lumadb (default)
```

### Supported SQL Commands

| Command | Status | Notes |
|---------|--------|-------|
| `SELECT` | ✅ | Basic queries |
| `INSERT` | 🔄 | Via ingestion APIs |
| `CREATE TABLE` | 🔄 | Planned |
| `DROP TABLE` | 🔄 | Planned |

---

## 2. Prometheus API

### Remote Write
```
POST http://localhost:9090/api/v1/write
Content-Type: application/x-protobuf
Content-Encoding: snappy
```

### Metrics Endpoint
```
GET http://localhost:9091/metrics
```

---

## 3. OTLP gRPC API

### Endpoint
```
grpc://localhost:4317
```

### Services

| Service | Method | Request | Response |
|---------|--------|---------|----------|
| `MetricsService` | `Export` | `ExportMetricsServiceRequest` | `ExportMetricsServiceResponse` |
| `LogsService` | `Export` | `ExportLogsServiceRequest` | `ExportLogsServiceResponse` |
| `TraceService` | `Export` | `ExportTraceServiceRequest` | `ExportTraceServiceResponse` |

### Example (Python)
```python
from opentelemetry.exporter.otlp.proto.grpc.metric_exporter import OTLPMetricExporter

exporter = OTLPMetricExporter(endpoint="localhost:4317", insecure=True)
```

---

## 4. Internal gRPC API

### Endpoint
```
grpc://localhost:50051
```

### Query Service

```protobuf
service QueryService {
  rpc Execute(QueryRequest) returns (QueryResponse);
  rpc StreamExecute(QueryRequest) returns (stream QueryResponse);
}

message QueryRequest {
  string query = 1;
  repeated bytes params = 2;
}

message QueryResponse {
  repeated bytes rows = 1;
  string error = 2;
}
```

---

## 5. Rust API (luma-core)

### Storage Operations

```rust
use luma_protocol_core::storage::tiering::MultiTierStorage;
use luma_protocol_core::storage::segment::Segment;

// Initialize storage
let storage = MultiTierStorage::new(PathBuf::from("./data")).await;

// Ingest segment
let segment = Segment::new("seg_001".to_string(), (1000, 2000));
storage.ingest(segment).await?;

// Flush to warm tier
storage.flush_to_warm("seg_001").await?;
```

### Metrics Storage

```rust
use luma_protocol_core::storage::metric_store::MetricsStorage;
use std::collections::HashMap;

let metrics = MetricsStorage::new();

// Insert sample
let mut labels = HashMap::new();
labels.insert("env".to_string(), "prod".to_string());
metrics.insert_sample("http_requests", labels, 1702540800, 42.0).await?;
```

### Inverted Index

```rust
use luma_protocol_core::indexing::inverted::InvertedIndex;

let index = InvertedIndex::new();

// Add documents
index.add_document(1, "hello world rust");
index.add_document(2, "hello golang");

// Search
let results = index.search_and(vec!["hello", "rust"]);
// Results: bitmap containing doc_id 1
```

---

## 6. Configuration API

### Config File (config.toml)

```toml
[general]
data_dir = "./data"
log_level = "info"

[server]
host = "127.0.0.1"
port = 8080

[metrics]
enabled = true
host = "127.0.0.1"
port = 9091
path = "/metrics"

[postgres]
enabled = true
host = "0.0.0.0"
port = 5432
max_connections = 100

[prometheus]
enabled = true
host = "0.0.0.0"
port = 9090
```

---

## 7. Error Codes

| Code | Description |
|------|-------------|
| `E001` | Authentication failed |
| `E002` | Rate limit exceeded |
| `E003` | Invalid query syntax |
| `E004` | Table not found |
| `E005` | Internal error |

---

## 8. Health Check

```bash
curl http://localhost:8080/health
# Response: OK
```

---

*Last Updated: December 2024*
