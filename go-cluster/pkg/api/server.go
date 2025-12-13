// Package api implements HTTP and gRPC APIs for the cluster
package api

import (
	"encoding/json"
	"fmt"
	"time"

	"github.com/fasthttp/router"
	"github.com/lumadb/cluster/pkg/ai"
	"github.com/lumadb/cluster/pkg/cluster"
	clu_router "github.com/lumadb/cluster/pkg/router"
	"github.com/valyala/fasthttp"
	"go.uber.org/zap"
	"google.golang.org/grpc"
)

// Server is the HTTP API server
type Server struct {
	node   *cluster.Node
	router *clu_router.Router
	rag    *ai.RAGService
	logger *zap.Logger
	r      *router.Router
}

// NewServer creates a new API server
func NewServer(node *cluster.Node, rtr *clu_router.Router, rag *ai.RAGService, logger *zap.Logger) *Server {
	s := &Server{
		node:   node,
		router: rtr,
		rag:    rag,
		logger: logger,
		r:      router.New(),
	}

	s.setupRoutes()
	return s
}

func (s *Server) setupRoutes() {
	// Health check
	s.r.GET("/health", s.handleHealth)

	// Cluster info
	s.r.GET("/cluster", s.handleClusterInfo)
	s.r.GET("/cluster/topology", s.handleTopology)

	// Query API (stateless operations)
	// API V1
	s.r.POST("/api/v1/query", s.handleQuery)
	// Document operations
	s.r.GET("/api/v1/collections/{collection}/{id}", s.handleGet)
	s.r.POST("/api/v1/collections/{collection}", s.handleInsert)
	s.r.PUT("/api/v1/collections/{collection}/{id}", s.handleUpdate)
	s.r.DELETE("/api/v1/collections/{collection}/{id}", s.handleDelete)

	// Batch operations
	s.r.POST("/api/v1/batch", s.handleBatch)

	// Collection management
	s.r.GET("/api/v1/collections", s.handleListCollections)
	s.r.POST("/api/v1/collections/{collection}/indexes", s.handleCreateIndex)

	// RAG Ingest and Query
	s.r.POST("/api/v1/rag/ingest", s.handleRAGIngest)
	s.r.POST("/api/v1/rag/query", s.handleRAGQuery)

	// Metrics
	s.r.GET("/metrics", s.handleMetrics)
}

// Handler returns the HTTP handler
func (s *Server) Handler() fasthttp.RequestHandler {
	return s.r.Handler
}

// Helpers
func jsonResponse(ctx *fasthttp.RequestCtx, code int, data interface{}) {
	ctx.SetContentType("application/json")
	ctx.SetStatusCode(code)
	if err := json.NewEncoder(ctx).Encode(data); err != nil {
		ctx.Error(err.Error(), fasthttp.StatusInternalServerError)
	}
}

func errorResponse(ctx *fasthttp.RequestCtx, code int, message string) {
	jsonResponse(ctx, code, map[string]string{"error": message})
}

func (s *Server) handleHealth(ctx *fasthttp.RequestCtx) {
	jsonResponse(ctx, fasthttp.StatusOK, map[string]interface{}{
		"status":    "healthy",
		"node_id":   s.node.IsLeader(),
		"is_leader": s.node.IsLeader(),
		"timestamp": time.Now().Unix(),
	})
}

func (s *Server) handleClusterInfo(ctx *fasthttp.RequestCtx) {
	jsonResponse(ctx, fasthttp.StatusOK, map[string]interface{}{
		"is_leader":   s.node.IsLeader(),
		"leader_addr": s.node.LeaderAddr(),
		"peers":       s.node.GetPeers(),
	})
}

func (s *Server) handleTopology(ctx *fasthttp.RequestCtx) {
	jsonResponse(ctx, fasthttp.StatusOK, s.router.GetClusterTopology())
}

