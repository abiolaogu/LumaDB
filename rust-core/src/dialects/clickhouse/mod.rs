//! ClickHouse Parser
//!
//! Parses ClickHouse SQL into the unified IR.

use crate::dialects::ir::*;
use regex::Regex;

pub struct ClickHouseParser;

impl ClickHouseParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClickHouseParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectParser for ClickHouseParser {
    fn parse(&self, query: &str) -> Result<QueryPlan, ParseError> {
        let query = query.trim();
        
        let mut plan = QueryPlan::new(Dialect::ClickHouse);
        plan.original_query = query.to_string();
        
        // Parse FROM
        let from_re = Regex::new(r"(?i)FROM\s+(\w+(?:\.\w+)?)").unwrap();
        if let Some(caps) = from_re.captures(query) {
            plan.sources.push(DataSource {
                name: caps[1].to_string(),
                database: None,
                retention_policy: None,
                alias: None,
                source_type: DataSourceType::Table,
            });
        }
        
        // Parse time functions: toStartOfHour, toStartOfDay, etc.
        let time_func_re = Regex::new(r"(?i)(toStartOf(Hour|Day|Week|Month|Year)|toDateTime|toDate)\s*\(\s*(\w+)\s*\)").unwrap();
        if let Some(caps) = time_func_re.captures(query) {
            let func = &caps[1].to_lowercase();
            let interval = match func.as_str() {
                "tostartofhour" => 3600000,
                "tostartofday" => 86400000,
                "tostartofweek" => 604800000,
                "tostartofmonth" => 2592000000, // ~30 days
                "tostartofyear" => 31536000000, // 365 days
                _ => 0,
            };
            
            if interval > 0 {
                plan.group_by.push(GroupBy {
                    expr: GroupByExpr::TimeBucket {
                        interval,
                        column: Some(caps[3].to_string()),
                    },
                });
            }
        }
        
        // Parse toIntervalHour, toIntervalDay, etc.
        let interval_re = Regex::new(r"(?i)toInterval(Second|Minute|Hour|Day|Week|Month|Year)\s*\(\s*(\d+)\s*\)").unwrap();
        if let Some(caps) = interval_re.captures(query) {
            let unit = &caps[1].to_lowercase();
            let value: i64 = caps[2].parse().unwrap_or(0);
            
            let _duration = match unit.as_str() {
                "second" => value * 1000,
                "minute" => value * 60000,
                "hour" => value * 3600000,
                "day" => value * 86400000,
                "week" => value * 604800000,
                "month" => value * 2592000000,
                "year" => value * 31536000000,
                _ => value,
            };
        }
        
        // Parse PREWHERE
        if query.to_uppercase().contains("PREWHERE") {
            plan.hints.custom.insert("has_prewhere".to_string(), "true".to_string());
        }
        
        // Parse WITH TOTALS
        if query.to_uppercase().contains("WITH TOTALS") {
            plan.hints.custom.insert("with_totals".to_string(), "true".to_string());
        }
        
        // Parse FINAL
        if query.to_uppercase().contains(" FINAL") {
            plan.hints.custom.insert("final".to_string(), "true".to_string());
        }
        
        // Parse SAMPLE
        let sample_re = Regex::new(r"(?i)SAMPLE\s+(\d+\.?\d*)").unwrap();
        if let Some(caps) = sample_re.captures(query) {
            plan.hints.custom.insert("sample".to_string(), caps[1].to_string());
        }
        
        // Parse arrayJoin
        if query.to_lowercase().contains("arrayjoin") {
            plan.hints.custom.insert("has_array_join".to_string(), "true".to_string());
        }
        
        // Parse aggregations
        let agg_re = Regex::new(r"(?i)(avg|sum|count|min|max|any|anyLast|uniq|uniqExact|median|quantile)\s*\(\s*(\w+|\*)?\s*\)").unwrap();
        for caps in agg_re.captures_iter(query) {
            let func_name = caps[1].to_lowercase();
            let column = caps.get(2).map(|m| m.as_str().to_string()).filter(|s| s != "*");
            
            let function = match func_name.as_str() {
                "avg" => AggFunction::Avg,
                "sum" => AggFunction::Sum,
                "count" => AggFunction::Count,
                "min" => AggFunction::Min,
                "max" => AggFunction::Max,
                "any" => AggFunction::First,
                "anylast" => AggFunction::Last,
                "uniq" | "uniqexact" => AggFunction::CountDistinct,
                "median" => AggFunction::Median,
                "quantile" => AggFunction::Percentile(0.5),
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
        
        // Parse WHERE
        let where_re = Regex::new(r"(?i)WHERE\s+(.+?)(?:GROUP BY|ORDER BY|LIMIT|HAVING|$)").unwrap();
        if let Some(caps) = where_re.captures(query) {
            let where_clause = &caps[1];
            
            // Time filter
            let time_re = Regex::new(r"(?i)(\w+)\s*>\s*now\(\)\s*-\s*toInterval(\w+)\s*\(\s*(\d+)\s*\)").unwrap();
            if let Some(t_caps) = time_re.captures(where_clause) {
                let unit = &t_caps[2].to_lowercase();
                let value: i64 = t_caps[3].parse().unwrap_or(0);
                
                let duration = match unit.as_str() {
                    "second" => value * 1000,
                    "minute" => value * 60000,
                    "hour" => value * 3600000,
                    "day" => value * 86400000,
                    _ => value,
                };
                
                plan.time_range = Some(TimeRange::Relative {
                    duration,
                    anchor: None,
                });
            }
        }
        
        // Parse GROUP BY
        let group_re = Regex::new(r"(?i)GROUP\s+BY\s+(.+?)(?:HAVING|ORDER BY|LIMIT|WITH|$)").unwrap();
        if let Some(caps) = group_re.captures(query) {
            for col in caps[1].split(',') {
                let col = col.trim();
                if !col.is_empty() && !col.to_lowercase().starts_with("tostart") {
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
        Dialect::ClickHouse
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_clickhouse() {
        let parser = ClickHouseParser::new();
        let query = "SELECT toStartOfHour(timestamp) AS hour, count() FROM events GROUP BY hour WITH TOTALS";
        
        let result = parser.parse(query);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert!(plan.hints.custom.contains_key("with_totals"));
    }
}
