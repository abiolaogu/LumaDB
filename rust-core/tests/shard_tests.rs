//! Unit tests for shard-per-core architecture
//!
//! Tests core affinity, message bus, router, and coordinator.

use luma_core::shard::{
    core_affinity::{CpuTopology, CoreAffinity},
    message_bus::ShardMessageBus,
    router::ShardRouter,
    core_shard::{CoreShard, ShardMessage},
};

#[cfg(test)]
mod affinity_tests {
    use super::*;

    #[test]
    fn test_cpu_topology_detection() {
        let topology = CpuTopology::detect();
        assert!(topology.logical_cores() >= 1);
    }

    #[test]
    fn test_core_affinity_create() {
        let affinity = CoreAffinity::new(0);
        // Just ensure it doesn't panic
        // Actually applying affinity requires running on Linux
    }
}

#[cfg(test)]
mod router_tests {
    use super::*;

    #[test]
    fn test_router_deterministic() {
        let router = ShardRouter::new(16);
        let key = b"test_key";
        
        let shard1 = router.route_key(key);
        let shard2 = router.route_key(key);
        
        assert_eq!(shard1, shard2, "Same key should always route to same shard");
    }

    #[test]
    fn test_router_distribution() {
        let router = ShardRouter::new(16);
        let mut counts = vec![0usize; 16];
        
        // Generate many keys and check distribution
        for i in 0..10000 {
            let key = format!("key_{}", i);
            let shard = router.route_key(key.as_bytes());
            counts[shard] += 1;
        }
        
        // Verify reasonable distribution (no shard should have 0)
        for (shard, count) in counts.iter().enumerate() {
            assert!(*count > 0, "Shard {} has no keys", shard);
        }
    }

    #[test]
    fn test_route_range_all_shards() {
        let router = ShardRouter::new(4);
        let shards = router.route_range(b"a", b"z");
        
        // For hash partitioning, range queries hit all shards
        assert_eq!(shards, vec![0, 1, 2, 3]);
    }
}

#[cfg(test)]
mod message_bus_tests {
    use super::*;

    #[test]
    fn test_message_bus_creation() {
        let (bus, receivers) = ShardMessageBus::new(4);
        assert_eq!(receivers.len(), 4);
    }

    #[test]
    fn test_send_to_shard() {
        let (bus, receivers) = ShardMessageBus::new(4);
        
        let msg = ShardMessage::Put(b"key".to_vec(), b"value".to_vec());
        bus.send_to_shard(0, msg.clone()).unwrap();
        
        let received = receivers[0].try_recv();
        assert!(received.is_ok());
    }
}

#[cfg(test)]
mod core_shard_tests {
    use super::*;

    #[test]
    fn test_core_shard_creation() {
        let shard = CoreShard::new(0);
        // Shard should be created without panicking
    }
}
