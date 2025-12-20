//! Ultra-High Performance Vector Search Module
//!
//! Target: 2,500,000+ QPS
//!
//! Features:
//! - SIMD-optimized distance calculations (AVX2/AVX-512/NEON)
//! - Lock-free HNSW index for concurrent insert/search
//! - Sharded architecture for linear scaling with cores
//! - Batch operations for maximum throughput

pub mod ultra_engine;

pub use ultra_engine::{
    UltraVectorEngine,
    ShardedVectorIndex,
    HNSWIndex,
    HNSWConfig,
    DistanceMetric,
    l2_distance_simd,
    inner_product_simd,
    normalize,
};

/// Quick-start: Create a vector engine with optimal defaults
pub fn create_engine(dim: usize) -> UltraVectorEngine {
    UltraVectorEngine::with_defaults(dim)
}

/// Create engine optimized for high recall (99%+)
pub fn create_high_recall_engine(dim: usize) -> UltraVectorEngine {
    UltraVectorEngine::new(dim, DistanceMetric::L2, HNSWConfig::high_recall())
}

/// Create engine optimized for maximum throughput
pub fn create_high_throughput_engine(dim: usize) -> UltraVectorEngine {
    UltraVectorEngine::new(dim, DistanceMetric::L2, HNSWConfig::high_throughput())
}
