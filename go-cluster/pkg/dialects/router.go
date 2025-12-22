// Package dialects provides multi-dialect query support for LumaDB
//
// This package implements HTTP routers and handlers for accepting queries
// in various time-series database dialects (InfluxQL, Flux, PromQL, etc.)
// and routing them to the unified query execution engine.
package dialects

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"sync"
	"time"
)

// Dialect represents a supported query dialect
type Dialect string

const (
	DialectInfluxQL   Dialect = "influxql"
	DialectFlux       Dialect = "flux"
	DialectPromQL     Dialect = "promql"
	DialectMetricsQL  Dialect = "metricsql"
	DialectTDengine   Dialect = "tdengine"
	DialectTimescale  Dialect = "timescaledb"
	DialectQuestDB    Dialect = "questdb"
	DialectClickHouse Dialect = "clickhouse"
	DialectDruidSQL   Dialect = "druidsql"
	DialectDruidJSON  Dialect = "druidnative"
	DialectOpenTSDB   Dialect = "opentsdb"
	DialectGraphite   Dialect = "graphite"
	DialectSQL        Dialect = "sql"
)

// QueryRequest represents an incoming query request
type QueryRequest struct {
	Query    string            `json:"query"`
	Database string            `json:"db,omitempty"`
	Start    *time.Time        `json:"start,omitempty"`
	End      *time.Time        `json:"end,omitempty"`
	Step     string            `json:"step,omitempty"`
	Timeout  string            `json:"timeout,omitempty"`
	Format   string            `json:"format,omitempty"`
	Params   map[string]string `json:"params,omitempty"`
}

// QueryResponse represents a query response
type QueryResponse struct {
	Status     string          `json:"status"`
	Data       interface{}     `json:"data,omitempty"`
	Error      string          `json:"error,omitempty"`
	ErrorType  string          `json:"errorType,omitempty"`
	Warnings   []string        `json:"warnings,omitempty"`
	Stats      *ExecutionStats `json:"stats,omitempty"`
	ResultType string          `json:"resultType,omitempty"`
}

// ExecutionStats contains query execution statistics
type ExecutionStats struct {
	ExecutionTimeMs float64 `json:"executionTimeMs"`
	RowsScanned     int64   `json:"rowsScanned"`
	BytesScanned    int64   `json:"bytesScanned"`
}

// Router handles multi-dialect query routing
type Router struct {
	mu       sync.RWMutex
	handlers map[Dialect]DialectHandler
	detector *DialectDetector
	executor QueryExecutor
}

// DialectHandler processes queries for a specific dialect
type DialectHandler interface {
	// Parse parses the query and returns a normalized form
	Parse(query string) (*ParsedQuery, error)
	// FormatResponse formats the result for the dialect's expected response format
	FormatResponse(result *QueryResult, format string) (interface{}, error)
	// Dialect returns the dialect this handler processes
	Dialect() Dialect
}

// QueryExecutor executes parsed queries
type QueryExecutor interface {
	Execute(query *ParsedQuery, opts *ExecuteOptions) (*QueryResult, error)
}

// ParsedQuery represents a parsed and normalized query
type ParsedQuery struct {
	Dialect       Dialect
	OriginalQuery string
	Database      string
	Sources       []DataSource
	TimeRange     *TimeRange
	Filters       []Filter
	Aggregations  []Aggregation
	GroupBy       []string
	OrderBy       []OrderBy
	Limit         int64
	Offset        int64
}

// DataSource represents a data source (table, metric, measurement)
type DataSource struct {
	Name     string
	Database string
	Alias    string
}

// TimeRange represents a time range for the query
type TimeRange struct {
	Start    time.Time
	End      time.Time
	Duration time.Duration
}

// Filter represents a filter condition
type Filter struct {
	Column   string
	Operator string
	Value    interface{}
}

// Aggregation represents an aggregation operation
type Aggregation struct {
	Function string
	Column   string
	Alias    string
}

// OrderBy represents an ordering specification
type OrderBy struct {
	Column    string
	Ascending bool
}

// QueryResult represents the result of a query execution
type QueryResult struct {
	Columns []ColumnMeta
	Rows    [][]interface{}
	Stats   ExecutionStats
}

// ColumnMeta represents column metadata
type ColumnMeta struct {
	Name   string
	Type   string
	IsTag  bool
	IsTime bool
}

// ExecuteOptions contains query execution options
type ExecuteOptions struct {
	Timeout  time.Duration
	Database string
	Step     time.Duration
}

// NewRouter creates a new dialect router
func NewRouter(executor QueryExecutor) *Router {
	r := &Router{
		handlers: make(map[Dialect]DialectHandler),
		detector: NewDialectDetector(),
		executor: executor,
	}

	// Register default handlers
	r.RegisterHandler(&InfluxQLHandler{})
	r.RegisterHandler(&FluxHandler{})
	r.RegisterHandler(&PromQLHandler{})
	r.RegisterHandler(&SQLHandler{})

	return r
}

