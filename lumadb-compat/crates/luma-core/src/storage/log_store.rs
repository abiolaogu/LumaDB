
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Log Storage (Loki/Elasticsearch compatible)
/// Features:
/// - Label-based indexing (Loki style)
/// - Full-text search capabilities (Bitmaps)
pub struct LogStorage {
    logs: Arc<RwLock<Vec<LogEntry>>>,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: i64,
    pub severity: String,
    pub message: String,
    pub attributes: HashMap<String, String>,
    pub resource_attributes: HashMap<String, String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
}

impl LogStorage {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn insert_log(&self, log: LogEntry) -> Result<(), tonic::Status> {
        let mut logs = self.logs.write().await;
        logs.push(log);
        Ok(())
    }
}
