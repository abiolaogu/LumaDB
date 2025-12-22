// Package tdengine provides TDengine-compatible REST API for LumaDB
package tdengine

import (
	"encoding/base64"
	"encoding/json"
	"io"
	"net/http"
	"strings"
	"time"
)

// API implements TDengine REST API handlers
type API struct {
	engine *Engine
}

// NewAPI creates a new TDengine API instance
func NewAPI(engine *Engine) *API {
	return &API{engine: engine}
}

// Register registers all TDengine API routes
func (a *API) Register(mux *http.ServeMux) {
	// REST SQL endpoint (primary)
	mux.HandleFunc("/rest/sql", a.SQL)
	mux.HandleFunc("/rest/sql/", a.SQLWithDB)

	// REST SQL with timing
	mux.HandleFunc("/rest/sqlt", a.SQLWithTiming)
	mux.HandleFunc("/rest/sqlt/", a.SQLWithTimingAndDB)

	// REST SQL with UTC
	mux.HandleFunc("/rest/sqlutc", a.SQLUTC)
	mux.HandleFunc("/rest/sqlutc/", a.SQLUTCWithDB)

	// InfluxDB line protocol
	mux.HandleFunc("/influxdb/v1/write", a.InfluxDBWrite)

	// OpenTSDB JSON
	mux.HandleFunc("/opentsdb/v1/put/json/", a.OpenTSDBJSON)

	// OpenTSDB Telnet
	mux.HandleFunc("/opentsdb/v1/put/telnet/", a.OpenTSDBTelnet)

	// Login for token
	mux.HandleFunc("/rest/login/", a.Login)

	// Health endpoints
	mux.HandleFunc("/health", a.Health)
	mux.HandleFunc("/ready", a.Ready)
}

// SQL handles /rest/sql endpoint
func (a *API) SQL(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	if !a.authenticate(r) {
		a.respondError(w, TSDB_CODE_TSC_AUTH_FAILURE, "Authentication failed")
		return
	}

	body, err := io.ReadAll(r.Body)
	if err != nil {
		a.respondError(w, TSDB_CODE_FAILED, err.Error())
		return
	}

	sql := string(body)

	// Get database from header or default
	db := r.Header.Get("X-TDengine-Database")

	opts := &ExecuteOptions{
		ReqID:    r.Header.Get("X-Request-ID"),
		Timezone: r.Header.Get("X-Timezone"),
	}

	result, err := a.engine.Execute(db, sql, opts)
	if err != nil {
		a.respondError(w, TSDB_CODE_FAILED, err.Error())
		return
	}

	a.respond(w, result)
}

// SQLWithDB handles /rest/sql/{db} endpoint
func (a *API) SQLWithDB(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	if !a.authenticate(r) {
		a.respondError(w, TSDB_CODE_TSC_AUTH_FAILURE, "Authentication failed")
		return
	}

	// Extract database from path
	db := strings.TrimPrefix(r.URL.Path, "/rest/sql/")

	body, err := io.ReadAll(r.Body)
	if err != nil {
		a.respondError(w, TSDB_CODE_FAILED, err.Error())
		return
	}

	sql := string(body)

	opts := &ExecuteOptions{
		ReqID:    r.Header.Get("X-Request-ID"),
		Timezone: r.Header.Get("X-Timezone"),
	}

	result, err := a.engine.Execute(db, sql, opts)
	if err != nil {
		a.respondError(w, TSDB_CODE_FAILED, err.Error())
		return
	}

	a.respond(w, result)
}

