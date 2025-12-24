#!/bin/bash
# LumaDB Linux Installation Script

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check root
if [[ $EUID -ne 0 ]]; then
   log_error "This script must be run as root"
   exit 1
fi

VERSION="${1:-latest}"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/lumadb"
DATA_DIR="/var/lib/lumadb"
LOG_DIR="/var/log/lumadb"

log_info "Installing LumaDB ${VERSION}..."

# Create user
if ! id -u lumadb &>/dev/null; then
    log_info "Creating lumadb user..."
    useradd --system --no-create-home --shell /usr/sbin/nologin lumadb
fi

# Create directories
log_info "Creating directories..."
mkdir -p "$CONFIG_DIR" "$DATA_DIR" "$LOG_DIR"
chown -R lumadb:lumadb "$DATA_DIR" "$LOG_DIR"
chmod 700 "$DATA_DIR" "$LOG_DIR"

# Download binary
log_info "Downloading LumaDB binary..."
ARCH=$(uname -m)
case $ARCH in
    x86_64) ARCH="amd64" ;;
    aarch64) ARCH="arm64" ;;
    *) log_error "Unsupported architecture: $ARCH"; exit 1 ;;
esac

if [[ "$VERSION" == "latest" ]]; then
    DOWNLOAD_URL="https://github.com/abiolaogu/LumaDB/releases/latest/download/lumadb-linux-${ARCH}.tar.gz"
else
    DOWNLOAD_URL="https://github.com/abiolaogu/LumaDB/releases/download/v${VERSION}/lumadb-linux-${ARCH}.tar.gz"
fi

curl -fsSL "$DOWNLOAD_URL" -o /tmp/lumadb.tar.gz
tar -xzf /tmp/lumadb.tar.gz -C "$INSTALL_DIR"
chmod +x "$INSTALL_DIR/lumadb"
rm /tmp/lumadb.tar.gz

# Install configuration
if [[ ! -f "$CONFIG_DIR/lumadb.yaml" ]]; then
    log_info "Installing default configuration..."
    cat > "$CONFIG_DIR/lumadb.yaml" << 'CONF'
server:
  node_id: 1
  data_dir: /var/lib/lumadb

api:
  rest:
    host: "0.0.0.0"
    port: 8080
  graphql:
    host: "0.0.0.0"
    port: 4000
  grpc:
    host: "0.0.0.0"
    port: 50051

kafka:
  host: "0.0.0.0"
  port: 9092

logging:
  level: info
  format: json
CONF
fi

# Install systemd service
log_info "Installing systemd service..."
cp "$(dirname "$0")/lumadb.service" /etc/systemd/system/
systemctl daemon-reload

# Enable and start
log_info "Enabling and starting LumaDB..."
systemctl enable lumadb
systemctl start lumadb

# Wait for startup
sleep 3

# Check status
if systemctl is-active --quiet lumadb; then
    log_info "LumaDB is running!"
else
    log_error "Failed to start LumaDB. Check: journalctl -u lumadb"
    exit 1
fi

echo ""
log_info "============================================"
log_info "LumaDB installed successfully!"
log_info "============================================"
echo ""
echo "Endpoints:"
echo "  REST API:  http://localhost:8080"
echo "  Kafka:     localhost:9092"
echo "  GraphQL:   http://localhost:4000"
echo "  gRPC:      localhost:50051"
echo ""
echo "Configuration: $CONFIG_DIR/lumadb.yaml"
echo "Data:          $DATA_DIR"
echo "Logs:          $LOG_DIR"
echo ""
echo "Commands:"
echo "  Status:    sudo systemctl status lumadb"
echo "  Logs:      sudo journalctl -u lumadb -f"
echo "  Stop:      sudo systemctl stop lumadb"
echo "  Restart:   sudo systemctl restart lumadb"
