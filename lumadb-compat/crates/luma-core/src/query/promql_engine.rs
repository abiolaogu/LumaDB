
use crate::parsing::promql::PromQLParser;
use crate::query::executor::QueryExecutor;
use crate::ir::{QueryPlan, Operation}; // Assuming these are what parser returns or we map to
use std::sync::Arc;

pub struct PromQLEngine {
    executor: Arc<QueryExecutor>,
}

impl PromQLEngine {
    pub fn new(executor: Arc<QueryExecutor>) -> Self {
        Self { executor }
    }

    pub async fn execute(&self, query_str: &str) -> Result<(), String> {
        // 1. Parse
        let op = PromQLParser::parse(query_str).map_err(|e| e.to_string())?;
        
        // 2. Plan (simple wrap for now)
        let plan = QueryPlan { steps: vec![op] };
        
        // 3. Execute
        self.executor.execute(plan).await.map(|_| ())
    }
}
