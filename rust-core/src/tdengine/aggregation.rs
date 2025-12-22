//! TDengine Aggregation Functions
//!
//! Implements all TDengine time-series aggregation functions:
//! - Basic: COUNT, SUM, AVG, MIN, MAX, FIRST, LAST
//! - Statistical: STDDEV, SPREAD, PERCENTILE, APERCENTILE
//! - Time-series: TWA, DIFF, DERIVATIVE, IRATE, ELAPSED
//! - Selection: TOP, BOTTOM, LAST_ROW, MODE
//! - Sample: SAMPLE, TAIL, UNIQUE

use std::collections::HashMap;
use super::window::{TimeSeriesRow, Value, AggExpr, AggFunction, get_column_value};

/// Compute aggregation for a set of rows
pub fn compute_aggregation(rows: &[&TimeSeriesRow], agg: &AggExpr) -> Value {
    let values: Vec<f64> = rows
        .iter()
        .filter_map(|r| get_column_value(r, &agg.column).as_f64())
        .collect();
    
    if values.is_empty() {
        return Value::Null;
    }
    
    match &agg.function {
        AggFunction::Count => Value::Int(values.len() as i64),
        
        AggFunction::Sum => Value::Float(values.iter().sum()),
        
        AggFunction::Avg => Value::Float(values.iter().sum::<f64>() / values.len() as f64),
        
        AggFunction::Min => Value::Float(values.iter().cloned().fold(f64::INFINITY, f64::min)),
        
        AggFunction::Max => Value::Float(values.iter().cloned().fold(f64::NEG_INFINITY, f64::max)),
        
        AggFunction::First => rows.first()
            .map(|r| get_column_value(r, &agg.column))
            .unwrap_or(Value::Null),
        
        AggFunction::Last => rows.last()
            .map(|r| get_column_value(r, &agg.column))
            .unwrap_or(Value::Null),
        
        AggFunction::Stddev => compute_stddev(&values),
        
        AggFunction::Spread => compute_spread(&values),
        
        AggFunction::Percentile(p) => compute_percentile(&values, *p),
        
        AggFunction::Apercentile(p) => compute_apercentile(&values, *p),
        
        AggFunction::Top(n) => compute_top(&values, *n),
        
        AggFunction::Bottom(n) => compute_bottom(&values, *n),
        
        AggFunction::Diff => compute_diff(&values),
        
        AggFunction::Derivative => compute_derivative(rows, &agg.column),
        
        AggFunction::Irate => compute_irate(rows, &agg.column),
        
        AggFunction::Twa => compute_twa(rows, &agg.column),
        
        AggFunction::Elapsed => compute_elapsed(rows),
        
        AggFunction::LastRow => compute_last_row(rows, &agg.column),
        
        AggFunction::Interp => Value::Null, // Requires special handling with time range
        
        AggFunction::Mode => compute_mode(&values),
        
        AggFunction::Hyperloglog => Value::Int(estimate_cardinality(&values)),
        
        AggFunction::Histogram => compute_histogram(&values),
        
        AggFunction::Sample(n) => compute_sample(&values, *n),
        
        AggFunction::Tail(n) => compute_tail(&values, *n),
        
        AggFunction::Unique => compute_unique(&values),
        
        AggFunction::StateCount => Value::Int(rows.len() as i64),
        
        AggFunction::StateDuration => compute_state_duration(rows),
    }
}

/// Standard deviation
fn compute_stddev(values: &[f64]) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    Value::Float(variance.sqrt())
}

/// Spread (max - min)
fn compute_spread(values: &[f64]) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    Value::Float(max - min)
}

/// Percentile using linear interpolation
fn compute_percentile(values: &[f64], p: f64) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let idx = ((p / 100.0) * (sorted.len() - 1) as f64) as usize;
    Value::Float(sorted[idx.min(sorted.len() - 1)])
}

/// Approximate percentile (same as percentile for now)
fn compute_apercentile(values: &[f64], p: f64) -> Value {
    compute_percentile(values, p)
}

/// Top N values
fn compute_top(values: &[f64], n: usize) -> Value {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
    sorted.truncate(n);
    Value::String(format!("{:?}", sorted))
}

/// Bottom N values
fn compute_bottom(values: &[f64], n: usize) -> Value {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    sorted.truncate(n);
    Value::String(format!("{:?}", sorted))
}

