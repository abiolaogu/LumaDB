//! TimescaleDB Parser
//!
//! Parses TimescaleDB SQL extensions into the unified IR.

use crate::dialects::ir::*;
use regex::Regex;

pub struct TimescaleParser;

impl TimescaleParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TimescaleParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectParser for TimescaleParser {
    fn parse(&self, query: &str) -> Result<QueryPlan, ParseError> {
        let query = query.trim();
        
        let mut plan = QueryPlan::new(Dialect::TimescaleDB);
        plan.original_query = query.to_string();
        
        // Extract FROM clause
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
        
        // Parse time_bucket('interval', column)
        let time_bucket_re = Regex::new(r#"(?i)time_bucket\s*\(\s*'([^']+)'\s*,\s*(\w+)\s*\)"#).unwrap();
        if let Some(caps) = time_bucket_re.captures(query) {
            let interval = parse_postgres_interval(&caps[1]);
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
                    column: Some(caps[2].to_string()),
                },
            });
        }
        
        // Parse time_bucket_gapfill
        let gapfill_re = Regex::new(r#"(?i)time_bucket_gapfill\s*\(\s*'([^']+)'\s*,\s*(\w+)\s*\)"#).unwrap();
        if let Some(caps) = gapfill_re.captures(query) {
            let interval = parse_postgres_interval(&caps[1]);
            plan.windows.push(Window {
                window_type: WindowType::Interval {
                    duration: interval,
                    offset: None,
                    sliding: None,
                },
                fill: Some(FillStrategy::Null),
            });
        }
        
        // Parse locf() - last observation carried forward
        if query.to_lowercase().contains("locf(") {
            if let Some(window) = plan.windows.last_mut() {
                window.fill = Some(FillStrategy::Previous);
            }
        }
        
        // Parse interpolate()
        if query.to_lowercase().contains("interpolate(") {
            if let Some(window) = plan.windows.last_mut() {
                window.fill = Some(FillStrategy::Linear);
            }
        }
        
        // Parse aggregations
        parse_sql_aggregations(query, &mut plan);
        
        // Parse WHERE clause
        parse_sql_where(query, &mut plan)?;
        
        // Parse GROUP BY
        parse_sql_group_by(query, &mut plan);
        
        // Parse ORDER BY
        parse_sql_order_by(query, &mut plan);
        
        // Parse LIMIT
        parse_sql_limit(query, &mut plan);
        
        Ok(plan)
    }
    
    fn dialect(&self) -> Dialect {
        Dialect::TimescaleDB
    }
}

fn parse_postgres_interval(s: &str) -> i64 {
    let s = s.trim().to_lowercase();
    
    // Match patterns like "5 minutes", "1 hour", "30 seconds"
    let patterns = [
        (r"(\d+)\s*(second|sec|s)", 1000i64),
        (r"(\d+)\s*(minute|min|m)", 60000),
        (r"(\d+)\s*(hour|hr|h)", 3600000),
        (r"(\d+)\s*(day|d)", 86400000),
        (r"(\d+)\s*(week|w)", 604800000),
    ];
    
    for (pattern, multiplier) in patterns {
        if let Some(caps) = Regex::new(pattern).unwrap().captures(&s) {
            let value: i64 = caps[1].parse().unwrap_or(0);
            return value * multiplier;
        }
    }
    
    0
}

fn parse_sql_aggregations(query: &str, plan: &mut QueryPlan) {
    let agg_re = Regex::new(r"(?i)(avg|sum|count|min|max|stddev|variance|first|last)\s*\(\s*(\w+|\*)\s*\)").unwrap();
    
    for caps in agg_re.captures_iter(query) {
        let func_name = caps[1].to_lowercase();
        let column = if &caps[2] == "*" { None } else { Some(caps[2].to_string()) };
        
        let function = match func_name.as_str() {
            "avg" => AggFunction::Avg,
            "sum" => AggFunction::Sum,
            "count" => AggFunction::Count,
            "min" => AggFunction::Min,
            "max" => AggFunction::Max,
            "stddev" => AggFunction::Stddev,
            "variance" => AggFunction::Variance,
            "first" => AggFunction::First,
            "last" => AggFunction::Last,
            _ => AggFunction::Custom(func_name),
        };
        
        plan.aggregations.push(Aggregation {
            function,
            column,
            args: vec![],
            alias: None,
            distinct: query.to_uppercase().contains("DISTINCT"),
        });
    }
}

