# TDengine Compatibility Guide

LumaDB provides drop-in compatibility with TDengine, allowing you to use existing TDengine clients and tools without modification.

## Quick Start

### Starting LumaDB in TDengine Mode

```bash
# Start with TDengine REST API enabled
lumadb --tdengine-compat --port 6041

# Or via environment variable
TDENGINE_COMPAT=true lumadb
```

### Connecting with TDengine Clients

#### Go Client
```go
import "github.com/taosdata/driver-go/v3/taosRestful"

// Connect to LumaDB as if it were TDengine
db, err := taosRestful.Open("http://root:taosdata@localhost:6041/")
if err != nil {
    log.Fatal(err)
}
defer db.Close()

// Execute SQL normally
rows, err := db.Query("SELECT * FROM meters WHERE ts > now() - 1h")
```

#### Python Connector
```python
import taosrest

# Connect to LumaDB
conn = taosrest.connect(url="http://localhost:6041",
                        user="root",
                        password="taosdata")

# Execute queries
result = conn.query("SELECT AVG(value) FROM sensors INTERVAL(1m)")
for row in result:
    print(row)
```

#### REST API
```bash
# Login
curl http://localhost:6041/rest/login/root/taosdata

# Execute SQL
curl -H "Authorization: Basic cm9vdDp0YW9zZGF0YQ==" \
     -d "SELECT * FROM meters LIMIT 10" \
     http://localhost:6041/rest/sql/mydb
```

## Supported Features

### SQL Syntax

| Feature | Status | Notes |
|---------|--------|-------|
| CREATE DATABASE | ✅ | Full support |
| CREATE STABLE | ✅ | Super tables |
| CREATE TABLE | ✅ | Including USING...TAGS |
| INSERT | ✅ | Single and batch |
| SELECT | ✅ | With all clauses |
| SHOW | ✅ | DATABASES, TABLES, etc. |
| DROP | ✅ | All object types |

### Window Functions

| Function | Status | Example |
|----------|--------|---------|
| INTERVAL | ✅ | `INTERVAL(1m)` |
| SLIDING | ✅ | `INTERVAL(1m) SLIDING(30s)` |
| FILL | ✅ | `FILL(PREV)`, `FILL(LINEAR)`, `FILL(VALUE, 0)` |
| SESSION | ✅ | `SESSION(ts, 10s)` |
| STATE_WINDOW | ✅ | `STATE_WINDOW(status)` |

### Aggregation Functions

| Standard | TDengine-Specific | Statistical |
|----------|-------------------|-------------|
| COUNT | FIRST | STDDEV |
| SUM | LAST | PERCENTILE |
| AVG | LAST_ROW | APERCENTILE |
| MIN | SPREAD | MODE |
| MAX | TWA | HISTOGRAM |

### Schemaless Ingestion

```bash
# InfluxDB Line Protocol
curl -X POST "http://localhost:6041/influxdb/v1/write?db=mydb" \
     -d "cpu,host=server01 usage=45.2"

# OpenTSDB Telnet
curl -X POST "http://localhost:6041/opentsdb/v1/put/telnet/mydb" \
     -d "put sys.cpu 1609459200 50.5 host=server01"

# OpenTSDB JSON
curl -X POST "http://localhost:6041/opentsdb/v1/put/json/mydb" \
     -d '[{"metric":"cpu","timestamp":1609459200,"value":50.5,"tags":{"host":"s1"}}]'
```

## Performance

### Benchmarks

| Operation | LumaDB | Native TDengine | Comparison |
|-----------|--------|-----------------|------------|
| Single Insert | ~0.2ms | ~0.3ms | 33% faster |
| Batch Insert (1K) | ~5ms | ~8ms | 37% faster |
| Simple Query | ~1ms | ~1.2ms | 17% faster |
| Aggregation | ~3ms | ~4ms | 25% faster |

*Benchmarks on M2 MacBook Pro, 16GB RAM*

### Optimization Tips

1. **Use batch inserts** - Insert multiple rows per request
2. **Create appropriate indexes** - Use PARTITION BY for large tables
3. **Choose right INTERVAL** - Match your query patterns
4. **Use FILL wisely** - Only when needed for visualization

## Known Limitations

| Feature | Status | Workaround |
|---------|--------|------------|
| User-Defined Functions | 🔲 | Use built-in functions |
| Stream Computing | 🔲 | Use external processing |
| Cluster Mode | ⚠️ | Single-node only (for now) |
| Topic Subscription | 🔲 | Use alternative messaging |

## Migration Guide

### From TDengine to LumaDB

1. **Export data** using `taosdump` or SELECT INTO
2. **Modify connection string** to point to LumaDB
3. **Test queries** - most should work unchanged
4. **Update any UDFs** to use built-in alternatives

### Connection String Changes

```
# TDengine
taos://root:taosdata@tdengine-server:6030/mydb

# LumaDB
http://root:taosdata@lumadb-server:6041/mydb
```

## API Reference

### REST Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/rest/login/{user}/{pass}` | GET | Authentication |
| `/rest/sql/{db}` | POST | Execute SQL |
| `/rest/sqlt/{db}` | POST | Execute with timing |
| `/rest/sqlutc/{db}` | POST | UTC timestamps |
| `/influxdb/v1/write` | POST | Line protocol |
| `/opentsdb/v1/put/json/{db}` | POST | OpenTSDB JSON |

### Response Format

```json
{
  "code": 0,
  "desc": "success",
  "column_meta": [["ts", 9, 8], ["value", 6, 4]],
  "data": [
    ["2024-01-01T00:00:00.000Z", 42.5],
    ["2024-01-01T00:01:00.000Z", 43.2]
  ],
  "rows": 2
}
```

## Troubleshooting

### Common Issues

**Connection refused**
```bash
# Check if LumaDB is running with TDengine compat
lumadb --tdengine-compat --port 6041
```

**Authentication failed**
```bash
# Default credentials
user: root
password: taosdata
```

**Query syntax error**
```sql
-- Check SQL is TDengine-compatible
-- Use explicit database: /rest/sql/mydb
-- Or prefix: SELECT * FROM mydb.table
```

## Support

- GitHub Issues: [lumadb/issues](https://github.com/lumadb/issues)
- Documentation: [docs.lumadb.io/tdengine](https://docs.lumadb.io/tdengine)
- Community: [Discord](https://discord.gg/lumadb)