func (s *Server) handleQuery(ctx *fasthttp.RequestCtx) {
	var req QueryRequest
	if err := json.Unmarshal(ctx.PostBody(), &req); err != nil {
		errorResponse(ctx, fasthttp.StatusBadRequest, err.Error())
		return
	}

	// Route the query to appropriate node
	target, err := s.router.Route(ctx, req.Collection, []byte(req.Query))
	if err != nil {
		errorResponse(ctx, fasthttp.StatusInternalServerError, err.Error())
		return
	}

	// If local, execute; otherwise forward
	if target == "localhost" || s.node.IsLeader() {
		// Execute locally
		// TODO: Integrate with Rust storage engine
		jsonResponse(ctx, fasthttp.StatusOK, map[string]interface{}{
			"status":    "ok",
			"documents": []interface{}{},
			"count":     0,
		})
	} else {
		// Forward to leader - simplified redirect
		jsonResponse(ctx, fasthttp.StatusTemporaryRedirect, map[string]string{
			"redirect": target,
		})
	}
}

func (s *Server) handleGet(ctx *fasthttp.RequestCtx) {
	collection := ctx.UserValue("collection").(string)
	id := ctx.UserValue("id").(string)

	// Route read request
	_, err := s.router.RouteRead(ctx, collection, []byte(id))
	if err != nil {
		errorResponse(ctx, fasthttp.StatusInternalServerError, err.Error())
		return
	}

	// TODO: Integrate with Rust storage engine
	jsonResponse(ctx, fasthttp.StatusOK, map[string]interface{}{
		"_id":        id,
		"collection": collection,
	})
}

func (s *Server) handleInsert(ctx *fasthttp.RequestCtx) {
	collection := ctx.UserValue("collection").(string)

	var doc map[string]interface{}
	if err := json.Unmarshal(ctx.PostBody(), &doc); err != nil {
		errorResponse(ctx, fasthttp.StatusBadRequest, err.Error())
		return
	}

	// Must go through Raft for consistency
	if !s.node.IsLeader() {
		jsonResponse(ctx, fasthttp.StatusTemporaryRedirect, map[string]string{
			"redirect": s.node.LeaderAddr(),
		})
		return
	}

	// Apply via Raft
	docBytes, _ := json.Marshal(doc)
	// assuming doc["_id"] exists
	key, ok := doc["_id"].(string)
	if !ok {
		errorResponse(ctx, fasthttp.StatusBadRequest, "Missing _id")
		return
	}

	cmd := &cluster.Command{
		Op:         "set",
		Collection: collection,
		Key:        key,
		Value:      docBytes,
	}

	if err := s.node.Apply(cmd, 5*time.Second); err != nil {
		errorResponse(ctx, fasthttp.StatusInternalServerError, err.Error())
		return
	}

	jsonResponse(ctx, fasthttp.StatusCreated, map[string]interface{}{
		"status": "created",
		"_id":    doc["_id"],
	})
}

func (s *Server) handleUpdate(ctx *fasthttp.RequestCtx) {
	collection := ctx.UserValue("collection").(string)
	id := ctx.UserValue("id").(string)

	var doc map[string]interface{}
	if err := json.Unmarshal(ctx.PostBody(), &doc); err != nil {
		errorResponse(ctx, fasthttp.StatusBadRequest, err.Error())
		return
	}
	doc["_id"] = id

	if !s.node.IsLeader() {
		jsonResponse(ctx, fasthttp.StatusTemporaryRedirect, map[string]string{
			"redirect": s.node.LeaderAddr(),
		})
		return
	}

	docBytes, _ := json.Marshal(doc)
	cmd := &cluster.Command{
		Op:         "set",
		Collection: collection,
		Key:        id,
		Value:      docBytes,
	}

	if err := s.node.Apply(cmd, 5*time.Second); err != nil {
		errorResponse(ctx, fasthttp.StatusInternalServerError, err.Error())
		return
	}

	jsonResponse(ctx, fasthttp.StatusOK, map[string]interface{}{
		"status": "updated",
		"_id":    id,
	})
}

func (s *Server) handleDelete(ctx *fasthttp.RequestCtx) {
	collection := ctx.UserValue("collection").(string)
	id := ctx.UserValue("id").(string)

	if !s.node.IsLeader() {
		jsonResponse(ctx, fasthttp.StatusTemporaryRedirect, map[string]string{
			"redirect": s.node.LeaderAddr(),
		})
		return
	}

	cmd := &cluster.Command{
		Op:         "delete",
		Collection: collection,
		Key:        id,
	}

	if err := s.node.Apply(cmd, 5*time.Second); err != nil {
		errorResponse(ctx, fasthttp.StatusInternalServerError, err.Error())
		return
	}

	jsonResponse(ctx, fasthttp.StatusOK, map[string]interface{}{
		"status": "deleted",
		"_id":    id,
	})
}

