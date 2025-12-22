//! InfluxQL Parser
//!
//! Parses InfluxDB 1.x Query Language (InfluxQL) into the unified IR.

use crate::dialects::ir::*;
use regex::Regex;

pub struct InfluxQLParser;

impl InfluxQLParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InfluxQLParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectParser for InfluxQLParser {
    fn parse(&self, query: &str) -> Result<QueryPlan, ParseError> {
        let query = query.trim();
        let upper = query.to_uppercase();
        
        let mut plan = QueryPlan::new(Dialect::InfluxQL);
        plan.original_query = query.to_string();
        
        if upper.starts_with("SELECT") {
            parse_select(query, &mut plan)?;
        } else if upper.starts_with("SHOW") {
            parse_show(query, &mut plan)?;
        } else if upper.starts_with("CREATE") {
            parse_create(query, &mut plan)?;
        } else if upper.starts_with("DROP") {
            parse_drop(query, &mut plan)?;
        } else {
            return Err(ParseError {
                message: format!("Unsupported InfluxQL statement: {}", query),
                position: None,
                line: None,
                column: None,
            });
        }
        
        Ok(plan)
    }
    
    fn dialect(&self) -> Dialect {
        Dialect::InfluxQL
    }
}

fn parse_select(query: &str, plan: &mut QueryPlan) -> Result<(), ParseError> {
    // Extract FROM clause
    let from_re = Regex::new(r#"(?i)FROM\s+["']?(\w+)["']?"#).unwrap();
    if let Some(caps) = from_re.captures(query) {
        plan.sources.push(DataSource {
            name: caps[1].to_string(),
            database: None,
            retention_policy: None,
            alias: None,
            source_type: DataSourceType::Measurement,
        });
    }
    
    // Extract WHERE clause for time range
    let time_re = Regex::new(r"(?i)WHERE\s+.*time\s*([><]=?)\s*now\(\)\s*-\s*(\d+)([smhd])").unwrap();
    if let Some(caps) = time_re.captures(query) {
        let value: i64 = caps[2].parse().unwrap_or(0);
        let unit = &caps[3];
        let duration_ms = match unit {
            "s" => value * 1000,
            "m" => value * 60 * 1000,
            "h" => value * 60 * 60 * 1000,
            "d" => value * 24 * 60 * 60 * 1000,
            _ => value,
        };
        
        plan.time_range = Some(TimeRange::Relative {
            duration: duration_ms,
            anchor: None,
        });
    }
    
    // Extract GROUP BY time()
    let group_time_re = Regex::new(r"(?i)GROUP\s+BY\s+time\s*\(\s*(\d+)([smhd])\s*\)").unwrap();
    if let Some(caps) = group_time_re.captures(query) {
        let value: i64 = caps[1].parse().unwrap_or(0);
        let unit = &caps[2];
        let interval_ms = match unit {
            "s" => value * 1000,
            "m" => value * 60 * 1000,
            "h" => value * 60 * 60 * 1000,
            "d" => value * 24 * 60 * 60 * 1000,
            _ => value,
        };
        
        plan.windows.push(Window {
            window_type: WindowType::Interval {
                duration: interval_ms,
                offset: None,
                sliding: None,
            },
            fill: None,
        });
    }
    
    // Extract FILL clause
    let fill_re = Regex::new(r"(?i)FILL\s*\(\s*(\w+)\s*\)").unwrap();
    if let Some(caps) = fill_re.captures(query) {
        let fill_type = caps[1].to_lowercase();
        let strategy = match fill_type.as_str() {
            "null" => FillStrategy::Null,
            "none" => FillStrategy::None,
            "previous" => FillStrategy::Previous,
            "linear" => FillStrategy::Linear,
            _ => FillStrategy::None,
        };
        
        if let Some(window) = plan.windows.last_mut() {
            window.fill = Some(strategy);
        }
    }
    
    // Parse aggregations (mean, sum, count, etc.) - simplified pattern
    let agg_re = Regex::new(r"(?i)(mean|sum|count|min|max|first|last|median|mode|stddev|spread|derivative|difference|integral)\s*\(").unwrap();
    for caps in agg_re.captures_iter(query) {
        let func_name = caps[1].to_lowercase();
        
        let function = match func_name.as_str() {
            "mean" => AggFunction::Avg,
            "sum" => AggFunction::Sum,
            "count" => AggFunction::Count,
            "min" => AggFunction::Min,
            "max" => AggFunction::Max,
            "first" => AggFunction::First,
            "last" => AggFunction::Last,
            "median" => AggFunction::Median,
            "mode" => AggFunction::Mode,
            "stddev" => AggFunction::Stddev,
            "spread" => AggFunction::Spread,
            "derivative" => AggFunction::Custom("derivative".to_string()),
            "difference" => AggFunction::Custom("difference".to_string()),
            "integral" => AggFunction::Integral,
            _ => AggFunction::Custom(func_name),
        };
        
        plan.aggregations.push(Aggregation {
            function,
            column: Some("value".to_string()),
            args: vec![],
            alias: None,
            distinct: false,
        });
    }
    
    // Parse LIMIT
    let limit_re = Regex::new(r"(?i)LIMIT\s+(\d+)").unwrap();
    if let Some(caps) = limit_re.captures(query) {
        plan.limit = caps[1].parse().ok();
    }
    
    // Parse ORDER BY
    let order_re = Regex::new(r"(?i)ORDER\s+BY\s+(\w+)(?:\s+(ASC|DESC))?").unwrap();
    if let Some(caps) = order_re.captures(query) {
        plan.order_by.push(OrderBy {
            column: caps[1].to_string(),
            ascending: caps.get(2).map(|m| m.as_str().to_uppercase() != "DESC").unwrap_or(true),
            nulls_first: None,
        });
    }
    
    Ok(())
}

fn parse_show(query: &str, plan: &mut QueryPlan) -> Result<(), ParseError> {
    let upper = query.to_uppercase();
    
    if upper.contains("MEASUREMENTS") {
        plan.sources.push(DataSource {
            name: "_measurements".to_string(),
            database: None,
            retention_policy: None,
            alias: None,
            source_type: DataSourceType::Table,
        });
    } else if upper.contains("TAG KEYS") {
        plan.sources.push(DataSource {
            name: "_tag_keys".to_string(),
            database: None,
            retention_policy: None,
            alias: None,
            source_type: DataSourceType::Table,
        });
    } else if upper.contains("TAG VALUES") {
        plan.sources.push(DataSource {
            name: "_tag_values".to_string(),
            database: None,
            retention_policy: None,
            alias: None,
            source_type: DataSourceType::Table,
        });
    } else if upper.contains("FIELD KEYS") {
        plan.sources.push(DataSource {
            name: "_field_keys".to_string(),
            database: None,
            retention_policy: None,
            alias: None,
            source_type: DataSourceType::Table,
        });
    } else if upper.contains("DATABASES") {
        plan.sources.push(DataSource {
            name: "_databases".to_string(),
            database: None,
            retention_policy: None,
            alias: None,
            source_type: DataSourceType::Table,
        });
    }
    
    Ok(())
}

fn parse_create(_query: &str, _plan: &mut QueryPlan) -> Result<(), ParseError> {
    Ok(())
}

fn parse_drop(_query: &str, _plan: &mut QueryPlan) -> Result<(), ParseError> {
    Ok(())
}

/// InfluxQL Translator - converts IR to InfluxQL
pub struct InfluxQLTranslator;

impl InfluxQLTranslator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InfluxQLTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectTranslator for InfluxQLTranslator {
    fn translate(&self, plan: &QueryPlan) -> Result<String, TranslateError> {
        let mut sql = String::new();
        
        sql.push_str("SELECT ");
        
        if plan.aggregations.is_empty() {
            sql.push_str("*");
        } else {
            let aggs: Vec<String> = plan.aggregations.iter().map(|agg| {
                let func = match &agg.function {
                    AggFunction::Avg => "mean",
                    AggFunction::Sum => "sum",
                    AggFunction::Count => "count",
                    AggFunction::Min => "min",
                    AggFunction::Max => "max",
                    AggFunction::First => "first",
                    AggFunction::Last => "last",
                    AggFunction::Median => "median",
                    AggFunction::Mode => "mode",
                    AggFunction::Stddev => "stddev",
                    AggFunction::Spread => "spread",
                    AggFunction::Rate => "derivative",
                    AggFunction::Custom(name) => name.as_str(),
                    _ => "mean",
                };
                
                let col = agg.column.as_deref().unwrap_or("value");
                format!("{}(\"{}\")", func, col)
            }).collect();
            sql.push_str(&aggs.join(", "));
        }
        
        if let Some(source) = plan.sources.first() {
            sql.push_str(&format!(" FROM \"{}\"", source.name));
        }
        
        if let Some(ref time_range) = plan.time_range {
            match time_range {
                TimeRange::Relative { duration, .. } => {
                    let (val, unit) = ms_to_influx_duration(*duration);
                    sql.push_str(&format!(" WHERE time > now() - {}{}", val, unit));
                }
                TimeRange::Absolute { start, end } => {
                    sql.push_str(&format!(" WHERE time >= {}ms AND time < {}ms", start, end));
                }
                _ => {}
            }
        }
        
        if let Some(window) = plan.windows.first() {
            if let WindowType::Interval { duration, .. } = &window.window_type {
                let (val, unit) = ms_to_influx_duration(*duration);
                sql.push_str(&format!(" GROUP BY time({}{})", val, unit));
                
                if let Some(ref fill) = window.fill {
                    let fill_str = match fill {
                        FillStrategy::None => "none",
                        FillStrategy::Null => "null",
                        FillStrategy::Previous => "previous",
                        FillStrategy::Linear => "linear",
                        FillStrategy::Value(Value::Float(v)) => return Ok(format!("{} FILL({})", sql, v)),
                        _ => "none",
                    };
                    sql.push_str(&format!(" FILL({})", fill_str));
                }
            }
        }
        
        for order in &plan.order_by {
            let dir = if order.ascending { "ASC" } else { "DESC" };
            sql.push_str(&format!(" ORDER BY {} {}", order.column, dir));
            break;
        }
        
        if let Some(limit) = plan.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        Ok(sql)
    }
    
    fn target_dialect(&self) -> Dialect {
        Dialect::InfluxQL
    }
}

fn ms_to_influx_duration(ms: i64) -> (i64, &'static str) {
    if ms >= 86400000 && ms % 86400000 == 0 {
        (ms / 86400000, "d")
    } else if ms >= 3600000 && ms % 3600000 == 0 {
        (ms / 3600000, "h")
    } else if ms >= 60000 && ms % 60000 == 0 {
        (ms / 60000, "m")
    } else if ms >= 1000 && ms % 1000 == 0 {
        (ms / 1000, "s")
    } else {
        (ms, "ms")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_select() {
        let parser = InfluxQLParser::new();
        let query = r#"SELECT mean("value") FROM "cpu" WHERE time > now() - 1h GROUP BY time(5m) FILL(null)"#;
        
        let result = parser.parse(query);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.sources.len(), 1);
        assert_eq!(plan.sources[0].name, "cpu");
    }
    
    #[test]
    fn test_translate_to_influxql() {
        let translator = InfluxQLTranslator::new();
        
        let mut plan = QueryPlan::new(Dialect::PromQL);
        plan.sources.push(DataSource {
            name: "cpu".to_string(),
            database: None,
            retention_policy: None,
            alias: None,
            source_type: DataSourceType::Measurement,
        });
        plan.aggregations.push(Aggregation {
            function: AggFunction::Avg,
            column: Some("value".to_string()),
            args: vec![],
            alias: None,
            distinct: false,
        });
        plan.time_range = Some(TimeRange::Relative {
            duration: 3600000,
            anchor: None,
        });
        
        let result = translator.translate(&plan);
        assert!(result.is_ok());
        
        let sql = result.unwrap();
        assert!(sql.contains("SELECT mean"));
        assert!(sql.contains("FROM \"cpu\""));
    }
}
