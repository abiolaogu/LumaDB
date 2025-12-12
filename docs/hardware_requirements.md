# Hardware Requirements for LumaDB v2.7.0

This document outlines the recommended hardware specifications for running LumaDB in various environments.

## 1. Development & Testing (Minimum)
Suitable for local development, CI/CD pipelines, and small functional tests.

- **CPU**: 2 vCPUs (x86_64 or ARM64)
- **RAM**: 4 GB
- **Storage**: 10 GB SSD
- **Network**: Standard 1Gbps Ethernet
- **OS**: Linux (Kernel 6.x+), macOS 14+, or Windows (WSL2)

## 2. Production (Recommended)
Suitable for high-performance workloads, moderate datasets, and production traffic.

- **CPU**: 4+ vCPUs (High Frequency, e.g., AWS c6i/c7g)
- **RAM**: 16 GB+ (LumaDB uses ~50% for Block Cache, ~25% for OS Page Cache)
- **Storage**: 100 GB+ NVMe SSD
    - **IOPS**: > 3,000 Provisioned IOPS
    - **Throughput**: > 125 MB/s
- **Network**: 10Gbps+ (Low Latency)
- **Cluster Size**: Minimum 3 nodes for Raft consensus high availability.

## 3. Extreme Performance (Hyperscale)
Suitable for massive datasets (TB+), real-time analytics, and high-throughput ingestion.

- **CPU**: 16+ vCPUs (Dedicated Cores)
- **RAM**: 64 GB+ (ECC Recommended)
- **Storage**: 1 TB+ NVMe SSD (RAID 0 or 10)
    - **IOPS**: > 10,000 IOPS
    - **File System**: XFS or EXT4 with `noatime`
- **Network**: 25Gbps+ (Placement Group enabled)
- **Topology**: Multi-AZ or Multi-Region deployment.

## 4. Specific Component Requirements

### Rust Core Engine
- **Memory**: Heavily relies on RAM for Memtables and Block Cache. Configurable via `luma.toml`.
- **Disk**: Requires fast random I/O for LSM-tree compaction. Avoid HDD / Rotating Rust.
- **CPU**: Benefits from AVX-512 (x86) or NEON (ARM) for SIMD aggregations.

### Go Cluster Coordinator
- **CPU**: scales linearly with thread count (`GOMAXPROCS`).
- **Network**: Sensitive to latency for Raft leader election (keep RTT < 50ms).

### Python AI Service
- **GPU**: Optional but recommended for Vector Embeddings (NVIDIA T4/A10G).
- **RAM**: Requires ~2GB extra per loaded ML model (e.g., BERT, ResNet).

## 5. Deployment Checklist
- [ ] **Disable Swap**: Prevent latency spikes during memory pressure.
- [ ] **NTP Sync**: Ensure clocks are synchronized for Raft consistency.
- [ ] **File Descriptors**: Increase `ulimit -n` to 65535+.
- [ ] **TCP Keepalive**: Tune kernel parameters for long-lived connections.
