//! QuestDB Parser
//!
//! Parses QuestDB SQL extensions (SAMPLE BY, LATEST ON, ASOF JOIN) into the unified IR.

use crate::dialects::ir::*;
use regex::Regex;

pub struct QuestDBParser;

impl QuestDBParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for QuestDBParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectParser for QuestDBParser {
    fn parse(&self, query: &str) -> Result<QueryPlan, ParseError> {
        let query = query.trim();
        
        let mut plan = QueryPlan::new(Dialect::QuestDB);
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
        
        // Parse SAMPLE BY
        let sample_re = Regex::new(r"(?i)SAMPLE\s+BY\s+(\d+)([smhd])").unwrap();
        if let Some(caps) = sample_re.captures(query) {
            let value: i64 = caps[1].parse().unwrap_or(0);
            let unit = &caps[2];
            
            let duration = match unit {
                "s" => value * 1000,
                "m" => value * 60000,
                "h" => value * 3600000,
                "d" => value * 86400000,
                _ => value,
            };
            
            plan.windows.push(Window {
                window_type: WindowType::SampleBy {
                    interval: format!("{}{}", value, unit),
                    align: None,
                },
                fill: None,
            });
            
            plan.group_by.push(GroupBy {
                expr: GroupByExpr::TimeBucket {
                    interval: duration,
                    column: None,
                },
            });
        }
        
        // Parse ALIGN TO
        let align_re = Regex::new(r"(?i)ALIGN\s+TO\s+(FIRST\s+OBSERVATION|CALENDAR)").unwrap();
        if let Some(caps) = align_re.captures(query) {
            if let Some(Window { window_type: WindowType::SampleBy { align, .. }, .. }) = plan.windows.last_mut() {
                *align = Some(caps[1].to_string());
            }
        }
        
        // Parse LATEST ON ... PARTITION BY
        let latest_re = Regex::new(r"(?i)LATEST\s+ON\s+(\w+)\s+PARTITION\s+BY\s+(\w+)").unwrap();
        if let Some(caps) = latest_re.captures(query) {
            plan.aggregations.push(Aggregation {
                function: AggFunction::LastRow,
                column: None,
                args: vec![],
                alias: None,
                distinct: false,
            });
            
            plan.group_by.push(GroupBy {
                expr: GroupByExpr::Column(caps[2].to_string()),
            });
        }
        
        // Parse ASOF JOIN
        if query.to_uppercase().contains("ASOF JOIN") {
            plan.hints.custom.insert("join_type".to_string(), "asof".to_string());
        }
        
        // Parse LT JOIN
        if query.to_uppercase().contains("LT JOIN") {
            plan.hints.custom.insert("join_type".to_string(), "lt".to_string());
        }
        
        // Parse SPLICE JOIN
        if query.to_uppercase().contains("SPLICE JOIN") {
            plan.hints.custom.insert("join_type".to_string(), "splice".to_string());
        }
        
        // Parse FILL
        let fill_re = Regex::new(r"(?i)FILL\s*\(\s*(PREV|NULL|NONE|LINEAR|(\d+\.?\d*))\s*\)").unwrap();
        if let Some(caps) = fill_re.captures(query) {
            let fill_type = &caps[1].to_uppercase();
            let strategy = match fill_type.as_str() {
                "PREV" => FillStrategy::Previous,
                "NULL" => FillStrategy::Null,
                "NONE" => FillStrategy::None,
                "LINEAR" => FillStrategy::Linear,
                _ => {
                    if let Ok(v) = caps[1].parse::<f64>() {
                        FillStrategy::Value(Value::Float(v))
                    } else {
                        FillStrategy::None
                    }
                }
            };
            
            if let Some(window) = plan.windows.last_mut() {
                window.fill = Some(strategy);
            }
        }
        
        // Parse standard SQL parts
        parse_sql_aggregations(query, &mut plan);
        parse_sql_where(query, &mut plan)?;
        parse_sql_order_by(query, &mut plan);
        parse_sql_limit(query, &mut plan);
        
        Ok(plan)
    }
    
    fn dialect(&self) -> Dialect {
        Dialect::QuestDB
    }
}

fn parse_sql_aggregations(query: &str, plan: &mut QueryPlan) {
    let agg_re = Regex::new(r"(?i)(avg|sum|count|min|max|first|last)\s*\(\s*(\w+)\s*\)").unwrap();
    
    for caps in agg_re.captures_iter(query) {
        let func_name = caps[1].to_lowercase();
        
        let function = match func_name.as_str() {
            "avg" => AggFunction::Avg,
            "sum" => AggFunction::Sum,
            "count" => AggFunction::Count,
            "min" => AggFunction::Min,
            "max" => AggFunction::Max,
            "first" => AggFunction::First,
            "last" => AggFunction::Last,
            _ => AggFunction::Custom(func_name),
        };
        
        plan.aggregations.push(Aggregation {
            function,
            column: Some(caps[2].to_string()),
            args: vec![],
            alias: None,
            distinct: false,
        });
    }
}

fn parse_sql_where(query: &str, plan: &mut QueryPlan) -> Result<(), ParseError> {
    // Parse time range in WHERE clause
    let time_re = Regex::new(r"(?i)WHERE\s+.*(\w+)\s*>\s*dateadd\s*\(\s*'([^']+)'\s*,\s*-(\d+)\s*,\s*now\s*\(\s*\)\s*\)").unwrap();
    
    if let Some(caps) = time_re.captures(query) {
        let unit = &caps[2].to_lowercase();
        let value: i64 = caps[3].parse().unwrap_or(0);
        
        let duration = match unit.as_str() {
            "s" | "second" => value * 1000,
            "m" | "minute" => value * 60000,
            "h" | "hour" => value * 3600000,
            "d" | "day" => value * 86400000,
            _ => value,
        };
        
        plan.time_range = Some(TimeRange::Relative {
            duration,
            anchor: None,
        });
    }
    
    Ok(())
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
    fn test_parse_sample_by() {
        let parser = QuestDBParser::new();
        let query = "SELECT ts, avg(value) FROM sensors SAMPLE BY 5m";
        
        let result = parser.parse(query);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert!(!plan.windows.is_empty());
    }
    
    #[test]
    fn test_parse_latest_on() {
        let parser = QuestDBParser::new();
        let query = "SELECT * FROM trades LATEST ON timestamp PARTITION BY symbol";
        
        let result = parser.parse(query);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert!(plan.aggregations.iter().any(|a| matches!(a.function, AggFunction::LastRow)));
    }
}
