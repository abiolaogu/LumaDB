# LumaDB User Manual

## Version 3.0.0 | December 2024

---

## 1. Getting Started

### Installation

#### Binary Release
```bash
# Download release
curl -LO https://github.com/lumadb/releases/latest/luma-server

# Make executable
chmod +x luma-server

# Run
./luma-server --config config.toml
```

#### Docker
```bash
docker run -p 5432:5432 -p 9090:9090 -p 4317:4317 lumadb/lumadb:latest
```

#### Build from Source
```bash
cd lumadb-compat
./release.sh
# Binary: target/release/luma-server
```

---

## 2. Configuration

### Example config.toml
```toml
[general]
data_dir = "./data"
log_level = "info"

[server]
host = "127.0.0.1"
port = 8080

[metrics]
enabled = true
port = 9091

[postgres]
enabled = true
port = 5432
max_connections = 100
```

---

## 3. Connecting

### PostgreSQL Client
```bash
psql -h localhost -p 5432 -U lumadb -d default
Password: lumadb
```

### Grafana
1. Add Data Source → Prometheus
2. URL: `http://lumadb:9090`
3. Save & Test

### OpenTelemetry Collector
```yaml
exporters:
  otlp:
    endpoint: "lumadb:4317"
    tls:
      insecure: true
```

---

## 4. Querying Data

### SQL (via PostgreSQL)
```sql
SELECT * FROM metrics WHERE name = 'http_requests_total';
```

### PromQL (via Prometheus API)
```bash
curl 'http://localhost:9090/api/v1/query?query=http_requests_total'
```

---

## 5. Ingesting Data

### Prometheus Scraping
Configure targets in LumaDB to scrape Prometheus endpoints.

### OTLP Push
Send OpenTelemetry data to `grpc://localhost:4317`

### Direct API
```rust
// Rust API
storage.metrics.insert_sample("cpu_usage", labels, timestamp, 85.5).await?;
```

---

## 6. Monitoring LumaDB

### Health Check
```bash
curl http://localhost:8080/health
```

### Metrics
```bash
curl http://localhost:9091/metrics
```

Key metrics:
- `lumadb_active_connections` - Current connections per protocol
- `lumadb_query_duration_seconds` - Query latency histogram
- `lumadb_ingestion_rate` - Samples ingested per second

---

## 7. Backup & Recovery

### WAL Files
Located in `{data_dir}/wal.log`

### Recovery
On restart, LumaDB automatically replays WAL entries:
```
INFO WAL recovery: recovered 1234 segments
```

---

## 8. Troubleshooting

| Issue | Solution |
|-------|----------|
| Connection refused on 5432 | Check `postgres.enabled = true` in config |
| Authentication failed | Verify username/password (default: lumadb/lumadb) |
| Rate limit exceeded | Wait 5 minutes or increase `max_requests` |
| High memory usage | Reduce `max_connections`, increase shard count |

---

## 9. Security Best Practices

1. **Change default password** in `AuthConfig`
2. **Enable TLS** for production (planned)
3. **Use firewall** to restrict port access
4. **Monitor rate limiting** logs for abuse

---

*Last Updated: December 2024*
