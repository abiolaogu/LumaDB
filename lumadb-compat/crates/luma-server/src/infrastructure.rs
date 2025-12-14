//! Production Infrastructure Module
//! Provides rate limiting, health checks, metrics, graceful shutdown

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use dashmap::DashMap;
use tracing::{info, warn};

// ============================================================================
// Rate Limiter
// ============================================================================

/// Token bucket rate limiter per IP address
pub struct RateLimiter {
    /// Tokens per IP: (tokens, last_refill_time)
    buckets: DashMap<String, (u64, Instant)>,
    /// Max tokens per bucket
    max_tokens: u64,
    /// Tokens added per second
    refill_rate: u64,
}

impl RateLimiter {
    pub fn new(max_tokens: u64, refill_rate: u64) -> Self {
        Self {
            buckets: DashMap::new(),
            max_tokens,
            refill_rate,
        }
    }

    /// Check if request is allowed, consuming one token
    pub fn allow(&self, ip: &str) -> bool {
        let now = Instant::now();
        
        let mut entry = self.buckets.entry(ip.to_string()).or_insert((self.max_tokens, now));
        let (tokens, last_refill) = entry.value_mut();
        
        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(*last_refill);
        let new_tokens = (elapsed.as_secs_f64() * self.refill_rate as f64) as u64;
        *tokens = (*tokens + new_tokens).min(self.max_tokens);
        *last_refill = now;
        
        // Try to consume a token
        if *tokens > 0 {
            *tokens -= 1;
            true
        } else {
            false
        }
    }

    /// Clean up old entries (call periodically)
    pub fn cleanup(&self, max_age: Duration) {
        let now = Instant::now();
        self.buckets.retain(|_, (_, last_refill)| {
            now.duration_since(*last_refill) < max_age
        });
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        // Default: 100 requests/sec, burst of 1000
        Self::new(1000, 100)
    }
}

// ============================================================================
// Connection Limiter (Semaphore-based)
// ============================================================================

/// Limits concurrent connections per protocol
pub struct ConnectionLimiter {
    semaphore: Arc<Semaphore>,
    active: AtomicU64,
    total: AtomicU64,
    max: u64,
}

impl ConnectionLimiter {
    pub fn new(max_connections: u64) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_connections as usize)),
            active: AtomicU64::new(0),
            total: AtomicU64::new(0),
            max: max_connections,
        }
    }

    /// Try to acquire a connection permit
    pub async fn acquire(&self) -> Option<ConnectionPermit> {
        match self.semaphore.clone().try_acquire_owned() {
            Ok(permit) => {
                self.active.fetch_add(1, Ordering::Relaxed);
                self.total.fetch_add(1, Ordering::Relaxed);
                Some(ConnectionPermit { 
                    _permit: permit,
                    active: &self.active,
                })
            }
            Err(_) => None,
        }
    }

    pub fn active_connections(&self) -> u64 {
        self.active.load(Ordering::Relaxed)
    }

    pub fn total_connections(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }

    pub fn max_connections(&self) -> u64 {
        self.max
    }
}

impl Default for ConnectionLimiter {
    fn default() -> Self {
        Self::new(10000)
    }
}

pub struct ConnectionPermit<'a> {
    _permit: tokio::sync::OwnedSemaphorePermit,
    active: &'a AtomicU64,
}

impl<'a> Drop for ConnectionPermit<'a> {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Ordering::Relaxed);
    }
}

// ============================================================================
// Health Check System
// ============================================================================

#[derive(Clone, Debug)]
pub struct HealthStatus {
    pub healthy: bool,
    pub message: String,
    pub latency_ms: f64,
}

#[derive(Clone, Debug)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub last_check: Instant,
}

