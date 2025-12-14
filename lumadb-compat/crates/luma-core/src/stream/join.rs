
use crate::query::batch::LumaBatch;

pub struct StreamJoin {
    pub left_stream: String,
    pub right_stream: String,
    pub window_ms: i64, // Join window (e.g., within 5s)
}

impl StreamJoin {
    pub fn new(left: &str, right: &str, window_ms: i64) -> Self {
        Self {
            left_stream: left.to_string(),
            right_stream: right.to_string(),
            window_ms,
        }
    }
    
    pub fn process(&self, left_batch: &LumaBatch, right_batch: &LumaBatch) {
        println!("Joining {} and {} within {}ms", self.left_stream, self.right_stream, self.window_ms);
        // TODO: Hash Join based on time window
    }
}
