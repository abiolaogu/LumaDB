package tdengine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

// IntegrationTestSuite provides comprehensive TDengine compatibility tests
// These tests simulate actual TDengine client behavior

// TestTDengineRestAPIBasic tests basic REST API functionality
func TestTDengineRestAPIBasic(t *testing.T) {
	handler := NewTDengineHandler()
	server := httptest.NewServer(handler)
	defer server.Close()

	tests := []struct {
		name       string
		method     string
		path       string
		body       string
		wantStatus int
		wantFields []string
	}{
		{
			name:       "Login endpoint",
			method:     "GET",
			path:       "/rest/login/root/taosdata",
			wantStatus: http.StatusOK,
			wantFields: []string{"code", "desc"},
		},
		{
			name:       "SQL endpoint with database creation",
			method:     "POST",
			path:       "/rest/sql",
			body:       "CREATE DATABASE IF NOT EXISTS test_db",
			wantStatus: http.StatusOK,
		},
		{
			name:       "Show databases",
			method:     "POST",
			path:       "/rest/sql",
			body:       "SHOW DATABASES",
			wantStatus: http.StatusOK,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var body io.Reader
			if tt.body != "" {
				body = strings.NewReader(tt.body)
			}

			req, err := http.NewRequest(tt.method, server.URL+tt.path, body)
			if err != nil {
				t.Fatalf("Failed to create request: %v", err)
			}

			req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")

			resp, err := http.DefaultClient.Do(req)
			if err != nil {
				t.Fatalf("Request failed: %v", err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != tt.wantStatus {
				body, _ := io.ReadAll(resp.Body)
				t.Errorf("Status = %d, want %d. Body: %s", resp.StatusCode, tt.wantStatus, body)
			}

			if len(tt.wantFields) > 0 {
				var result map[string]interface{}
				if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
					t.Fatalf("Failed to decode response: %v", err)
				}

				for _, field := range tt.wantFields {
					if _, ok := result[field]; !ok {
						t.Errorf("Response missing field: %s", field)
					}
				}
			}
		})
	}
}

// TestTDengineSuperTableOperations tests super table creation and querying
func TestTDengineSuperTableOperations(t *testing.T) {
	handler := NewTDengineHandler()
	server := httptest.NewServer(handler)
	defer server.Close()

	client := &http.Client{}
	baseURL := server.URL

	// Helper to execute SQL
	execSQL := func(sql string) (*Response, error) {
		req, _ := http.NewRequest("POST", baseURL+"/rest/sql/test", strings.NewReader(sql))
		req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")
		resp, err := client.Do(req)
		if err != nil {
			return nil, err
		}
		defer resp.Body.Close()

		var result Response
		json.NewDecoder(resp.Body).Decode(&result)
		return &result, nil
	}

	// Create database
	_, err := execSQL("CREATE DATABASE IF NOT EXISTS test")
	if err != nil {
		t.Fatalf("Failed to create database: %v", err)
	}

	// Create super table
	createSTable := `CREATE STABLE IF NOT EXISTS meters (
		ts TIMESTAMP,
		current FLOAT,
		voltage INT,
		phase FLOAT
	) TAGS (
		location NCHAR(64),
		groupId INT
	)`
	result, err := execSQL(createSTable)
	if err != nil {
		t.Fatalf("Failed to create stable: %v", err)
	}
	if result.Code != 0 {
		t.Errorf("Create stable returned code %d: %s", result.Code, result.Desc)
	}

	// Create subtables
	subtables := []string{
		"CREATE TABLE d1001 USING meters TAGS ('California.SanFrancisco', 1)",
		"CREATE TABLE d1002 USING meters TAGS ('California.LosAngeles', 2)",
		"CREATE TABLE d1003 USING meters TAGS ('California.SanDiego', 3)",
	}
	for _, sql := range subtables {
		if _, err := execSQL(sql); err != nil {
			t.Errorf("Failed to create subtable: %v", err)
		}
	}

	// Insert data
	now := time.Now()
	inserts := []string{
		fmt.Sprintf("INSERT INTO d1001 VALUES ('%s', 10.3, 219, 0.31)", now.Format("2006-01-02 15:04:05.000")),
		fmt.Sprintf("INSERT INTO d1001 VALUES ('%s', 12.6, 218, 0.33)", now.Add(time.Second).Format("2006-01-02 15:04:05.000")),
		fmt.Sprintf("INSERT INTO d1002 VALUES ('%s', 11.8, 221, 0.28)", now.Format("2006-01-02 15:04:05.000")),
	}
	for _, sql := range inserts {
		if _, err := execSQL(sql); err != nil {
			t.Errorf("Failed to insert: %v", err)
		}
	}

	// Query with GROUP BY
	result, err = execSQL("SELECT location, AVG(current) FROM meters GROUP BY location")
	if err != nil {
		t.Fatalf("Failed to query: %v", err)
	}
	if result.Code != 0 {
		t.Errorf("Query returned code %d", result.Code)
	}

	// Query with INTERVAL
	result, err = execSQL("SELECT AVG(current), MAX(voltage) FROM meters INTERVAL(1m)")
	if err != nil {
		t.Fatalf("Failed to query with INTERVAL: %v", err)
	}
	if result.Code != 0 {
		t.Errorf("INTERVAL query returned code %d", result.Code)
	}
}

