//! TDengine Window Functions
//!
//! Implements all TDengine windowing capabilities:
//! - INTERVAL (tumbling/sliding time windows)
//! - SESSION (session-based windows)
//! - STATE_WINDOW (state-change windows)
//! - EVENT_WINDOW (event-driven windows)
//! - COUNT_WINDOW (row-count windows)

use std::collections::HashMap;
use super::parser::FillClause;
use super::aggregation::compute_aggregation;

/// Window processor trait
pub trait WindowProcessor: Send + Sync {
    fn process(&self, data: &[TimeSeriesRow], agg_exprs: &[AggExpr]) -> Vec<WindowResult>;
    fn window_type(&self) -> &'static str;
}

/// Time-series row
#[derive(Clone, Debug)]
pub struct TimeSeriesRow {
    pub timestamp: i64,
    pub values: Vec<Value>,
    pub tags: HashMap<String, String>,
    pub table_name: String,
}

/// Aggregated value
#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String),
    Binary(Vec<u8>),
    Timestamp(i64),
}

impl Value {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(v) => Some(*v as f64),
            Value::UInt(v) => Some(*v as f64),
            Value::Float(v) => Some(*v),
            _ => None,
        }
    }
    
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(v) => Some(*v),
            Value::UInt(v) => Some(*v as i64),
            Value::Float(v) => Some(*v as i64),
            Value::Timestamp(v) => Some(*v),
            _ => None,
        }
    }
    
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::UInt(a), Value::UInt(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Binary(a), Value::Binary(b)) => a == b,
            (Value::Timestamp(a), Value::Timestamp(b)) => a == b,
            _ => false,
        }
    }
}

/// Aggregation expression
#[derive(Clone, Debug)]
pub struct AggExpr {
    pub function: AggFunction,
    pub column: String,
    pub alias: Option<String>,
}

#[derive(Clone, Debug)]
pub enum AggFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    First,
    Last,
    Stddev,
    Spread,
    Percentile(f64),
    Apercentile(f64),
    Top(usize),
    Bottom(usize),
    Diff,
    Derivative,
    Irate,
    Twa,           // Time-weighted average
    Elapsed,
    LastRow,
    Interp,
    Mode,
    Hyperloglog,
    Histogram,
    Sample(usize),
    Tail(usize),
    Unique,
    StateCount,
    StateDuration,
}

/// Window result
#[derive(Clone, Debug)]
pub struct WindowResult {
    pub window_start: i64,
    pub window_end: i64,
    pub window_duration: i64,
    pub values: HashMap<String, Value>,
    pub partition_key: Option<String>,
}

// ============================================================================
// INTERVAL WINDOW (Time-based)
// ============================================================================

pub struct IntervalWindow {
    pub interval_ms: i64,
    pub offset_ms: i64,
    pub sliding_ms: Option<i64>,
    pub fill: FillClause,
}

impl IntervalWindow {
    pub fn new(interval: &str, offset: Option<&str>, sliding: Option<&str>) -> Self {
        Self {
            interval_ms: parse_duration(interval),
            offset_ms: offset.map(parse_duration).unwrap_or(0),
            sliding_ms: sliding.map(parse_duration),
            fill: FillClause::None,
        }
    }
    
    pub fn with_fill(mut self, fill: FillClause) -> Self {
        self.fill = fill;
        self
    }
}

