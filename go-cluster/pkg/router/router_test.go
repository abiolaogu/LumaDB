package router

import (
	"context"
	"os"
	"testing"

	"github.com/lumadb/cluster/pkg/cluster"
	"github.com/lumadb/cluster/pkg/config"
	"go.uber.org/zap"
)

func createTestNode(t *testing.T) *cluster.Node {
	tmpDir, err := os.MkdirTemp("", "lumadb-router-test-*")
	if err != nil {
		t.Fatalf("Failed to create temp dir: %v", err)
	}
	// Note: In a real test we'd defer cleanup, but for helper functions it's tricky.
	// We rely on OS cleanup or specific test cleanup.

	cfg := config.DefaultConfig()
	cfg.DataDir = tmpDir
	cfg.NodeID = "node1"
	cfg.RaftAddr = "127.0.0.1:0"

	logger := zap.NewNop()

	node, err := cluster.NewNode(cfg, logger)
	if err != nil {
		t.Fatalf("Failed to create node: %v", err)
	}
	return node
}

func TestNewRouter(t *testing.T) {
	node := createTestNode(t)
	defer node.Shutdown()
	defer os.RemoveAll(node.GetConfig().DataDir)

	r := NewRouter(node, zap.NewNop())
	if r == nil {
		t.Fatal("NewRouter returned nil")
	}
}

func TestRouter_Route(t *testing.T) {
	node := createTestNode(t)
	defer node.Shutdown()
	defer os.RemoveAll(node.GetConfig().DataDir)

	r := NewRouter(node, zap.NewNop())

	// Test basic routing (should default to localhost or leader)
	// Since node is not bootstrapped in this helper, behavior depends on default state
	// In Node constructor, we init active shards.

	ctx := context.Background()
	addr, err := r.Route(ctx, "users", []byte("key1"))
	if err != nil {
		t.Fatalf("Route failed: %v", err)
	}

	if addr == "" {
		t.Error("Route returned empty address")
	}
}

func TestRouter_RouteRead(t *testing.T) {
	node := createTestNode(t)
	defer node.Shutdown()
	defer os.RemoveAll(node.GetConfig().DataDir)

	r := NewRouter(node, zap.NewNop())

	ctx := context.Background()
	addr, err := r.RouteRead(ctx, "users", []byte("key1"))
	if err != nil {
		t.Fatalf("RouteRead failed: %v", err)
	}

	if addr == "" {
		t.Error("RouteRead returned empty address")
	}
}

func TestRouter_ConnectionPool(t *testing.T) {
	node := createTestNode(t)
	defer node.Shutdown()
	defer os.RemoveAll(node.GetConfig().DataDir)

	r := NewRouter(node, zap.NewNop())

	addr := "127.0.0.1:8080"
	conn, err := r.GetConnection(addr)
	if err != nil {
		t.Fatalf("GetConnection failed: %v", err)
	}

	if conn.addr != addr {
		t.Errorf("Connection addr mismatch: got %s want %s", conn.addr, addr)
	}

	r.ReleaseConnection(conn)
}