/// Centralized health checker for all protocols
pub struct HealthChecker {
    components: DashMap<String, ComponentHealth>,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            components: DashMap::new(),
        }
    }

    /// Register a component health status
    pub fn report(&self, name: &str, healthy: bool, message: &str, latency_ms: f64) {
        self.components.insert(name.to_string(), ComponentHealth {
            name: name.to_string(),
            status: HealthStatus {
                healthy,
                message: message.to_string(),
                latency_ms,
            },
            last_check: Instant::now(),
        });
    }

    /// Get overall health
    pub fn is_healthy(&self) -> bool {
        self.components.iter().all(|c| c.status.healthy)
    }

    /// Get all component health statuses
    pub fn get_all(&self) -> Vec<ComponentHealth> {
        self.components.iter().map(|c| c.value().clone()).collect()
    }

    /// Get health as JSON
    pub fn to_json(&self) -> serde_json::Value {
        let components: Vec<serde_json::Value> = self.get_all().iter().map(|c| {
            serde_json::json!({
                "name": c.name,
                "healthy": c.status.healthy,
                "message": c.status.message,
                "latency_ms": c.status.latency_ms,
            })
        }).collect();
        
        serde_json::json!({
            "status": if self.is_healthy() { "healthy" } else { "unhealthy" },
            "components": components
        })
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Graceful Shutdown
// ============================================================================

/// Shutdown coordination for all protocols
pub struct ShutdownCoordinator {
    /// Signal that shutdown has been requested
    shutdown_requested: AtomicBool,
    /// Notifier for shutdown signal
    notify: tokio::sync::Notify,
    /// Active operations counter
    active_operations: AtomicU64,
}

impl ShutdownCoordinator {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            shutdown_requested: AtomicBool::new(false),
            notify: tokio::sync::Notify::new(),
            active_operations: AtomicU64::new(0),
        })
    }

    /// Check if shutdown has been requested
    pub fn is_shutdown(&self) -> bool {
        self.shutdown_requested.load(Ordering::SeqCst)
    }

    /// Request shutdown
    pub fn shutdown(&self) {
        info!("Shutdown requested");
        self.shutdown_requested.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// Wait for shutdown signal
    pub async fn wait_for_shutdown(&self) {
        while !self.is_shutdown() {
            self.notify.notified().await;
        }
    }

    /// Register an active operation
    pub fn begin_operation(&self) -> OperationGuard {
        self.active_operations.fetch_add(1, Ordering::Relaxed);
        OperationGuard { coordinator: self }
    }

    /// Wait for all operations to complete (with timeout)
    pub async fn wait_for_drain(&self, timeout: Duration) -> bool {
        let start = Instant::now();
        while self.active_operations.load(Ordering::Relaxed) > 0 {
            if start.elapsed() > timeout {
                warn!("Drain timeout, {} operations still active", 
                    self.active_operations.load(Ordering::Relaxed));
                return false;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        true
    }

    pub fn active_operations(&self) -> u64 {
        self.active_operations.load(Ordering::Relaxed)
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self {
            shutdown_requested: AtomicBool::new(false),
            notify: tokio::sync::Notify::new(),
            active_operations: AtomicU64::new(0),
        }
    }
}

pub struct OperationGuard<'a> {
    coordinator: &'a ShutdownCoordinator,
}

impl<'a> Drop for OperationGuard<'a> {
    fn drop(&mut self) {
        self.coordinator.active_operations.fetch_sub(1, Ordering::Relaxed);
    }
}

// ============================================================================
// Prometheus Metrics
// ============================================================================

/// Simple Prometheus metrics collector
pub struct MetricsCollector {
    /// Counter metrics
    counters: DashMap<String, AtomicU64>,
    /// Gauge metrics
    gauges: DashMap<String, AtomicU64>,
    /// Histogram buckets (simplified)
    histograms: Arc<DashMap<String, RwLock<Vec<f64>>>>,
    /// Labels for each metric
    #[allow(dead_code)]
    labels: DashMap<String, HashMap<String, String>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: DashMap::new(),
            gauges: DashMap::new(),
            histograms: Arc::new(DashMap::new()),
            labels: DashMap::new(),
        }
    }

    /// Increment a counter
    pub fn inc_counter(&self, name: &str, value: u64) {
        self.counters
            .entry(name.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(value, Ordering::Relaxed);
    }

    /// Set a gauge value
    pub fn set_gauge(&self, name: &str, value: u64) {
        self.gauges
            .entry(name.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .store(value, Ordering::Relaxed);
    }

    /// Record a histogram value
    pub async fn observe_histogram(&self, name: &str, value: f64) {
        let entry = self.histograms
            .entry(name.to_string())
            .or_insert_with(|| RwLock::new(Vec::new()));
        let mut values = entry.write().await;
        values.push(value);
        // Keep last 10000 values
        if values.len() > 10000 {
            values.drain(0..1000);
        }
    }

    /// Export metrics in Prometheus text format
    pub async fn export(&self) -> String {
        let mut output = String::new();
        
        // Export counters
        for entry in self.counters.iter() {
            output.push_str(&format!(
                "# TYPE {} counter\n{} {}\n",
                entry.key(),
                entry.key(),
                entry.value().load(Ordering::Relaxed)
            ));
        }
        
        // Export gauges
        for entry in self.gauges.iter() {
            output.push_str(&format!(
                "# TYPE {} gauge\n{} {}\n",
                entry.key(),
                entry.key(),
                entry.value().load(Ordering::Relaxed)
            ));
        }
        
        // Export histograms (simplified: just count, sum, p50, p99)
        for entry in self.histograms.iter() {
            let name = entry.key();
            let values = entry.value().read().await;
            if values.is_empty() {
                continue;
            }
            
            let count = values.len();
            let sum: f64 = values.iter().sum();
            let mut sorted = values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            
            let p50 = sorted.get(count / 2).copied().unwrap_or(0.0);
            let p99 = sorted.get((count * 99) / 100).copied().unwrap_or(0.0);
            
            output.push_str(&format!(
                "# TYPE {} histogram\n\
                {}_count {}\n\
                {}_sum {}\n\
                {}{{quantile=\"0.5\"}} {}\n\
                {}{{quantile=\"0.99\"}} {}\n",
                name, name, count, name, sum, name, p50, name, p99
            ));
        }
        
        output
    }

    /// Common metrics helpers
    pub fn record_query(&self, protocol: &str, latency_ms: f64, success: bool) {
        self.inc_counter(&format!("{}_queries_total", protocol), 1);
        if !success {
            self.inc_counter(&format!("{}_queries_errors_total", protocol), 1);
        }
        // Record latency (fire and forget)
        let name = format!("{}_query_latency_ms", protocol);
        let histograms = self.histograms.clone();
        tokio::spawn(async move {
            let entry = histograms
                .entry(name.clone())
                .or_insert_with(|| RwLock::new(Vec::new()));
            let mut values = entry.write().await;
            values.push(latency_ms);
            if values.len() > 10000 {
                values.drain(0..1000);
            }
        });
    }

    pub fn record_connection(&self, protocol: &str, active: u64) {
        self.set_gauge(&format!("{}_connections_active", protocol), active);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Server Context (combines all infrastructure)
// ============================================================================

/// Shared server context for all protocols
pub struct ServerContext {
    pub rate_limiter: RateLimiter,
    pub connection_limiters: DashMap<String, ConnectionLimiter>,
    pub health: HealthChecker,
    pub shutdown: Arc<ShutdownCoordinator>,
    pub metrics: MetricsCollector,
}

impl ServerContext {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            rate_limiter: RateLimiter::default(),
            connection_limiters: DashMap::new(),
            health: HealthChecker::default(),
            shutdown: ShutdownCoordinator::new(),
            metrics: MetricsCollector::default(),
        })
    }

    /// Get or create connection limiter for protocol
    pub fn ensure_connection_limiter(&self, protocol: &str, max: u64) {
        self.connection_limiters
            .entry(protocol.to_string())
            .or_insert_with(|| ConnectionLimiter::new(max));
    }

    /// Get active connections for protocol
    pub fn get_active_connections(&self, protocol: &str) -> u64 {
        self.connection_limiters.get(protocol)
            .map(|l| l.active_connections())
            .unwrap_or(0)
    }

    /// Check if connection is allowed (rate limit + connection limit)
    pub async fn allow_connection(&self, protocol: &str, ip: &str) -> bool {
        // Check shutdown
        if self.shutdown.is_shutdown() {
            return false;
        }
        
        // Check rate limit
        if !self.rate_limiter.allow(ip) {
            warn!("Rate limit exceeded for {}", ip);
            return false;
        }
        
        true
    }
}

