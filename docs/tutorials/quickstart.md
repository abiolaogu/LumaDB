# LumaDB Quick Start Tutorial

## Get up and running in 5 minutes

---

## Step 1: Download and Run

```bash
# Download the release
curl -LO https://github.com/lumadb/releases/latest/luma-server
chmod +x luma-server

# Create minimal config
cat > config.toml << EOF
[general]
data_dir = "./data"
log_level = "info"

[server]
host = "127.0.0.1"
port = 8080

[postgres]
enabled = true
port = 5432

[metrics]
enabled = true
port = 9091
EOF

# Run LumaDB
./luma-server --config config.toml
```

---

## Step 2: Connect with PostgreSQL Client

```bash
# Install psql if needed
# macOS: brew install postgresql
# Ubuntu: apt install postgresql-client

psql -h localhost -p 5432 -U lumadb
# Password: lumadb

# Run a query
SELECT 1;
```

---

## Step 3: Configure Prometheus (Optional)

Add LumaDB as a remote_write target:

```yaml
# prometheus.yml
remote_write:
  - url: "http://localhost:9090/api/v1/write"
```

Or configure LumaDB to scrape your targets.

---

## Step 4: Send OpenTelemetry Data

### Python Example

```python
from opentelemetry import metrics
from opentelemetry.exporter.otlp.proto.grpc.metric_exporter import OTLPMetricExporter
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.metrics.export import PeriodicExportingMetricReader

# Configure exporter
exporter = OTLPMetricExporter(endpoint="localhost:4317", insecure=True)
reader = PeriodicExportingMetricReader(exporter)
provider = MeterProvider(metric_readers=[reader])
metrics.set_meter_provider(provider)

# Create and use meter
meter = metrics.get_meter("my-app")
counter = meter.create_counter("requests_total")
counter.add(1, {"path": "/api/users"})
```

---

## Step 5: Monitor LumaDB

```bash
# Health check
curl http://localhost:8080/health

# View metrics
curl http://localhost:9091/metrics
```

---

## Next Steps

- [User Manual](./user_manual.md) - Detailed configuration options
- [API Reference](./api-reference/api_reference.md) - All endpoints
- [Training Manual](./training_manual.md) - In-depth learning

---

*Congratulations! You're now running LumaDB.*
