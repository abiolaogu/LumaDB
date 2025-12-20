use crossbeam::channel::{Sender, Receiver, unbounded, bounded};
use std::sync::Arc;
use crate::shard::core_shard::ShardMessage;

/// Bus for inter-shard communication
pub struct ShardMessageBus {
    senders: Vec<Sender<ShardMessage>>,
}

impl ShardMessageBus {
    pub fn new(num_shards: usize) -> (Self, Vec<Receiver<ShardMessage>>) {
        let mut senders = Vec::with_capacity(num_shards);
        let mut receivers = Vec::with_capacity(num_shards);

        for _ in 0..num_shards {
            // Unbounded channel preferred for throughput, 
            // but in prod might want bounded for backpressure.
            // Using unbounded to avoid deadlocks in scatter-gather.
            let (s, r) = unbounded();
            senders.push(s);
            receivers.push(r);
        }

        (Self { senders }, receivers)
    }

    /// Send message to specific shard
    pub fn send_to_shard(&self, shard_id: usize, msg: ShardMessage) -> Result<(), crossbeam::channel::SendError<ShardMessage>> {
        if shard_id >= self.senders.len() {
             // Silently drop or error? Ideally panic or Result.
             // For high perf, caller should ensure validity.
             return Ok(());
        }
        self.senders[shard_id].send(msg)
    }

    /// Broadcast to all shards
    pub fn broadcast(&self, msg: ShardMessage) {
        for sender in &self.senders {
        let _ = sender.send(msg.clone());
        }
    }
}