impl WindowProcessor for IntervalWindow {
    fn process(&self, data: &[TimeSeriesRow], agg_exprs: &[AggExpr]) -> Vec<WindowResult> {
        if data.is_empty() {
            return Vec::new();
        }
        
        let sliding = self.sliding_ms.unwrap_or(self.interval_ms);
        let min_ts = data.iter().map(|r| r.timestamp).min().unwrap();
        let max_ts = data.iter().map(|r| r.timestamp).max().unwrap();
        
        // Align to window boundary
        let start = ((min_ts - self.offset_ms) / self.interval_ms) * self.interval_ms + self.offset_ms;
        
        let mut results = Vec::new();
        let mut window_start = start;
        
        while window_start <= max_ts {
            let window_end = window_start + self.interval_ms;
            
            // Collect rows in this window
            let window_rows: Vec<&TimeSeriesRow> = data
                .iter()
                .filter(|r| r.timestamp >= window_start && r.timestamp < window_end)
                .collect();
            
            // Compute aggregations
            let mut values = HashMap::new();
            values.insert("_wstart".to_string(), Value::Timestamp(window_start));
            values.insert("_wend".to_string(), Value::Timestamp(window_end));
            values.insert("_wduration".to_string(), Value::Int(self.interval_ms));
            
            if !window_rows.is_empty() {
                for agg in agg_exprs {
                    let result = compute_aggregation(&window_rows, agg);
                    let key = agg.alias.clone().unwrap_or_else(|| format!("{:?}({})", agg.function, agg.column));
                    values.insert(key, result);
                }
            } else {
                // Apply fill strategy
                for agg in agg_exprs {
                    let key = agg.alias.clone().unwrap_or_else(|| format!("{:?}({})", agg.function, agg.column));
                    let fill_value = match &self.fill {
                        FillClause::None => continue,
                        FillClause::Null | FillClause::NullF => Value::Null,
                        FillClause::Value(vals) => {
                            vals.first().map(|v| Value::Float(*v)).unwrap_or(Value::Null)
                        }
                        FillClause::Prev => {
                            results.last()
                                .and_then(|r: &WindowResult| r.values.get(&key).cloned())
                                .unwrap_or(Value::Null)
                        }
                        FillClause::Next => {
                            // Look ahead for next non-empty window
                            Value::Null // Simplified
                        }
                        FillClause::Linear => {
                            // Linear interpolation between surrounding values
                            Value::Null // Simplified
                        }
                        FillClause::Nearest => {
                            // Use nearest available value
                            Value::Null // Simplified
                        }
                    };
                    values.insert(key, fill_value);
                }
            }
            
            results.push(WindowResult {
                window_start,
                window_end,
                window_duration: self.interval_ms,
                values,
                partition_key: None,
            });
            
            window_start += sliding;
        }
        
        results
    }
    
    fn window_type(&self) -> &'static str {
        "INTERVAL"
    }
}

// ============================================================================
// SESSION WINDOW
// ============================================================================

pub struct SessionWindow {
    pub ts_column: String,
    pub tolerance_ms: i64,
}

impl SessionWindow {
    pub fn new(ts_col: &str, tolerance: &str) -> Self {
        Self {
            ts_column: ts_col.to_string(),
            tolerance_ms: parse_duration(tolerance),
        }
    }
}

impl WindowProcessor for SessionWindow {
    fn process(&self, data: &[TimeSeriesRow], agg_exprs: &[AggExpr]) -> Vec<WindowResult> {
        if data.is_empty() {
            return Vec::new();
        }
        
        // Sort by timestamp
        let mut sorted: Vec<&TimeSeriesRow> = data.iter().collect();
        sorted.sort_by_key(|r| r.timestamp);
        
        let mut results = Vec::new();
        let mut session_start = sorted[0].timestamp;
        let mut session_rows: Vec<&TimeSeriesRow> = vec![sorted[0]];
        let mut prev_ts = sorted[0].timestamp;
        
        for row in sorted.iter().skip(1) {
            if row.timestamp - prev_ts > self.tolerance_ms {
                // New session
                let window_end = prev_ts;
                results.push(self.create_result(session_start, window_end, &session_rows, agg_exprs));
                
                session_start = row.timestamp;
                session_rows.clear();
            }
            session_rows.push(row);
            prev_ts = row.timestamp;
        }
        
        // Final session
        if !session_rows.is_empty() {
            results.push(self.create_result(session_start, prev_ts, &session_rows, agg_exprs));
        }
        
        results
    }
    
    fn window_type(&self) -> &'static str {
        "SESSION"
    }
}

impl SessionWindow {
    fn create_result(&self, start: i64, end: i64, rows: &[&TimeSeriesRow], agg_exprs: &[AggExpr]) -> WindowResult {
        let mut values = HashMap::new();
        values.insert("_wstart".to_string(), Value::Timestamp(start));
        values.insert("_wend".to_string(), Value::Timestamp(end));
        values.insert("_wduration".to_string(), Value::Int(end - start));
        
        for agg in agg_exprs {
            let result = compute_aggregation(rows, agg);
            let key = agg.alias.clone().unwrap_or_else(|| format!("{:?}({})", agg.function, agg.column));
            values.insert(key, result);
        }
        
        WindowResult {
            window_start: start,
            window_end: end,
            window_duration: end - start,
            values,
            partition_key: None,
        }
    }
}

