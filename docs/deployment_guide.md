# LumaDB Deployment Guide

## Production Deployment

**Version:** 4.1.0 | **Last Updated:** December 2024

---

## 1. Prerequisites

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 4 cores | 16 cores |
| RAM | 8 GB | 32 GB |
| Storage | 50 GB SSD | 500 GB NVMe |
| Network | 1 Gbps | 10 Gbps |
| OS | Ubuntu 22.04+ | Ubuntu 24.04 |

### Software Requirements

```bash
# Rust 1.70+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Build tools
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev
```

---

## 2. Building for Production

```bash
# Clone repository
git clone https://github.com/abiolaogu/LumaDB.git
cd LumaDB/lumadb-compat

# Build optimized release binary
cargo build --release -p luma-server

# Binary location
ls -la target/release/luma-server
```

### Build Options

```bash
# With all features
cargo build --release -p luma-server --features full

# Minimal build (PostgreSQL only)
cargo build --release -p luma-server --no-default-features --features postgres
```

---

## 3. Configuration

### config.toml

```toml
# LumaDB Configuration

[server]
bind = "0.0.0.0"
data_dir = "/var/lib/lumadb/data"
wal_dir = "/var/lib/lumadb/wal"

[protocols.postgres]
port = 5432
max_connections = 10000

[protocols.mysql]
port = 3306
max_connections = 5000

[protocols.redis]
port = 6379
max_connections = 50000

[protocols.elasticsearch]
port = 9200
max_connections = 1000

[protocols.cassandra]
port = 9042
max_connections = 2000

[protocols.mongodb]
port = 27017
max_connections = 5000

[security]
rate_limit_burst = 1000
rate_limit_per_second = 100
require_tls = false

[performance]
wal_sync_mode = "EveryN"
wal_sync_interval = 100
max_segment_size = "64MB"
```

---

## 4. Running LumaDB

### Direct Execution

```bash
./target/release/luma-server --config config.toml
```

### Systemd Service

Create `/etc/systemd/system/lumadb.service`:

```ini
[Unit]
Description=LumaDB Multi-Protocol Database
After=network.target

[Service]
Type=simple
User=lumadb
Group=lumadb
WorkingDirectory=/opt/lumadb
ExecStart=/opt/lumadb/luma-server --config /etc/lumadb/config.toml
Restart=always
RestartSec=5
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable lumadb
sudo systemctl start lumadb
sudo systemctl status lumadb
```

---

## 5. Docker Deployment

### Dockerfile

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p luma-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/luma-server /usr/local/bin/
COPY config.toml /etc/lumadb/
EXPOSE 5432 3306 6379 9200 9042 27017
CMD ["luma-server", "--config", "/etc/lumadb/config.toml"]
```

### Docker Compose

```yaml
version: '3.8'
services:
  lumadb:
    build: .
    ports:
      - "5432:5432"   # PostgreSQL
      - "3306:3306"   # MySQL
      - "6379:6379"   # Redis
      - "9200:9200"   # Elasticsearch
      - "9042:9042"   # Cassandra
      - "27017:27017" # MongoDB
    volumes:
      - lumadb_data:/var/lib/lumadb
    environment:
      - RUST_LOG=info
    restart: unless-stopped

volumes:
  lumadb_data:
```

---

## 6. Kubernetes Deployment

### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: lumadb
spec:
  replicas: 3
  selector:
    matchLabels:
      app: lumadb
  template:
    metadata:
      labels:
        app: lumadb
    spec:
      containers:
      - name: lumadb
        image: lumadb:4.1.0
        ports:
        - containerPort: 5432
        - containerPort: 3306
        - containerPort: 6379
        - containerPort: 9200
        resources:
          requests:
            memory: "4Gi"
            cpu: "2"
          limits:
            memory: "16Gi"
            cpu: "8"
        livenessProbe:
          httpGet:
            path: /health
            port: 9200
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 9200
          initialDelaySeconds: 5
          periodSeconds: 10
```

### Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: lumadb
spec:
  type: LoadBalancer
  ports:
  - name: postgres
    port: 5432
  - name: mysql
    port: 3306
  - name: redis
    port: 6379
  - name: elasticsearch
    port: 9200
  - name: cassandra
    port: 9042
  - name: mongodb
    port: 27017
  selector:
    app: lumadb
```

---

## 7. Health Checks

### HTTP Endpoints

```bash
# Elasticsearch health
curl http://localhost:9200/_cluster/health

# ClickHouse ping
curl http://localhost:8123/ping

# Prometheus metrics
curl http://localhost:9090/metrics
```

### Protocol Verification

```bash
# PostgreSQL
psql -h localhost -p 5432 -U lumadb -c "SELECT 1"

# MySQL
mysql -h localhost -P 3306 -u root -e "SELECT 1"

# Redis
redis-cli -h localhost -p 6379 PING

# MongoDB
mongosh --host localhost:27017 --eval "db.runCommand({ping:1})"
```

---

## 8. Monitoring

### Prometheus Scrape Config

```yaml
scrape_configs:
  - job_name: 'lumadb'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: /metrics
```

### Grafana Dashboard

Import dashboard ID: `lumadb-overview`

Key metrics:
- `lumadb_connections_active`
- `lumadb_queries_total`
- `lumadb_query_latency_ms`
- `lumadb_wal_writes_total`

---

## 9. Security Hardening

### TLS Configuration

```toml
[security]
require_tls = true
cert_path = "/etc/lumadb/certs/server.crt"
key_path = "/etc/lumadb/certs/server.key"
```

### Firewall Rules

```bash
# Allow only necessary ports
sudo ufw allow 5432/tcp  # PostgreSQL
sudo ufw allow 6379/tcp  # Redis
sudo ufw deny 9200/tcp   # Block Elasticsearch externally
```

---

## 10. Known Security Advisories

| Advisory | Severity | Status | Notes |
|----------|----------|--------|-------|
| RUSTSEC-2025-0009 | Low | Transitive | ring via rcgen (TLS cert generation) |
| RUSTSEC-2023-0086 | Low | Transitive | lexical-core via arrow |
| RUSTSEC-2024-0436 | Warning | Unmaintained | paste (proc-macro) |
| RUSTSEC-2025-0010 | Warning | Unmaintained | ring 0.16.x |

These are in transitive dependencies and don't affect core LumaDB functionality.

---

## 11. Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| Port already in use | Check for existing processes: `lsof -i :5432` |
| Permission denied | Run as root or use `setcap`: `sudo setcap 'cap_net_bind_service=+ep' luma-server` |
| Out of memory | Increase system limits or reduce `max_connections` |
| WAL corruption | Delete WAL directory and restart (data loss) |

### Logs

```bash
# View logs
journalctl -u lumadb -f

# Debug mode
RUST_LOG=debug ./luma-server --config config.toml
```

---

**Repository:** https://github.com/abiolaogu/LumaDB
**Support:** Open an issue on GitHub