// SQLWithTiming handles /rest/sqlt endpoint with timing info
func (a *API) SQLWithTiming(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	if !a.authenticate(r) {
		a.respondError(w, TSDB_CODE_TSC_AUTH_FAILURE, "Authentication failed")
		return
	}

	start := time.Now()

	body, err := io.ReadAll(r.Body)
	if err != nil {
		a.respondError(w, TSDB_CODE_FAILED, err.Error())
		return
	}

	sql := string(body)
	db := r.Header.Get("X-TDengine-Database")

	opts := &ExecuteOptions{
		ReqID:    r.Header.Get("X-Request-ID"),
		Timezone: r.Header.Get("X-Timezone"),
	}

	result, err := a.engine.Execute(db, sql, opts)
	if err != nil {
		a.respondError(w, TSDB_CODE_FAILED, err.Error())
		return
	}

	timing := time.Since(start).Microseconds()

	timingResp := &TimingResponse{
		Response: *result,
		Timing:   timing,
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(timingResp)
}

// SQLWithTimingAndDB handles /rest/sqlt/{db} endpoint
func (a *API) SQLWithTimingAndDB(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	if !a.authenticate(r) {
		a.respondError(w, TSDB_CODE_TSC_AUTH_FAILURE, "Authentication failed")
		return
	}

	start := time.Now()

	db := strings.TrimPrefix(r.URL.Path, "/rest/sqlt/")

	body, err := io.ReadAll(r.Body)
	if err != nil {
		a.respondError(w, TSDB_CODE_FAILED, err.Error())
		return
	}

	sql := string(body)

	opts := &ExecuteOptions{
		ReqID:    r.Header.Get("X-Request-ID"),
		Timezone: r.Header.Get("X-Timezone"),
	}

	result, err := a.engine.Execute(db, sql, opts)
	if err != nil {
		a.respondError(w, TSDB_CODE_FAILED, err.Error())
		return
	}

	timing := time.Since(start).Microseconds()

	timingResp := &TimingResponse{
		Response: *result,
		Timing:   timing,
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(timingResp)
}

// SQLUTC handles /rest/sqlutc endpoint with UTC timestamps
func (a *API) SQLUTC(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	if !a.authenticate(r) {
		a.respondError(w, TSDB_CODE_TSC_AUTH_FAILURE, "Authentication failed")
		return
	}

	body, err := io.ReadAll(r.Body)
	if err != nil {
		a.respondError(w, TSDB_CODE_FAILED, err.Error())
		return
	}

	sql := string(body)
	db := r.Header.Get("X-TDengine-Database")

	opts := &ExecuteOptions{
		ReqID:    r.Header.Get("X-Request-ID"),
		Timezone: "UTC",
	}

	result, err := a.engine.Execute(db, sql, opts)
	if err != nil {
		a.respondError(w, TSDB_CODE_FAILED, err.Error())
		return
	}

	a.respond(w, result)
}

// SQLUTCWithDB handles /rest/sqlutc/{db} endpoint
func (a *API) SQLUTCWithDB(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	if !a.authenticate(r) {
		a.respondError(w, TSDB_CODE_TSC_AUTH_FAILURE, "Authentication failed")
		return
	}

	db := strings.TrimPrefix(r.URL.Path, "/rest/sqlutc/")

	body, err := io.ReadAll(r.Body)
	if err != nil {
		a.respondError(w, TSDB_CODE_FAILED, err.Error())
		return
	}

	sql := string(body)

	opts := &ExecuteOptions{
		ReqID:    r.Header.Get("X-Request-ID"),
		Timezone: "UTC",
	}

	result, err := a.engine.Execute(db, sql, opts)
	if err != nil {
		a.respondError(w, TSDB_CODE_FAILED, err.Error())
		return
	}

	a.respond(w, result)
}

// InfluxDBWrite handles InfluxDB line protocol writes
func (a *API) InfluxDBWrite(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	if !a.authenticate(r) {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	db := r.URL.Query().Get("db")
	if db == "" {
		http.Error(w, "Database required", http.StatusBadRequest)
		return
	}

	precision := r.URL.Query().Get("precision")
	if precision == "" {
		precision = "ns" // Default to nanoseconds
	}

	body, _ := io.ReadAll(r.Body)
	lines := strings.Split(string(body), "\n")

	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}

		parsed, err := parseInfluxDBLine(line, precision)
		if err != nil {
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}

		if err := a.engine.WriteInfluxDB(db, parsed); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
	}

	w.WriteHeader(http.StatusNoContent)
}

// OpenTSDBJSON handles OpenTSDB JSON protocol
func (a *API) OpenTSDBJSON(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	if !a.authenticate(r) {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	db := strings.TrimPrefix(r.URL.Path, "/opentsdb/v1/put/json/")

	var points []OpenTSDBPoint
	if err := json.NewDecoder(r.Body).Decode(&points); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	for _, point := range points {
		if err := a.engine.WriteOpenTSDBJSON(db, &point); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
	}

	w.WriteHeader(http.StatusNoContent)
}

// OpenTSDBTelnet handles OpenTSDB telnet protocol
func (a *API) OpenTSDBTelnet(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	if !a.authenticate(r) {
		http.Error(w, "Unauthorized", http.StatusUnauthorized)
		return
	}

	db := strings.TrimPrefix(r.URL.Path, "/opentsdb/v1/put/telnet/")

	body, _ := io.ReadAll(r.Body)
	lines := strings.Split(string(body), "\n")

	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}

		if err := a.engine.WriteOpenTSDBTelnet(db, line); err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
	}

	w.WriteHeader(http.StatusNoContent)
}

// Login handles /rest/login for token generation
func (a *API) Login(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Generate token from Basic auth
	auth := r.Header.Get("Authorization")
	if !strings.HasPrefix(auth, "Basic ") {
		a.respondError(w, TSDB_CODE_TSC_AUTH_FAILURE, "Missing authentication")
		return
	}

	decoded, err := base64.StdEncoding.DecodeString(strings.TrimPrefix(auth, "Basic "))
	if err != nil {
		a.respondError(w, TSDB_CODE_TSC_AUTH_FAILURE, "Invalid authentication encoding")
		return
	}

	parts := strings.SplitN(string(decoded), ":", 2)
	if len(parts) != 2 {
		a.respondError(w, TSDB_CODE_TSC_AUTH_FAILURE, "Invalid credentials format")
		return
	}

	if !a.engine.Authenticate(parts[0], parts[1]) {
		a.respondError(w, TSDB_CODE_TSC_AUTH_FAILURE, "Authentication failed")
		return
	}

	// Return TDengine-style token (the base64 encoded credentials for now)
	token := strings.TrimPrefix(auth, "Basic ")

	a.respond(w, &Response{
		Code: 0,
		Desc: token,
	})
}

