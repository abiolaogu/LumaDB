//! TSDB Core Engine
//!
//! Unified time-series storage engine with:
//! - Gorilla compression (10-15x ratio)
//! - Segment-based storage (hot/warm/cold tiers)
//! - Label/tag indexing with cardinality optimization
//! - SIMD-accelerated aggregations

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use dashmap::DashMap;

/// Time-series sample (timestamp + value)
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub timestamp_ms: i64,
    pub value: f64,
}

/// Metric series with labels
#[derive(Debug, Clone)]
pub struct Series {
    pub labels: HashMap<String, String>,
    pub samples: Vec<Sample>,
}

/// Segment - time-bounded chunk of compressed data
#[derive(Debug)]
pub struct Segment {
    pub id: u64,
    pub min_time: i64,
    pub max_time: i64,
    pub series_count: usize,
    pub sample_count: usize,
    /// Compressed data (Gorilla encoded)
    pub data: Vec<u8>,
    /// Label index: label_key -> label_value -> series_ids
    pub label_index: HashMap<String, HashMap<String, Vec<u64>>>,
    /// Tier: hot (in-memory), warm (SSD), cold (HDD/S3)
    pub tier: SegmentTier,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegmentTier {
    Hot,   // In-memory, most recent
    Warm,  // SSD, recent
    Cold,  // HDD/S3, archive
}

/// TSDB Engine
pub struct TsdbEngine {
    /// Series by fingerprint (hash of labels)
    series_map: DashMap<u64, Arc<RwLock<Series>>>,
    /// Active (hot) segment for writes
    active_segment: RwLock<ActiveSegment>,
    /// Sealed segments (read-only, compressed)
    segments: RwLock<Vec<Arc<Segment>>>,
    /// Label inverted index: label_name -> label_value -> fingerprints
    label_index: DashMap<String, DashMap<String, Vec<u64>>>,
    /// Metric name -> fingerprints
    metric_index: DashMap<String, Vec<u64>>,
    /// Configuration
    config: TsdbConfig,
}

/// Active segment for writes (uncompressed)
struct ActiveSegment {
    id: u64,
    min_time: i64,
    max_time: i64,
    /// Series data before compression
    series_data: HashMap<u64, Vec<Sample>>,
    sample_count: usize,
    created_at: std::time::Instant,
}

/// TSDB Configuration
#[derive(Clone)]
pub struct TsdbConfig {
    /// Segment size (samples before sealing)
    pub segment_samples: usize,
    /// Segment duration (ms before sealing)
    pub segment_duration_ms: i64,
    /// Retention period (ms)
    pub retention_ms: i64,
    /// Enable Gorilla compression
    pub compression: bool,
}

impl Default for TsdbConfig {
    fn default() -> Self {
        Self {
            segment_samples: 100_000,
            segment_duration_ms: 2 * 60 * 60 * 1000, // 2 hours
            retention_ms: 15 * 24 * 60 * 60 * 1000,  // 15 days
            compression: true,
        }
    }
}

impl TsdbEngine {
    pub fn new(config: TsdbConfig) -> Self {
        Self {
            series_map: DashMap::new(),
            active_segment: RwLock::new(ActiveSegment {
                id: 0,
                min_time: i64::MAX,
                max_time: i64::MIN,
                series_data: HashMap::new(),
                sample_count: 0,
                created_at: std::time::Instant::now(),
            }),
            segments: RwLock::new(Vec::new()),
            label_index: DashMap::new(),
            metric_index: DashMap::new(),
            config,
        }
    }

    /// Ingest samples for a series
    pub fn ingest(&self, labels: HashMap<String, String>, samples: Vec<Sample>) {
        let fingerprint = self.compute_fingerprint(&labels);
        let metric_name = labels.get("__name__").cloned().unwrap_or_default();
        
        // Update or create series
        self.series_map.entry(fingerprint).or_insert_with(|| {
            // Index labels
            for (k, v) in &labels {
                self.label_index
                    .entry(k.clone())
                    .or_default()
                    .entry(v.clone())
                    .or_default()
                    .push(fingerprint);
            }
            
            // Index metric name
            self.metric_index
                .entry(metric_name.clone())
                .or_default()
                .push(fingerprint);
            
            Arc::new(RwLock::new(Series {
                labels,
                samples: Vec::new(),
            }))
        });
        
        // Add samples to active segment
        let mut active = self.active_segment.write();
        
        for sample in &samples {
            active.min_time = active.min_time.min(sample.timestamp_ms);
            active.max_time = active.max_time.max(sample.timestamp_ms);
        }
        
        active.series_data
            .entry(fingerprint)
            .or_default()
            .extend(samples.iter().copied());
        
        active.sample_count += samples.len();
        
        // Check if we need to seal the segment
        let should_seal = active.sample_count >= self.config.segment_samples
            || active.created_at.elapsed().as_millis() as i64 >= self.config.segment_duration_ms;
        
        if should_seal {
            drop(active);
            self.seal_active_segment();
        }
    }

    /// Seal active segment and create new one
    fn seal_active_segment(&self) {
        let mut active = self.active_segment.write();
        
        if active.sample_count == 0 {
            return;
        }
        
        // Create sealed segment
        let segment = self.compress_segment(&active);
        
        // Add to segments list
        self.segments.write().push(Arc::new(segment));
        
        // Reset active segment
        *active = ActiveSegment {
            id: active.id + 1,
            min_time: i64::MAX,
            max_time: i64::MIN,
            series_data: HashMap::new(),
            sample_count: 0,
            created_at: std::time::Instant::now(),
        };
    }

