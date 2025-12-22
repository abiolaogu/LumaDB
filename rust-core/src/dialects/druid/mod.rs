//! Druid Parser
//!
//! Parses Apache Druid SQL and Native JSON queries into the unified IR.

use crate::dialects::ir::*;
use regex::Regex;

// ============================================================================
// Druid SQL Parser
// ============================================================================

pub struct DruidSQLParser;

impl DruidSQLParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DruidSQLParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectParser for DruidSQLParser {
    fn parse(&self, query: &str) -> Result<QueryPlan, ParseError> {
        let query = query.trim();
        
        let mut plan = QueryPlan::new(Dialect::DruidSQL);
        plan.original_query = query.to_string();
        
        // Parse FROM
        let from_re = Regex::new(r"(?i)FROM\s+(\w+)").unwrap();
        if let Some(caps) = from_re.captures(query) {
            plan.sources.push(DataSource {
                name: caps[1].to_string(),
                database: None,
                retention_policy: None,
                alias: None,
                source_type: DataSourceType::Table,
            });
        }
        
        // Parse __time column and TIME_FLOOR
        let time_floor_re = Regex::new(r"(?i)TIME_FLOOR\s*\(\s*__time\s*,\s*'([^']+)'\s*\)").unwrap();
        if let Some(caps) = time_floor_re.captures(query) {
            let interval = parse_iso8601_duration(&caps[1]);
            plan.windows.push(Window {
                window_type: WindowType::Interval {
                    duration: interval,
                    offset: None,
                    sliding: None,
                },
                fill: None,
            });
            
            plan.group_by.push(GroupBy {
                expr: GroupByExpr::TimeBucket {
                    interval,
                    column: Some("__time".to_string()),
                },
            });
        }
        
        // Parse FLOOR(__time TO ...)
        let floor_re = Regex::new(r"(?i)FLOOR\s*\(\s*__time\s+TO\s+(SECOND|MINUTE|HOUR|DAY|WEEK|MONTH|YEAR)\s*\)").unwrap();
        if let Some(caps) = floor_re.captures(query) {
            let unit = &caps[1].to_uppercase();
            let interval = match unit.as_str() {
                "SECOND" => 1000,
                "MINUTE" => 60000,
                "HOUR" => 3600000,
                "DAY" => 86400000,
                "WEEK" => 604800000,
                "MONTH" => 2592000000,
                "YEAR" => 31536000000,
                _ => 0,
            };
            
            if interval > 0 {
                plan.group_by.push(GroupBy {
                    expr: GroupByExpr::TimeBucket {
                        interval,
                        column: Some("__time".to_string()),
                    },
                });
            }
        }
        
        // Parse APPROX_COUNT_DISTINCT
        if query.to_uppercase().contains("APPROX_COUNT_DISTINCT") {
            let approx_re = Regex::new(r"(?i)APPROX_COUNT_DISTINCT\s*\(\s*(\w+)\s*\)").unwrap();
            for caps in approx_re.captures_iter(query) {
                plan.aggregations.push(Aggregation {
                    function: AggFunction::HyperLogLog,
                    column: Some(caps[1].to_string()),
                    args: vec![],
                    alias: None,
                    distinct: false,
                });
            }
        }
        
        // Parse standard aggregations
        let agg_re = Regex::new(r"(?i)(sum|count|min|max|avg)\s*\(\s*(\w+|\*)?\s*\)").unwrap();
        for caps in agg_re.captures_iter(query) {
            let func_name = caps[1].to_lowercase();
            let column = caps.get(2).map(|m| m.as_str().to_string()).filter(|s| s != "*");
            
            let function = match func_name.as_str() {
                "sum" => AggFunction::Sum,
                "count" => AggFunction::Count,
                "min" => AggFunction::Min,
                "max" => AggFunction::Max,
                "avg" => AggFunction::Avg,
                _ => AggFunction::Custom(func_name),
            };
            
            plan.aggregations.push(Aggregation {
                function,
                column,
                args: vec![],
                alias: None,
                distinct: false,
            });
        }
        
        // Parse TIME_IN_INTERVAL
        let time_interval_re = Regex::new(r"(?i)TIME_IN_INTERVAL\s*\(\s*__time\s*,\s*'([^']+)'\s*\)").unwrap();
        if let Some(caps) = time_interval_re.captures(query) {
            plan.hints.custom.insert("time_interval".to_string(), caps[1].to_string());
        }
        
        // Parse GROUP BY
        let group_re = Regex::new(r"(?i)GROUP\s+BY\s+(.+?)(?:ORDER BY|LIMIT|HAVING|$)").unwrap();
        if let Some(caps) = group_re.captures(query) {
            for col in caps[1].split(',') {
                let col = col.trim();
                if !col.is_empty() && col != "1" && !col.to_uppercase().contains("FLOOR") && !col.to_uppercase().contains("TIME_FLOOR") {
                    plan.group_by.push(GroupBy {
                        expr: GroupByExpr::Column(col.to_string()),
                    });
                }
            }
        }
        
        // Parse ORDER BY
        let order_re = Regex::new(r"(?i)ORDER\s+BY\s+(\w+)\s*(ASC|DESC)?").unwrap();
        if let Some(caps) = order_re.captures(query) {
            plan.order_by.push(OrderBy {
                column: caps[1].to_string(),
                ascending: caps.get(2).map(|m| m.as_str().to_uppercase() != "DESC").unwrap_or(true),
                nulls_first: None,
            });
        }
        
        // Parse LIMIT
        let limit_re = Regex::new(r"(?i)LIMIT\s+(\d+)").unwrap();
        if let Some(caps) = limit_re.captures(query) {
            plan.limit = caps[1].parse().ok();
        }
        
        Ok(plan)
    }
    
