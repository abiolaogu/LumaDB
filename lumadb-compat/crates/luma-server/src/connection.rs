use std::collections::HashMap;
use std::sync::Arc;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, SemaphorePermit, RwLock};
use crate::config::Config;
use crate::metrics::ACTIVE_CONNECTIONS;

/// Rate limiter configuration
#[derive(Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Window duration
    pub window: Duration,
    /// Ban duration after exceeding limit
    pub ban_duration: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
            ban_duration: Duration::from_secs(300),
        }
    }
}

/// Token bucket entry for IP tracking
struct IpBucket {
    tokens: u32,
    last_refill: Instant,
    banned_until: Option<Instant>,
}

/// IP-based rate limiter
pub struct RateLimiter {
    config: RateLimitConfig,
    buckets: RwLock<HashMap<IpAddr, IpBucket>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: RwLock::new(HashMap::new()),
        }
    }
    
    /// Check if IP is allowed, consuming a token if so
    pub async fn check(&self, ip: IpAddr) -> bool {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();
        
        let bucket = buckets.entry(ip).or_insert_with(|| IpBucket {
            tokens: self.config.max_requests,
            last_refill: now,
            banned_until: None,
        });
        
        // Check if banned
        if let Some(ban_until) = bucket.banned_until {
            if now < ban_until {
                tracing::warn!("IP {} is banned until {:?}", ip, ban_until);
                return false;
            }
            bucket.banned_until = None;
        }
        
        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill);
        if elapsed >= self.config.window {
            bucket.tokens = self.config.max_requests;
            bucket.last_refill = now;
        }
        
        // Try to consume a token
        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            // Ban the IP
            bucket.banned_until = Some(now + self.config.ban_duration);
            tracing::warn!("Rate limit exceeded for IP {}, banning for {:?}", ip, self.config.ban_duration);
            false
        }
    }
}

#[derive(Clone)]
pub struct ConnectionManager {
    semaphores: HashMap<String, Arc<Semaphore>>,
    pub rate_limiter: Arc<RateLimiter>,
}

impl ConnectionManager {
    pub fn new(config: &Config) -> Self {
        let mut semaphores = HashMap::new();

        // Helper to add semaphore for a protocol
        let mut add_sem = |protocol: &str, limit: u32| {
            semaphores.insert(
                protocol.to_string(),
                Arc::new(Semaphore::new(limit as usize)),
            );
        };

        #[cfg(feature = "postgres")]
        if let Some(c) = &config.postgres { add_sem("postgres", c.max_connections); }

        #[cfg(feature = "mysql")]
        if let Some(c) = &config.mysql { add_sem("mysql", 100); // Default if max_connections not on MySqlConfig
        }

        #[cfg(feature = "cassandra")]
        if let Some(_c) = &config.cassandra { add_sem("cassandra", 1000); }

        #[cfg(feature = "mongodb")]
        if let Some(_c) = &config.mongodb { add_sem("mongodb", 1000); }

        #[cfg(feature = "influxdb")]
        if let Some(_c) = &config.influxdb { add_sem("influxdb", 1000); } 

        #[cfg(feature = "prometheus")]
        if let Some(_c) = &config.prometheus { add_sem("prometheus", 1000); }

        Self { 
            semaphores,
            rate_limiter: Arc::new(RateLimiter::new(RateLimitConfig::default())),
        }
    }

    pub fn get_semaphore(&self, protocol: &str) -> Option<Arc<Semaphore>> {
        self.semaphores.get(protocol).cloned()
    }

    pub async fn acquire(&self, protocol: &str) -> Option<ConnectionPermit<'_>> {
        if let Some(sem) = self.semaphores.get(protocol) {
            match sem.acquire().await {
                Ok(permit) => {
                    ACTIVE_CONNECTIONS.with_label_values(&[protocol]).inc();
                    Some(ConnectionPermit {
                        _permit: permit,
                        protocol: protocol.to_string(),
                    })
                }
                Err(_) => None,
            }
        } else {
            None
        }
    }
    
    /// Check rate limit for an IP before processing
    pub async fn check_rate_limit(&self, ip: IpAddr) -> bool {
        self.rate_limiter.check(ip).await
    }
}

pub struct ConnectionPermit<'a> {
    _permit: SemaphorePermit<'a>,
    protocol: String,
}

impl<'a> Drop for ConnectionPermit<'a> {
    fn drop(&mut self) {
        ACTIVE_CONNECTIONS.with_label_values(&[&self.protocol]).dec();
    }
}
