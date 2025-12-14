
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::query::batch::LumaBatch;
use crate::types::Value;

#[derive(Debug, Clone)]
pub enum WindowType {
    Tumbling { size_ms: i64 },
    Sliding { size_ms: i64, slide_ms: i64 },
    Session { timeout_ms: i64 },
}

pub struct WindowedView {
    pub name: String,
    pub window_type: WindowType,
    pub aggregate_expr: String, // e.g., "sum(value)"
    // window_start -> group_key -> state
    pub windows: Arc<RwLock<HashMap<i64, HashMap<Vec<Value>, AggregateState>>>>,
}

#[derive(Debug, Clone)]
pub struct AggregateState {
    pub count: i64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
}

impl AggregateState {
    pub fn new() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            min: f64::MAX,
            max: f64::MIN,
        }
    }
    
    pub fn update(&mut self, val: f64) {
        self.count += 1;
        self.sum += val;
        if val < self.min { self.min = val; }
        if val > self.max { self.max = val; }
    }
}

impl WindowedView {
    pub fn new(name: String, window_type: WindowType, aggregate_expr: String) -> Self {
        Self {
            name,
            window_type,
            aggregate_expr,
            windows: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn process_batch(&self, batch: &LumaBatch, timestamp_col_idx: usize, val_col_idx: usize) {
        // Mock processing: iterate rows, determine window, update state
        // In real impl: use vectorization
        println!("Processing batch for view {}", self.name);
    }
}
