use crate::Value;
use serde::{Serialize, Deserialize};

pub mod planner;
pub use planner::QueryPlanner;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub steps: Vec<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Scan {
        table: String,
        alias: Option<String>,
        filter: Option<Box<Expression>>,
        columns: Vec<String>,
    },
    Project {
        exprs: Vec<Expression>,
    },
    Filter {
        predicate: Expression,
    },
    // Aggregations
    Aggregate {
        group_by: Vec<Expression>,
        aggregates: Vec<AggregateFunction>,
    },
    // Specialized for Time-Series
    WindowAggregate {
        window: String, // "5m", "1h"
        function: AggregateFunction,
    },
     // Specialized for Vector
    VectorSearch {
        column: String,
        vector: Vec<f32>,
        k: usize,
    },
    // Specialized for LumaText (Full-Text)
    TextSearch {
        query: String, // "search terms"
    },
    Sort {
        by: Vec<SortExpression>,
    },
    Limit {
        limit: usize,
        offset: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expression {
    Column(String),
    Literal(Value),
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    FunctionCall {
        name: String,
        args: Vec<Expression>,
    },
    Wildcard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryOperator {
    Eq, Neq, Gt, Lt, Gte, Lte,
    And, Or,
    Plus, Minus, Multiply, Divide, Modulo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateFunction {
    pub name: String, // sum, count, avg, min, max, hll_count, etc.
    pub args: Vec<Expression>,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortExpression {
    pub expr: Expression,
    pub asc: bool,
}