// ============================================================================
// STATE WINDOW
// ============================================================================

pub struct StateWindow {
    pub state_column: String,
}

impl StateWindow {
    pub fn new(column: &str) -> Self {
        Self {
            state_column: column.to_string(),
        }
    }
}

impl WindowProcessor for StateWindow {
    fn process(&self, data: &[TimeSeriesRow], agg_exprs: &[AggExpr]) -> Vec<WindowResult> {
        if data.is_empty() {
            return Vec::new();
        }
        
        let mut sorted: Vec<&TimeSeriesRow> = data.iter().collect();
        sorted.sort_by_key(|r| r.timestamp);
        
        let mut results = Vec::new();
        let mut state_start = sorted[0].timestamp;
        let mut current_state = get_column_value(sorted[0], &self.state_column);
        let mut state_rows: Vec<&TimeSeriesRow> = vec![sorted[0]];
        
        for row in sorted.iter().skip(1) {
            let row_state = get_column_value(row, &self.state_column);
            
            if row_state != current_state {
                // State changed - close window
                let window_end = state_rows.last().map(|r| r.timestamp).unwrap_or(state_start);
                results.push(self.create_result(state_start, window_end, &current_state, &state_rows, agg_exprs));
                
                state_start = row.timestamp;
                current_state = row_state;
                state_rows.clear();
            }
            state_rows.push(row);
        }
        
        // Final state
        if !state_rows.is_empty() {
            let window_end = state_rows.last().map(|r| r.timestamp).unwrap_or(state_start);
            results.push(self.create_result(state_start, window_end, &current_state, &state_rows, agg_exprs));
        }
        
        results
    }
    
    fn window_type(&self) -> &'static str {
        "STATE_WINDOW"
    }
}

impl StateWindow {
    fn create_result(&self, start: i64, end: i64, state: &Value, rows: &[&TimeSeriesRow], agg_exprs: &[AggExpr]) -> WindowResult {
        let mut values = HashMap::new();
        values.insert("_wstart".to_string(), Value::Timestamp(start));
        values.insert("_wend".to_string(), Value::Timestamp(end));
        values.insert("_wduration".to_string(), Value::Int(end - start));
        values.insert(format!("{}_state", self.state_column), state.clone());
        
        for agg in agg_exprs {
            let result = compute_aggregation(rows, agg);
            let key = agg.alias.clone().unwrap_or_else(|| format!("{:?}({})", agg.function, agg.column));
            values.insert(key, result);
        }
        
        WindowResult {
            window_start: start,
            window_end: end,
            window_duration: end - start,
            values,
            partition_key: None,
        }
    }
}

// ============================================================================
// EVENT WINDOW
// ============================================================================

/// Event window with start/end conditions
pub struct EventWindow {
    pub start_fn: Box<dyn Fn(&TimeSeriesRow) -> bool + Send + Sync>,
    pub end_fn: Box<dyn Fn(&TimeSeriesRow) -> bool + Send + Sync>,
    pub true_for_ms: Option<i64>,
}

impl EventWindow {
    pub fn new<F, G>(start_fn: F, end_fn: G) -> Self
    where
        F: Fn(&TimeSeriesRow) -> bool + Send + Sync + 'static,
        G: Fn(&TimeSeriesRow) -> bool + Send + Sync + 'static,
    {
        Self {
            start_fn: Box::new(start_fn),
            end_fn: Box::new(end_fn),
            true_for_ms: None,
        }
    }
    
    pub fn with_true_for(mut self, duration_ms: i64) -> Self {
        self.true_for_ms = Some(duration_ms);
        self
    }
}