// RegisterHandler registers a dialect handler
func (r *Router) RegisterHandler(handler DialectHandler) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.handlers[handler.Dialect()] = handler
}

// GetHandler returns the handler for a dialect
func (r *Router) GetHandler(dialect Dialect) (DialectHandler, bool) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	h, ok := r.handlers[dialect]
	return h, ok
}

// ServeHTTP implements http.Handler for the router
func (r *Router) ServeHTTP(w http.ResponseWriter, req *http.Request) {
	// Route based on URL path
	path := req.URL.Path

	switch {
	case strings.HasPrefix(path, "/api/v1/query"):
		// PromQL-style endpoint
		r.handlePromQL(w, req)
	case strings.HasPrefix(path, "/query"):
		// InfluxQL-style endpoint
		r.handleInfluxQL(w, req)
	case strings.HasPrefix(path, "/api/v2/query"):
		// Flux-style endpoint
		r.handleFlux(w, req)
	case strings.HasPrefix(path, "/druid/v2"):
		// Druid-style endpoint
		r.handleDruid(w, req)
	case strings.HasPrefix(path, "/api/query"):
		// OpenTSDB-style endpoint
		r.handleOpenTSDB(w, req)
	case strings.HasPrefix(path, "/render"):
		// Graphite-style endpoint
		r.handleGraphite(w, req)
	case strings.HasPrefix(path, "/exec"):
		// QuestDB-style endpoint
		r.handleQuestDB(w, req)
	case strings.HasPrefix(path, "/dialect/auto"):
		// Auto-detect and execute
		r.handleAutoDetect(w, req)
	default:
		// Generic SQL endpoint
		r.handleSQL(w, req)
	}
}

// handlePromQL handles Prometheus-style queries
func (r *Router) handlePromQL(w http.ResponseWriter, req *http.Request) {
	query := req.URL.Query().Get("query")
	if query == "" && req.Method == "POST" {
		if err := req.ParseForm(); err == nil {
			query = req.FormValue("query")
		}
	}

	if query == "" {
		r.writeError(w, http.StatusBadRequest, "missing query parameter", "bad_data")
		return
	}

	handler, ok := r.GetHandler(DialectPromQL)
	if !ok {
		r.writeError(w, http.StatusInternalServerError, "PromQL handler not registered", "internal")
		return
	}

	parsed, err := handler.Parse(query)
	if err != nil {
		r.writeError(w, http.StatusBadRequest, err.Error(), "bad_data")
		return
	}

	// Parse time parameters
	opts := &ExecuteOptions{}
	if t := req.URL.Query().Get("time"); t != "" {
		// Parse instant query time
	}
	if start := req.URL.Query().Get("start"); start != "" {
		// Parse range start
	}
	if end := req.URL.Query().Get("end"); end != "" {
		// Parse range end
	}
	if step := req.URL.Query().Get("step"); step != "" {
		if d, err := time.ParseDuration(step); err == nil {
			opts.Step = d
		}
	}
	if timeout := req.URL.Query().Get("timeout"); timeout != "" {
		if d, err := time.ParseDuration(timeout); err == nil {
			opts.Timeout = d
		}
	}

	result, err := r.executor.Execute(parsed, opts)
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "execution")
		return
	}

	response, err := handler.FormatResponse(result, "prometheus")
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "internal")
		return
	}

	r.writeJSON(w, http.StatusOK, QueryResponse{
		Status: "success",
		Data:   response,
	})
}

// handleInfluxQL handles InfluxDB 1.x style queries
func (r *Router) handleInfluxQL(w http.ResponseWriter, req *http.Request) {
	query := req.URL.Query().Get("q")
	if query == "" && req.Method == "POST" {
		body, _ := io.ReadAll(req.Body)
		query = string(body)
	}

	if query == "" {
		r.writeError(w, http.StatusBadRequest, "missing query parameter", "bad_request")
		return
	}

	handler, ok := r.GetHandler(DialectInfluxQL)
	if !ok {
		r.writeError(w, http.StatusInternalServerError, "InfluxQL handler not registered", "internal")
		return
	}

	parsed, err := handler.Parse(query)
	if err != nil {
		r.writeError(w, http.StatusBadRequest, err.Error(), "bad_request")
		return
	}

	opts := &ExecuteOptions{
		Database: req.URL.Query().Get("db"),
	}

	result, err := r.executor.Execute(parsed, opts)
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "execution")
		return
	}

	response, err := handler.FormatResponse(result, "influxdb")
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "internal")
		return
	}

	r.writeJSON(w, http.StatusOK, response)
}

