use super::ir::*;
use crate::{Database, Result, Document};
use std::sync::Arc;

pub struct Executor {
    db: Arc<Database>,
}

impl Executor {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn execute(&self, plan: QueryPlan) -> Result<ExecutionResult> {
        match plan {
            QueryPlan::Select(plan) => {
                let docs = self.db.scan(&plan.collection, |_| true).await?;
                // In a real implementation, we would filter and project here.
                // For now we pass the docs and the requested projection so the caller (pg_wire) can format it.
                Ok(ExecutionResult::Select(docs, plan.projection))
            },
            QueryPlan::Insert(plan) => {
                let count = plan.documents.len();
                self.db.batch_insert(&plan.collection, plan.documents).await?;
                Ok(ExecutionResult::Modify { affected: count as u64 })
            },
            QueryPlan::Update(_) => {
                // Todo
                Ok(ExecutionResult::Modify { affected: 0 })
            },
            QueryPlan::Delete(_) => {
                // Todo
                Ok(ExecutionResult::Modify { affected: 0 })
            },
            QueryPlan::Ping => {
                 Ok(ExecutionResult::Ping)
            },
            QueryPlan::Schema(_) => {
                Ok(ExecutionResult::Ping)
            }
        }
    }
}

pub enum ExecutionResult {
    Select(Vec<Document>, Option<Vec<String>>),
    Modify { affected: u64 },
    Ping,
}