impl WindowProcessor for EventWindow {
    fn process(&self, data: &[TimeSeriesRow], agg_exprs: &[AggExpr]) -> Vec<WindowResult> {
        if data.is_empty() {
            return Vec::new();
        }
        
        let mut sorted: Vec<&TimeSeriesRow> = data.iter().collect();
        sorted.sort_by_key(|r| r.timestamp);
        
        let mut results = Vec::new();
        let mut in_window = false;
        let mut window_start = 0i64;
        let mut window_rows: Vec<&TimeSeriesRow> = Vec::new();
        
        for row in sorted {
            if !in_window && (self.start_fn)(row) {
                in_window = true;
                window_start = row.timestamp;
                window_rows.clear();
            }
            
            if in_window {
                window_rows.push(row);
                
                if (self.end_fn)(row) {
                    let window_end = row.timestamp;
                    let duration = window_end - window_start;
                    
                    // Check true_for constraint
                    if self.true_for_ms.map(|t| duration >= t).unwrap_or(true) {
                        results.push(self.create_result(window_start, window_end, &window_rows, agg_exprs));
                    }
                    
                    in_window = false;
                }
            }
        }
        
        results
    }
    
    fn window_type(&self) -> &'static str {
        "EVENT_WINDOW"
    }
}

impl EventWindow {
    fn create_result(&self, start: i64, end: i64, rows: &[&TimeSeriesRow], agg_exprs: &[AggExpr]) -> WindowResult {
        let mut values = HashMap::new();
        values.insert("_wstart".to_string(), Value::Timestamp(start));
        values.insert("_wend".to_string(), Value::Timestamp(end));
        values.insert("_wduration".to_string(), Value::Int(end - start));
        
        for agg in agg_exprs {
            let result = compute_aggregation(rows, agg);
            let key = agg.alias.clone().unwrap_or_else(|| format!("{:?}({})", agg.function, agg.column));
            values.insert(key, result);
        }
        
        WindowResult {
            window_start: start,
            window_end: end,
            window_duration: end - start,
            values,
            partition_key: None,
        }
    }
}

// ============================================================================
// COUNT WINDOW
// ============================================================================

pub struct CountWindow {
    pub count: usize,
    pub sliding: Option<usize>,
}

impl CountWindow {
    pub fn new(count: i64, sliding: Option<i64>) -> Self {
        Self {
            count: count as usize,
            sliding: sliding.map(|s| s as usize),
        }
    }
}

impl WindowProcessor for CountWindow {
    fn process(&self, data: &[TimeSeriesRow], agg_exprs: &[AggExpr]) -> Vec<WindowResult> {
        if data.is_empty() {
            return Vec::new();
        }
        
        let mut sorted: Vec<&TimeSeriesRow> = data.iter().collect();
        sorted.sort_by_key(|r| r.timestamp);
        
        let sliding = self.sliding.unwrap_or(self.count);
        let mut results = Vec::new();
        let mut start_idx = 0;
        
        while start_idx < sorted.len() {
            let end_idx = (start_idx + self.count).min(sorted.len());
            let window_rows: Vec<&TimeSeriesRow> = sorted[start_idx..end_idx].to_vec();
            
            if !window_rows.is_empty() {
                let window_start = window_rows.first().unwrap().timestamp;
                let window_end = window_rows.last().unwrap().timestamp;
                
                let mut values = HashMap::new();
                values.insert("_wstart".to_string(), Value::Timestamp(window_start));
                values.insert("_wend".to_string(), Value::Timestamp(window_end));
                values.insert("_wduration".to_string(), Value::Int(window_end - window_start));
                
                for agg in agg_exprs {
                    let result = compute_aggregation(&window_rows, agg);
                    let key = agg.alias.clone().unwrap_or_else(|| format!("{:?}({})", agg.function, agg.column));
                    values.insert(key, result);
                }
                
                results.push(WindowResult {
                    window_start,
                    window_end,
                    window_duration: window_end - window_start,
                    values,
                    partition_key: None,
                });
            }
            
            start_idx += sliding;
        }
        
        results
    }
    
