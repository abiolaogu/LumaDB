
use crate::ir::{QueryPlan, Operation};

#[derive(Default)]
pub struct QueryPlanner;

impl QueryPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn plan(&self, op: Operation) -> QueryPlan {
        // In the future, this would apply optimizations like:
        // - Predicate Pushdown
        // - Column Pruning
        // - Join Reordering
        
        // For now, wrap in a simple plan
        QueryPlan {
            steps: vec![op], 
        }
    }
}