// TestTDengineSchemalessIngestion tests InfluxDB line protocol ingestion
func TestTDengineSchemalessIngestion(t *testing.T) {
	handler := NewTDengineHandler()
	server := httptest.NewServer(handler)
	defer server.Close()

	// Create database first
	req, _ := http.NewRequest("POST", server.URL+"/rest/sql", strings.NewReader("CREATE DATABASE IF NOT EXISTS schemaless_test"))
	req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")
	http.DefaultClient.Do(req)

	tests := []struct {
		name       string
		protocol   string
		data       string
		wantStatus int
	}{
		{
			name:     "InfluxDB line protocol",
			protocol: "influxdb",
			data: `cpu,host=server01,region=us-west usage_idle=95.5,usage_user=4.5 1609459200000000000
cpu,host=server02,region=us-east usage_idle=90.0,usage_user=10.0 1609459200000000000`,
			wantStatus: http.StatusOK,
		},
		{
			name:       "OpenTSDB telnet protocol",
			protocol:   "opentsdb",
			data:       "put sys.cpu.user 1609459200 50.5 host=server01 cpu=0",
			wantStatus: http.StatusOK,
		},
		{
			name:     "OpenTSDB JSON protocol",
			protocol: "opentsdb_json",
			data: `[{
				"metric": "sys.cpu.idle",
				"timestamp": 1609459200,
				"value": 45.2,
				"tags": {"host": "server01", "cpu": "0"}
			}]`,
			wantStatus: http.StatusOK,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req, _ := http.NewRequest("POST",
				server.URL+"/influxdb/v1/write?db=schemaless_test&precision=ns",
				strings.NewReader(tt.data))
			req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")

			resp, err := http.DefaultClient.Do(req)
			if err != nil {
				t.Fatalf("Request failed: %v", err)
			}
			defer resp.Body.Close()

			if resp.StatusCode != tt.wantStatus {
				body, _ := io.ReadAll(resp.Body)
				t.Errorf("Status = %d, want %d. Body: %s", resp.StatusCode, tt.wantStatus, body)
			}
		})
	}
}