    fn window_type(&self) -> &'static str {
        "COUNT_WINDOW"
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

pub fn parse_duration(s: &str) -> i64 {
    let s = s.trim();
    
    // Handle special cases
    if s.is_empty() {
        return 0;
    }
    
    // Find where the number ends and unit begins
    let (num_str, unit) = if s.ends_with("ms") {
        (&s[..s.len()-2], "ms")
    } else if s.ends_with('a') {
        // 'a' is a TDengine alias for milliseconds
        (&s[..s.len()-1], "ms")
    } else if s.ends_with('s') {
        (&s[..s.len()-1], "s")
    } else if s.ends_with('m') {
        (&s[..s.len()-1], "m")
    } else if s.ends_with('h') {
        (&s[..s.len()-1], "h")
    } else if s.ends_with('d') {
        (&s[..s.len()-1], "d")
    } else if s.ends_with('w') {
        (&s[..s.len()-1], "w")
    } else if s.ends_with('n') {
        // nanoseconds
        let num: i64 = s[..s.len()-1].parse().unwrap_or(0);
        return num / 1_000_000; // Convert to ms
    } else if s.ends_with('u') {
        // microseconds
        let num: i64 = s[..s.len()-1].parse().unwrap_or(0);
        return num / 1_000; // Convert to ms
    } else {
        (s, "ms")
    };
    
    let num: i64 = num_str.trim().parse().unwrap_or(0);
    
    match unit {
        "ms" | "a" => num,
        "s" => num * 1000,
        "m" => num * 60 * 1000,
        "h" => num * 60 * 60 * 1000,
        "d" => num * 24 * 60 * 60 * 1000,
        "w" => num * 7 * 24 * 60 * 60 * 1000,
        _ => num,
    }
}

pub fn get_column_value(row: &TimeSeriesRow, column: &str) -> Value {
    // For wildcard or empty column, return first value
    if column == "*" || column.is_empty() {
        return row.values.first().cloned().unwrap_or(Value::Null);
    }
    
    // Try to find column by index (simplified)
    // In a real implementation, we'd have column name to index mapping
    row.values.first().cloned().unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("1s"), 1000);
        assert_eq!(parse_duration("5m"), 300000);
        assert_eq!(parse_duration("1h"), 3600000);
        assert_eq!(parse_duration("1d"), 86400000);
        assert_eq!(parse_duration("100ms"), 100);
        assert_eq!(parse_duration("1000a"), 1000);
    }
    
    #[test]
    fn test_interval_window() {
        let window = IntervalWindow::new("1m", None, None);
        
        let data = vec![
            TimeSeriesRow {
                timestamp: 0,
                values: vec![Value::Float(1.0)],
                tags: HashMap::new(),
                table_name: "test".to_string(),
            },
            TimeSeriesRow {
                timestamp: 30_000,
                values: vec![Value::Float(2.0)],
                tags: HashMap::new(),
                table_name: "test".to_string(),
            },
            TimeSeriesRow {
                timestamp: 60_000,
                values: vec![Value::Float(3.0)],
                tags: HashMap::new(),
                table_name: "test".to_string(),
            },
        ];
        
        let agg_exprs = vec![
            AggExpr {
                function: AggFunction::Avg,
                column: "value".to_string(),
                alias: Some("avg_value".to_string()),
            },
        ];
        
        let results = window.process(&data, &agg_exprs);
        assert_eq!(results.len(), 2);
    }
    
    #[test]
    fn test_session_window() {
        let window = SessionWindow::new("ts", "30s");
        
        let data = vec![
            TimeSeriesRow {
                timestamp: 0,
                values: vec![Value::Float(1.0)],
                tags: HashMap::new(),
                table_name: "test".to_string(),
            },
            TimeSeriesRow {
                timestamp: 10_000,
                values: vec![Value::Float(2.0)],
                tags: HashMap::new(),
                table_name: "test".to_string(),
            },
            TimeSeriesRow {
                timestamp: 100_000, // Gap > 30s, new session
                values: vec![Value::Float(3.0)],
                tags: HashMap::new(),
                table_name: "test".to_string(),
            },
        ];
        
        let results = window.process(&data, &[]);
        assert_eq!(results.len(), 2);
    }
    
    #[test]
    fn test_count_window() {
        let window = CountWindow::new(2, None);
        
        let data: Vec<TimeSeriesRow> = (0..5).map(|i| TimeSeriesRow {
            timestamp: i * 1000,
            values: vec![Value::Float(i as f64)],
            tags: HashMap::new(),
            table_name: "test".to_string(),
        }).collect();
        
        let results = window.process(&data, &[]);
        assert_eq!(results.len(), 3); // 5 rows / 2 = 3 windows (with overlap handling)
    }
}
