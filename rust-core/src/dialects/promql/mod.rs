//! PromQL Parser
//!
//! Parses Prometheus Query Language (PromQL) into the unified IR.

use crate::dialects::ir::*;
use regex::Regex;

pub struct PromQLParser;

impl PromQLParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PromQLParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectParser for PromQLParser {
    fn parse(&self, query: &str) -> Result<QueryPlan, ParseError> {
        let query = query.trim();
        
        let mut plan = QueryPlan::new(Dialect::PromQL);
        plan.original_query = query.to_string();
        
        parse_promql_expr(query, &mut plan)?;
        
        Ok(plan)
    }
    
    fn dialect(&self) -> Dialect {
        Dialect::PromQL
    }
}

fn parse_promql_expr(expr: &str, plan: &mut QueryPlan) -> Result<(), ParseError> {
    let expr = expr.trim();
    
    // Check for aggregation operators (sum, avg, count, etc.)
    let agg_re = Regex::new(r"^(sum|avg|min|max|count|stddev|stdvar|topk|bottomk|quantile|count_values)\s*(by|without)?\s*\(([^)]*)\)\s*\((.+)\)$").unwrap();
    if let Some(caps) = agg_re.captures(expr) {
        let agg_name = &caps[1];
        let modifier = caps.get(2).map(|m| m.as_str());
        let labels = &caps[3];
        let inner = &caps[4];
        
        // Parse the aggregation
        let function = match agg_name.to_lowercase().as_str() {
            "sum" => AggFunction::Sum,
            "avg" => AggFunction::Avg,
            "min" => AggFunction::Min,
            "max" => AggFunction::Max,
            "count" => AggFunction::Count,
            "stddev" => AggFunction::Stddev,
            "stdvar" => AggFunction::Variance,
            "topk" => AggFunction::TopK(10),
            "bottomk" => AggFunction::BottomK(10),
            "quantile" => AggFunction::Percentile(0.5),
            _ => AggFunction::Custom(agg_name.to_string()),
        };
        
        plan.aggregations.push(Aggregation {
            function,
            column: None,
            args: vec![],
            alias: None,
            distinct: false,
        });
        
        // Parse group by labels
        if modifier.is_some() {
            for label in labels.split(',') {
                let label = label.trim().trim_matches('"');
                if !label.is_empty() {
                    plan.group_by.push(GroupBy {
                        expr: GroupByExpr::Tag(label.to_string()),
                    });
                }
            }
        }
        
        // Recursively parse inner expression
        parse_promql_expr(inner, plan)?;
        return Ok(());
    }
    
    // Check for functions (rate, irate, increase, etc.)
    let func_re = Regex::new(r"^(\w+)\s*\((.+)\)$").unwrap();
    if let Some(caps) = func_re.captures(expr) {
        let func_name = &caps[1];
        let inner = &caps[2];
        
        let function = match func_name.to_lowercase().as_str() {
            "rate" => Some(AggFunction::Rate),
            "irate" => Some(AggFunction::Irate),
            "increase" => Some(AggFunction::Increase),
            "delta" => Some(AggFunction::Delta),
            "idelta" => Some(AggFunction::Idelta),
            "deriv" => Some(AggFunction::Deriv),
            "predict_linear" => Some(AggFunction::PredictLinear),
            "resets" => Some(AggFunction::Resets),
            "changes" => Some(AggFunction::Changes),
            "histogram_quantile" => {
                // Parse quantile value
                let parts: Vec<&str> = inner.splitn(2, ',').collect();
                let q = parts.first()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .unwrap_or(0.9);
                Some(AggFunction::HistogramQuantile(q))
            }
            "label_replace" => {
                // label_replace(v, dst, replacement, src, regex)
                None // Handle as transformation
            }
            "label_join" => None,
            "abs" | "ceil" | "floor" | "round" | "sqrt" | "ln" | "log2" | "log10" | "exp" => None,
            "avg_over_time" | "sum_over_time" | "min_over_time" | "max_over_time" 
            | "count_over_time" | "stddev_over_time" | "quantile_over_time" => {
                let base_func = func_name.trim_end_matches("_over_time");
                Some(match base_func {
                    "avg" => AggFunction::Avg,
                    "sum" => AggFunction::Sum,
                    "min" => AggFunction::Min,
                    "max" => AggFunction::Max,
                    "count" => AggFunction::Count,
                    "stddev" => AggFunction::Stddev,
                    _ => AggFunction::Custom(func_name.to_string()),
                })
            }
            _ => None,
        };
        
        if let Some(func) = function {
            plan.aggregations.push(Aggregation {
                function: func,
                column: None,
                args: vec![],
                alias: None,
                distinct: false,
            });
        }
        
        // Handle math functions as transformations
        let transform = match func_name.to_lowercase().as_str() {
            "abs" => Some(TransformType::Abs),
            "ceil" => Some(TransformType::Ceil),
            "floor" => Some(TransformType::Floor),
            "round" => Some(TransformType::Round(None)),
            "sqrt" => Some(TransformType::Sqrt),
            "ln" | "log2" | "log10" => Some(TransformType::Log(None)),
            "exp" => Some(TransformType::Exp),
            _ => None,
        };
        
        if let Some(t) = transform {
            plan.transformations.push(Transformation {
                transform_type: t,
                column: None,
                alias: None,
            });
        }
        
        // Recursively parse inner expression
        parse_promql_expr(inner, plan)?;
        return Ok(());
    }
    
    // Parse metric selector: metric_name{label1="value1", ...}[duration]
    let selector_re = Regex::new(r#"^([a-zA-Z_:][a-zA-Z0-9_:]*)\s*(?:\{([^}]*)\})?\s*(?:\[(\d+[smhdwy])\])?\s*(?:offset\s+(\d+[smhdwy]))?(?:\s*@\s*(\d+))?$"#).unwrap();
    if let Some(caps) = selector_re.captures(expr) {
        let metric_name = &caps[1];
        let labels = caps.get(2).map(|m| m.as_str());
        let range = caps.get(3).map(|m| m.as_str());
        let offset = caps.get(4).map(|m| m.as_str());
        let at_time = caps.get(5).map(|m| m.as_str());
        
        // Set metric as data source
        plan.sources.push(DataSource {
            name: metric_name.to_string(),
            database: None,
            retention_policy: None,
            alias: None,
            source_type: DataSourceType::Metric,
        });
        
        // Parse label matchers
        if let Some(labels_str) = labels {
            parse_label_matchers(labels_str, plan)?;
        }
        
        // Parse range vector
        if let Some(range_str) = range {
            let duration = parse_promql_duration(range_str);
            plan.windows.push(Window {
                window_type: WindowType::Range { duration },
                fill: None,
            });
        }
        
        // Parse offset
        if let Some(offset_str) = offset {
            let offset_ms = parse_promql_duration(offset_str);
            plan.hints.custom.insert("offset_ms".to_string(), offset_ms.to_string());
        }
        
        // Parse @ modifier
        if let Some(ts) = at_time {
            if let Ok(timestamp) = ts.parse::<i64>() {
                plan.time_range = Some(TimeRange::Absolute {
                    start: timestamp * 1000,
                    end: timestamp * 1000,
                });
            }
        }
        
        return Ok(());
    }
    
    // Handle binary operators
    for op in &["+", "-", "*", "/", "%", "^", "==", "!=", ">", "<", ">=", "<=", "and", "or", "unless"] {
        if expr.contains(op) {
            // For now, just parse the left side
            let parts: Vec<&str> = expr.splitn(2, op).collect();
            if parts.len() == 2 {
                parse_promql_expr(parts[0].trim(), plan)?;
                return Ok(());
            }
        }
    }
    
    Ok(())
}

fn parse_label_matchers(labels_str: &str, plan: &mut QueryPlan) -> Result<(), ParseError> {
    // Parse: label="value", label!="value", label=~"regex", label!~"regex"
    let matcher_re = Regex::new(r#"(\w+)\s*(=~|!~|!=|=)\s*"([^"]*)""#).unwrap();
    
    for caps in matcher_re.captures_iter(labels_str) {
        let label = &caps[1];
        let op = &caps[2];
        let value = &caps[3];
        
        let condition = match op {
            "=" => FilterCondition::Comparison {
                column: label.to_string(),
                op: ComparisonOp::Eq,
                value: Value::String(value.to_string()),
            },
            "!=" => FilterCondition::Comparison {
                column: label.to_string(),
                op: ComparisonOp::NotEq,
                value: Value::String(value.to_string()),
            },
            "=~" => FilterCondition::Regex {
                column: label.to_string(),
                pattern: value.to_string(),
                negated: false,
            },
            "!~" => FilterCondition::Regex {
                column: label.to_string(),
                pattern: value.to_string(),
                negated: true,
            },
            _ => continue,
        };
        
        plan.filters.push(Filter { condition });
    }
    
    Ok(())
}

fn parse_promql_duration(s: &str) -> i64 {
    let s = s.trim();
    
    let re = Regex::new(r"(\d+)([smhdwy])").unwrap();
    
    if let Some(caps) = re.captures(s) {
        let value: i64 = caps[1].parse().unwrap_or(0);
        let unit = &caps[2];
        
        match unit {
            "s" => value * 1000,
            "m" => value * 60 * 1000,
            "h" => value * 60 * 60 * 1000,
            "d" => value * 24 * 60 * 60 * 1000,
            "w" => value * 7 * 24 * 60 * 60 * 1000,
            "y" => value * 365 * 24 * 60 * 60 * 1000,
            _ => value,
        }
    } else {
        0
    }
}

/// PromQL Translator - converts IR to PromQL
pub struct PromQLTranslator;

impl PromQLTranslator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PromQLTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectTranslator for PromQLTranslator {
    fn translate(&self, plan: &QueryPlan) -> Result<String, TranslateError> {
        let mut query = String::new();
        
        // Build the inner selector
        let mut selector = String::new();
        
        if let Some(source) = plan.sources.first() {
            selector.push_str(&source.name);
        } else {
            selector.push_str("metric");
        }
        
        // Build label matchers
        let mut matchers = Vec::new();
        for filter in &plan.filters {
            if let FilterCondition::Comparison { column, op, value } = &filter.condition {
                let op_str = match op {
                    ComparisonOp::Eq => "=",
                    ComparisonOp::NotEq => "!=",
                    _ => "=",
                };
                if let Value::String(v) = value {
                    matchers.push(format!("{}{}\"{}\"", column, op_str, v));
                }
            } else if let FilterCondition::Regex { column, pattern, negated } = &filter.condition {
                let op_str = if *negated { "!~" } else { "=~" };
                matchers.push(format!("{}{}\"{}\"", column, op_str, pattern));
            }
        }
        
        if !matchers.is_empty() {
            selector.push('{');
            selector.push_str(&matchers.join(", "));
            selector.push('}');
        }
        
        // Add range vector
        if let Some(window) = plan.windows.first() {
            if let WindowType::Range { duration } = &window.window_type {
                let dur_str = ms_to_promql_duration(*duration);
                selector.push_str(&format!("[{}]", dur_str));
            }
        }
        
        query = selector;
        
        // Wrap with functions/aggregations
        for agg in &plan.aggregations {
            let func_name = match &agg.function {
                AggFunction::Rate => "rate",
                AggFunction::Irate => "irate",
                AggFunction::Increase => "increase",
                AggFunction::Delta => "delta",
                AggFunction::Avg => "avg_over_time",
                AggFunction::Sum => "sum_over_time",
                AggFunction::Min => "min_over_time",
                AggFunction::Max => "max_over_time",
                AggFunction::Count => "count_over_time",
                AggFunction::Stddev => "stddev_over_time",
                AggFunction::HistogramQuantile(q) => {
                    query = format!("histogram_quantile({}, {})", q, query);
                    continue;
                }
                _ => continue,
            };
            query = format!("{}({})", func_name, query);
        }
        
        // Wrap with aggregation operators
        let agg_ops: Vec<(String, Vec<String>)> = plan.aggregations.iter()
            .filter_map(|agg| {
                match &agg.function {
                    AggFunction::Sum => Some("sum".to_string()),
                    AggFunction::Avg => Some("avg".to_string()),
                    AggFunction::Min => Some("min".to_string()),
                    AggFunction::Max => Some("max".to_string()),
                    AggFunction::Count => Some("count".to_string()),
                    AggFunction::Stddev => Some("stddev".to_string()),
                    AggFunction::TopK(k) => Some(format!("topk({}", k)),
                    AggFunction::BottomK(k) => Some(format!("bottomk({}", k)),
                    _ => None,
                }
            })
            .map(|op| {
                let labels_list: Vec<String> = plan.group_by.iter()
                    .filter_map(|g| match &g.expr {
                        GroupByExpr::Tag(t) => Some(t.clone()),
                        _ => None,
                    })
                    .collect();
                (op, labels_list)
            })
            .collect();
        
        for (op, labels) in agg_ops {
            if labels.is_empty() {
                if op.starts_with("topk") || op.starts_with("bottomk") {
                    query = format!("{}, {})", op, query);
                } else {
                    query = format!("{}({})", op, query);
                }
            } else {
                query = format!("{} by ({}) ({})", op, labels.join(", "), query);
            }
        }
        
        Ok(query)
    }
    
    fn target_dialect(&self) -> Dialect {
        Dialect::PromQL
    }
}

fn ms_to_promql_duration(ms: i64) -> String {
    if ms >= 86400000 && ms % 86400000 == 0 {
        format!("{}d", ms / 86400000)
    } else if ms >= 3600000 && ms % 3600000 == 0 {
        format!("{}h", ms / 3600000)
    } else if ms >= 60000 && ms % 60000 == 0 {
        format!("{}m", ms / 60000)
    } else if ms >= 1000 && ms % 1000 == 0 {
        format!("{}s", ms / 1000)
    } else {
        format!("{}ms", ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_metric() {
        let parser = PromQLParser::new();
        let result = parser.parse("http_requests_total");
        
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.sources.len(), 1);
        assert_eq!(plan.sources[0].name, "http_requests_total");
    }
    
    #[test]
    fn test_parse_metric_with_labels() {
        let parser = PromQLParser::new();
        let result = parser.parse(r#"http_requests_total{job="api", status="200"}"#);
        
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.sources.len(), 1);
        assert_eq!(plan.filters.len(), 2);
    }
    
    #[test]
    fn test_parse_rate() {
        let parser = PromQLParser::new();
        let result = parser.parse(r#"rate(http_requests_total{job="api"}[5m])"#);
        
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.aggregations.len(), 1);
        assert!(matches!(plan.aggregations[0].function, AggFunction::Rate));
    }
    
    #[test]
    fn test_parse_aggregation() {
        let parser = PromQLParser::new();
        let result = parser.parse(r#"sum by (job) (rate(http_requests_total[5m]))"#);
        
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert!(plan.aggregations.iter().any(|a| matches!(a.function, AggFunction::Sum)));
    }
    
    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_promql_duration("5m"), 300000);
        assert_eq!(parse_promql_duration("1h"), 3600000);
        assert_eq!(parse_promql_duration("7d"), 604800000);
    }
}
