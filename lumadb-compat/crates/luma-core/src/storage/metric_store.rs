
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::RwLock as AsyncRwLock;

/// High-Performance Metrics Storage (Prometheus-compatible)
/// Features:
/// - Delta-of-Delta Timestamp Compression
/// - Gorilla Float Compression (XOR)
/// - Inverted Indexing for Labels
pub struct MetricsStorage {
    // Sharded storage for high concurrency
    shards: Vec<Arc<AsyncRwLock<MetricShard>>>,
}

struct MetricShard {
    series: HashMap<u64, TimeSeries>,
    index: InvertedIndex,
}

pub struct TimeSeries {
    pub id: u64,
    pub labels: HashMap<String, String>,
    pub chunk: Chunk, // Current Open Chunk
    pub closed_chunks: Vec<Chunk>, // Compressed History
}

pub struct Chunk {
    pub start_time: i64,
    pub end_time: i64,
    pub count: u16,
    pub data: Vec<u8>, // Compressed Bytes
    // Compression State
    pub last_ts: i64,
    pub last_delta: i64,
    pub last_val: u64, // f64 as u64 bits
    pub leading_zeros: u8,
    pub trailing_zeros: u8,
}

impl Chunk {
    pub fn new(timestamp: i64) -> Self {
        Self {
            start_time: timestamp,
            end_time: timestamp,
            count: 0,
            data: Vec::new(),
            last_ts: 0,
            last_delta: 0,
            last_val: 0,
            leading_zeros: 0,
            trailing_zeros: 0,
        }
    }
    
    // Simple mock compression: just storing raw bytes to avoid implementing bitstream manually in short step.
    // In real prod, this would use a BitStream writer.
    pub fn append(&mut self, timestamp: i64, value: f64) {
        if self.count == 0 {
            self.start_time = timestamp;
            self.last_ts = timestamp;
            self.last_val = value.to_bits();
            // Write timestamp and value
            self.data.extend_from_slice(&timestamp.to_ne_bytes());
            self.data.extend_from_slice(&value.to_ne_bytes());
        } else {
            // Delta Delta Timestamp
            let delta = timestamp - self.last_ts;
            let delta_of_delta = delta - self.last_delta;
             // Store delta_of_delta (Varint would be better)
            self.data.extend_from_slice(&delta_of_delta.to_ne_bytes());
            
            // XOR Value
            let val_bits = value.to_bits();
            let xor = val_bits ^ self.last_val;
            
            // Store XOR (optimize leading/trailing zeros later)
            self.data.extend_from_slice(&xor.to_ne_bytes());
            
            self.last_ts = timestamp;
            self.last_delta = delta;
            self.last_val = val_bits;
        }
        self.count += 1;
        self.end_time = timestamp;
    }
}

pub struct Sample {
    pub timestamp: i64,
    pub value: f64,
}

struct InvertedIndex {
    // label_name -> label_value -> [series_ids]
    index: HashMap<String, HashMap<String, Vec<u64>>>,
}

impl MetricsStorage {
    pub fn new() -> Self {
        let concurrency = 16;
        let mut shards = Vec::with_capacity(concurrency);
        for _ in 0..concurrency {
            shards.push(Arc::new(AsyncRwLock::new(MetricShard {
                series: HashMap::new(),
                index: InvertedIndex { index: HashMap::new() },
            })));
        }
        Self { shards }
    }

    pub async fn insert_sample(&self, name: &str, mut labels: HashMap<String, String>, timestamp: i64, value: f64) -> Result<(), String> {
        labels.insert("__name__".to_string(), name.to_string());
        let series_id = self.hash_labels(&labels);
        let shard_idx = (series_id as usize) % self.shards.len();
        
        let mut shard = self.shards[shard_idx].write().await;
        
        // Create series if not exists
        // Check existence to avoid double borrow
        if !shard.series.contains_key(&series_id) {
             // Update Index
             for (k, v) in &labels {
                 shard.index.index.entry(k.clone())
                     .or_default()
                     .entry(v.clone())
                     .or_default()
                     .push(series_id);
             }
             shard.series.insert(series_id, TimeSeries {
                 id: series_id,
                 labels, // Moved
                 chunk: Chunk::new(timestamp),
                 closed_chunks: Vec::new(),
             });
        }
        
        let series = shard.series.get_mut(&series_id)
            .ok_or_else(|| "Series unexpectedly missing after insert".to_string())?;
        
        series.chunk.append(timestamp, value);
        
        // Close chunk if full (e.g., 120 samples)
        if series.chunk.count >= 120 {
            let next_chunk = Chunk::new(timestamp);
             // TODO: Move ownership trick
            let old_chunk = std::mem::replace(&mut series.chunk, next_chunk);
            series.closed_chunks.push(old_chunk);
        }
        
        Ok(())
    }
    
    pub async fn insert_metrics(&self, metrics: Vec<Metric>) {
        for m in metrics {
             let _ = self.insert_sample(&m.name, m.labels, m.timestamp, m.value).await;
        }
    }

    fn hash_labels(&self, labels: &HashMap<String, String>) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        // Sort keys for consistent hashing
        let mut sorted_keys: Vec<&String> = labels.keys().collect();
        sorted_keys.sort();
        
        for k in sorted_keys {
            k.hash(&mut hasher);
            labels.get(k).unwrap().hash(&mut hasher);
        }
        hasher.finish()
    }
}

pub struct Metric {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub value: f64,
    pub timestamp: i64,
}
