package api

import (
	"context"
	"encoding/json"

	pb "github.com/lumadb/cluster/pkg/api/pb"
	"github.com/lumadb/cluster/pkg/cluster"
	"go.uber.org/zap"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// LumaGRPCServer implements the LumaService gRPC interface
type LumaGRPCServer struct {
	pb.UnimplementedLumaServiceServer
	node   *cluster.Node
	logger *zap.Logger
}

// RegisterGRPCServer registers the LumaService with the gRPC server
func RegisterGRPCServer(s *grpc.Server, node *cluster.Node, logger *zap.Logger) {
	srv := &LumaGRPCServer{
		node:   node,
		logger: logger,
	}
	pb.RegisterLumaServiceServer(s, srv)
}

// Execute handles single query execution
func (s *LumaGRPCServer) Execute(ctx context.Context, req *pb.QueryRequest) (*pb.QueryResponse, error) {
	s.logger.Debug("Received Execute request", zap.String("query", req.Query), zap.String("dialect", req.Dialect))

	// Parse the query or use the raw query string depending on dialect.
	// Ideally we would parse this to determine if it's read or write.
	// For now, let's assume raw commands wrapped in JSON if coming from lumadb-compat special logic,
	// OR we forward this to a hypothetical query engine.

	// BUT current lumadb-compat produces LumaIR.
	// `req.Payload` might contain serialized LumaIR.
	// `go-cluster` needs to understand LumaIR or just blindly pass it to `luma_core` via CGO?
	// `luma_core` (Rust) definitely understands LumaIR (it's defined there).
	// So `go-cluster` should act as a proxy to `luma_core`.

	// However, `node.go` uses `luma_core` via CGO but only exposes basic CRUD: Insert, Get, Update, Delete.
	// And `Query` which takes a JSON query string.

	// If `req.Dialect` is "lumair-json", we can pass `req.Query` (which should be JSON) to `node.RunQuery`.
	// But `node.RunQuery` logic is: `n.db.Query(collection, query)`.
	// `n.db.Query` calls `C.luma_query`.

	// We need to map `req` to `C.luma_query`.

	// What about writes?
	// If it's a write, it MUST go through Raft (`node.Apply`).
	// `luma_query` might handle writes if `luma_core` handles them, but Raft needs to sequence it.
	// This is the tricky part. `go-cluster` manages consensus.
	// If the query is "INSERT ...", `go-cluster` needs to know it's a write.

	// Short-term solution:
	// 1. If we can distinguish Read/Write, we route accordingly.
	// 2. If it's a write, we create a Raft command.

	// For now, let's support "mql" dialect which maps to `RunQuery` (Read)
	// and special commands for Write.
	// Or, assume `Execute` is Read-Only unless specified? No.

	// As per `node.go`:
	// `RunQuery` -> DB.Query.
	// `InsertDocument` -> Raft -> DB.Insert.

	// If `lumadb-compat` translates "INSERT INTO users ..." to LumaIR "Insert { ... }",
	// We should send a `QueryRequest` with payload defining the Insert.

	// Let's implement a naive pass-through to `RunQuery` for now, assuming read-mostly validation.
	// For writes, `lumadb-compat` might need to use `req.Payload` to specify "Type: Write".

	// Implementation:
	// Try to execute via Node.

	// Detect collection
	collection := req.Collection
	if collection == "" {
		// Parse from query? Or error.
		// Let's require collection for now if possible.
		// Only some queries leverage collection directly.
	}

	// Construct DB Query
	// We just pass the query string to the underlying engine.
	// Limitation: Writes won't be replicated if we just use `RunQuery`.
	// We need a way to support writes via this API.

	// TODO: Protocol should explicitly support Write ops.
	// For "Execute", we will assume it *could* be a write if we had a better parser.
	// Let's look at `req.Dialect`.

	results, err := s.node.RunQuery(req.Collection, map[string]interface{}{
		"q":       req.Query,
		"dialect": req.Dialect,
	})

	if err != nil {
		return &pb.QueryResponse{
			Success: false,
			Error:   err.Error(),
		}, nil
	}

	// Serialize results
	resBytes, _ := json.Marshal(results)

	return &pb.QueryResponse{
		Success:     true,
		Result:      resBytes,
		ContentType: "json",
	}, nil
}

func (s *LumaGRPCServer) Stream(req *pb.QueryRequest, stream pb.LumaService_StreamServer) error {
	return status.Errorf(codes.Unimplemented, "Stream not implemented")
}
