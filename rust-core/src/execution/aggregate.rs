use serde_json::Value;

pub struct Aggregator;

impl Aggregator {
    pub fn sum(values: Vec<Value>) -> f64 {
        values.iter().filter_map(|v| v.as_f64()).sum()
    }

    pub fn avg(values: Vec<Value>) -> f64 {
        let (sum, count) = values.iter().filter_map(|v| v.as_f64()).fold((0.0, 0), |acc, x| (acc.0 + x, acc.1 + 1));
        if count == 0 {
            0.0
        } else {
            sum / count as f64
        }
    }

    pub fn min(values: Vec<Value>) -> f64 {
        values.iter().filter_map(|v| v.as_f64()).fold(f64::INFINITY, |a, b| a.min(b))
    }

    pub fn max(values: Vec<Value>) -> f64 {
        values.iter().filter_map(|v| v.as_f64()).fold(f64::NEG_INFINITY, |a, b| a.max(b))
    }
}
