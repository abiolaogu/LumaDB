// Package api provides unified TSDB routing
//
// Routes requests to appropriate LumaDB TSDB engines:
// - /api/v1/* → Prometheus (port 9090)
// - /write, /query → InfluxDB (port 8086)
// - /druid/* → Druid (port 8888)
package api

import (
	"io"
	"net/http"
	"net/http/httputil"
	"net/url"
	"strings"
)

// TsdbRouter routes TSDB requests to the appropriate backend
type TsdbRouter struct {
	PrometheusURL *url.URL
	InfluxDBURL   *url.URL
	DruidURL      *url.URL

	prometheusProxy *httputil.ReverseProxy
	influxdbProxy   *httputil.ReverseProxy
	druidProxy      *httputil.ReverseProxy
}

// NewTsdbRouter creates a new TSDB router
func NewTsdbRouter(prometheusAddr, influxdbAddr, druidAddr string) (*TsdbRouter, error) {
	prometheusURL, err := url.Parse(prometheusAddr)
	if err != nil {
		return nil, err
	}

	influxdbURL, err := url.Parse(influxdbAddr)
	if err != nil {
		return nil, err
	}

	druidURL, err := url.Parse(druidAddr)
	if err != nil {
		return nil, err
	}

	return &TsdbRouter{
		PrometheusURL:   prometheusURL,
		InfluxDBURL:     influxdbURL,
		DruidURL:        druidURL,
		prometheusProxy: httputil.NewSingleHostReverseProxy(prometheusURL),
		influxdbProxy:   httputil.NewSingleHostReverseProxy(influxdbURL),
		druidProxy:      httputil.NewSingleHostReverseProxy(druidURL),
	}, nil
}

// DefaultTsdbRouter creates a router with default local ports
func DefaultTsdbRouter() (*TsdbRouter, error) {
	return NewTsdbRouter(
		"http://localhost:9090", // Prometheus
		"http://localhost:8086", // InfluxDB
		"http://localhost:8888", // Druid
	)
}

// ServeHTTP implements http.Handler
func (r *TsdbRouter) ServeHTTP(w http.ResponseWriter, req *http.Request) {
	path := req.URL.Path

	// Route based on path prefix
	switch {
	// Prometheus endpoints
	case strings.HasPrefix(path, "/api/v1/"):
		r.prometheusProxy.ServeHTTP(w, req)

	case strings.HasPrefix(path, "/-/"):
		// Prometheus health checks
		r.prometheusProxy.ServeHTTP(w, req)

	// Druid endpoints
	case strings.HasPrefix(path, "/druid/"):
		r.druidProxy.ServeHTTP(w, req)

	case strings.HasPrefix(path, "/status"):
		// Druid status
		r.druidProxy.ServeHTTP(w, req)

	// InfluxDB v2 endpoints
	case strings.HasPrefix(path, "/api/v2/"):
		r.influxdbProxy.ServeHTTP(w, req)

	// InfluxDB v1 endpoints
	case path == "/write" || strings.HasPrefix(path, "/write?"):
		r.influxdbProxy.ServeHTTP(w, req)

	case path == "/query" || strings.HasPrefix(path, "/query?"):
		r.influxdbProxy.ServeHTTP(w, req)

	case path == "/ping" || path == "/health" || path == "/ready":
		r.influxdbProxy.ServeHTTP(w, req)

	// Default: return routing info
	default:
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		io.WriteString(w, `{
  "name": "LumaDB Universal TSDB Router",
  "version": "1.0.0",
  "engines": {
    "prometheus": {"port": 9090, "paths": ["/api/v1/*", "/-/*"]},
    "influxdb": {"port": 8086, "paths": ["/write", "/query", "/api/v2/*", "/ping", "/health"]},
    "druid": {"port": 8888, "paths": ["/druid/*", "/status"]}
  }
}`)
	}
}

// StartRouter starts the unified TSDB router on the given port
func StartRouter(port string) error {
	router, err := DefaultTsdbRouter()
	if err != nil {
		return err
	}

	http.Handle("/", router)
	return http.ListenAndServe(":"+port, nil)
}

// HealthCheck returns health status of all backends
type HealthStatus struct {
	Prometheus bool `json:"prometheus"`
	InfluxDB   bool `json:"influxdb"`
	Druid      bool `json:"druid"`
	Healthy    bool `json:"healthy"`
}

// CheckHealth checks all backend health endpoints
func (r *TsdbRouter) CheckHealth() HealthStatus {
	status := HealthStatus{}

	// Check Prometheus
	if resp, err := http.Get(r.PrometheusURL.String() + "/-/healthy"); err == nil {
		status.Prometheus = resp.StatusCode == 200
		resp.Body.Close()
	}

	// Check InfluxDB
	if resp, err := http.Get(r.InfluxDBURL.String() + "/ping"); err == nil {
		status.InfluxDB = resp.StatusCode == 204 || resp.StatusCode == 200
		resp.Body.Close()
	}

	// Check Druid
	if resp, err := http.Get(r.DruidURL.String() + "/status/health"); err == nil {
		status.Druid = resp.StatusCode == 200
		resp.Body.Close()
	}

	status.Healthy = status.Prometheus && status.InfluxDB && status.Druid
	return status
}
