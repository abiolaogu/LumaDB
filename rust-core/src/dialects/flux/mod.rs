//! Flux Parser
//!
//! Parses InfluxDB 2.x/3.x Flux language into the unified IR.
//! Flux uses a functional pipe-based syntax.

use crate::dialects::ir::*;
use regex::Regex;

pub struct FluxParser;

impl FluxParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FluxParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectParser for FluxParser {
    fn parse(&self, query: &str) -> Result<QueryPlan, ParseError> {
        let query = query.trim();
        
        let mut plan = QueryPlan::new(Dialect::Flux);
        plan.original_query = query.to_string();
        
        // Parse from(bucket: "...")
        let bucket_re = Regex::new(r#"from\s*\(\s*bucket\s*:\s*"([^"]+)"\s*\)"#).unwrap();
        if let Some(caps) = bucket_re.captures(query) {
            plan.sources.push(DataSource {
                name: caps[1].to_string(),
                database: Some(caps[1].to_string()),
                retention_policy: None,
                alias: None,
                source_type: DataSourceType::Measurement,
            });
        }
        
        // Parse |> range(start: ...)
        let range_re = Regex::new(r#"\|>\s*range\s*\(\s*start\s*:\s*(-?\d+[smhd]|[^,)]+)(?:\s*,\s*stop\s*:\s*([^)]+))?\s*\)"#).unwrap();
        if let Some(caps) = range_re.captures(query) {
            let start = &caps[1];
            let duration = parse_flux_duration(start).abs();  // Use absolute value
            
            if duration > 0 {
                plan.time_range = Some(TimeRange::Relative {
                    duration,
                    anchor: None,
                });
            }
        }
        
        // Parse |> filter(fn: (r) => ...)
        let filter_re = Regex::new(r#"\|>\s*filter\s*\(\s*fn\s*:\s*\([^)]*\)\s*=>\s*([^)]+)\)"#).unwrap();
        for caps in filter_re.captures_iter(query) {
            let filter_expr = &caps[1];
            
            // Parse r._measurement == "..."
            let measurement_re = Regex::new(r#"r\._measurement\s*==\s*"([^"]+)""#).unwrap();
            if let Some(m_caps) = measurement_re.captures(filter_expr) {
                if plan.sources.is_empty() {
                    plan.sources.push(DataSource {
                        name: m_caps[1].to_string(),
                        database: None,
                        retention_policy: None,
                        alias: None,
                        source_type: DataSourceType::Measurement,
                    });
                }
            }
            
            // Parse r._field == "..."
            let field_re = Regex::new(r#"r\._field\s*==\s*"([^"]+)""#).unwrap();
            if let Some(f_caps) = field_re.captures(filter_expr) {
                plan.filters.push(Filter {
                    condition: FilterCondition::Comparison {
                        column: "_field".to_string(),
                        op: ComparisonOp::Eq,
                        value: Value::String(f_caps[1].to_string()),
                    },
                });
            }
            
            // Parse tag filters r.tag == "value"
            let tag_re = Regex::new(r#"r\.(\w+)\s*==\s*"([^"]+)""#).unwrap();
            for t_caps in tag_re.captures_iter(filter_expr) {
                let tag = &t_caps[1];
                if tag != "_measurement" && tag != "_field" {
                    plan.filters.push(Filter {
                        condition: FilterCondition::Comparison {
                            column: tag.to_string(),
                            op: ComparisonOp::Eq,
                            value: Value::String(t_caps[2].to_string()),
                        },
                    });
                }
            }
        }
        
        // Parse |> aggregateWindow(every: 5m, fn: mean)
        let agg_window_re = Regex::new(r#"\|>\s*aggregateWindow\s*\(\s*every\s*:\s*(\d+[smhd])\s*,\s*fn\s*:\s*(\w+)"#).unwrap();
        if let Some(caps) = agg_window_re.captures(query) {
            let interval = parse_flux_duration(&caps[1]);
            let func = &caps[2];
            
            plan.windows.push(Window {
                window_type: WindowType::Interval {
                    duration: interval,
                    offset: None,
                    sliding: None,
                },
                fill: None,
            });
            
            plan.aggregations.push(Aggregation {
                function: match func.to_lowercase().as_str() {
                    "mean" => AggFunction::Avg,
                    "sum" => AggFunction::Sum,
                    "count" => AggFunction::Count,
                    "min" => AggFunction::Min,
                    "max" => AggFunction::Max,
                    "first" => AggFunction::First,
                    "last" => AggFunction::Last,
                    "median" => AggFunction::Median,
                    "stddev" => AggFunction::Stddev,
                    _ => AggFunction::Custom(func.to_string()),
                },
                column: Some("_value".to_string()),
                args: vec![],
                alias: None,
                distinct: false,
            });
        }
        
        // Parse simple aggregations |> mean(), |> sum(), etc.
        let simple_agg_re = Regex::new(r#"\|>\s*(mean|sum|count|min|max|first|last|median|stddev|spread)\s*\(\s*\)"#).unwrap();
        for caps in simple_agg_re.captures_iter(query) {
            let func = &caps[1];
            plan.aggregations.push(Aggregation {
                function: match func.to_lowercase().as_str() {
                    "mean" => AggFunction::Avg,
                    "sum" => AggFunction::Sum,
                    "count" => AggFunction::Count,
                    "min" => AggFunction::Min,
                    "max" => AggFunction::Max,
                    "first" => AggFunction::First,
                    "last" => AggFunction::Last,
                    "median" => AggFunction::Median,
                    "stddev" => AggFunction::Stddev,
                    "spread" => AggFunction::Spread,
                    _ => AggFunction::Custom(func.to_string()),
                },
                column: Some("_value".to_string()),
                args: vec![],
                alias: None,
                distinct: false,
            });
        }
        
        // Parse |> group(columns: [...])
        let group_re = Regex::new(r#"\|>\s*group\s*\(\s*columns\s*:\s*\[([^\]]+)\]"#).unwrap();
        if let Some(caps) = group_re.captures(query) {
            let columns = &caps[1];
            for col in columns.split(',') {
                let col = col.trim().trim_matches('"');
                plan.group_by.push(GroupBy {
                    expr: GroupByExpr::Column(col.to_string()),
                });
            }
        }
        
        // Parse |> limit(n: ...)
        let limit_re = Regex::new(r#"\|>\s*limit\s*\(\s*n\s*:\s*(\d+)\s*\)"#).unwrap();
        if let Some(caps) = limit_re.captures(query) {
            plan.limit = caps[1].parse().ok();
        }
        
        // Parse |> sort(columns: [...], desc: ...)
        let sort_re = Regex::new(r#"\|>\s*sort\s*\(\s*columns\s*:\s*\[([^\]]+)\](?:\s*,\s*desc\s*:\s*(true|false))?"#).unwrap();
        if let Some(caps) = sort_re.captures(query) {
            let columns = &caps[1];
            let desc = caps.get(2).map(|m| m.as_str() == "true").unwrap_or(false);
            
            for col in columns.split(',') {
                let col = col.trim().trim_matches('"');
                plan.order_by.push(OrderBy {
                    column: col.to_string(),
                    ascending: !desc,
                    nulls_first: None,
                });
            }
        }
        
        // Parse |> derivative()
        if query.contains("|> derivative") {
            plan.transformations.push(Transformation {
                transform_type: TransformType::Derivative { unit: None },
                column: Some("_value".to_string()),
                alias: None,
            });
        }
        
        // Parse |> difference()
        if query.contains("|> difference") {
            plan.transformations.push(Transformation {
                transform_type: TransformType::Difference,
                column: Some("_value".to_string()),
                alias: None,
            });
        }
        
        // Parse |> fill(...)
        let fill_re = Regex::new(r#"\|>\s*fill\s*\(\s*(?:value\s*:\s*([^,)]+)|usePrevious\s*:\s*true)"#).unwrap();
        if let Some(caps) = fill_re.captures(query) {
            let strategy = if query.contains("usePrevious: true") {
                FillStrategy::Previous
            } else if let Some(val) = caps.get(1) {
                FillStrategy::Value(Value::Float(val.as_str().parse().unwrap_or(0.0)))
            } else {
                FillStrategy::Null
            };
            
            if let Some(window) = plan.windows.last_mut() {
                window.fill = Some(strategy);
            }
        }
        
        Ok(plan)
    }
    
    fn dialect(&self) -> Dialect {
        Dialect::Flux
    }
}

fn parse_flux_duration(s: &str) -> i64 {
    let s = s.trim();
    
    // Handle negative durations
    let (negative, s) = if s.starts_with('-') {
        (true, &s[1..])
    } else {
        (false, s)
    };
    
    // Parse number and unit
    let re = Regex::new(r"(\d+)([smhd])").unwrap();
    
    let ms = if let Some(caps) = re.captures(s) {
        let value: i64 = caps[1].parse().unwrap_or(0);
        let unit = &caps[2];
        
        match unit {
            "s" => value * 1000,
            "m" => value * 60 * 1000,
            "h" => value * 60 * 60 * 1000,
            "d" => value * 24 * 60 * 60 * 1000,
            _ => value,
        }
    } else {
        0
    };
    
    if negative { -ms } else { ms }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_flux_query() {
        let parser = FluxParser::new();
        let query = r#"from(bucket: "my-bucket")
            |> range(start: -1h)
            |> filter(fn: (r) => r._measurement == "cpu")
            |> aggregateWindow(every: 5m, fn: mean)
            |> yield(name: "result")"#;
        
        let result = parser.parse(query);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.sources.len(), 1);
        assert!(plan.time_range.is_some());
    }
    
    #[test]
    fn test_parse_flux_duration() {
        assert_eq!(parse_flux_duration("5m"), 300000);
        assert_eq!(parse_flux_duration("1h"), 3600000);
        assert_eq!(parse_flux_duration("-1h"), -3600000);
        assert_eq!(parse_flux_duration("24h"), 86400000);
    }
}