// Health handles health check endpoint
func (a *API) Health(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	w.Write([]byte("LumaDB TDengine-Compatible API is healthy"))
}

// Ready handles readiness check endpoint
func (a *API) Ready(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	w.Write([]byte("ready"))
}

// authenticate checks request authentication
func (a *API) authenticate(r *http.Request) bool {
	auth := r.Header.Get("Authorization")

	// Support Basic authentication
	if strings.HasPrefix(auth, "Basic ") {
		decoded, err := base64.StdEncoding.DecodeString(strings.TrimPrefix(auth, "Basic "))
		if err != nil {
			return false
		}
		parts := strings.SplitN(string(decoded), ":", 2)
		if len(parts) != 2 {
			return false
		}
		return a.engine.Authenticate(parts[0], parts[1])
	}

	// Support Taosd token
	if strings.HasPrefix(auth, "Taosd ") {
		token := strings.TrimPrefix(auth, "Taosd ")
		return a.engine.ValidateToken(token)
	}

	// Support URL token parameter
	token := r.URL.Query().Get("token")
	if token != "" {
		return a.engine.ValidateToken(token)
	}

	return false
}

// respond writes a JSON response
func (a *API) respond(w http.ResponseWriter, result *Response) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(result)
}

// respondError writes an error response
func (a *API) respondError(w http.ResponseWriter, code int, desc string) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(&Response{
		Code: code,
		Desc: desc,
	})
}

// parseInfluxDBLine parses InfluxDB line protocol
func parseInfluxDBLine(line, precision string) (*InfluxDBLineProtocol, error) {
	// Format: measurement[,tag=value...] field=value[,field=value...] [timestamp]
	result := &InfluxDBLineProtocol{
		Tags:   make(map[string]string),
		Fields: make(map[string]interface{}),
	}

	// Split by space (max 3 parts: measurement+tags, fields, timestamp)
	parts := strings.SplitN(line, " ", 3)
	if len(parts) < 2 {
		return nil, nil // Skip malformed lines
	}

	// Parse measurement and tags
	measurementAndTags := strings.Split(parts[0], ",")
	result.Measurement = measurementAndTags[0]

	for _, tag := range measurementAndTags[1:] {
		kv := strings.SplitN(tag, "=", 2)
		if len(kv) == 2 {
			result.Tags[kv[0]] = kv[1]
		}
	}

	// Parse fields
	fieldParts := strings.Split(parts[1], ",")
	for _, field := range fieldParts {
		kv := strings.SplitN(field, "=", 2)
		if len(kv) == 2 {
			// Simple float parsing
			result.Fields[kv[0]] = parseFieldValue(kv[1])
		}
	}

	// Parse timestamp
	if len(parts) > 2 {
		ts := parseInt64(parts[2])
		result.Timestamp = normalizeTimestamp(ts, precision)
	} else {
		result.Timestamp = time.Now().UnixNano() / int64(time.Millisecond)
	}

	return result, nil
}

func parseFieldValue(s string) interface{} {
	s = strings.TrimSpace(s)

	// Boolean
	if s == "true" || s == "t" || s == "T" {
		return true
	}
	if s == "false" || s == "f" || s == "F" {
		return false
	}

	// Integer (ends with 'i')
	if strings.HasSuffix(s, "i") {
		return parseInt64(s[:len(s)-1])
	}

	// String (quoted)
	if strings.HasPrefix(s, "\"") && strings.HasSuffix(s, "\"") {
		return s[1 : len(s)-1]
	}

	// Float
	return parseFloat64(s)
}

func parseInt64(s string) int64 {
	var result int64
	for _, c := range s {
		if c >= '0' && c <= '9' {
			result = result*10 + int64(c-'0')
		} else if c == '-' && result == 0 {
			// Handle negative numbers
		}
	}
	return result
}

func parseFloat64(s string) float64 {
	var result float64
	var decimal float64 = 0
	var afterDecimal bool
	var divisor float64 = 10
	negative := false

	for i, c := range s {
		if c == '-' && i == 0 {
			negative = true
			continue
		}
		if c == '.' {
			afterDecimal = true
			continue
		}
		if c >= '0' && c <= '9' {
			if afterDecimal {
				decimal += float64(c-'0') / divisor
				divisor *= 10
			} else {
				result = result*10 + float64(c-'0')
			}
		}
	}

	result += decimal
	if negative {
		result = -result
	}
	return result
}

func normalizeTimestamp(ts int64, precision string) int64 {
	switch precision {
	case "ns":
		return ts / 1_000_000 // Convert to ms
	case "us", "u":
		return ts / 1_000 // Convert to ms
	case "ms":
		return ts
	case "s":
		return ts * 1_000 // Convert to ms
	default:
		return ts / 1_000_000 // Default ns to ms
	}
}
