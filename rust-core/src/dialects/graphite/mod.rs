//! Graphite Parser
//!
//! Parses Graphite render API queries into the unified IR.

use crate::dialects::ir::*;
use regex::Regex;

pub struct GraphiteParser;

impl GraphiteParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GraphiteParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectParser for GraphiteParser {
    fn parse(&self, query: &str) -> Result<QueryPlan, ParseError> {
        let query = query.trim();
        
        let mut plan = QueryPlan::new(Dialect::Graphite);
        plan.original_query = query.to_string();
        
        // Parse the outermost function or metric path
        parse_graphite_expr(query, &mut plan)?;
        
        Ok(plan)
    }
    
    fn dialect(&self) -> Dialect {
        Dialect::Graphite
    }
}

fn parse_graphite_expr(expr: &str, plan: &mut QueryPlan) -> Result<(), ParseError> {
    let expr = expr.trim();
    
    // Check if it's a function call
    let func_re = Regex::new(r"^(\w+)\s*\((.+)\)$").unwrap();
    
    if let Some(caps) = func_re.captures(expr) {
        let func_name = &caps[1].to_lowercase();
        let args = &caps[2];
        
        match func_name.as_str() {
            "summarize" => {
                // summarize(metric, "interval", "function")
                let parts: Vec<&str> = split_args(args);
                if parts.len() >= 2 {
                    // Parse interval
                    let interval = parts[1].trim().trim_matches('"').trim_matches('\'');
                    let duration = parse_graphite_interval(interval);
                    
                    plan.windows.push(Window {
                        window_type: WindowType::Interval {
                            duration,
                            offset: None,
                            sliding: None,
                        },
                        fill: None,
                    });
                    
                    // Parse aggregation function
                    if parts.len() >= 3 {
                        let agg = parts[2].trim().trim_matches('"').trim_matches('\'').to_lowercase();
                        let function = match agg.as_str() {
                            "sum" => AggFunction::Sum,
                            "avg" | "average" => AggFunction::Avg,
                            "min" => AggFunction::Min,
                            "max" => AggFunction::Max,
                            "count" => AggFunction::Count,
                            "last" => AggFunction::Last,
                            "first" => AggFunction::First,
                            _ => AggFunction::Custom(agg),
                        };
                        plan.aggregations.push(Aggregation {
                            function,
                            column: None,
                            args: vec![],
                            alias: None,
                            distinct: false,
                        });
                    }
                    
                    // Parse inner expression
                    parse_graphite_expr(parts[0], plan)?;
                }
            }
            
            "alias" => {
                // alias(metric, "name")
                let parts: Vec<&str> = split_args(args);
                if parts.len() >= 2 {
                    let alias = parts[1].trim().trim_matches('"').trim_matches('\'');
                    plan.output_format.options.insert("alias".to_string(), alias.to_string());
                    parse_graphite_expr(parts[0], plan)?;
                }
            }
            
            "scale" => {
                // scale(metric, factor)
                let parts: Vec<&str> = split_args(args);
                if parts.len() >= 2 {
                    let factor: f64 = parts[1].trim().parse().unwrap_or(1.0);
                    plan.transformations.push(Transformation {
                        transform_type: TransformType::Custom {
                            name: "scale".to_string(),
                            args: vec![Value::Float(factor)],
                        },
                        column: None,
                        alias: None,
                    });
                    parse_graphite_expr(parts[0], plan)?;
                }
            }
            
            "offset" => {
                // offset(metric, value)
                let parts: Vec<&str> = split_args(args);
                if parts.len() >= 2 {
                    let offset: f64 = parts[1].trim().parse().unwrap_or(0.0);
                    plan.transformations.push(Transformation {
                        transform_type: TransformType::Custom {
                            name: "offset".to_string(),
                            args: vec![Value::Float(offset)],
                        },
                        column: None,
                        alias: None,
                    });
                    parse_graphite_expr(parts[0], plan)?;
                }
            }
            
            "derivative" => {
                plan.transformations.push(Transformation {
                    transform_type: TransformType::Derivative { unit: None },
                    column: None,
                    alias: None,
                });
                parse_graphite_expr(args, plan)?;
            }
            
            "integral" => {
                plan.aggregations.push(Aggregation {
                    function: AggFunction::Integral,
                    column: None,
                    args: vec![],
                    alias: None,
                    distinct: false,
                });
                parse_graphite_expr(args, plan)?;
            }
            
            "movingaverage" | "movingmedian" => {
                let parts: Vec<&str> = split_args(args);
                if parts.len() >= 2 {
                    let points: usize = parts[1].trim().parse().unwrap_or(10);
                    plan.transformations.push(Transformation {
                        transform_type: TransformType::MovingAverage { points },
                        column: None,
                        alias: None,
                    });
                    parse_graphite_expr(parts[0], plan)?;
                }
            }
            
            "highestcurrent" | "highestmax" | "highestAverage" => {
                let parts: Vec<&str> = split_args(args);
                if parts.len() >= 2 {
                    let n: usize = parts[1].trim().parse().unwrap_or(10);
                    plan.aggregations.push(Aggregation {
                        function: AggFunction::TopK(n),
                        column: None,
                        args: vec![],
                        alias: None,
                        distinct: false,
                    });
                    parse_graphite_expr(parts[0], plan)?;
                }
            }
            
            "lowestcurrent" | "lowestAverage" => {
                let parts: Vec<&str> = split_args(args);
                if parts.len() >= 2 {
                    let n: usize = parts[1].trim().parse().unwrap_or(10);
                    plan.aggregations.push(Aggregation {
                        function: AggFunction::BottomK(n),
                        column: None,
                        args: vec![],
                        alias: None,
                        distinct: false,
                    });
                    parse_graphite_expr(parts[0], plan)?;
                }
            }
            
            "abs" => {
                plan.transformations.push(Transformation {
                    transform_type: TransformType::Abs,
                    column: None,
                    alias: None,
                });
                parse_graphite_expr(args, plan)?;
            }
            
            "averageseries" | "avg" => {
                plan.aggregations.push(Aggregation {
                    function: AggFunction::Avg,
                    column: None,
                    args: vec![],
                    alias: None,
                    distinct: false,
                });
                parse_graphite_expr(args, plan)?;
            }
            
            "sumseries" | "sum" => {
                plan.aggregations.push(Aggregation {
                    function: AggFunction::Sum,
                    column: None,
                    args: vec![],
                    alias: None,
                    distinct: false,
                });
                parse_graphite_expr(args, plan)?;
            }
            
            "minseries" | "min" => {
                plan.aggregations.push(Aggregation {
                    function: AggFunction::Min,
                    column: None,
                    args: vec![],
                    alias: None,
                    distinct: false,
                });
                parse_graphite_expr(args, plan)?;
            }
            
            "maxseries" | "max" => {
                plan.aggregations.push(Aggregation {
                    function: AggFunction::Max,
                    column: None,
                    args: vec![],
                    alias: None,
                    distinct: false,
                });
                parse_graphite_expr(args, plan)?;
            }
            
            _ => {
                // Unknown function - just parse args
                for arg in split_args(args) {
                    let _ = parse_graphite_expr(arg, plan);
                }
            }
        }
    } else {
        // It's a metric path
        plan.sources.push(DataSource {
            name: expr.to_string(),
            database: None,
            retention_policy: None,
            alias: None,
            source_type: DataSourceType::Metric,
        });
    }
    
    Ok(())
}

fn split_args(args: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    
    for (i, c) in args.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&args[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    
    if start < args.len() {
        parts.push(&args[start..]);
    }
    
    parts
}

fn parse_graphite_interval(s: &str) -> i64 {
    let re = Regex::new(r"(\d+)([smhdw])").unwrap();
    
    if let Some(caps) = re.captures(s) {
        let value: i64 = caps[1].parse().unwrap_or(0);
        let unit = &caps[2];
        
        match unit {
            "s" => value * 1000,
            "m" => value * 60000,
            "h" => value * 3600000,
            "d" => value * 86400000,
            "w" => value * 604800000,
            _ => value,
        }
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_summarize() {
        let parser = GraphiteParser::new();
        let query = r#"summarize(servers.*.cpu.user, "1h", "avg")"#;
        
        let result = parser.parse(query);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert!(!plan.windows.is_empty());
        assert!(!plan.aggregations.is_empty());
    }
    
    #[test]
    fn test_parse_metric_path() {
        let parser = GraphiteParser::new();
        let query = "servers.web01.cpu.user";
        
        let result = parser.parse(query);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.sources.len(), 1);
        assert_eq!(plan.sources[0].name, "servers.web01.cpu.user");
    }
}
