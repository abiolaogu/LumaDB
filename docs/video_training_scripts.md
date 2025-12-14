# LumaDB Video Training Scripts

## Version 3.0.0 | December 2024

---

## Video 1: Introduction to LumaDB (5 min)

### Script

**[0:00-0:30] Hook**
> "What if you could replace Prometheus, Elasticsearch, and PostgreSQL with a single 7.7 MB binary? That's LumaDB."

**[0:30-1:30] Problem Statement**
> - Managing multiple databases is complex
> - Different query languages, protocols, storage engines
> - High operational overhead

**[1:30-3:00] Solution Overview**
> - Single binary, multi-protocol
> - PostgreSQL, Prometheus, OTLP compatibility
> - Unified storage with multi-tier architecture

**[3:00-4:30] Demo**
> - Start LumaDB: `./luma-server`
> - Connect with psql
> - Show Prometheus metrics
> - Query logs via SQL

**[4:30-5:00] CTA**
> "Get started at github.com/lumadb"

---

## Video 2: Installation & Setup (8 min)

### Script

**[0:00-2:00] Binary Installation**
```bash
curl -LO https://github.com/lumadb/releases/latest/luma-server
chmod +x luma-server
./luma-server --config config.toml
```

**[2:00-4:00] Docker Deployment**
```bash
docker run -p 5432:5432 -p 9090:9090 lumadb/lumadb:latest
```

**[4:00-6:00] Configuration**
- Walk through config.toml sections
- Explain port assignments
- Show data_dir setup

**[6:00-8:00] Verification**
```bash
psql -h localhost -p 5432 -U lumadb
curl http://localhost:9091/metrics
```

---

## Video 3: Connecting Applications (10 min)

### Script

**[0:00-3:00] PostgreSQL Driver**
- Python: psycopg2
- Node.js: pg
- Go: pgx

**[3:00-6:00] OpenTelemetry**
- Configure OTLP exporter
- Point to localhost:4317
- Verify traces in LumaDB

**[6:00-10:00] Prometheus Integration**
- Add LumaDB as remote_write target
- Configure scrape targets
- Build Grafana dashboard

---

## Video 4: Security Best Practices (6 min)

### Script

**[0:00-2:00] Authentication**
- MD5 password verification
- Change default credentials
- Connection logging

**[2:00-4:00] Rate Limiting**
- Token bucket algorithm
- 100 requests/min default
- Ban duration configuration

**[4:00-6:00] Network Security**
- Firewall configuration
- Port exposure guidelines
- Future: TLS support

---

## Video 5: Monitoring LumaDB (7 min)

### Script

**[0:00-2:00] Metrics Endpoint**
- /metrics path
- Key metrics to monitor
- Prometheus scrape config

**[2:00-5:00] Grafana Dashboard**
- Import pre-built dashboard
- Active connections panel
- Query latency histogram
- Ingestion rate

**[5:00-7:00] Alerting**
- High connection count
- Rate limit exceeded
- WAL lag

---

## Video 6: Troubleshooting (8 min)

### Script

**[0:00-2:00] Connection Issues**
- Check port binding
- Verify firewall
- Test with telnet

**[2:00-4:00] Authentication Failures**
- Verify credentials
- Check client configuration
- Review logs

**[4:00-6:00] Performance Issues**
- Enable debug logging
- Check tier distribution
- Monitor memory

**[6:00-8:00] Recovery**
- WAL replay on restart
- Backup procedures
- Disaster recovery

---

## Production Notes

- **Resolution:** 1920x1080
- **Audio:** Professional microphone, quiet environment
- **Screen Recording:** Terminal and browser side-by-side
- **Editing:** Cut pauses, add annotations

---

*Last Updated: December 2024*
