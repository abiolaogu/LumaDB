
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use crate::Value;
use crate::ir::{Expression, BinaryOperator};
use hyperloglog::HyperLogLog;
use tdigest::TDigest;

pub type Row = HashMap<String, Value>;

/// Materialized view definition
pub struct MaterializedView {
    pub name: String,
    pub source_table: String,
    pub query: ViewQuery,
    pub state: Arc<RwLock<ViewState>>,
    pub trigger: ViewTrigger,
}

pub struct ViewQuery {
    pub select: Vec<SelectExpr>,
    pub from: String,
    pub where_clause: Option<Expression>,
    pub group_by: Vec<String>,
    pub window: Option<WindowSpec>,
}

#[derive(Clone)]
pub enum SelectExpr {
    Column(String),
    Aggregate {
        function: AggregateFunction,
        column: String,
        alias: Option<String>,
    },
    Expression {
        expr: Expression,
        alias: String,
    },
}

#[derive(Clone, Debug)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    CountDistinct,
    ApproxCountDistinct,
    Percentile(f64),
}

#[derive(Clone)]
pub enum WindowSpec {
    Tumbling { size: Duration },
    Sliding { size: Duration, slide: Duration },
    Session { gap: Duration },
}

#[derive(Clone)]
pub enum ViewTrigger {
    OnInsert,
    Periodic(Duration),
    AfterRows(usize),
    Manual,
}

pub struct ViewState {
    pub groups: HashMap<Vec<Value>, GroupAggregates>,
    pub last_updated: i64,
    pub rows_processed: u64,
}

pub struct GroupAggregates {
    pub values: HashMap<String, Value>,
    // Internal state for complex aggs (e.g. TDigest, HLL) could be stored serialized in Value::Bytes
    // or we could use a separate map if we want to keep them as structs.
    // For now, we will assume serialization into Value::Bytes for persistence support.
}

impl MaterializedView {
    pub fn new(name: String, query: ViewQuery, trigger: ViewTrigger) -> Self {
        Self {
            name,
            source_table: query.from.clone(),
            query,
            state: Arc::new(RwLock::new(ViewState {
                groups: HashMap::new(),
                last_updated: chrono::Utc::now().timestamp_micros(),
                rows_processed: 0,
            })),
            trigger,
        }
    }

    pub fn on_insert(&self, rows: &[Row]) {
        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("MaterializedView: failed to acquire write lock: {}", e);
                return;
            }
        };

        for row in rows {
            if let Some(filter) = &self.query.where_clause {
                if !evaluate_expr(filter, row) {
                    continue;
                }
            }

            let group_key = self.compute_group_key(row);
            let aggregates = state.groups.entry(group_key).or_insert_with(|| GroupAggregates {
                values: HashMap::new(),
            });

            for select_expr in &self.query.select {
                match select_expr {
                    SelectExpr::Aggregate { function, column, alias } => {
                        let agg_name = match alias {
                            Some(a) => a.clone(),
                            None => format!("{:?}_{}", function, column),
                        };
                        self.update_aggregate(aggregates, &agg_name, function, column, row);
                    }
                    _ => {}
                }
            }
            state.rows_processed += 1;
        }
        state.last_updated = chrono::Utc::now().timestamp_micros();
    }

    fn compute_group_key(&self, row: &Row) -> Vec<Value> {
        self.query.group_by.iter()
            .map(|col| row.get(col).cloned().unwrap_or(Value::Null))
            .collect()
    }

    fn update_aggregate(&self, aggregates: &mut GroupAggregates, agg_name: &str, function: &AggregateFunction, column: &str, row: &Row) {
        // Handle "Count(*)" where column is "*"
        let value = if column == "*" {
            Value::Int64(1)
        } else {
            row.get(column).cloned().unwrap_or(Value::Null)
        };

        match function {
            AggregateFunction::Count => {
                let current = aggregates.values.get(agg_name).and_then(|v| v.as_i64()).unwrap_or(0);
                aggregates.values.insert(agg_name.to_string(), Value::Int64(current + 1));
            }
            AggregateFunction::Sum => {
                let current = aggregates.values.get(agg_name).and_then(|v| as_f64(v)).unwrap_or(0.0);
                let new_val = as_f64(&value).unwrap_or(0.0);
                aggregates.values.insert(agg_name.to_string(), Value::Float64(current + new_val));
            }
            AggregateFunction::Max => {
                // Simplified max logic for numeric/comparable types
                // Real implementation would implement PartialOrd for Value
                 // Placeholder
            }
             AggregateFunction::Min => {
                // Placeholder
            }
            _ => {} // Implement others as needed
        }
    }
}

// Helper to evaluate expression against a Row (HashMap)
// Minimal implementation
fn evaluate_expr(expr: &Expression, row: &Row) -> bool {
    match expr {
        Expression::BinaryOp { op, left, right } => {
            let l_val = eval_value(left, row);
            let r_val = eval_value(right, row);
            
            match op {
                BinaryOperator::Eq => l_val == r_val,
                BinaryOperator::Gt => as_f64(&l_val).unwrap_or(0.0) > as_f64(&r_val).unwrap_or(0.0),
                _ => false // TODO: Complete Logic
            }
        },
        _ => true
    }
}

fn eval_value(expr: &Expression, row: &Row) -> Value {
    match expr {
        Expression::Column(name) => row.get(name).cloned().unwrap_or(Value::Null),
        Expression::Literal(v) => v.clone(),
        _ => Value::Null
    }
}

fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Float64(f) => Some(*f),
        Value::Float32(f) => Some(*f as f64),
        Value::Int64(i) => Some(*i as f64),
        Value::Int32(i) => Some(*i as f64),
        _ => None
    }
}
