# LumaDB Hardware Requirements

## Version 3.0.0 | December 2024

---

## 1. Minimum Requirements

### Development/Testing
| Component | Requirement |
|-----------|-------------|
| **CPU** | 2 cores (x86_64 or ARM64) |
| **RAM** | 4 GB |
| **Storage** | 10 GB SSD |
| **OS** | Linux, macOS, Windows (WSL2) |
| **Rust** | 1.75+ (build only) |

### Binary Size
- Release binary: **7.7 MB**
- Docker image: **~50 MB** (Alpine-based)

---

## 2. Recommended Production

### Small Deployment (< 100K series)
| Component | Requirement |
|-----------|-------------|
| **CPU** | 4 cores |
| **RAM** | 16 GB |
| **Storage** | 100 GB NVMe SSD |
| **Network** | 1 Gbps |

### Medium Deployment (100K - 1M series)
| Component | Requirement |
|-----------|-------------|
| **CPU** | 8 cores |
| **RAM** | 32 GB |
| **Storage** | 500 GB NVMe SSD |
| **Network** | 10 Gbps |

### Large Deployment (1M+ series)
| Component | Requirement |
|-----------|-------------|
| **CPU** | 16+ cores |
| **RAM** | 64+ GB |
| **Storage** | 1+ TB NVMe SSD |
| **Network** | 25 Gbps |

---

## 3. Storage Tiers

| Tier | Technology | Latency | Cost |
|------|------------|---------|------|
| **Hot** | RAM (DashMap) | ~100ns | $$$ |
| **Warm** | NVMe SSD | ~1ms | $$ |
| **Cold** | HDD/Object Store | ~10ms | $ |

### Storage Estimation

```
Time-series formula:
  Storage = series_count × samples_per_day × bytes_per_sample × retention_days

Example (100K series, 1 sample/15s, 30 days):
  100,000 × 5,760 × 0.17 bytes × 30 = ~3 GB (with Gorilla compression)
```

---

## 4. Network Requirements

### Ports

| Port | Protocol | Service |
|------|----------|---------|
| 5432 | TCP | PostgreSQL |
| 9090 | TCP | Prometheus API |
| 4317 | TCP | OTLP gRPC |
| 8080 | TCP | HTTP API |
| 50051 | TCP | Internal gRPC |
| 9091 | TCP | Metrics Endpoint |

### Bandwidth Estimation

| Workload | Ingestion Rate | Network |
|----------|----------------|---------|
| Light | 10K samples/sec | 10 Mbps |
| Medium | 100K samples/sec | 100 Mbps |
| Heavy | 1M samples/sec | 1 Gbps |

---

## 5. Cloud Recommendations

### AWS
| Tier | Instance | Notes |
|------|----------|-------|
| Dev | t3.medium | 2 vCPU, 4 GB |
| Small | m5.xlarge | 4 vCPU, 16 GB |
| Medium | m5.2xlarge | 8 vCPU, 32 GB |
| Large | m5.4xlarge | 16 vCPU, 64 GB |

### GCP
| Tier | Instance | Notes |
|------|----------|-------|
| Dev | e2-medium | 2 vCPU, 4 GB |
| Small | n2-standard-4 | 4 vCPU, 16 GB |
| Medium | n2-standard-8 | 8 vCPU, 32 GB |
| Large | n2-standard-16 | 16 vCPU, 64 GB |

### Azure
| Tier | Instance | Notes |
|------|----------|-------|
| Dev | Standard_D2s_v3 | 2 vCPU, 8 GB |
| Small | Standard_D4s_v3 | 4 vCPU, 16 GB |
| Medium | Standard_D8s_v3 | 8 vCPU, 32 GB |
| Large | Standard_D16s_v3 | 16 vCPU, 64 GB |

---

## 6. Kubernetes Resources

```yaml
resources:
  requests:
    memory: "2Gi"
    cpu: "1000m"
  limits:
    memory: "8Gi"
    cpu: "4000m"
```

---

*Last Updated: December 2024*