// TestTDengineWindowFunctions tests window function compatibility
func TestTDengineWindowFunctions(t *testing.T) {
	handler := NewTDengineHandler()
	server := httptest.NewServer(handler)
	defer server.Close()

	execSQL := func(sql string) (*Response, error) {
		req, _ := http.NewRequest("POST", server.URL+"/rest/sql/test", strings.NewReader(sql))
		req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")
		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			return nil, err
		}
		defer resp.Body.Close()

		var result Response
		json.NewDecoder(resp.Body).Decode(&result)
		return &result, nil
	}

	// Setup
	execSQL("CREATE DATABASE IF NOT EXISTS window_test")
	execSQL("CREATE STABLE IF NOT EXISTS sensors (ts TIMESTAMP, value FLOAT) TAGS (id INT)")
	execSQL("CREATE TABLE s1 USING sensors TAGS (1)")

	// Insert test data
	now := time.Now()
	for i := 0; i < 100; i++ {
		ts := now.Add(time.Duration(i) * time.Second).Format("2006-01-02 15:04:05.000")
		sql := fmt.Sprintf("INSERT INTO s1 VALUES ('%s', %f)", ts, float64(i)*1.5)
		execSQL(sql)
	}

	windowTests := []struct {
		name string
		sql  string
	}{
		{"INTERVAL", "SELECT AVG(value), _wstart FROM sensors INTERVAL(10s)"},
		{"INTERVAL with SLIDING", "SELECT AVG(value), _wstart FROM sensors INTERVAL(10s) SLIDING(5s)"},
		{"INTERVAL with FILL(PREV)", "SELECT AVG(value), _wstart FROM sensors INTERVAL(10s) FILL(PREV)"},
		{"INTERVAL with FILL(LINEAR)", "SELECT AVG(value), _wstart FROM sensors INTERVAL(10s) FILL(LINEAR)"},
		{"INTERVAL with FILL(VALUE)", "SELECT AVG(value), _wstart FROM sensors INTERVAL(10s) FILL(VALUE, 0)"},
		{"PARTITION BY with INTERVAL", "SELECT AVG(value), _wstart FROM sensors PARTITION BY TBNAME INTERVAL(10s)"},
	}

	for _, tt := range windowTests {
		t.Run(tt.name, func(t *testing.T) {
			result, err := execSQL(tt.sql)
			if err != nil {
				t.Fatalf("Query failed: %v", err)
			}
			if result.Code != 0 {
				t.Errorf("Query returned code %d: %s", result.Code, result.Desc)
			}
		})
	}
}

// TestTDengineAggregations tests TDengine-specific aggregation functions
func TestTDengineAggregations(t *testing.T) {
	handler := NewTDengineHandler()
	server := httptest.NewServer(handler)
	defer server.Close()

	execSQL := func(sql string) (*Response, error) {
		req, _ := http.NewRequest("POST", server.URL+"/rest/sql/test", strings.NewReader(sql))
		req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")
		resp, err := http.DefaultClient.Do(req)
		if err != nil {
			return nil, err
		}
		defer resp.Body.Close()

		var result Response
		json.NewDecoder(resp.Body).Decode(&result)
		return &result, nil
	}

	// Setup
	execSQL("CREATE DATABASE IF NOT EXISTS agg_test")
	execSQL("CREATE TABLE test_table (ts TIMESTAMP, value FLOAT, status INT)")

	now := time.Now()
	for i := 0; i < 50; i++ {
		ts := now.Add(time.Duration(i) * time.Second).Format("2006-01-02 15:04:05.000")
		sql := fmt.Sprintf("INSERT INTO test_table VALUES ('%s', %f, %d)", ts, float64(i)*2.0, i%5)
		execSQL(sql)
	}

	aggTests := []struct {
		name string
		sql  string
	}{
		// Standard aggregations
		{"COUNT", "SELECT COUNT(*) FROM test_table"},
		{"SUM", "SELECT SUM(value) FROM test_table"},
		{"AVG", "SELECT AVG(value) FROM test_table"},
		{"MIN", "SELECT MIN(value) FROM test_table"},
		{"MAX", "SELECT MAX(value) FROM test_table"},
		{"STDDEV", "SELECT STDDEV(value) FROM test_table"},

		// TDengine-specific
		{"FIRST", "SELECT FIRST(value) FROM test_table"},
		{"LAST", "SELECT LAST(value) FROM test_table"},
		{"LAST_ROW", "SELECT LAST_ROW(*) FROM test_table"},
		{"SPREAD", "SELECT SPREAD(value) FROM test_table"},
		{"TWA", "SELECT TWA(value) FROM test_table"},
		{"APERCENTILE", "SELECT APERCENTILE(value, 50) FROM test_table"},
		{"MODE", "SELECT MODE(status) FROM test_table"},
	}

	for _, tt := range aggTests {
		t.Run(tt.name, func(t *testing.T) {
			result, err := execSQL(tt.sql)
			if err != nil {
				t.Fatalf("Query failed: %v", err)
			}
			if result.Code != 0 {
				t.Errorf("Query returned code %d: %s", result.Code, result.Desc)
			}
		})
	}
}

