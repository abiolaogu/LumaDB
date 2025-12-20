use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use std::collections::HashMap;

/// Token Bucket Rate Limiter
pub struct RateLimiter {
    tokens_per_second: f64,
    max_tokens: f64,
    buckets: Mutex<HashMap<String, TokenBucket>>,
}

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(rate: f64, capacity: f64) -> Self {
        Self {
            tokens_per_second: rate,
            max_tokens: capacity,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Check if connection from IP is allowed
    pub fn check(&self, ip: &str) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        
        let now = Instant::now();
        let bucket = buckets.entry(ip.to_string()).or_insert(TokenBucket {
            tokens: self.max_tokens,
            last_refill: now,
        });

        // Refill
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        if elapsed > 0.0 {
            bucket.tokens = (bucket.tokens + elapsed * self.tokens_per_second).min(self.max_tokens);
            bucket.last_refill = now;
        }

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}
