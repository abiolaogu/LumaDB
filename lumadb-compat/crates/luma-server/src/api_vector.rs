
use warp::Filter;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct VectorSearchRequest {
    pub index: String,
    pub vector: Vec<f32>,
    pub k: usize,
}

#[derive(Debug, Serialize)]
pub struct VectorSearchResult {
    pub matches: Vec<VectorMatch>,
    pub took_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct VectorMatch {
    pub id: u64,
    pub score: f32,
}

use std::sync::Arc;
use luma_protocol_core::query::executor::QueryExecutor;
use luma_protocol_core::ir::{QueryPlan, Operation};

pub fn vector_routes(executor: Arc<QueryExecutor>) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let executor_filter = warp::any().map(move || executor.clone());

    let search = warp::path("v1")
        .and(warp::path("vector"))
        .and(warp::path("search"))
        .and(warp::post())
        .and(warp::body::json())
        .and(executor_filter)
        .and_then(handle_search);

    search
}

async fn handle_search(req: VectorSearchRequest, executor: Arc<QueryExecutor>) -> Result<impl warp::Reply, warp::Rejection> {
    // Construct Query Plan
    let op = Operation::VectorSearch { 
        column: "embeddings".to_string(), // Default for now
        vector: req.vector, 
        k: req.k 
    };
    let plan = QueryPlan { steps: vec![op] };

    let start = std::time::Instant::now();
    
    // Execute via Core Engine (returns LumaBatch)
    match executor.execute(plan).await {
        Ok(batch) => {
            // Convert LumaBatch to VectorMatches
            // Assuming simplified schema: [score]
            let mut matches = vec![];
            
            // Iterate rows via batch helper
            // In a real impl, we'd use zero-copy arrow access
            for (i, row) in batch.rows().enumerate() {
                if let Some(score_val) = row.get(0) {
                   let score = match score_val {
                       luma_protocol_core::Value::Float64(f) => *f as f32,
                       _ => 0.0,
                   };
                   matches.push(VectorMatch {
                       id: i as u64, // Mock ID based on index
                       score,
                   });
                }
            }
            
            let took_ms = start.elapsed().as_millis() as u64;
            Ok(warp::reply::json(&VectorSearchResult { matches, took_ms }))
        },
        Err(e) => {
            eprintln!("Query Execution Failed: {}", e);
            Err(warp::reject::not_found()) // Simplified error
        }
    }
}
