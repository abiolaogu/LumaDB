use async_trait::async_trait;
use crate::Value;
use crate::Result;

/// Represents a query to be executed by the backend engine
#[derive(Debug, Clone)]
pub struct QueryRequest {
    pub query: String,
    pub params: Vec<Value>, 
}

/// Represents a result from the backend engine
#[derive(Debug)]
pub struct QueryResult {
    pub rows: Vec<Vec<Value>>,
    pub row_count: usize,
}

#[async_trait]
pub trait QueryProcessor: Send + Sync {
    async fn process(&self, request: QueryRequest) -> Result<QueryResult>;
}

pub struct MockQueryProcessor;

#[async_trait]
impl QueryProcessor for MockQueryProcessor {
    async fn process(&self, request: QueryRequest) -> Result<QueryResult> {
        tracing::info!("EXECUTING MOCK QUERY: {}", request.query);
        Ok(QueryResult {
            rows: vec![],
            row_count: 0,
        })
    }
}
