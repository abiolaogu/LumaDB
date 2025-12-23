# LumaDB Operations & Administration Manual

## Complete Guide for Day-2 Operations

**Version:** 3.0.0 | **Last Updated:** December 2024

---

## Table of Contents

1. [System Administration](#1-system-administration)
2. [Performance Management](#2-performance-management)
3. [Capacity Planning](#3-capacity-planning)
4. [Troubleshooting Guide](#4-troubleshooting-guide)
5. [Disaster Recovery](#5-disaster-recovery)
6. [Maintenance Procedures](#6-maintenance-procedures)
7. [Security Operations](#7-security-operations)
8. [Runbook Procedures](#8-runbook-procedures)

---

## 1. System Administration

### 1.1 Service Management

**Starting/Stopping Services:**
```bash
# Systemd (Linux)
sudo systemctl start lumadb
sudo systemctl stop lumadb
sudo systemctl restart lumadb
sudo systemctl status lumadb

# Docker
docker-compose up -d
docker-compose down
docker-compose restart lumadb

# Kubernetes
kubectl rollout restart statefulset/lumadb -n lumadb
kubectl scale statefulset lumadb --replicas=5 -n lumadb
```

**Health Checks:**
```bash
# Basic health
curl http://localhost:8080/health

# Readiness (for load balancers)
curl http://localhost:8080/ready

# Detailed status
curl http://localhost:8080/status | jq .

# Cluster health
curl http://localhost:8080/cluster/health
```

### 1.2 Configuration Management

**View Current Configuration:**
```bash
curl http://localhost:8080/admin/config | jq .
```

**Runtime Configuration Changes:**
```bash
# Update log level
curl -X PATCH http://localhost:8080/admin/config \
  -H "Content-Type: application/json" \
  -d '{"log_level": "debug"}'

# Update cache size (requires restart)
curl -X PATCH http://localhost:8080/admin/config \
  -H "Content-Type: application/json" \
  -d '{"block_cache_size": 268435456}'
```

**Configuration File Locations:**
| File | Purpose |
|------|---------|
| `/etc/lumadb/config.toml` | Main configuration |
| `/etc/lumadb/rbac.yaml` | Access control |
| `/etc/lumadb/triggers.yaml` | Event triggers |
| `/var/lib/lumadb/` | Data directory |
| `/var/log/lumadb/` | Log files |

### 1.3 User Management

**Create User:**
```bash
lumadb user create --username alice --role developer
# Or via API
curl -X POST http://localhost:8080/admin/users \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "role": "developer", "password": "secure-password"}'
```

**List Users:**
```bash
lumadb user list
curl http://localhost:8080/admin/users | jq .
```

**Update User Role:**
```bash
lumadb user update alice --role admin
curl -X PATCH http://localhost:8080/admin/users/alice \
  -d '{"role": "admin"}'
```

**Revoke Access:**
```bash
lumadb user disable alice
lumadb user delete alice
```

### 1.4 Log Management

**Log Locations:**
```
/var/log/lumadb/
├── server.log          # Main server log
├── query.log           # Query audit log
├── slow-query.log      # Slow queries (>100ms)
├── access.log          # API access log
└── error.log           # Error log
```

**Log Rotation Configuration:**
```yaml
# /etc/logrotate.d/lumadb
/var/log/lumadb/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    postrotate
        systemctl reload lumadb
    endscript
}
```

**Real-time Log Viewing:**
```bash
# All logs
journalctl -u lumadb -f

# Query log
tail -f /var/log/lumadb/query.log

# Filter for errors
journalctl -u lumadb -p err -f

# Docker logs
docker logs -f lumadb --since 1h
```

---

## 2. Performance Management

### 2.1 Performance Monitoring

**Key Metrics Dashboard:**
```
┌─────────────────────────────────────────────────────────────────────┐
│                    LumaDB Performance Dashboard                      │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  THROUGHPUT          LATENCY             RESOURCES                  │
│  ───────────         ───────             ─────────                  │
│  Queries/sec: 45K    p50: 0.8ms          CPU: 45%                   │
│  Inserts/sec: 12K    p95: 2.1ms          Memory: 12GB/16GB         │
│  Reads/sec: 85K      p99: 4.5ms          Disk I/O: 450MB/s         │
│                                                                      │
│  CACHE                STORAGE             CLUSTER                   │
│  ─────                ───────             ───────                   │
│  Hit Rate: 94%       Total: 245GB        Nodes: 3/3                │
│  Size: 128MB         Hot: 12GB           Leader: node-1            │
│  Evictions: 120/min  Warm: 180GB         Repl Lag: 45ms            │
│                                           │
└─────────────────────────────────────────────────────────────────────┘
```

**Prometheus Queries:**
```promql
# Request rate
sum(rate(lumadb_requests_total[5m])) by (method)

# Latency percentiles
histogram_quantile(0.99, sum(rate(lumadb_query_duration_bucket[5m])) by (le))

# Error rate
sum(rate(lumadb_requests_total{status="error"}[5m])) / sum(rate(lumadb_requests_total[5m]))

# Cache efficiency
sum(rate(lumadb_cache_hits[5m])) / sum(rate(lumadb_cache_requests[5m]))

# Memory usage
lumadb_memory_bytes / lumadb_memory_limit_bytes
```

### 2.2 Query Performance Analysis

**Identify Slow Queries:**
```sql
-- View slow query log
SELECT * FROM system.slow_queries
WHERE duration_ms > 100
ORDER BY duration_ms DESC
LIMIT 20;

-- Query statistics
SELECT
  query_pattern,
  COUNT(*) as executions,
  AVG(duration_ms) as avg_duration,
  MAX(duration_ms) as max_duration,
  SUM(rows_scanned) as total_rows_scanned
FROM system.query_stats
WHERE timestamp > NOW() - INTERVAL '1 hour'
GROUP BY query_pattern
ORDER BY avg_duration DESC;
```

**Explain Query Plan:**
```sql
EXPLAIN ANALYZE SELECT * FROM orders
WHERE user_id = 'user-123'
AND created_at > '2024-01-01';

-- Output:
-- Seq Scan on orders  (cost=0.00..1250.00 rows=125 width=128) (actual time=0.015..12.345 rows=118 loops=1)
--   Filter: ((user_id = 'user-123') AND (created_at > '2024-01-01'))
--   Rows Removed by Filter: 9882
-- Planning Time: 0.125 ms
-- Execution Time: 12.456 ms
```

### 2.3 Index Optimization

**Analyze Index Usage:**
```sql
-- Index statistics
SELECT
  index_name,
  table_name,
  index_type,
  size_bytes,
  usage_count,
  last_used
FROM system.indexes
ORDER BY usage_count DESC;

-- Unused indexes (candidates for removal)
SELECT index_name, table_name
FROM system.indexes
WHERE usage_count = 0
AND created_at < NOW() - INTERVAL '30 days';

-- Missing index suggestions
SELECT * FROM system.missing_index_suggestions
ORDER BY estimated_improvement DESC
LIMIT 10;
```

**Create Optimized Indexes:**
```sql
-- Based on query patterns
CREATE INDEX CONCURRENTLY idx_orders_user_date
ON orders (user_id, created_at DESC);

-- Partial index for common filter
CREATE INDEX idx_active_users
ON users (email)
WHERE status = 'active';

-- Covering index to avoid table lookup
CREATE INDEX idx_orders_covering
ON orders (user_id, created_at)
INCLUDE (total, status);
```

### 2.4 Memory Tuning

**Memory Allocation:**
```toml
# config.toml
[memory]
# Write buffer (per collection)
memtable_size = 67108864      # 64 MB

# Read cache
block_cache_size = 536870912  # 512 MB (for 16GB system)
row_cache_size = 134217728    # 128 MB

# Bloom filters (reduce false positive reads)
bloom_bits_per_key = 10       # ~1% false positive rate

# OS page cache (let OS manage remaining RAM)
use_mmap = true
```

**Memory Usage Breakdown:**
```bash
curl http://localhost:8080/admin/memory | jq .
# {
#   "total_bytes": 8589934592,
#   "memtables": 268435456,
#   "block_cache": 536870912,
#   "row_cache": 134217728,
#   "indexes": 452984832,
#   "other": 156237824
# }
```

---

## 3. Capacity Planning

### 3.1 Storage Estimation

**Formula:**
```
Storage = (Raw Data Size × Replication Factor) / Compression Ratio + Indexes + WAL

Example:
- Raw data: 100 GB
- Replication: 3
- Compression: 3x (LZ4)
- Indexes: ~20% of data
- WAL: ~10% buffer

Storage = (100 × 3) / 3 + 20 + 10 = 130 GB
```

**Monitoring Growth:**
```sql
-- Storage by collection
SELECT
  collection_name,
  document_count,
  data_size_bytes,
  index_size_bytes,
  total_size_bytes
FROM system.collection_stats
ORDER BY total_size_bytes DESC;

-- Growth rate
SELECT
  date_trunc('day', timestamp) as day,
  SUM(bytes_written) as daily_writes
FROM system.storage_stats
WHERE timestamp > NOW() - INTERVAL '30 days'
GROUP BY day
ORDER BY day;
```

### 3.2 Hardware Sizing Guide

| Workload | CPU | RAM | Storage | Network |
|----------|-----|-----|---------|---------|
| **Development** | 4 cores | 8 GB | 100 GB SSD | 1 Gbps |
| **Small Production** | 8 cores | 32 GB | 500 GB NVMe | 10 Gbps |
| **Medium Production** | 16 cores | 64 GB | 2 TB NVMe | 10 Gbps |
| **Large Production** | 32 cores | 128 GB | 4 TB NVMe | 25 Gbps |
| **Enterprise** | 64+ cores | 256+ GB | 8+ TB NVMe | 100 Gbps |

### 3.3 Scaling Decisions

**Vertical Scaling (Scale Up):**
- Add more RAM for better cache hit rates
- Add CPU cores for parallel query processing
- Upgrade to faster NVMe for I/O-bound workloads

**Horizontal Scaling (Scale Out):**
- Add nodes when single node reaches 70% capacity
- Shard data across nodes for write scaling
- Add read replicas for read scaling

**Auto-scaling Rules (Kubernetes):**
```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: lumadb-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: StatefulSet
    name: lumadb
  minReplicas: 3
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
```

---

## 4. Troubleshooting Guide

### 4.1 Common Issues & Solutions

#### High Query Latency

**Symptoms:** p99 latency > 100ms, slow query log filling up

**Diagnosis:**
```bash
# Check slow queries
tail -100 /var/log/lumadb/slow-query.log

# Check for full table scans
grep "Seq Scan" /var/log/lumadb/query.log | tail -20

# Check cache hit rate
curl http://localhost:8080/metrics | grep cache_hit
```

**Solutions:**
1. Create missing indexes
2. Increase cache size
3. Optimize query patterns
4. Add read replicas

#### High Memory Usage

**Symptoms:** OOM kills, swap usage, degraded performance

**Diagnosis:**
```bash
# Memory breakdown
curl http://localhost:8080/admin/memory | jq .

# Check memtable flush frequency
grep "memtable flush" /var/log/lumadb/server.log | tail -20

# OS memory stats
free -h
```

**Solutions:**
1. Reduce memtable size
2. Trigger manual compaction
3. Increase container memory limits
4. Enable memory-mapped I/O

#### Replication Lag

**Symptoms:** Stale reads, cluster health warnings

**Diagnosis:**
```bash
# Check replication status
curl http://localhost:8080/cluster/replication | jq .

# Check network latency
ping -c 10 node-2

# Check follower logs
ssh node-2 journalctl -u lumadb | grep replication
```

**Solutions:**
1. Check network connectivity
2. Increase replication timeout
3. Reduce write batch size
4. Add more I/O capacity to followers

#### Disk Space Issues

**Symptoms:** Write failures, compaction stalls

**Diagnosis:**
```bash
# Disk usage
df -h /var/lib/lumadb

# Large files
du -sh /var/lib/lumadb/* | sort -h

# Pending compactions
curl http://localhost:8080/admin/compaction | jq .
```

**Solutions:**
1. Trigger emergency compaction
2. Delete old WAL files
3. Archive cold data
4. Expand storage

### 4.2 Diagnostic Commands

```bash
# Full system diagnostics
lumadb diagnose --output /tmp/diagnostics.tar.gz

# Connection test
lumadb ping --host localhost --port 8080

# Validate configuration
lumadb config validate /etc/lumadb/config.toml

# Check data integrity
lumadb fsck --collection users

# Force compaction
lumadb compact --collection orders --level all

# Dump internal state
lumadb debug dump-state > state.json
```

### 4.3 Emergency Procedures

**Emergency Read-Only Mode:**
```bash
# Enable read-only mode
curl -X POST http://localhost:8080/admin/read-only \
  -d '{"enabled": true, "reason": "Emergency maintenance"}'

# Disable
curl -X POST http://localhost:8080/admin/read-only \
  -d '{"enabled": false}'
```

**Emergency Shutdown:**
```bash
# Graceful shutdown (flushes data)
lumadb shutdown --graceful --timeout 60s

# Force shutdown (may lose unflushed data)
lumadb shutdown --force
```

**Data Recovery Mode:**
```bash
# Start in recovery mode
lumadb start --recovery-mode

# Rebuild indexes
lumadb rebuild-indexes --collection all

# Validate and repair
lumadb repair --collection orders --fix
```

---

## 5. Disaster Recovery

### 5.1 Backup Strategy

**Backup Types:**
| Type | Frequency | Retention | RPO |
|------|-----------|-----------|-----|
| Full | Weekly | 4 weeks | 7 days |
| Incremental | Daily | 2 weeks | 1 day |
| WAL Streaming | Continuous | 7 days | Minutes |

**Backup Script:**
```bash
#!/bin/bash
# /usr/local/bin/lumadb-backup.sh

DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backups/lumadb"
S3_BUCKET="s3://lumadb-backups"

# Create backup
lumadb backup \
  --destination "${BACKUP_DIR}/${DATE}" \
  --type incremental \
  --compression zstd \
  --parallel 4

# Upload to S3
aws s3 sync "${BACKUP_DIR}/${DATE}" "${S3_BUCKET}/${DATE}"

# Cleanup old local backups (keep 7 days)
find "${BACKUP_DIR}" -type d -mtime +7 -exec rm -rf {} \;

# Verify backup
lumadb backup verify --source "${BACKUP_DIR}/${DATE}"
```

### 5.2 Recovery Procedures

**Full Recovery:**
```bash
# Stop the service
systemctl stop lumadb

# Clear data directory
rm -rf /var/lib/lumadb/data/*

# Restore from backup
lumadb restore \
  --source s3://lumadb-backups/20240115_020000 \
  --target /var/lib/lumadb/data

# Start service
systemctl start lumadb

# Verify
lumadb health --verify-data
```

**Point-in-Time Recovery:**
```bash
# Restore to specific timestamp
lumadb restore \
  --source s3://lumadb-backups/20240115_020000 \
  --point-in-time "2024-01-15T14:30:00Z" \
  --target /var/lib/lumadb/data
```

### 5.3 Failover Procedures

**Automatic Failover (Raft):**
- Leader failure detected within 1-5 seconds
- New leader elected automatically
- Clients reconnect to new leader

**Manual Failover:**
```bash
# Force leader change
curl -X POST http://localhost:8080/cluster/transfer-leadership \
  -d '{"target_node": "node-2"}'

# Verify new leader
curl http://localhost:8080/cluster/status | jq .leader
```

---

## 6. Maintenance Procedures

### 6.1 Regular Maintenance Tasks

**Daily:**
- [ ] Review error logs
- [ ] Check backup completion
- [ ] Monitor disk space
- [ ] Verify replication status

**Weekly:**
- [ ] Analyze slow query log
- [ ] Review index usage
- [ ] Check for unused indexes
- [ ] Validate backup integrity

**Monthly:**
- [ ] Full backup test restore
- [ ] Security audit review
- [ ] Capacity planning review
- [ ] Performance baseline comparison

### 6.2 Compaction Management

```bash
# View compaction status
curl http://localhost:8080/admin/compaction | jq .

# Trigger manual compaction
lumadb compact --collection orders

# Schedule off-peak compaction
cat > /etc/cron.d/lumadb-compact << 'EOF'
0 3 * * * lumadb lumadb compact --all --level 0-2
0 4 * * 0 lumadb lumadb compact --all --level all
EOF
```

### 6.3 Upgrade Procedures

**Rolling Upgrade:**
```bash
# 1. Upgrade one node at a time
for node in node-3 node-2 node-1; do
  echo "Upgrading $node..."

  # Drain connections
  kubectl drain $node --ignore-daemonsets

  # Update image
  kubectl set image statefulset/lumadb \
    lumadb=lumadb/lumadb:v3.1.0 \
    -n lumadb

  # Wait for ready
  kubectl rollout status statefulset/lumadb -n lumadb

  # Verify health
  curl http://$node:8080/health

  # Uncordon node
  kubectl uncordon $node

  sleep 60  # Allow stabilization
done
```

---

## 7. Security Operations

### 7.1 Security Checklist

**Pre-Production:**
- [ ] Change default passwords
- [ ] Enable TLS encryption
- [ ] Configure firewall rules
- [ ] Set up RBAC policies
- [ ] Enable audit logging
- [ ] Disable unnecessary protocols

**Runtime:**
- [ ] Monitor failed login attempts
- [ ] Review access patterns
- [ ] Rotate API keys/tokens
- [ ] Update security patches
- [ ] Review audit logs

### 7.2 Audit Log Analysis

```bash
# Failed authentication attempts
grep "authentication failed" /var/log/lumadb/access.log | \
  awk '{print $1}' | sort | uniq -c | sort -rn | head -10

# Unusual query patterns
grep "DELETE\|DROP\|TRUNCATE" /var/log/lumadb/query.log

# Admin actions
grep "admin" /var/log/lumadb/audit.log | tail -100
```

### 7.3 Incident Response

**Security Incident Playbook:**
1. **Detect:** Alert triggered or anomaly detected
2. **Contain:** Enable read-only mode, block suspicious IPs
3. **Investigate:** Review audit logs, identify scope
4. **Eradicate:** Remove compromised credentials, patch vulnerabilities
5. **Recover:** Restore from clean backup if needed
6. **Learn:** Post-incident review, update procedures

---

## 8. Runbook Procedures

### 8.1 Node Failure Recovery

```
RUNBOOK: Node Failure Recovery
================================

SYMPTOMS:
- Node unreachable
- Cluster reports node down
- Alerts: ClusterNodeDown

IMMEDIATE ACTIONS:
1. Verify node status
   $ ping node-X
   $ ssh node-X systemctl status lumadb

2. If node recoverable:
   $ ssh node-X systemctl restart lumadb
   $ curl http://node-X:8080/health

3. If node unrecoverable:
   $ kubectl delete pod lumadb-X -n lumadb
   # StatefulSet will recreate

4. Verify cluster health:
   $ curl http://localhost:8080/cluster/health

5. Verify data integrity:
   $ lumadb fsck --node node-X

ESCALATION:
- If recovery fails after 30 minutes, engage on-call DBA
- If data loss suspected, initiate DR procedure
```

### 8.2 Performance Degradation

```
RUNBOOK: Performance Degradation
=================================

SYMPTOMS:
- Latency > SLA threshold
- Alerts: HighQueryLatency
- User complaints

DIAGNOSIS:
1. Check system resources:
   $ top, iostat -x 1, vmstat 1

2. Check LumaDB metrics:
   $ curl http://localhost:8080/metrics | grep -E 'latency|queue'

3. Check slow queries:
   $ tail -100 /var/log/lumadb/slow-query.log

REMEDIATION:
1. If CPU bound:
   - Kill expensive queries
   - Scale horizontally

2. If Memory bound:
   - Trigger compaction
   - Increase cache size or scale

3. If I/O bound:
   - Check disk health
   - Reduce compaction parallelism
   - Scale storage

4. If Query bound:
   - Add missing indexes
   - Optimize query patterns
```

### 8.3 Disk Space Emergency

```
RUNBOOK: Disk Space Emergency
==============================

SYMPTOMS:
- Disk usage > 90%
- Write failures
- Alerts: DiskSpaceCritical

IMMEDIATE ACTIONS:
1. Enable read-only mode:
   $ curl -X POST http://localhost:8080/admin/read-only \
       -d '{"enabled": true}'

2. Clear temporary files:
   $ rm -rf /var/lib/lumadb/tmp/*

3. Trigger emergency compaction:
   $ lumadb compact --emergency --delete-tombstones

4. Archive old WAL:
   $ lumadb wal archive --older-than 7d --destination /archive

5. Delete expired data:
   $ lumadb gc --force --collection logs

6. Re-enable writes when space available:
   $ curl -X POST http://localhost:8080/admin/read-only \
       -d '{"enabled": false}'

PREVENTION:
- Set up disk space alerts at 70%, 80%, 90%
- Configure automatic WAL archival
- Implement data retention policies
```

---

*Document Version: 3.0.0*
*Last Updated: December 2024*
