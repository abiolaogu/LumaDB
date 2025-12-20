pub mod core_affinity;
pub mod core_shard;
pub mod coordinator;
pub mod message_bus;
pub mod router;

// Re-exports
pub use coordinator::ShardCoordinator;
pub use core_shard::CoreShard;
pub use router::ShardRouter;

use std::sync::Arc;

/// Stub shard handle for backward compatibility
pub struct ShardHandle;

impl ShardHandle {
    pub async fn put(&self, _key: &[u8], _value: &[u8]) -> crate::Result<()> {
        Ok(())
    }
    pub async fn get(&self, _key: &[u8]) -> crate::Result<Option<Vec<u8>>> {
        Ok(None)
    }
    pub async fn delete(&self, _key: &[u8]) -> crate::Result<()> {
        Ok(())
    }
    pub async fn scan_prefix(&self, _prefix: &[u8]) -> crate::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(vec![])
    }
    pub async fn index_vector(&self, _key: &[u8], _vector: &[f32]) -> crate::Result<()> {
        Ok(())
    }
    pub async fn flush(&self) -> crate::Result<()> {
        Ok(())
    }
    pub async fn compact(&self) -> crate::Result<()> {
        Ok(())
    }
}

// Legacy ShardManager for backward compat with existing code
pub struct ShardManager {
    num_shards: usize,
}

impl ShardManager {
    pub async fn new(_config: &crate::config::Config) -> crate::Result<Self> {
        Ok(Self { num_shards: num_cpus::get() })
    }

    pub fn get_shard(&self, _key: &[u8]) -> Arc<ShardHandle> {
        Arc::new(ShardHandle)
    }

    pub fn all_shards(&self) -> Vec<Arc<ShardHandle>> {
        (0..self.num_shards).map(|_| Arc::new(ShardHandle)).collect()
    }

    pub async fn delete_prefix(&self, _prefix: &[u8]) -> crate::Result<()> {
        Ok(())
    }

    pub fn search_vector(&self, _query: &[f32], _k: usize) -> Vec<(Vec<u8>, f32)> {
        vec![]
    }
}