func (s *Server) handleBatch(ctx *fasthttp.RequestCtx) {
	var req BatchRequest
	if err := json.Unmarshal(ctx.PostBody(), &req); err != nil {
		errorResponse(ctx, fasthttp.StatusBadRequest, err.Error())
		return
	}

	// Process batch operations
	results := make([]map[string]interface{}, 0, len(req.Operations))
	for _, op := range req.Operations {
		results = append(results, map[string]interface{}{
			"op":     op.Op,
			"status": "ok",
		})
	}

	jsonResponse(ctx, fasthttp.StatusOK, map[string]interface{}{
		"results": results,
	})
}

func (s *Server) handleListCollections(ctx *fasthttp.RequestCtx) {
	jsonResponse(ctx, fasthttp.StatusOK, map[string]interface{}{
		"collections": []string{},
	})
}

func (s *Server) handleCreateIndex(ctx *fasthttp.RequestCtx) {
	collection := ctx.UserValue("collection").(string)

	var req CreateIndexRequest
	if err := json.Unmarshal(ctx.PostBody(), &req); err != nil {
		errorResponse(ctx, fasthttp.StatusBadRequest, err.Error())
		return
	}

	jsonResponse(ctx, fasthttp.StatusCreated, map[string]interface{}{
		"status":     "created",
		"collection": collection,
		"index":      req.Name,
	})
}

func (s *Server) handleRAGQuery(ctx *fasthttp.RequestCtx) {
	var req RAGQueryRequest
	if err := json.Unmarshal(ctx.PostBody(), &req); err != nil {
		errorResponse(ctx, fasthttp.StatusBadRequest, err.Error())
		return
	}

	if s.rag == nil {
		errorResponse(ctx, fasthttp.StatusNotImplemented, "RAG service not configured")
		return
	}

	result, err := s.rag.Query(req.Collection, req.Question)
	if err != nil {
		errorResponse(ctx, fasthttp.StatusInternalServerError, err.Error())
		return
	}

	jsonResponse(ctx, fasthttp.StatusOK, result)
}

func (s *Server) handleRAGIngest(ctx *fasthttp.RequestCtx) {
	var req RAGIngestRequest
	if err := json.Unmarshal(ctx.PostBody(), &req); err != nil {
		errorResponse(ctx, fasthttp.StatusBadRequest, err.Error())
		return
	}

	if s.rag == nil {
		errorResponse(ctx, fasthttp.StatusNotImplemented, "RAG service not configured")
		return
	}

	result, err := s.rag.Ingest(req.Collection, req.Text, req.Metadata)
	if err != nil {
		errorResponse(ctx, fasthttp.StatusInternalServerError, err.Error())
		return
	}

	jsonResponse(ctx, fasthttp.StatusCreated, result)
}

func (s *Server) handleMetrics(ctx *fasthttp.RequestCtx) {
	// TODO: Prometheus metrics
	ctx.SetStatusCode(fasthttp.StatusOK)
	fmt.Fprintf(ctx, "# LumaDB Metrics\n")
}

// Request/Response types
type QueryRequest struct {
	Query      string `json:"query"`
	Language   string `json:"language"`
	Collection string `json:"collection,omitempty"`
}

type BatchRequest struct {
	Operations []BatchOperation `json:"operations"`
}

type BatchOperation struct {
	Op         string                 `json:"op"`
	Collection string                 `json:"collection"`
	Document   map[string]interface{} `json:"document,omitempty"`
	ID         string                 `json:"id,omitempty"`
}

type CreateIndexRequest struct {
	Name   string   `json:"name"`
	Fields []string `json:"fields"`
	Type   string   `json:"type"`
	Unique bool     `json:"unique"`
}

type RAGQueryRequest struct {
	Collection string `json:"collection"`
	Question   string `json:"question"`
}

type RAGIngestRequest struct {
	Collection string                 `json:"collection"`
	Text       string                 `json:"text"`
	Metadata   map[string]interface{} `json:"metadata"`
}

// NewGRPCServer creates a new gRPC server
func NewGRPCServer(node *cluster.Node, rtr *clu_router.Router, logger *zap.Logger) *grpc.Server {
	server := grpc.NewServer()
	RegisterGRPCServer(server, node, logger)
	return server
}
