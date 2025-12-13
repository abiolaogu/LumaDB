use crate::{Database, Document, Result};
use std::sync::Arc;
use super::query::{QueryParser, Executor, ExecutionResult};

pub struct Translator {
    executor: Executor,
}

impl Translator {
    pub fn new(db: Arc<Database>) -> Self {
        Self { 
            executor: Executor::new(db) 
        }
    }

    /// Execute a SQL query
    pub async fn execute_sql(&self, sql: &str) -> Result<Vec<Document>> {
        let plan = QueryParser::parse_sql(sql)?;
        let result = self.executor.execute(plan).await?;
        
        match result {
            ExecutionResult::Select(docs) => Ok(docs),
            ExecutionResult::Modify { .. } => Ok(vec![]),
            ExecutionResult::Ping => Ok(vec![]),
        }
    }

    /// Execute a Mongo Command (BSON)
    pub async fn execute_mongo(&self, cmd: bson::Document) -> Result<bson::Document> {
        let plan = QueryParser::parse_mongo(cmd)?;
        let result = self.executor.execute(plan).await?;
        
        match result {
             ExecutionResult::Select(docs) => {
                 // Convert back to BSON
                let mut result_docs = Vec::new();
                for d in docs {
                    let mut b_doc = bson::Document::new();
                    for (k, v) in d.data {
                        if let Ok(bson_val) = bson::to_bson(&v) {
                            b_doc.insert(k, bson_val);
                        }
                    }
                    result_docs.push(bson::Bson::Document(b_doc));
                }
                
                Ok(bson::doc! { 
                    "ok": 1, 
                    "cursor": { 
                        "firstBatch": result_docs,
                        "id": 0i64,
                        "ns": "db.coll"
                    } 
                })
             },
             ExecutionResult::Modify { affected } => {
                 Ok(bson::doc! { "ok": 1, "n": affected as i64 })
             },
             ExecutionResult::Ping => {
                 Ok(bson::doc! { "ok": 1 })
             }
        }
    }
}
