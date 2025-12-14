# LumaDB Deployment Guide

## Version 3.0.0 | December 2024

---

## 1. Pre-Deployment Checklist

### 1.1 Build Verification

- [x] Release build successful: `./release.sh`
- [x] Binary size: 7.7 MB
- [x] Unit tests passing: 9/9
- [x] Security scan completed

### 1.2 Configuration Review

- [ ] Change default credentials
- [ ] Set appropriate data_dir
- [ ] Configure log_level for production
- [ ] Review rate limit settings

---

## 2. Deployment Methods

### 2.1 Binary Deployment

```bash
# Production run
./luma-server --config /etc/lumadb/config.toml

# As systemd service
sudo cp lumadb.service /etc/systemd/system/
sudo systemctl enable lumadb
sudo systemctl start lumadb
```

**Systemd Unit File:**
```ini
[Unit]
Description=LumaDB Database Server
After=network.target

[Service]
Type=simple
User=lumadb
ExecStart=/usr/local/bin/luma-server --config /etc/lumadb/config.toml
Restart=always
RestartSec=5
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
```

### 2.2 Docker Deployment

```bash
# Build image
docker build -t lumadb:3.0.0 .

# Run with volumes
docker run -d \
  --name lumadb \
  -p 5432:5432 \
  -p 9090:9090 \
  -p 4317:4317 \
  -v lumadb-data:/data \
  lumadb:3.0.0
```

### 2.3 Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: lumadb
spec:
  replicas: 1
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
        image: lumadb:3.0.0
        ports:
        - containerPort: 5432
        - containerPort: 9090
        - containerPort: 4317
        resources:
          requests:
            memory: "2Gi"
            cpu: "1000m"
          limits:
            memory: "8Gi"
            cpu: "4000m"
        volumeMounts:
        - name: data
          mountPath: /data
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: lumadb-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: lumadb
spec:
  selector:
    app: lumadb
  ports:
  - name: postgres
    port: 5432
  - name: prometheus
    port: 9090
  - name: otlp
    port: 4317
```

---

## 3. Post-Deployment Verification

```bash
# Health check
curl http://localhost:8080/health

# PostgreSQL connection
psql -h localhost -p 5432 -U lumadb -c "SELECT 1"

# Metrics endpoint
curl http://localhost:9091/metrics | head -20

# Logs
journalctl -u lumadb -f
```

---

## 4. Monitoring Setup

### Prometheus Scrape Config

```yaml
scrape_configs:
  - job_name: 'lumadb'
    static_configs:
      - targets: ['localhost:9091']
```

### Key Alerts

```yaml
groups:
- name: lumadb
  rules:
  - alert: LumaDBHighConnections
    expr: lumadb_active_connections > 90
    for: 5m
  - alert: LumaDBRateLimited
    expr: rate(lumadb_rate_limit_exceeded[5m]) > 1
    for: 1m
```

---

## 5. Backup & Recovery

### Backup
```bash
# Stop writes (optional)
# Copy WAL and data
cp -r /data/wal.log /backup/wal-$(date +%Y%m%d).log
cp -r /data/segments /backup/segments-$(date +%Y%m%d)
```

### Recovery
```bash
# Restore files
cp /backup/wal-*.log /data/wal.log

# Restart (automatic WAL replay)
systemctl restart lumadb
```

---

*Last Updated: December 2024*
