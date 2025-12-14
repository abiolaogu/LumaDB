# LumaDB Training Manual

## Version 3.0.0 | December 2024

---

## Part 1: Administrator Training

### 1.1 Installation & Setup (30 minutes)

**Objective:** Deploy LumaDB in development and production environments.

1. **Binary Installation**
   ```bash
   ./luma-server --config config.toml
   ```

2. **Docker Deployment**
   ```bash
   docker run -p 5432:5432 -p 9090:9090 lumadb/lumadb:latest
   ```

3. **Configuration Deep Dive**
   - `data_dir`: Where WAL and segments are stored
   - `log_level`: debug, info, warn, error
   - Port configurations for each protocol

### 1.2 Security Configuration (20 minutes)

**Objective:** Secure a production deployment.

1. **Change Default Credentials**
   - Edit `AuthConfig` in source or use environment variables
   - Username/password for PostgreSQL access

2. **Rate Limiting**
   - Default: 100 requests/minute per IP
   - Ban duration: 5 minutes
   - Monitor `lumadb_rate_limit_exceeded` metric

3. **Network Security**
   - Use firewall to restrict ports
   - Enable TLS (when available)

### 1.3 Monitoring & Alerting (20 minutes)

**Objective:** Set up observability for LumaDB.

1. **Metrics Endpoint**
   ```bash
   curl http://localhost:9091/metrics
   ```

2. **Key Metrics**
   - `lumadb_active_connections{protocol="postgres"}`
   - `lumadb_query_duration_seconds`
   - `lumadb_wal_entries_total`

3. **Grafana Dashboard**
   - Import dashboard from `docs/grafana/lumadb-dashboard.json`

---

## Part 2: Developer Training

### 2.1 Connecting Applications (30 minutes)

**Objective:** Integrate LumaDB with your applications.

1. **PostgreSQL Driver**
   ```python
   import psycopg2
   conn = psycopg2.connect(
       host="localhost", port=5432,
       user="lumadb", password="lumadb"
   )
   ```

2. **OpenTelemetry SDK**
   ```python
   from opentelemetry.exporter.otlp.proto.grpc.metric_exporter import OTLPMetricExporter
   exporter = OTLPMetricExporter(endpoint="localhost:4317", insecure=True)
   ```

3. **Prometheus Client**
   ```bash
   curl -X POST http://localhost:9090/api/v1/write -d @metrics.pb
   ```

### 2.2 Query Patterns (20 minutes)

**Objective:** Write efficient queries.

1. **Simple Queries**
   ```sql
   SELECT * FROM metrics LIMIT 100;
   ```

2. **Full-Text Search**
   ```sql
   SELECT * FROM logs WHERE text_search(message, 'error timeout');
   ```

3. **Vector Search**
   ```sql
   SELECT * FROM embeddings ORDER BY vector_distance(embedding, ?) LIMIT 10;
   ```

### 2.3 Best Practices (15 minutes)

1. **Batch Ingestion** - Send data in batches of 1000+ samples
2. **Label Cardinality** - Keep unique label combinations < 1M
3. **Query Optimization** - Use time ranges to limit scan scope

---

## Part 3: SRE Training

### 3.1 Capacity Planning (20 minutes)

| Workload | Series | RAM | Storage |
|----------|--------|-----|---------|
| Small | 100K | 16 GB | 100 GB |
| Medium | 1M | 32 GB | 500 GB |
| Large | 10M | 64 GB | 2 TB |

### 3.2 Disaster Recovery (20 minutes)

1. **WAL Backup**
   ```bash
   cp data/wal.log /backup/wal-$(date +%Y%m%d).log
   ```

2. **Recovery**
   - Stop LumaDB
   - Restore WAL file
   - Start LumaDB (automatic replay)

### 3.3 Troubleshooting Guide (15 minutes)

| Symptom | Cause | Solution |
|---------|-------|----------|
| Connection refused | Port not listening | Check config, restart |
| Auth failed | Wrong credentials | Verify username/password |
| High latency | Cold data access | Check tier usage |
| OOM | Too many series | Increase RAM, reduce cardinality |

---

## Certification Quiz

1. What port does the PostgreSQL protocol use by default?
2. How does LumaDB authenticate PostgreSQL connections?
3. What is the default rate limit per IP?
4. Where is the WAL file stored?
5. How do you connect Grafana to LumaDB?

---

*Last Updated: December 2024*
