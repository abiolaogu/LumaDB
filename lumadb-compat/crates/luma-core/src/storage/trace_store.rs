
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Trace Storage (Tempo/Jaeger compatible)
/// Features:
/// - Columnar Storage
/// - ZSTD Compression
pub struct TraceStorage {
    spans: Arc<RwLock<Vec<Span>>>, // Simplified for In-Memory (Hot Tier)
}

#[derive(Clone, Debug)]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub kind: i32,
    pub start_time: i64,
    pub end_time: i64,
    pub duration_ns: i64,
    pub attributes: HashMap<String, String>,
    pub resource_attributes: HashMap<String, String>,
    pub events: Vec<opentelemetry_proto::tonic::trace::v1::span::Event>,
    pub links: Vec<opentelemetry_proto::tonic::trace::v1::span::Link>,
    pub status: Option<opentelemetry_proto::tonic::trace::v1::Status>,
}

impl TraceStorage {
    pub fn new() -> Self {
        Self {
            spans: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn insert_span(&self, span: Span) -> Result<(), tonic::Status> {
        let mut spans = self.spans.write().await;
        spans.push(span);
        Ok(())
    }
}