    fn dialect(&self) -> Dialect {
        Dialect::DruidSQL
    }
}

fn parse_iso8601_duration(s: &str) -> i64 {
    // Parse ISO8601 duration like PT1H, PT5M, P1D
    let s = s.to_uppercase();
    
    if s.starts_with("PT") {
        let rest = &s[2..];
        if rest.ends_with('H') {
            return rest.trim_end_matches('H').parse::<i64>().unwrap_or(0) * 3600000;
        } else if rest.ends_with('M') {
            return rest.trim_end_matches('M').parse::<i64>().unwrap_or(0) * 60000;
        } else if rest.ends_with('S') {
            return rest.trim_end_matches('S').parse::<i64>().unwrap_or(0) * 1000;
        }
    } else if s.starts_with('P') {
        let rest = &s[1..];
        if rest.ends_with('D') {
            return rest.trim_end_matches('D').parse::<i64>().unwrap_or(0) * 86400000;
        } else if rest.ends_with('W') {
            return rest.trim_end_matches('W').parse::<i64>().unwrap_or(0) * 604800000;
        }
    }
    
    0
}

// ============================================================================
// Druid Native JSON Parser
// ============================================================================

pub struct DruidNativeParser;

impl DruidNativeParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DruidNativeParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectParser for DruidNativeParser {
    fn parse(&self, query: &str) -> Result<QueryPlan, ParseError> {
        let query = query.trim();
        
        let mut plan = QueryPlan::new(Dialect::DruidNative);
        plan.original_query = query.to_string();
        
        // Parse JSON using regex (simplified)
        
        // Parse queryType
        let query_type_re = Regex::new(r#""queryType"\s*:\s*"(\w+)""#).unwrap();
        if let Some(caps) = query_type_re.captures(query) {
            plan.hints.custom.insert("query_type".to_string(), caps[1].to_string());
        }
        
        // Parse dataSource
        let datasource_re = Regex::new(r#""dataSource"\s*:\s*"([^"]+)""#).unwrap();
        if let Some(caps) = datasource_re.captures(query) {
            plan.sources.push(DataSource {
                name: caps[1].to_string(),
                database: None,
                retention_policy: None,
                alias: None,
                source_type: DataSourceType::Table,
            });
        }
        
        // Parse granularity
        let granularity_re = Regex::new(r#""granularity"\s*:\s*(?:"([^"]+)"|\{[^}]*"period"\s*:\s*"([^"]+)")"#).unwrap();
        if let Some(caps) = granularity_re.captures(query) {
            let gran = caps.get(1).or(caps.get(2)).map(|m| m.as_str()).unwrap_or("all");
            let interval = match gran.to_uppercase().as_str() {
                "SECOND" => 1000,
                "MINUTE" => 60000,
                "HOUR" => 3600000,
                "DAY" => 86400000,
                _ => parse_iso8601_duration(gran),
            };
            
            if interval > 0 {
                plan.windows.push(Window {
                    window_type: WindowType::Interval {
                        duration: interval,
                        offset: None,
                        sliding: None,
                    },
                    fill: None,
                });
            }
        }
        
        // Parse intervals
        let intervals_re = Regex::new(r#""intervals"\s*:\s*\[\s*"([^"]+)""#).unwrap();
        if let Some(caps) = intervals_re.captures(query) {
            plan.hints.custom.insert("interval".to_string(), caps[1].to_string());
        }
        
        // Parse aggregations
        let agg_re = Regex::new(r#""type"\s*:\s*"(longSum|doubleSum|count|longMin|longMax|doubleMin|doubleMax|hyperUnique)""#).unwrap();
        for caps in agg_re.captures_iter(query) {
            let agg_type = &caps[1];
            let function = match agg_type {
                "longSum" | "doubleSum" => AggFunction::Sum,
                "count" => AggFunction::Count,
                "longMin" | "doubleMin" => AggFunction::Min,
                "longMax" | "doubleMax" => AggFunction::Max,
                "hyperUnique" => AggFunction::HyperLogLog,
                _ => AggFunction::Custom(agg_type.to_string()),
            };
            
            plan.aggregations.push(Aggregation {
                function,
                column: None,
                args: vec![],
                alias: None,
                distinct: false,
            });
        }
        
        // Parse threshold (for topN queries)
        let threshold_re = Regex::new(r#""threshold"\s*:\s*(\d+)"#).unwrap();
        if let Some(caps) = threshold_re.captures(query) {
            plan.limit = caps[1].parse().ok();
        }
        
        // Parse limit
        let limit_re = Regex::new(r#""limit"\s*:\s*(\d+)"#).unwrap();
        if let Some(caps) = limit_re.captures(query) {
            plan.limit = caps[1].parse().ok();
        }
        
        Ok(plan)
    }
    
    fn dialect(&self) -> Dialect {
        Dialect::DruidNative
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_druid_sql() {
        let parser = DruidSQLParser::new();
        let query = "SELECT FLOOR(__time TO HOUR), COUNT(*) FROM datasource GROUP BY 1";
        
        let result = parser.parse(query);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert!(!plan.group_by.is_empty());
    }
    
    #[test]
    fn test_parse_druid_native() {
        let parser = DruidNativeParser::new();
        let query = r#"{"queryType": "timeseries", "dataSource": "wikipedia", "granularity": "hour", "intervals": ["2021-01-01/2021-01-02"]}"#;
        
        let result = parser.parse(query);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.sources.len(), 1);
    }
    
    #[test]
    fn test_parse_iso8601_duration() {
        assert_eq!(parse_iso8601_duration("PT1H"), 3600000);
        assert_eq!(parse_iso8601_duration("PT5M"), 300000);
        assert_eq!(parse_iso8601_duration("P1D"), 86400000);
    }
}
