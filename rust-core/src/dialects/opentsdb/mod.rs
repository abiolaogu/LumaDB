//! OpenTSDB Parser
//!
//! Parses OpenTSDB JSON query format into the unified IR.

use crate::dialects::ir::*;
use regex::Regex;

pub struct OpenTSDBParser;

impl OpenTSDBParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenTSDBParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectParser for OpenTSDBParser {
    fn parse(&self, query: &str) -> Result<QueryPlan, ParseError> {
        let query = query.trim();
        
        let mut plan = QueryPlan::new(Dialect::OpenTSDB);
        plan.original_query = query.to_string();
        
        // Parse start time
        let start_re = Regex::new(r#""start"\s*:\s*(\d+)"#).unwrap();
        let end_re = Regex::new(r#""end"\s*:\s*(\d+)"#).unwrap();
        
        let start = start_re.captures(query).and_then(|c| c[1].parse::<i64>().ok());
        let end = end_re.captures(query).and_then(|c| c[1].parse::<i64>().ok());
        
        if let (Some(s), Some(e)) = (start, end) {
            plan.time_range = Some(TimeRange::Absolute {
                start: s * 1000, // OpenTSDB uses seconds
                end: e * 1000,
            });
        } else if let Some(s) = start {
            plan.time_range = Some(TimeRange::Since { start: s * 1000 });
        }
        
        // Parse metric
        let metric_re = Regex::new(r#""metric"\s*:\s*"([^"]+)""#).unwrap();
        for caps in metric_re.captures_iter(query) {
            plan.sources.push(DataSource {
                name: caps[1].to_string(),
                database: None,
                retention_policy: None,
                alias: None,
                source_type: DataSourceType::Metric,
            });
        }
        
        // Parse aggregator
        let agg_re = Regex::new(r#""aggregator"\s*:\s*"(\w+)""#).unwrap();
        for caps in agg_re.captures_iter(query) {
            let agg_name = &caps[1].to_lowercase();
            let function = match agg_name.as_str() {
                "sum" | "zimsum" => AggFunction::Sum,
                "avg" => AggFunction::Avg,
                "min" | "mimmin" => AggFunction::Min,
                "max" | "mimmax" => AggFunction::Max,
                "count" => AggFunction::Count,
                "first" => AggFunction::First,
                "last" => AggFunction::Last,
                "dev" => AggFunction::Stddev,
                _ => AggFunction::Custom(agg_name.clone()),
            };
            
            plan.aggregations.push(Aggregation {
                function,
                column: None,
                args: vec![],
                alias: None,
                distinct: false,
            });
        }
        
        // Parse downsample
        let downsample_re = Regex::new(r#""downsample"\s*:\s*"(\d+)([smhdw])-(\w+)""#).unwrap();
        if let Some(caps) = downsample_re.captures(query) {
            let value: i64 = caps[1].parse().unwrap_or(0);
            let unit = &caps[2];
            let _agg = &caps[3];
            
            let duration = match unit {
                "s" => value * 1000,
                "m" => value * 60000,
                "h" => value * 3600000,
                "d" => value * 86400000,
                "w" => value * 604800000,
                _ => value,
            };
            
            plan.windows.push(Window {
                window_type: WindowType::Interval {
                    duration,
                    offset: None,
                    sliding: None,
                },
                fill: None,
            });
        }
        
        // Parse tags (filters)
        let tag_re = Regex::new(r#""(\w+)"\s*:\s*"(\w+)""#).unwrap();
        // Check if we're in a "tags" object
        if query.contains("\"tags\"") {
            let tags_section_re = Regex::new(r#""tags"\s*:\s*\{([^}]+)\}"#).unwrap();
            if let Some(tags_caps) = tags_section_re.captures(query) {
                let tags_content = &tags_caps[1];
                for tag_cap in tag_re.captures_iter(tags_content) {
                    plan.filters.push(Filter {
                        condition: FilterCondition::Comparison {
                            column: tag_cap[1].to_string(),
                            op: ComparisonOp::Eq,
                            value: Value::String(tag_cap[2].to_string()),
                        },
                    });
                }
            }
        }
        
        // Parse rate
        if query.contains("\"rate\"") && query.contains("true") {
            plan.aggregations.push(Aggregation {
                function: AggFunction::Rate,
                column: None,
                args: vec![],
                alias: None,
                distinct: false,
            });
        }
        
        Ok(plan)
    }
    
    fn dialect(&self) -> Dialect {
        Dialect::OpenTSDB
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_opentsdb_query() {
        let parser = OpenTSDBParser::new();
        let query = r#"{
            "start": 1609459200,
            "end": 1609545600,
            "queries": [{
                "metric": "sys.cpu.user",
                "aggregator": "avg",
                "downsample": "1h-avg",
                "tags": {"host": "web01"}
            }]
        }"#;
        
        let result = parser.parse(query);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert!(!plan.sources.is_empty());
        assert!(plan.time_range.is_some());
    }
}
