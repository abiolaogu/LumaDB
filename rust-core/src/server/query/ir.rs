use crate::types::{Value, Document};
use std::collections::HashMap;

/// The Unified Query Plan
#[derive(Debug, Clone)]
pub enum QueryPlan {
    Select(SelectPlan),
    Insert(InsertPlan),
    Update(UpdatePlan),
    Delete(DeletePlan),
    // Administrative commands
    Ping,
    Schema(String), // e.g. "SHOW TABLES"
}

#[derive(Debug, Clone)]
pub struct SelectPlan {
    pub collection: String,
    pub filter: Option<FilterExpression>,
    pub projection: Option<Vec<String>>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct InsertPlan {
    pub collection: String,
    pub documents: Vec<Document>,
}

#[derive(Debug, Clone)]
pub struct UpdatePlan {
    pub collection: String,
    pub filter: Option<FilterExpression>,
    pub update: UpdateExpression,
}

#[derive(Debug, Clone)]
pub struct DeletePlan {
    pub collection: String,
    pub filter: Option<FilterExpression>,
}

#[derive(Debug, Clone)]
pub enum FilterExpression {
    Eq(String, Value),
    Gt(String, Value),
    Lt(String, Value),
    And(Box<FilterExpression>, Box<FilterExpression>),
    Or(Box<FilterExpression>, Box<FilterExpression>),
}

#[derive(Debug, Clone)]
pub enum UpdateExpression {
    Set(HashMap<String, Value>),
    // Could add Inc, Mul, etc.
}