/// Difference between last and first value
fn compute_diff(values: &[f64]) -> Value {
    if values.len() >= 2 {
        Value::Float(values.last().unwrap() - values.first().unwrap())
    } else {
        Value::Null
    }
}

/// Derivative (change rate per second)
fn compute_derivative(rows: &[&TimeSeriesRow], column: &str) -> Value {
    if rows.len() < 2 {
        return Value::Null;
    }
    
    let v1 = get_column_value(rows.first().unwrap(), column).as_f64();
    let v2 = get_column_value(rows.last().unwrap(), column).as_f64();
    
    match (v1, v2) {
        (Some(v1), Some(v2)) => {
            let t1 = rows.first().unwrap().timestamp;
            let t2 = rows.last().unwrap().timestamp;
            let dt = (t2 - t1) as f64 / 1000.0; // Convert to seconds
            
            if dt > 0.0 {
                Value::Float((v2 - v1) / dt)
            } else {
                Value::Null
            }
        }
        _ => Value::Null,
    }
}

/// Instant rate (using last two points)
fn compute_irate(rows: &[&TimeSeriesRow], column: &str) -> Value {
    if rows.len() < 2 {
        return Value::Null;
    }
    
    let n = rows.len();
    let v1 = get_column_value(rows[n - 2], column).as_f64();
    let v2 = get_column_value(rows[n - 1], column).as_f64();
    
    match (v1, v2) {
        (Some(v1), Some(v2)) => {
            let t1 = rows[n - 2].timestamp;
            let t2 = rows[n - 1].timestamp;
            let dt = (t2 - t1) as f64 / 1000.0;
            
            if dt > 0.0 {
                Value::Float((v2 - v1) / dt)
            } else {
                Value::Null
            }
        }
        _ => Value::Null,
    }
}

/// Time-weighted average
fn compute_twa(rows: &[&TimeSeriesRow], column: &str) -> Value {
    if rows.len() < 2 {
        return rows.first()
            .and_then(|r| get_column_value(r, column).as_f64())
            .map(Value::Float)
            .unwrap_or(Value::Null);
    }
    
    let mut weighted_sum = 0.0;
    let mut total_duration = 0i64;
    
    for i in 0..rows.len() - 1 {
        let v1 = get_column_value(rows[i], column).as_f64().unwrap_or(0.0);
        let v2 = get_column_value(rows[i + 1], column).as_f64().unwrap_or(0.0);
        let duration = rows[i + 1].timestamp - rows[i].timestamp;
        
        // Trapezoidal integration
        weighted_sum += (v1 + v2) / 2.0 * duration as f64;
        total_duration += duration;
    }
    
    if total_duration > 0 {
        Value::Float(weighted_sum / total_duration as f64)
    } else {
        Value::Null
    }
}

/// Elapsed time between first and last row
fn compute_elapsed(rows: &[&TimeSeriesRow]) -> Value {
    if rows.len() >= 2 {
        let start = rows.first().unwrap().timestamp;
        let end = rows.last().unwrap().timestamp;
        Value::Int(end - start)
    } else {
        Value::Int(0)
    }
}

/// Last row with non-null value
fn compute_last_row(rows: &[&TimeSeriesRow], column: &str) -> Value {
    rows.iter()
        .rev()
        .find_map(|r| {
            let v = get_column_value(r, column);
            if !v.is_null() {
                Some(v)
            } else {
                None
            }
        })
        .unwrap_or(Value::Null)
}

/// Mode (most frequent value)
fn compute_mode(values: &[f64]) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    
    let mut counts: HashMap<i64, usize> = HashMap::new();
    
    // Use integer representation for counting (multiply by 1000000 for precision)
    for v in values {
        let key = (v * 1_000_000.0) as i64;
        *counts.entry(key).or_insert(0) += 1;
    }
    
    counts.into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(key, _)| Value::Float(key as f64 / 1_000_000.0))
        .unwrap_or(Value::Null)
}

/// Estimate cardinality using simple counting (HyperLogLog placeholder)
fn estimate_cardinality(values: &[f64]) -> i64 {
    let unique: std::collections::HashSet<i64> = values
        .iter()
        .map(|v| (v * 1_000_000.0) as i64)
        .collect();
    unique.len() as i64
}

