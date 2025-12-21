# Apache Superset Integration Guide

LumaDB provides full compatibility with Apache Superset through its PostgreSQL wire protocol.

## Quick Start

### 1. Connection Configuration

In Superset, add a new database connection:

| Field | Value |
|-------|-------|
| **Database Type** | PostgreSQL |
| **Host** | `localhost` (or your LumaDB host) |
| **Port** | `5432` |
| **Database** | `lumadb` |
| **Username** | `admin` |
| **Password** | `admin` |

**SQLAlchemy URI:**
```
postgresql://admin:admin@localhost:5432/lumadb
```

### 2. Test Connection

```bash
# Using psql
psql -h localhost -p 5432 -U admin -d lumadb

# Test query
SELECT * FROM your_collection LIMIT 10;
```

---

## Supported Superset Features

### ✅ Fully Supported

| Feature | Status | Notes |
|---------|--------|-------|
| SQL Lab | ✅ | Full SQL query interface |
| Charts | ✅ | All chart types work |
| Dashboards | ✅ | Real-time dashboards |
| Explore | ✅ | Visual query builder |
| Table Columns | ✅ | Auto-discovery via `information_schema` |
| Data Types | ✅ | TEXT, INTEGER, REAL, BOOLEAN, TIMESTAMP |
| Aggregations | ✅ | COUNT, SUM, AVG, MIN, MAX |
| Filtering | ✅ | WHERE clauses |
| Grouping | ✅ | GROUP BY |
| Sorting | ✅ | ORDER BY |
| Pagination | ✅ | LIMIT, OFFSET |

### ⚠️ Partially Supported

| Feature | Status | Workaround |
|---------|--------|------------|
| Joins | ⚠️ | Use denormalized collections |
| Subqueries | ⚠️ | Flatten queries |
| Window Functions | ⚠️ | Use post-processing |

---

## Schema Discovery

LumaDB exposes PostgreSQL-compatible system catalogs:

### List Tables (Collections)
```sql
SELECT table_name 
FROM information_schema.tables 
WHERE table_schema = 'public';
```

### List Columns
```sql
SELECT column_name, data_type 
FROM information_schema.columns 
WHERE table_name = 'your_collection';
```

### Table Statistics
```sql
SELECT relname, n_live_tup 
FROM pg_stat_user_tables;
```

---

## SQL Functions

LumaDB supports these SQL functions commonly used by Superset:

### Aggregation Functions
```sql
SELECT 
    COUNT(*) as total,
    COUNT(DISTINCT category) as unique_categories,
    SUM(amount) as total_amount,
    AVG(amount) as avg_amount,
    MIN(timestamp) as first_record,
    MAX(timestamp) as last_record
FROM transactions;
```

### Date Functions
```sql
-- Date truncation for time series
SELECT 
    DATE_TRUNC('day', timestamp) as day,
    COUNT(*) as events
FROM events
GROUP BY DATE_TRUNC('day', timestamp)
ORDER BY day;

-- Date extraction
SELECT 
    EXTRACT(YEAR FROM timestamp) as year,
    EXTRACT(MONTH FROM timestamp) as month,
    COUNT(*) as count
FROM data
GROUP BY year, month;
```

### String Functions
```sql
SELECT 
    LOWER(name) as name_lower,
    UPPER(status) as status_upper,
    CONCAT(first_name, ' ', last_name) as full_name,
    LENGTH(description) as desc_length
FROM users;
```

### Conditional Functions
```sql
SELECT 
    CASE 
        WHEN amount > 1000 THEN 'high'
        WHEN amount > 100 THEN 'medium'
        ELSE 'low'
    END as tier,
    COUNT(*) as count
FROM transactions
GROUP BY tier;

-- COALESCE for null handling
SELECT COALESCE(category, 'Uncategorized') as category
FROM items;
```

---

## Time Series Dashboards

### TSDB Data in Superset

LumaDB's TSDB (Prometheus/InfluxDB/Druid compatible) data is accessible:

```sql
-- Prometheus-style metrics
SELECT 
    timestamp,
    metric_name,
    value,
    labels->>'instance' as instance
FROM prometheus_metrics
WHERE metric_name = 'http_requests_total'
  AND timestamp > NOW() - INTERVAL '1 hour';

-- InfluxDB-style measurements
SELECT 
    time,
    host,
    cpu_usage,
    memory_usage
FROM telegraf_cpu
WHERE time > NOW() - INTERVAL '24 hours';
```

### Time Series Chart Configuration

1. **Select Time Column:** Choose `timestamp` or `time`
2. **Time Grain:** Select appropriate granularity (minute, hour, day)
3. **Metrics:** Add aggregations like `AVG(value)`, `MAX(cpu_usage)`
4. **Group By:** Add dimensions like `host`, `region`

---

## Vector Search in Superset

Query vector similarity results:

```sql
-- Find similar items (pre-computed)
SELECT 
    id,
    title,
    similarity_score
FROM vector_search_results
WHERE query_id = 'current_query'
ORDER BY similarity_score DESC
LIMIT 10;
```

---

## Performance Optimization

### Indexing
```sql
-- Create index for faster filtering
CREATE INDEX idx_timestamp ON events(timestamp);
CREATE INDEX idx_category ON products(category);
```

### Query Hints
```sql
-- Use LIMIT for large datasets
SELECT * FROM big_table LIMIT 10000;

-- Be specific with columns
SELECT id, name, value FROM data;  -- Faster than SELECT *
```

### Caching

Configure Superset caching:
```python
# superset_config.py
CACHE_CONFIG = {
    'CACHE_TYPE': 'redis',
    'CACHE_DEFAULT_TIMEOUT': 300,
    'CACHE_KEY_PREFIX': 'lumadb_',
}
```

---

## Troubleshooting

### Connection Issues

```bash
# Check LumaDB is running
curl http://localhost:8080/health

# Check PostgreSQL port
nc -zv localhost 5432
```

### Query Errors

| Error | Solution |
|-------|----------|
| `relation does not exist` | Collection doesn't exist - check spelling |
| `column does not exist` | Field not in documents - check schema |
| `syntax error` | Verify SQL syntax |

### Performance Issues

1. **Slow queries:** Add LIMIT, create indexes
2. **Timeout:** Increase Superset query timeout
3. **Memory:** Reduce result set size

---

## Example Dashboard

### Metrics Dashboard SQL

```sql
-- Total requests by endpoint
SELECT 
    endpoint,
    COUNT(*) as requests,
    AVG(response_time_ms) as avg_latency
FROM http_logs
WHERE timestamp > NOW() - INTERVAL '7 days'
GROUP BY endpoint
ORDER BY requests DESC;

-- Error rate over time
SELECT 
    DATE_TRUNC('hour', timestamp) as hour,
    COUNT(*) FILTER (WHERE status >= 500) * 100.0 / COUNT(*) as error_rate
FROM http_logs
WHERE timestamp > NOW() - INTERVAL '24 hours'
GROUP BY hour
ORDER BY hour;
```

---

## Reference

- **LumaDB Port:** 5432 (PostgreSQL protocol)
- **Prometheus Port:** 9090
- **InfluxDB Port:** 8086
- **Druid Port:** 8888
- **HTTP API:** 8080