    /// Compress segment data
    fn compress_segment(&self, active: &ActiveSegment) -> Segment {
        let mut data = Vec::new();
        let mut label_index: HashMap<String, HashMap<String, Vec<u64>>> = HashMap::new();
        let mut series_count = 0;
        
        for (fingerprint, samples) in &active.series_data {
            series_count += 1;
            
            // Get series labels
            if let Some(series) = self.series_map.get(fingerprint) {
                let labels = &series.read().labels;
                for (k, v) in labels {
                    label_index
                        .entry(k.clone())
                        .or_default()
                        .entry(v.clone())
                        .or_default()
                        .push(*fingerprint);
                }
            }
            
            // Compress samples using Gorilla encoding
            if self.config.compression {
                let compressed = super::gorilla::compress_samples(samples);
                data.extend(compressed);
            } else {
                // Store uncompressed
                for sample in samples {
                    data.extend(&sample.timestamp_ms.to_le_bytes());
                    data.extend(&sample.value.to_le_bytes());
                }
            }
        }
        
        Segment {
            id: active.id,
            min_time: active.min_time,
            max_time: active.max_time,
            series_count,
            sample_count: active.sample_count,
            data,
            label_index,
            tier: SegmentTier::Hot,
        }
    }

    /// Query samples for a metric
    pub fn query(
        &self,
        matchers: &[LabelMatcher],
        start_ms: i64,
        end_ms: i64,
    ) -> Vec<Series> {
        let mut results = Vec::new();
        
        // Find matching series
        let fingerprints = self.find_matching_series(matchers);
        
        for fp in fingerprints {
            if let Some(series_ref) = self.series_map.get(&fp) {
                let series = series_ref.read();
                
                // Collect samples in time range
                let samples: Vec<Sample> = series.samples.iter()
                    .filter(|s| s.timestamp_ms >= start_ms && s.timestamp_ms <= end_ms)
                    .copied()
                    .collect();
                
                if !samples.is_empty() {
                    results.push(Series {
                        labels: series.labels.clone(),
                        samples,
                    });
                }
            }
        }
        
        // Also query sealed segments
        let segments = self.segments.read();
        for segment in segments.iter() {
            if segment.max_time < start_ms || segment.min_time > end_ms {
                continue; // Segment doesn't overlap time range
            }
            
            // TODO: Decompress and query segment data
        }
        
        results
    }

    /// Find series matching label matchers
    fn find_matching_series(&self, matchers: &[LabelMatcher]) -> Vec<u64> {
        if matchers.is_empty() {
            // Return all series
            return self.series_map.iter().map(|r| *r.key()).collect();
        }
        
        let mut result_set: Option<Vec<u64>> = None;
        
        for matcher in matchers {
            let matching = self.find_series_for_matcher(matcher);
            
            result_set = match result_set {
                None => Some(matching),
                Some(current) => {
                    // Intersect
                    Some(current.into_iter().filter(|fp| matching.contains(fp)).collect())
                }
            };
        }
        
        result_set.unwrap_or_default()
    }

    fn find_series_for_matcher(&self, matcher: &LabelMatcher) -> Vec<u64> {
        match matcher.match_type {
            MatchType::Equal => {
                self.label_index
                    .get(&matcher.name)
                    .and_then(|values| values.get(&matcher.value).map(|fps| fps.clone()))
                    .unwrap_or_default()
            }
            MatchType::NotEqual => {
                let exclude = self.label_index
                    .get(&matcher.name)
                    .and_then(|values| values.get(&matcher.value).map(|fps| fps.clone()))
                    .unwrap_or_default();
                
                self.series_map.iter()
                    .map(|r| *r.key())
                    .filter(|fp| !exclude.contains(fp))
                    .collect()
            }
            MatchType::Regex | MatchType::NotRegex => {
                // Simplified: just return all for regex
                self.series_map.iter().map(|r| *r.key()).collect()
            }
        }
    }

    /// Compute fingerprint (hash) for label set
    fn compute_fingerprint(&self, labels: &HashMap<String, String>) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut sorted: Vec<_> = labels.iter().collect();
        sorted.sort_by_key(|(k, _)| *k);
        
        let mut hasher = DefaultHasher::new();
        for (k, v) in sorted {
            k.hash(&mut hasher);
            v.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Get all label names
    pub fn label_names(&self) -> Vec<String> {
        self.label_index.iter().map(|r| r.key().clone()).collect()
    }

    /// Get all values for a label
    pub fn label_values(&self, name: &str) -> Vec<String> {
        self.label_index
            .get(name)
            .map(|values| values.iter().map(|r| r.key().clone()).collect())
            .unwrap_or_default()
    }

    /// Get series count
    pub fn series_count(&self) -> usize {
        self.series_map.len()
    }

    /// Get sample count
    pub fn sample_count(&self) -> usize {
        let active = self.active_segment.read();
        let sealed: usize = self.segments.read().iter().map(|s| s.sample_count).sum();
        active.sample_count + sealed
    }
}

/// Label matcher for queries
#[derive(Debug, Clone)]
pub struct LabelMatcher {
    pub name: String,
    pub value: String,
    pub match_type: MatchType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MatchType {
    Equal,
    NotEqual,
    Regex,
    NotRegex,
}
