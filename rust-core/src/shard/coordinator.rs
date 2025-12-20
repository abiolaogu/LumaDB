use crate::shard::core_shard::{CoreShard, ShardMessage};
use crate::shard::message_bus::ShardMessageBus;
use crate::shard::router::ShardRouter;
use crate::Result;
use crossbeam::channel::{bounded, RecvError};
use std::thread;

pub struct ShardCoordinator {
    router: ShardRouter,
    bus: ShardMessageBus,
    // Keep handles to join later
    worker_handles: Vec<thread::JoinHandle<()>>,
}

impl ShardCoordinator {
    pub fn new(num_shards: usize) -> Self {
        let router = ShardRouter::new(num_shards);
        let (bus, receivers) = ShardMessageBus::new(num_shards);
        
        let mut worker_handles = Vec::new();

        for (i, rx) in receivers.into_iter().enumerate() {
            let handle = thread::spawn(move || {
                let mut shard = CoreShard::new(i);
                shard.start_loop(rx);
            });
            worker_handles.push(handle);
        }

        Self {
            router,
            bus,
            worker_handles,
        }
    }

    pub fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let shard_id = self.router.route_key(&key);
        // Using explicit unwrap for now, but should handle error
        self.bus.send_to_shard(shard_id, ShardMessage::Put(key, value))
            .map_err(|_| crate::LumaError::Internal("Shard channel closed".into()))?;
        Ok(())
    }

    pub fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>> {
        let shard_id = self.router.route_key(&key);
        let (tx, rx) = bounded(1);
        
        self.bus.send_to_shard(shard_id, ShardMessage::Get(key, tx))
            .map_err(|_| crate::LumaError::Internal("Shard channel closed".into()))?;
            
        rx.recv().map_err(|_| crate::LumaError::Internal("No response from shard".into()))
    }
}
