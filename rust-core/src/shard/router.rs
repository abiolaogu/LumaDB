use xxhash_rust::xxh3::xxh3_64;

/// Shard Router using Jump Hash or Rendezvous Hash
/// For "Shard-Per-Core" with fixed core count, simple Modulo or consistent hashing is fine.
/// We'll use simple hashing for now as rebalancing is complex.
#[derive(Clone)]
pub struct ShardRouter {
    num_shards: usize,
}

impl ShardRouter {
    pub fn new(num_shards: usize) -> Self {
        Self { num_shards }
    }

    /// Route a key to a specific shard ID
    pub fn route_key(&self, key: &[u8]) -> usize {
        let hash = xxh3_64(key);
        (hash % self.num_shards as u64) as usize
    }

    /// Route a key range to a set of shard IDs
    /// For simple hash partitioning, a range scan might hit ALL shards.
    pub fn route_range(&self, _start: &[u8], _end: &[u8]) -> Vec<usize> {
        // Naive implementation: return all shards
        // Improved: If using range partitioning, we'd check ranges.
        // For hash partitioning, we must scan all.
        (0..self.num_shards).collect()
    }
}
