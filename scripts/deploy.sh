#!/bin/bash
# LumaDB Deployment Script

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

DEPLOY_TARGET="${1:-docker}"

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║              LumaDB Deployment                            ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

cd "$(dirname "$0")/.."

case "$DEPLOY_TARGET" in
    docker)
        log_info "Deploying with Docker Compose..."
        docker-compose -f deploy/docker/docker-compose.yml up -d
        
        log_info "Waiting for LumaDB to start..."
        sleep 5
        
        if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
            log_info "LumaDB is running!"
            echo ""
            echo "Endpoints:"
            echo "  REST API:  http://localhost:8080"
            echo "  Kafka:     localhost:9092"
            echo "  GraphQL:   http://localhost:4000"
        else
            log_warn "LumaDB may still be starting. Check logs: docker-compose logs -f"
        fi
        ;;
        
    cluster)
        log_info "Deploying 3-node cluster with Docker Compose..."
        docker-compose -f deploy/docker/docker-compose.yml --profile cluster up -d
        
        log_info "Waiting for cluster to form..."
        sleep 10
        
        echo ""
        echo "Cluster Endpoints:"
        echo "  Node 1:  http://localhost:8081 (Kafka: 9093)"
        echo "  Node 2:  http://localhost:8082 (Kafka: 9094)"
        echo "  Node 3:  http://localhost:8083 (Kafka: 9095)"
        ;;
        
    kubernetes|k8s)
        log_info "Deploying to Kubernetes..."
        
        kubectl apply -f deploy/kubernetes/namespace.yaml
        kubectl apply -f deploy/kubernetes/rbac.yaml
        kubectl apply -f deploy/kubernetes/configmap.yaml
        kubectl apply -f deploy/kubernetes/service.yaml
        kubectl apply -f deploy/kubernetes/statefulset.yaml
        
        log_info "Waiting for pods to be ready..."
        kubectl -n lumadb wait --for=condition=ready pod -l app=lumadb --timeout=300s
        
        log_info "LumaDB deployed to Kubernetes!"
        kubectl -n lumadb get pods
        ;;
        
    systemd)
        log_info "Deploying as systemd service..."
        
        if [[ $EUID -ne 0 ]]; then
            log_error "Systemd deployment requires root. Run with sudo."
            exit 1
        fi
        
        ./deploy/systemd/install.sh
        ;;
        
    *)
        log_error "Unknown deployment target: $DEPLOY_TARGET"
        echo ""
        echo "Usage: $0 [target]"
        echo ""
        echo "Targets:"
        echo "  docker      Deploy single node with Docker Compose (default)"
        echo "  cluster     Deploy 3-node cluster with Docker Compose"
        echo "  kubernetes  Deploy to Kubernetes"
        echo "  k8s         Alias for kubernetes"
        echo "  systemd     Install as systemd service (requires root)"
        exit 1
        ;;
esac

echo ""
log_info "Deployment complete!"