// handleFlux handles InfluxDB 2.x/3.x Flux queries
func (r *Router) handleFlux(w http.ResponseWriter, req *http.Request) {
	body, err := io.ReadAll(req.Body)
	if err != nil {
		r.writeError(w, http.StatusBadRequest, "failed to read request body", "bad_request")
		return
	}

	var fluxReq struct {
		Query   string `json:"query"`
		Dialect struct {
			Annotations []string `json:"annotations"`
		} `json:"dialect"`
	}

	if err := json.Unmarshal(body, &fluxReq); err != nil {
		// Try plain text
		fluxReq.Query = string(body)
	}

	handler, ok := r.GetHandler(DialectFlux)
	if !ok {
		r.writeError(w, http.StatusInternalServerError, "Flux handler not registered", "internal")
		return
	}

	parsed, err := handler.Parse(fluxReq.Query)
	if err != nil {
		r.writeError(w, http.StatusBadRequest, err.Error(), "bad_request")
		return
	}

	opts := &ExecuteOptions{}
	if org := req.Header.Get("X-Org"); org != "" {
		opts.Database = org
	}

	result, err := r.executor.Execute(parsed, opts)
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "execution")
		return
	}

	// Flux returns CSV by default
	response, err := handler.FormatResponse(result, "csv")
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "internal")
		return
	}

	w.Header().Set("Content-Type", "text/csv; charset=utf-8")
	fmt.Fprint(w, response)
}

// handleDruid handles Apache Druid queries
func (r *Router) handleDruid(w http.ResponseWriter, req *http.Request) {
	body, err := io.ReadAll(req.Body)
	if err != nil {
		r.writeError(w, http.StatusBadRequest, "failed to read request body", "bad_request")
		return
	}

	// Druid uses JSON native queries
	handler, ok := r.GetHandler(DialectDruidJSON)
	if !ok {
		// Fallback to SQL handler
		handler, ok = r.GetHandler(DialectSQL)
		if !ok {
			r.writeError(w, http.StatusInternalServerError, "Druid handler not registered", "internal")
			return
		}
	}

	parsed, err := handler.Parse(string(body))
	if err != nil {
		r.writeError(w, http.StatusBadRequest, err.Error(), "bad_request")
		return
	}

	opts := &ExecuteOptions{}
	result, err := r.executor.Execute(parsed, opts)
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "execution")
		return
	}

	response, err := handler.FormatResponse(result, "druid")
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "internal")
		return
	}

	r.writeJSON(w, http.StatusOK, response)
}

// handleOpenTSDB handles OpenTSDB queries
func (r *Router) handleOpenTSDB(w http.ResponseWriter, req *http.Request) {
	body, err := io.ReadAll(req.Body)
	if err != nil {
		r.writeError(w, http.StatusBadRequest, "failed to read request body", "bad_request")
		return
	}

	handler, ok := r.GetHandler(DialectOpenTSDB)
	if !ok {
		r.writeError(w, http.StatusInternalServerError, "OpenTSDB handler not registered", "internal")
		return
	}

	parsed, err := handler.Parse(string(body))
	if err != nil {
		r.writeError(w, http.StatusBadRequest, err.Error(), "bad_request")
		return
	}

	opts := &ExecuteOptions{}
	result, err := r.executor.Execute(parsed, opts)
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "execution")
		return
	}

	response, err := handler.FormatResponse(result, "opentsdb")
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "internal")
		return
	}

	r.writeJSON(w, http.StatusOK, response)
}

// handleGraphite handles Graphite render API queries
func (r *Router) handleGraphite(w http.ResponseWriter, req *http.Request) {
	target := req.URL.Query().Get("target")
	if target == "" && req.Method == "POST" {
		if err := req.ParseForm(); err == nil {
			target = req.FormValue("target")
		}
	}

	if target == "" {
		r.writeError(w, http.StatusBadRequest, "missing target parameter", "bad_request")
		return
	}

	handler, ok := r.GetHandler(DialectGraphite)
	if !ok {
		r.writeError(w, http.StatusInternalServerError, "Graphite handler not registered", "internal")
		return
	}

	parsed, err := handler.Parse(target)
	if err != nil {
		r.writeError(w, http.StatusBadRequest, err.Error(), "bad_request")
		return
	}

	opts := &ExecuteOptions{}
	// Parse from/until parameters
	if from := req.URL.Query().Get("from"); from != "" {
		// Parse graphite time format
	}
	if until := req.URL.Query().Get("until"); until != "" {
		// Parse graphite time format
	}

	result, err := r.executor.Execute(parsed, opts)
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "execution")
		return
	}

	format := req.URL.Query().Get("format")
	if format == "" {
		format = "json"
	}

	response, err := handler.FormatResponse(result, format)
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "internal")
		return
	}

	r.writeJSON(w, http.StatusOK, response)
}

