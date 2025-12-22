//! MetricsQL Parser
//!
//! Parses VictoriaMetrics MetricsQL (PromQL superset) into the unified IR.

use crate::dialects::ir::*;
use crate::dialects::promql::PromQLParser;
use regex::Regex;

pub struct MetricsQLParser {
    promql_parser: PromQLParser,
}

impl MetricsQLParser {
    pub fn new() -> Self {
        Self {
            promql_parser: PromQLParser::new(),
        }
    }
}

impl Default for MetricsQLParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectParser for MetricsQLParser {
    fn parse(&self, query: &str) -> Result<QueryPlan, ParseError> {
        let query = query.trim();
        
        // First, try to parse MetricsQL-specific extensions
        let mut plan = self.promql_parser.parse(query)?;
        plan.source_dialect = Dialect::MetricsQL;
        
        // Parse MetricsQL-specific functions
        parse_metricsql_extensions(query, &mut plan)?;
        
        Ok(plan)
    }
    
    fn dialect(&self) -> Dialect {
        Dialect::MetricsQL
    }
}

fn parse_metricsql_extensions(query: &str, plan: &mut QueryPlan) -> Result<(), ParseError> {
    // MetricsQL-specific range functions
    let range_funcs = [
        ("range_avg", AggFunction::Avg),
        ("range_min", AggFunction::Min),
        ("range_max", AggFunction::Max),
        ("range_sum", AggFunction::Sum),
        ("range_first", AggFunction::First),
        ("range_last", AggFunction::Last),
        ("range_median", AggFunction::Median),
        ("range_quantile", AggFunction::Percentile(0.5)),
    ];
    
    for (func_name, agg_func) in range_funcs {
        if query.to_lowercase().contains(func_name) {
            plan.aggregations.push(Aggregation {
                function: agg_func,
                column: None,
                args: vec![],
                alias: Some(func_name.to_string()),
                distinct: false,
            });
        }
    }
    
    // MetricsQL topk/bottomk variants
    let topk_re = Regex::new(r"(?i)(topk_avg|topk_max|topk_min|topk_last|bottomk_avg|bottomk_max|bottomk_min|bottomk_last)\s*\(\s*(\d+)").unwrap();
    for caps in topk_re.captures_iter(query) {
        let func = &caps[1].to_lowercase();
        let k: usize = caps[2].parse().unwrap_or(10);
        
        let function = if func.starts_with("topk") {
            AggFunction::TopK(k)
        } else {
            AggFunction::BottomK(k)
        };
        
        plan.aggregations.push(Aggregation {
            function,
            column: None,
            args: vec![],
            alias: Some(func.clone()),
            distinct: false,
        });
    }
    
    // MetricsQL label functions
    if query.to_lowercase().contains("label_set") {
        plan.hints.custom.insert("has_label_set".to_string(), "true".to_string());
    }
    
    if query.to_lowercase().contains("label_del") {
        plan.hints.custom.insert("has_label_del".to_string(), "true".to_string());
    }
    
    if query.to_lowercase().contains("label_keep") {
        plan.hints.custom.insert("has_label_keep".to_string(), "true".to_string());
    }
    
    // MetricsQL rollup functions
    let rollup_funcs = ["rollup", "rollup_rate", "rollup_deriv", "rollup_increase", "rollup_delta"];
    for func in rollup_funcs {
        if query.to_lowercase().contains(func) {
            plan.hints.custom.insert("rollup_function".to_string(), func.to_string());
        }
    }
    
    // MetricsQL keep_metric_names
    if query.contains("keep_metric_names") {
        plan.hints.custom.insert("keep_metric_names".to_string(), "true".to_string());
    }
    
    // MetricsQL step parameter
    let step_re = Regex::new(r"(?i)\[([^:]+):([^\]]+)\]").unwrap();
    if let Some(caps) = step_re.captures(query) {
        let _range = &caps[1];
        let step = &caps[2];
        plan.hints.custom.insert("step".to_string(), step.to_string());
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_metricsql() {
        let parser = MetricsQLParser::new();
        let query = "topk_avg(5, rate(http_requests_total[5m]))";
        
        let result = parser.parse(query);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.source_dialect, Dialect::MetricsQL);
    }
    
    #[test]
    fn test_parse_range_functions() {
        let parser = MetricsQLParser::new();
        let query = "range_avg(http_requests_total[1h])";
        
        let result = parser.parse(query);
        assert!(result.is_ok());
    }
}
