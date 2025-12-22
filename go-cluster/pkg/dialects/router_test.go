package dialects

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

// MockExecutor implements QueryExecutor for testing
type MockExecutor struct {
	Result *QueryResult
	Error  error
}

func (m *MockExecutor) Execute(query *ParsedQuery, opts *ExecuteOptions) (*QueryResult, error) {
	if m.Error != nil {
		return nil, m.Error
	}
	if m.Result != nil {
		return m.Result, nil
	}
	return &QueryResult{
		Columns: []ColumnMeta{
			{Name: "time", Type: "timestamp", IsTime: true},
			{Name: "value", Type: "float64"},
		},
		Rows: [][]interface{}{
			{1700000000, 42.5},
			{1700000060, 43.2},
		},
		Stats: ExecutionStats{
			ExecutionTimeMs: 5.2,
			RowsScanned:     100,
		},
	}, nil
}

func TestDialectDetector_PromQL(t *testing.T) {
	detector := NewDialectDetector()

	tests := []struct {
		query   string
		want    Dialect
		minConf float64
	}{
		{"rate(http_requests_total[5m])", DialectPromQL, 0.5},
		{"sum by (job) (rate(errors_total[1h]))", DialectPromQL, 0.5},
		{"http_requests_total{job=\"api\"}", DialectPromQL, 0.3},
		{"histogram_quantile(0.95, rate(request_duration_seconds_bucket[5m]))", DialectPromQL, 0.5},
	}

	for _, tt := range tests {
		dialect, confidence := detector.Detect(tt.query)
		if dialect != tt.want {
			t.Errorf("Detect(%q) dialect = %v, want %v", tt.query, dialect, tt.want)
		}
		if confidence < tt.minConf {
			t.Errorf("Detect(%q) confidence = %v, want >= %v", tt.query, confidence, tt.minConf)
		}
	}
}

func TestDialectDetector_InfluxQL(t *testing.T) {
	detector := NewDialectDetector()

	tests := []struct {
		query   string
		want    Dialect
		minConf float64
	}{
		{"SELECT mean(value) FROM cpu WHERE time > now() - 1h GROUP BY time(5m)", DialectInfluxQL, 0.5},
		{"SHOW MEASUREMENTS", DialectInfluxQL, 0.3},
		{"SELECT * FROM cpu GROUP BY time(1m) FILL(null)", DialectInfluxQL, 0.5},
	}

	for _, tt := range tests {
		dialect, confidence := detector.Detect(tt.query)
		if dialect != tt.want {
			t.Errorf("Detect(%q) dialect = %v, want %v", tt.query, dialect, tt.want)
		}
		if confidence < tt.minConf {
			t.Errorf("Detect(%q) confidence = %v, want >= %v", tt.query, confidence, tt.minConf)
		}
	}
}

func TestDialectDetector_Flux(t *testing.T) {
	detector := NewDialectDetector()

	tests := []struct {
		query   string
		want    Dialect
		minConf float64
	}{
		{`from(bucket: "test") |> range(start: -1h)`, DialectFlux, 0.5},
		{`from(bucket: "test") |> filter(fn: (r) => r._measurement == "cpu") |> aggregateWindow(every: 1m, fn: mean)`, DialectFlux, 0.5},
	}

	for _, tt := range tests {
		dialect, confidence := detector.Detect(tt.query)
		if dialect != tt.want {
			t.Errorf("Detect(%q) dialect = %v, want %v", tt.query, dialect, tt.want)
		}
		if confidence < tt.minConf {
			t.Errorf("Detect(%q) confidence = %v, want >= %v", tt.query, confidence, tt.minConf)
		}
	}
}

func TestDialectDetector_TDengine(t *testing.T) {
	detector := NewDialectDetector()

	tests := []struct {
		query   string
		want    Dialect
		minConf float64
	}{
		{"SELECT avg(value) FROM meters INTERVAL(10s)", DialectTDengine, 0.5},
		{"CREATE STABLE meters (ts TIMESTAMP, value FLOAT) TAGS (location NCHAR(20))", DialectTDengine, 0.5},
		{"SELECT LAST_ROW(*) FROM meters PARTITION BY TBNAME", DialectTDengine, 0.5},
	}

	for _, tt := range tests {
		dialect, confidence := detector.Detect(tt.query)
		if dialect != tt.want {
			t.Errorf("Detect(%q) dialect = %v, want %v", tt.query, dialect, tt.want)
		}
		if confidence < tt.minConf {
			t.Errorf("Detect(%q) confidence = %v, want >= %v", tt.query, confidence, tt.minConf)
		}
	}
}