impl Default for ServerContext {
    fn default() -> Self {
        Self {
            rate_limiter: RateLimiter::default(),
            connection_limiters: DashMap::new(),
            health: HealthChecker::default(),
            shutdown: Arc::new(ShutdownCoordinator::default()),
            metrics: MetricsCollector::default(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(10, 10);
        for _ in 0..10 {
            assert!(limiter.allow("127.0.0.1"));
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(5, 1);
        for _ in 0..5 {
            assert!(limiter.allow("127.0.0.1"));
        }
        assert!(!limiter.allow("127.0.0.1"));
    }

    #[test]
    fn test_health_checker_reports() {
        let health = HealthChecker::new();
        health.report("test", true, "OK", 1.0);
        assert!(health.is_healthy());
        
        health.report("test2", false, "Error", 100.0);
        assert!(!health.is_healthy());
    }

    #[tokio::test]
    async fn test_shutdown_coordinator() {
        let coordinator = ShutdownCoordinator::new();
        assert!(!coordinator.is_shutdown());
        
        coordinator.shutdown();
        assert!(coordinator.is_shutdown());
    }

    #[test]
    fn test_metrics_counter() {
        let metrics = MetricsCollector::new();
        metrics.inc_counter("test_counter", 1);
        metrics.inc_counter("test_counter", 5);
        
        let value = metrics.counters.get("test_counter")
            .map(|v| v.load(Ordering::Relaxed))
            .unwrap_or(0);
        assert_eq!(value, 6);
    }
}
