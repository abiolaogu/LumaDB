# Docker Hub & CI/CD Setup Guide

## 1. Docker Hub Configuration

### Create Docker Hub Account

1. Go to https://hub.docker.com
2. Create account or sign in
3. Create repository: `lumadb/lumadb`

### Generate Access Token

1. Go to **Account Settings** → **Security**
2. Click **New Access Token**
3. Name: `github-actions`
4. Permissions: **Read, Write, Delete**
5. Copy the token (save securely)

---

## 2. GitHub Secrets Configuration

Navigate to your repository:
**Settings** → **Secrets and variables** → **Actions**

Add these secrets:

| Secret Name | Value |
|-------------|-------|
| `DOCKERHUB_USERNAME` | Your Docker Hub username |
| `DOCKERHUB_TOKEN` | Access token from step above |

### How to Add Secrets

1. Click **New repository secret**
2. Enter name: `DOCKERHUB_USERNAME`
3. Enter value: `your-dockerhub-username`
4. Click **Add secret**
5. Repeat for `DOCKERHUB_TOKEN`

---

## 3. Quick Deployment Commands

### Local Docker

```bash
# Build and run
docker-compose up -d

# With monitoring stack
docker-compose -f docker-compose.monitoring.yml up -d

# View logs
docker-compose logs -f lumadb

# Stop
docker-compose down
```

### Kubernetes

```bash
# Deploy
kubectl apply -f k8s/lumadb.yaml

# Check status
kubectl get pods -l app=lumadb
kubectl get svc lumadb

# View logs
kubectl logs -l app=lumadb -f

# Port forward for local access
kubectl port-forward svc/lumadb 5432:5432 6379:6379 9200:9200
```

---

## 4. Monitoring Access

### Grafana

- **URL:** http://localhost:3000
- **Username:** `admin`
- **Password:** `lumadb123`
- **Dashboard:** LumaDB Overview (pre-configured)

### Prometheus

- **URL:** http://localhost:9091
- **Targets:** http://localhost:9091/targets

---

## 5. Verify Deployment

### Test All Protocols

```bash
# PostgreSQL
psql -h localhost -p 5432 -U lumadb -c "SELECT 1"

# MySQL
mysql -h localhost -P 3306 -u root -e "SELECT 1"

# Redis
redis-cli -h localhost -p 6379 PING

# Elasticsearch
curl http://localhost:9200/_cluster/health

# MongoDB
mongosh --host localhost:27017 --eval "db.ping()"

# Cassandra
cqlsh localhost 9042 -e "SELECT release_version FROM system.local"

# ClickHouse
curl "http://localhost:8123/?query=SELECT%201"
```

---

## 6. Production Checklist

### Before Deployment

- [ ] Docker Hub secrets configured
- [ ] CI/CD pipeline tested
- [ ] TLS certificates generated
- [ ] Firewall rules configured
- [ ] Backup strategy in place
- [ ] Monitoring alerts configured

### After Deployment

- [ ] All health checks passing
- [ ] Metrics appearing in Grafana
- [ ] All protocols responding
- [ ] WAL persistence verified
- [ ] Rate limiting tested

---

## 7. Scaling

### Horizontal Scaling

```bash
# Kubernetes
kubectl scale deployment lumadb --replicas=5

# Docker Swarm
docker service scale lumadb_lumadb=5
```

### Vertical Scaling

Edit resource limits in:
- `docker-compose.yml` → `deploy.resources`
- `k8s/lumadb.yaml` → `resources.limits`

---

## 8. Backup & Recovery

### Backup Data

```bash
# Docker
docker exec lumadb tar -czvf /backup/data.tar.gz /var/lib/lumadb/data

# Copy out
docker cp lumadb:/backup/data.tar.gz ./backups/
```

### WAL Checkpoint

```bash
# Force WAL checkpoint before backup
curl -X POST http://localhost:9090/wal/checkpoint
```

---

## 9. Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| Container won't start | Check `docker logs lumadb` |
| Port conflict | Stop conflicting services |
| Out of memory | Increase container limits |
| Slow queries | Check Grafana latency dashboard |

### Debug Mode

```bash
# Enable debug logging
docker run -e RUST_LOG=debug lumadb/lumadb:4.1.0
```

---

## 10. Support

- **Repository:** https://github.com/abiolaogu/LumaDB
- **Issues:** https://github.com/abiolaogu/LumaDB/issues
- **Documentation:** https://github.com/abiolaogu/LumaDB/docs