func TestInfluxQLHandler_Parse(t *testing.T) {
	handler := &InfluxQLHandler{}

	query := `SELECT mean("value") FROM "cpu" WHERE time > now() - 1h GROUP BY time(5m) LIMIT 100`
	parsed, err := handler.Parse(query)
	if err != nil {
		t.Fatalf("Parse() error = %v", err)
	}

	if len(parsed.Sources) != 1 || parsed.Sources[0].Name != "cpu" {
		t.Errorf("Parse() sources = %v, want [{cpu}]", parsed.Sources)
	}

	if parsed.TimeRange == nil {
		t.Error("Parse() TimeRange is nil")
	}

	if len(parsed.Aggregations) != 1 || parsed.Aggregations[0].Function != "mean" {
		t.Errorf("Parse() aggregations = %v, want [mean]", parsed.Aggregations)
	}

	if parsed.Limit != 100 {
		t.Errorf("Parse() Limit = %v, want 100", parsed.Limit)
	}
}

func TestFluxHandler_Parse(t *testing.T) {
	handler := &FluxHandler{}

	query := `from(bucket: "my-bucket")
		|> range(start: -1h)
		|> filter(fn: (r) => r._measurement == "cpu")
		|> aggregateWindow(every: 5m, fn: mean)
		|> limit(n: 50)`

	parsed, err := handler.Parse(query)
	if err != nil {
		t.Fatalf("Parse() error = %v", err)
	}

	if parsed.Database != "my-bucket" {
		t.Errorf("Parse() Database = %v, want my-bucket", parsed.Database)
	}

	if parsed.TimeRange == nil {
		t.Error("Parse() TimeRange is nil")
	}

	if parsed.Limit != 50 {
		t.Errorf("Parse() Limit = %v, want 50", parsed.Limit)
	}
}

func TestPromQLHandler_Parse(t *testing.T) {
	handler := &PromQLHandler{}

	query := `rate(http_requests_total{job="api", status="200"}[5m])`
	parsed, err := handler.Parse(query)
	if err != nil {
		t.Fatalf("Parse() error = %v", err)
	}

	if len(parsed.Sources) != 1 || parsed.Sources[0].Name != "http_requests_total" {
		t.Errorf("Parse() sources = %v, want [{http_requests_total}]", parsed.Sources)
	}

	if len(parsed.Filters) != 2 {
		t.Errorf("Parse() filters count = %v, want 2", len(parsed.Filters))
	}

	if len(parsed.Aggregations) == 0 || parsed.Aggregations[0].Function != "rate" {
		t.Errorf("Parse() aggregations = %v, want [rate]", parsed.Aggregations)
	}
}

func TestRouter_PromQLEndpoint(t *testing.T) {
	executor := &MockExecutor{}
	router := NewRouter(executor)

	req := httptest.NewRequest("GET", "/api/v1/query?query=rate(http_requests_total[5m])", nil)
	w := httptest.NewRecorder()

	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("ServeHTTP() status = %v, want %v", w.Code, http.StatusOK)
	}

	var resp QueryResponse
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("Failed to decode response: %v", err)
	}

	if resp.Status != "success" {
		t.Errorf("Response status = %v, want success", resp.Status)
	}
}

func TestRouter_InfluxQLEndpoint(t *testing.T) {
	executor := &MockExecutor{}
	router := NewRouter(executor)

	req := httptest.NewRequest("GET", "/query?db=test&q=SELECT+mean(value)+FROM+cpu", nil)
	w := httptest.NewRecorder()

	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("ServeHTTP() status = %v, want %v", w.Code, http.StatusOK)
	}
}

func TestRouter_AutoDetectEndpoint(t *testing.T) {
	executor := &MockExecutor{}
	router := NewRouter(executor)

	body := `{"query": "rate(http_requests_total[5m])"}`
	req := httptest.NewRequest("POST", "/dialect/auto", bytes.NewBufferString(body))
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()

	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("ServeHTTP() status = %v, want %v", w.Code, http.StatusOK)
	}

	var resp map[string]interface{}
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("Failed to decode response: %v", err)
	}

	if resp["status"] != "success" {
		t.Errorf("Response status = %v, want success", resp["status"])
	}

	if resp["detected_dialect"] != string(DialectPromQL) {
		t.Errorf("Detected dialect = %v, want %v", resp["detected_dialect"], DialectPromQL)
	}
}

func TestRouter_MissingQuery(t *testing.T) {
	executor := &MockExecutor{}
	router := NewRouter(executor)

	req := httptest.NewRequest("GET", "/api/v1/query", nil)
	w := httptest.NewRecorder()

	router.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("ServeHTTP() status = %v, want %v", w.Code, http.StatusBadRequest)
	}

	var resp QueryResponse
	if err := json.NewDecoder(w.Body).Decode(&resp); err != nil {
		t.Fatalf("Failed to decode response: %v", err)
	}

	if resp.Status != "error" {
		t.Errorf("Response status = %v, want error", resp.Status)
	}
}