/// Compute histogram buckets
fn compute_histogram(values: &[f64]) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    
    let num_buckets = 10;
    let bucket_width = (max - min) / num_buckets as f64;
    
    let mut buckets = vec![0usize; num_buckets];
    
    for v in values {
        let bucket_idx = if bucket_width > 0.0 {
            ((v - min) / bucket_width).floor() as usize
        } else {
            0
        };
        let bucket_idx = bucket_idx.min(num_buckets - 1);
        buckets[bucket_idx] += 1;
    }
    
    Value::String(format!("{:?}", buckets))
}

/// Random sample of N values
fn compute_sample(values: &[f64], n: usize) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    
    // Simple sampling: take evenly spaced samples
    let step = values.len().max(1) / n.max(1);
    let samples: Vec<f64> = (0..n)
        .map(|i| values[(i * step).min(values.len() - 1)])
        .collect();
    
    Value::String(format!("{:?}", samples))
}

/// Last N values
fn compute_tail(values: &[f64], n: usize) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    
    let start = values.len().saturating_sub(n);
    let tail: Vec<f64> = values[start..].to_vec();
    
    Value::String(format!("{:?}", tail))
}

/// Unique values
fn compute_unique(values: &[f64]) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    
    let unique: std::collections::HashSet<i64> = values
        .iter()
        .map(|v| (v * 1_000_000.0) as i64)
        .collect();
    
    let unique_values: Vec<f64> = unique
        .into_iter()
        .map(|v| v as f64 / 1_000_000.0)
        .collect();
    
    Value::String(format!("{:?}", unique_values))
}

/// Duration in current state
fn compute_state_duration(rows: &[&TimeSeriesRow]) -> Value {
    if rows.len() < 2 {
        return Value::Int(0);
    }
    
    let start = rows.first().unwrap().timestamp;
    let end = rows.last().unwrap().timestamp;
    Value::Int(end - start)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compute_avg() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let rows: Vec<TimeSeriesRow> = values.iter().enumerate().map(|(i, v)| {
            TimeSeriesRow {
                timestamp: i as i64 * 1000,
                values: vec![Value::Float(*v)],
                tags: std::collections::HashMap::new(),
                table_name: "test".to_string(),
            }
        }).collect();
        
        let row_refs: Vec<&TimeSeriesRow> = rows.iter().collect();
        
        let agg = AggExpr {
            function: AggFunction::Avg,
            column: "value".to_string(),
            alias: None,
        };
        
        let result = compute_aggregation(&row_refs, &agg);
        
        if let Value::Float(v) = result {
            assert!((v - 3.0).abs() < 0.001);
        } else {
            panic!("Expected Float value");
        }
    }
    
    #[test]
    fn test_compute_stddev() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let result = compute_stddev(&values);
        
        if let Value::Float(v) = result {
            assert!((v - 2.0).abs() < 0.1);
        } else {
            panic!("Expected Float value");
        }
    }
    
    #[test]
    fn test_compute_percentile() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = compute_percentile(&values, 50.0);
        
        if let Value::Float(v) = result {
            assert!((v - 5.0).abs() < 1.0);
        } else {
            panic!("Expected Float value");
        }
    }
    
    #[test]
    fn test_compute_spread() {
        let values = vec![1.0, 5.0, 3.0, 9.0, 2.0];
        let result = compute_spread(&values);
        
        if let Value::Float(v) = result {
            assert!((v - 8.0).abs() < 0.001);
        } else {
            panic!("Expected Float value");
        }
    }
    
    #[test]
    fn test_compute_twa() {
        let rows: Vec<TimeSeriesRow> = vec![
            TimeSeriesRow {
                timestamp: 0,
                values: vec![Value::Float(0.0)],
                tags: std::collections::HashMap::new(),
                table_name: "test".to_string(),
            },
            TimeSeriesRow {
                timestamp: 1000,
                values: vec![Value::Float(10.0)],
                tags: std::collections::HashMap::new(),
                table_name: "test".to_string(),
            },
        ];
        
        let row_refs: Vec<&TimeSeriesRow> = rows.iter().collect();
        let result = compute_twa(&row_refs, "value");
        
        // Trapezoidal: (0 + 10) / 2 = 5
        if let Value::Float(v) = result {
            assert!((v - 5.0).abs() < 0.001);
        } else {
            panic!("Expected Float value");
        }
    }
}