// handleQuestDB handles QuestDB-style queries
func (r *Router) handleQuestDB(w http.ResponseWriter, req *http.Request) {
	query := req.URL.Query().Get("query")
	if query == "" && req.Method == "POST" {
		body, _ := io.ReadAll(req.Body)
		query = string(body)
	}

	if query == "" {
		r.writeError(w, http.StatusBadRequest, "missing query parameter", "bad_request")
		return
	}

	handler, ok := r.GetHandler(DialectQuestDB)
	if !ok {
		// Fallback to SQL
		handler, ok = r.GetHandler(DialectSQL)
		if !ok {
			r.writeError(w, http.StatusInternalServerError, "QuestDB handler not registered", "internal")
			return
		}
	}

	parsed, err := handler.Parse(query)
	if err != nil {
		r.writeError(w, http.StatusBadRequest, err.Error(), "bad_request")
		return
	}

	opts := &ExecuteOptions{}
	result, err := r.executor.Execute(parsed, opts)
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "execution")
		return
	}

	response, err := handler.FormatResponse(result, "questdb")
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "internal")
		return
	}

	r.writeJSON(w, http.StatusOK, response)
}

// handleSQL handles generic SQL queries
func (r *Router) handleSQL(w http.ResponseWriter, req *http.Request) {
	query := req.URL.Query().Get("query")
	if query == "" && req.Method == "POST" {
		body, _ := io.ReadAll(req.Body)

		// Try to parse as JSON first
		var jsonReq struct {
			Query string `json:"query"`
		}
		if err := json.Unmarshal(body, &jsonReq); err == nil && jsonReq.Query != "" {
			query = jsonReq.Query
		} else {
			query = string(body)
		}
	}

	if query == "" {
		r.writeError(w, http.StatusBadRequest, "missing query parameter", "bad_request")
		return
	}

	handler, ok := r.GetHandler(DialectSQL)
	if !ok {
		r.writeError(w, http.StatusInternalServerError, "SQL handler not registered", "internal")
		return
	}

	parsed, err := handler.Parse(query)
	if err != nil {
		r.writeError(w, http.StatusBadRequest, err.Error(), "bad_request")
		return
	}

	opts := &ExecuteOptions{
		Database: req.URL.Query().Get("database"),
	}

	result, err := r.executor.Execute(parsed, opts)
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "execution")
		return
	}

	response, err := handler.FormatResponse(result, "json")
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "internal")
		return
	}

	r.writeJSON(w, http.StatusOK, response)
}

// handleAutoDetect auto-detects the dialect and executes
func (r *Router) handleAutoDetect(w http.ResponseWriter, req *http.Request) {
	body, err := io.ReadAll(req.Body)
	if err != nil {
		r.writeError(w, http.StatusBadRequest, "failed to read request body", "bad_request")
		return
	}

	var queryReq QueryRequest
	if err := json.Unmarshal(body, &queryReq); err != nil {
		// Try plain text
		queryReq.Query = string(body)
	}

	if queryReq.Query == "" {
		r.writeError(w, http.StatusBadRequest, "missing query", "bad_request")
		return
	}

	// Detect dialect
	dialect, confidence := r.detector.Detect(queryReq.Query)

	handler, ok := r.GetHandler(dialect)
	if !ok {
		r.writeError(w, http.StatusInternalServerError,
			fmt.Sprintf("no handler for detected dialect: %s", dialect), "internal")
		return
	}

	parsed, err := handler.Parse(queryReq.Query)
	if err != nil {
		r.writeError(w, http.StatusBadRequest, err.Error(), "bad_request")
		return
	}

	opts := &ExecuteOptions{
		Database: queryReq.Database,
	}

	result, err := r.executor.Execute(parsed, opts)
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "execution")
		return
	}

	targetFormat := queryReq.Format
	if targetFormat == "" {
		targetFormat = "json"
	}

	response, err := handler.FormatResponse(result, targetFormat)
	if err != nil {
		r.writeError(w, http.StatusInternalServerError, err.Error(), "internal")
		return
	}

	r.writeJSON(w, http.StatusOK, map[string]interface{}{
		"status":           "success",
		"detected_dialect": dialect,
		"confidence":       confidence,
		"data":             response,
	})
}

// writeJSON writes a JSON response
func (r *Router) writeJSON(w http.ResponseWriter, status int, data interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(data)
}

// writeError writes an error response
func (r *Router) writeError(w http.ResponseWriter, status int, message, errorType string) {
	r.writeJSON(w, status, QueryResponse{
		Status:    "error",
		Error:     message,
		ErrorType: errorType,
	})
}