fn parse_sql_where(query: &str, plan: &mut QueryPlan) -> Result<(), ParseError> {
    let where_re = Regex::new(r"(?i)WHERE\s+(.+?)(?:GROUP BY|ORDER BY|LIMIT|$)").unwrap();
    
    if let Some(caps) = where_re.captures(query) {
        let where_clause = &caps[1];
        
        // Parse time comparisons
        let time_re = Regex::new(r"(?i)(\w+)\s*(>|>=|<|<=)\s*NOW\(\)\s*-\s*(INTERVAL\s*)?'?(\d+)\s*(second|minute|hour|day|week)s?'?").unwrap();
        if let Some(t_caps) = time_re.captures(where_clause) {
            let value: i64 = t_caps[4].parse().unwrap_or(0);
            let unit = &t_caps[5].to_lowercase();
            
            let duration = match unit.as_str() {
                "second" => value * 1000,
                "minute" => value * 60000,
                "hour" => value * 3600000,
                "day" => value * 86400000,
                "week" => value * 604800000,
                _ => value,
            };
            
            plan.time_range = Some(TimeRange::Relative {
                duration,
                anchor: None,
            });
        }
    }
    
    Ok(())
}

fn parse_sql_group_by(query: &str, plan: &mut QueryPlan) {
    let group_re = Regex::new(r"(?i)GROUP\s+BY\s+(.+?)(?:HAVING|ORDER BY|LIMIT|$)").unwrap();
    
    if let Some(caps) = group_re.captures(query) {
        let group_clause = &caps[1];
        
        for col in group_clause.split(',') {
            let col = col.trim();
            // Skip time_bucket references
            if !col.to_lowercase().contains("time_bucket") && !col.is_empty() {
                // Check if it's a number (column position)
                if col.parse::<usize>().is_ok() {
                    continue;
                }
                plan.group_by.push(GroupBy {
                    expr: GroupByExpr::Column(col.to_string()),
                });
            }
        }
    }
}

fn parse_sql_order_by(query: &str, plan: &mut QueryPlan) {
    let order_re = Regex::new(r"(?i)ORDER\s+BY\s+(\w+)\s*(ASC|DESC)?").unwrap();
    
    if let Some(caps) = order_re.captures(query) {
        plan.order_by.push(OrderBy {
            column: caps[1].to_string(),
            ascending: caps.get(2).map(|m| m.as_str().to_uppercase() != "DESC").unwrap_or(true),
            nulls_first: None,
        });
    }
}

fn parse_sql_limit(query: &str, plan: &mut QueryPlan) {
    let limit_re = Regex::new(r"(?i)LIMIT\s+(\d+)").unwrap();
    
    if let Some(caps) = limit_re.captures(query) {
        plan.limit = caps[1].parse().ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_time_bucket() {
        let parser = TimescaleParser::new();
        let query = "SELECT time_bucket('5 minutes', time) AS bucket, avg(temperature) FROM conditions GROUP BY bucket";
        
        let result = parser.parse(query);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert!(!plan.windows.is_empty());
    }
    
    #[test]
    fn test_parse_postgres_interval() {
        assert_eq!(parse_postgres_interval("5 minutes"), 300000);
        assert_eq!(parse_postgres_interval("1 hour"), 3600000);
        assert_eq!(parse_postgres_interval("30 seconds"), 30000);
    }
}