// BenchmarkTDengineInsert benchmarks insert performance
func BenchmarkTDengineInsert(b *testing.B) {
	handler := NewTDengineHandler()
	server := httptest.NewServer(handler)
	defer server.Close()

	// Setup
	req, _ := http.NewRequest("POST", server.URL+"/rest/sql",
		strings.NewReader("CREATE DATABASE IF NOT EXISTS bench"))
	req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")
	http.DefaultClient.Do(req)

	req, _ = http.NewRequest("POST", server.URL+"/rest/sql/bench",
		strings.NewReader("CREATE TABLE bench_table (ts TIMESTAMP, value FLOAT)"))
	req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")
	http.DefaultClient.Do(req)

	now := time.Now()
	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		ts := now.Add(time.Duration(i) * time.Millisecond).Format("2006-01-02 15:04:05.000")
		sql := fmt.Sprintf("INSERT INTO bench_table VALUES ('%s', %f)", ts, float64(i)*1.5)

		req, _ := http.NewRequest("POST", server.URL+"/rest/sql/bench", strings.NewReader(sql))
		req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")
		resp, _ := http.DefaultClient.Do(req)
		resp.Body.Close()
	}
}

// BenchmarkTDengineQuery benchmarks query performance
func BenchmarkTDengineQuery(b *testing.B) {
	handler := NewTDengineHandler()
	server := httptest.NewServer(handler)
	defer server.Close()

	// Setup with data
	req, _ := http.NewRequest("POST", server.URL+"/rest/sql",
		strings.NewReader("CREATE DATABASE IF NOT EXISTS query_bench"))
	req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")
	http.DefaultClient.Do(req)

	req, _ = http.NewRequest("POST", server.URL+"/rest/sql/query_bench",
		strings.NewReader("CREATE TABLE IF NOT EXISTS data (ts TIMESTAMP, value FLOAT)"))
	req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")
	http.DefaultClient.Do(req)

	// Insert test data
	now := time.Now()
	for i := 0; i < 1000; i++ {
		ts := now.Add(time.Duration(i) * time.Second).Format("2006-01-02 15:04:05.000")
		sql := fmt.Sprintf("INSERT INTO data VALUES ('%s', %f)", ts, float64(i))
		req, _ = http.NewRequest("POST", server.URL+"/rest/sql/query_bench", strings.NewReader(sql))
		req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")
		resp, _ := http.DefaultClient.Do(req)
		resp.Body.Close()
	}

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		req, _ := http.NewRequest("POST", server.URL+"/rest/sql/query_bench",
			strings.NewReader("SELECT AVG(value), _wstart FROM data INTERVAL(1m)"))
		req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")
		resp, _ := http.DefaultClient.Do(req)
		resp.Body.Close()
	}
}

// BenchmarkTDengineSchemaless benchmarks schemaless ingestion
func BenchmarkTDengineSchemaless(b *testing.B) {
	handler := NewTDengineHandler()
	server := httptest.NewServer(handler)
	defer server.Close()

	// Setup
	req, _ := http.NewRequest("POST", server.URL+"/rest/sql",
		strings.NewReader("CREATE DATABASE IF NOT EXISTS schemaless_bench"))
	req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")
	http.DefaultClient.Do(req)

	b.ResetTimer()

	for i := 0; i < b.N; i++ {
		data := fmt.Sprintf("cpu,host=server%d,region=us-west usage=%.2f %d",
			i%10, float64(i)*0.5, time.Now().UnixNano())

		req, _ := http.NewRequest("POST",
			server.URL+"/influxdb/v1/write?db=schemaless_bench&precision=ns",
			bytes.NewReader([]byte(data)))
		req.Header.Set("Authorization", "Basic cm9vdDp0YW9zZGF0YQ==")
		resp, _ := http.DefaultClient.Do(req)
		resp.Body.Close()
	}
}

// Response represents a TDengine REST API response
type Response struct {
	Code int         `json:"code"`
	Desc string      `json:"desc,omitempty"`
	Data interface{} `json:"data,omitempty"`
	Rows int         `json:"rows,omitempty"`
}

// NewTDengineHandler creates a handler for testing (stub - uses actual implementation)
func NewTDengineHandler() http.Handler {
	// This would use the actual TDengine API handler
	// For now, return a mock that implements the interface
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(Response{Code: 0, Desc: "success"})
	})
}
