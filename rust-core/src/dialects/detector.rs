//! Dialect Auto-Detection
//!
//! Automatically detects the query language from query text using
//! pattern matching and keyword analysis.

use std::collections::HashMap;
use regex::Regex;
use super::ir::Dialect;

/// Dialect detector with pattern-based detection
pub struct DialectDetector {
    patterns: HashMap<Dialect, Vec<Regex>>,
    keywords: HashMap<Dialect, Vec<&'static str>>,
}

impl Default for DialectDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectDetector {
    pub fn new() -> Self {
        let mut detector = Self {
            patterns: HashMap::new(),
            keywords: HashMap::new(),
        };
        
        detector.init_patterns();
        detector.init_keywords();
        
        detector
    }
    
    fn init_patterns(&mut self) {
        // InfluxQL patterns
        self.patterns.insert(Dialect::InfluxQL, vec![
            Regex::new(r#"(?i)SELECT\s+.+\s+FROM\s+["\w]+\s+(WHERE|GROUP BY|LIMIT|ORDER BY|FILL|TZ)"#).unwrap(),
            Regex::new(r"(?i)\s+GROUP\s+BY\s+time\s*\(").unwrap(),
            Regex::new(r"(?i)SHOW\s+(MEASUREMENTS|TAG\s+KEYS|TAG\s+VALUES|FIELD\s+KEYS|DATABASES|RETENTION\s+POLICIES|SERIES)").unwrap(),
            Regex::new(r"(?i)CREATE\s+(DATABASE|RETENTION\s+POLICY|CONTINUOUS\s+QUERY)").unwrap(),
        ]);
        
        // Flux patterns
        self.patterns.insert(Dialect::Flux, vec![
            Regex::new(r#"from\s*\(\s*bucket\s*:"#).unwrap(),
            Regex::new(r"\|>\s*range\s*\(").unwrap(),
            Regex::new(r"\|>\s*filter\s*\(").unwrap(),
            Regex::new(r"\|>\s*aggregateWindow\s*\(").unwrap(),
            Regex::new(r"\|>\s*yield\s*\(").unwrap(),
            Regex::new(r"\|>\s*map\s*\(").unwrap(),
        ]);
        
        // PromQL patterns
        self.patterns.insert(Dialect::PromQL, vec![
            Regex::new(r"\w+\s*\{[^}]*\}\s*(\[[\w]+\])?").unwrap(),
            Regex::new(r"(rate|irate|increase|delta|deriv|predict_linear|histogram_quantile)\s*\(").unwrap(),
            Regex::new(r"(sum|avg|min|max|count|stddev|topk|bottomk|quantile)\s*(by|without)\s*\(").unwrap(),
            Regex::new(r"\s+offset\s+\d+[smhdwy]").unwrap(),
            Regex::new(r"\[\d+[smhdwy]\]").unwrap(),
        ]);
        
        // TDengine patterns
        self.patterns.insert(Dialect::TDengine, vec![
            Regex::new(r"(?i)CREATE\s+STABLE").unwrap(),
            Regex::new(r"(?i)USING\s+\w+\s+TAGS\s*\(").unwrap(),
            Regex::new(r"(?i)INTERVAL\s*\(\s*\d+[smhd]\s*\)").unwrap(),
            Regex::new(r"(?i)PARTITION\s+BY\s+TBNAME").unwrap(),
            Regex::new(r"(?i)(STATE_WINDOW|SESSION|EVENT_WINDOW|COUNT_WINDOW)\s*\(").unwrap(),
            Regex::new(r"(?i)LAST_ROW\s*\(").unwrap(),
        ]);
        
        // TimescaleDB patterns
        self.patterns.insert(Dialect::TimescaleDB, vec![
            Regex::new(r"(?i)time_bucket\s*\(").unwrap(),
            Regex::new(r"(?i)time_bucket_gapfill\s*\(").unwrap(),
            Regex::new(r"(?i)CREATE\s+HYPERTABLE").unwrap(),
            Regex::new(r"(?i)(locf|interpolate)\s*\(").unwrap(),
        ]);
        
        // QuestDB patterns
        self.patterns.insert(Dialect::QuestDB, vec![
            Regex::new(r"(?i)SAMPLE\s+BY").unwrap(),
            Regex::new(r"(?i)LATEST\s+ON").unwrap(),
            Regex::new(r"(?i)ASOF\s+JOIN").unwrap(),
            Regex::new(r"(?i)(LT|SPLICE)\s+JOIN").unwrap(),
        ]);
        
        // ClickHouse patterns
        self.patterns.insert(Dialect::ClickHouse, vec![
            Regex::new(r"(?i)ENGINE\s*=\s*(MergeTree|ReplacingMergeTree|SummingMergeTree|AggregatingMergeTree)").unwrap(),
            Regex::new(r"(?i)(toDateTime|toDate|toStartOfHour|toStartOfDay)\s*\(").unwrap(),
            Regex::new(r"(?i)arrayJoin\s*\(").unwrap(),
            Regex::new(r"(?i)WITH\s+TOTALS").unwrap(),
            Regex::new(r"(?i)PREWHERE").unwrap(),
            Regex::new(r"(?i)GLOBAL\s+(IN|JOIN)").unwrap(),
        ]);
        
        // Druid SQL patterns
        self.patterns.insert(Dialect::DruidSQL, vec![
            Regex::new(r"(?i)__time").unwrap(),
            Regex::new(r"(?i)FLOOR\s*\(\s*__time").unwrap(),
            Regex::new(r"(?i)TIME_FLOOR\s*\(").unwrap(),
            Regex::new(r"(?i)APPROX_COUNT_DISTINCT\s*\(").unwrap(),
        ]);
        
        // Druid Native (JSON)
        self.patterns.insert(Dialect::DruidNative, vec![
            Regex::new(r#""queryType"\s*:\s*"(timeseries|topN|groupBy|scan|search)""#).unwrap(),
            Regex::new(r#""dataSource"\s*:"#).unwrap(),
            Regex::new(r#""granularity"\s*:"#).unwrap(),
        ]);
        
        // OpenTSDB patterns
        self.patterns.insert(Dialect::OpenTSDB, vec![
            Regex::new(r#""queries"\s*:\s*\["#).unwrap(),
            Regex::new(r#""metric"\s*:\s*""#).unwrap(),
            Regex::new(r#""aggregator"\s*:\s*"(sum|avg|min|max|count)""#).unwrap(),
        ]);
        
        // Graphite patterns
        self.patterns.insert(Dialect::Graphite, vec![
            Regex::new(r"(summarize|derivative|integral|movingAverage|alias)\s*\(").unwrap(),
            Regex::new(r"\*\.\*\.").unwrap(),
        ]);
        
        // MetricsQL patterns (PromQL superset)
        self.patterns.insert(Dialect::MetricsQL, vec![
            Regex::new(r"(range_quantile|range_median|range_avg|range_first|range_last)\s*\(").unwrap(),
            Regex::new(r"(topk_avg|topk_max|topk_min|bottomk_avg)\s*\(").unwrap(),
        ]);
    }
    
    fn init_keywords(&mut self) {
        self.keywords.insert(Dialect::InfluxQL, vec![
            "FILL(", "SLIMIT", "SOFFSET", "TZ(", "INTO",
            "SHOW MEASUREMENTS", "SHOW TAG", "SHOW FIELD",
            "GROUP BY time(",
        ]);
        
        self.keywords.insert(Dialect::Flux, vec![
            "|>", "from(bucket:", "range(", "filter(fn:",
            "aggregateWindow(", "map(fn:", "pivot(",
        ]);
        
        self.keywords.insert(Dialect::PromQL, vec![
            "rate(", "irate(", "increase(", "histogram_quantile(",
            "sum by", "sum without", "avg by", "count by",
            "__name__", "job=", "instance=",
        ]);
        
        self.keywords.insert(Dialect::TDengine, vec![
            "CREATE STABLE", "USING", "TAGS(", "INTERVAL(",
            "PARTITION BY", "STATE_WINDOW", "SESSION(",
            "LAST_ROW(", "TWA(", "SPREAD(", "_wstart", "_wend",
            "FILL(PREV)", "FILL(LINEAR)", "TBNAME",
        ]);
        
        self.keywords.insert(Dialect::TimescaleDB, vec![
            "time_bucket(", "time_bucket_gapfill(",
            "CREATE HYPERTABLE", "locf(", "interpolate(",
            "add_retention_policy", "add_compression_policy",
        ]);
        
        self.keywords.insert(Dialect::QuestDB, vec![
            "SAMPLE BY", "LATEST ON", "ASOF JOIN",
            "LT JOIN", "SPLICE JOIN", "designated timestamp",
        ]);
        
        self.keywords.insert(Dialect::ClickHouse, vec![
            "MergeTree", "ReplacingMergeTree", "ENGINE=",
            "toDateTime(", "toStartOfHour(", "arrayJoin(",
            "PREWHERE", "GLOBAL IN", "WITH TOTALS", "FINAL",
        ]);
        
        self.keywords.insert(Dialect::DruidSQL, vec![
            "__time", "TIME_FLOOR(", "TIME_SHIFT(",
            "APPROX_COUNT_DISTINCT(", "DS_HLL", "DS_THETA",
        ]);
        
        self.keywords.insert(Dialect::Graphite, vec![
            "summarize(", "alias(", "scale(", "offset(",
            "derivative(", "integral(", "movingAverage(",
        ]);
    }
    
    /// Detect the query dialect
    pub fn detect(&self, query: &str) -> Dialect {
        let query = query.trim();
        
        // Check if JSON (Druid native, OpenTSDB)
        if query.starts_with('{') || query.starts_with('[') {
            if self.matches_patterns(query, Dialect::DruidNative) {
                return Dialect::DruidNative;
            }
            if self.matches_patterns(query, Dialect::OpenTSDB) {
                return Dialect::OpenTSDB;
            }
        }
        
        // Score each dialect
        let mut scores: HashMap<Dialect, i32> = HashMap::new();
        
        // Check patterns
        for (dialect, patterns) in &self.patterns {
            for pattern in patterns {
                if pattern.is_match(query) {
                    *scores.entry(*dialect).or_insert(0) += 10;
                }
            }
        }
        
        // Check keywords
        let query_upper = query.to_uppercase();
        for (dialect, keywords) in &self.keywords {
            for keyword in keywords {
                if query_upper.contains(&keyword.to_uppercase()) {
                    *scores.entry(*dialect).or_insert(0) += 5;
                }
            }
        }
        
        // Find highest scoring dialect
        let best = scores.into_iter()
            .max_by_key(|(_, score)| *score);
        
        if let Some((dialect, score)) = best {
            if score >= 5 {
                return dialect;
            }
        }
        
        // Fallback heuristics
        if query_upper.starts_with("SELECT") || query_upper.starts_with("SHOW") {
            // Generic SQL - default to TimescaleDB for time-series
            return Dialect::SQL;
        }
        
        // Check for PromQL-style metric selector
        if Regex::new(r"^[a-zA-Z_:][a-zA-Z0-9_:]*(\{.*\})?(\[.*\])?$")
            .map(|r| r.is_match(query))
            .unwrap_or(false)
        {
            return Dialect::PromQL;
        }
        
        // Default to SQL
        Dialect::SQL
    }
    
    fn matches_patterns(&self, query: &str, dialect: Dialect) -> bool {
        if let Some(patterns) = self.patterns.get(&dialect) {
            for pattern in patterns {
                if pattern.is_match(query) {
                    return true;
                }
            }
        }
        false
    }
    
    /// Get detection confidence score (0.0 - 1.0)
    pub fn detect_with_confidence(&self, query: &str) -> (Dialect, f64) {
        let query = query.trim();
        let mut scores: HashMap<Dialect, i32> = HashMap::new();
        
        // Pattern matching
        for (dialect, patterns) in &self.patterns {
            for pattern in patterns {
                if pattern.is_match(query) {
                    *scores.entry(*dialect).or_insert(0) += 10;
                }
            }
        }
        
        // Keyword matching
        let query_upper = query.to_uppercase();
        for (dialect, keywords) in &self.keywords {
            for keyword in keywords {
                if query_upper.contains(&keyword.to_uppercase()) {
                    *scores.entry(*dialect).or_insert(0) += 5;
                }
            }
        }
        
        let total_score: i32 = scores.values().sum();
        
        if let Some((dialect, score)) = scores.into_iter().max_by_key(|(_, s)| *s) {
            let confidence = if total_score > 0 {
                score as f64 / total_score as f64
            } else {
                0.0
            };
            (dialect, confidence)
        } else {
            (Dialect::SQL, 0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_influxql() {
        let detector = DialectDetector::new();
        
        assert_eq!(
            detector.detect(r#"SELECT mean(value) FROM cpu WHERE time > now() - 1h GROUP BY time(5m) FILL(null)"#),
            Dialect::InfluxQL
        );
        assert_eq!(
            detector.detect("SHOW MEASUREMENTS"),
            Dialect::InfluxQL
        );
    }
    
    #[test]
    fn test_detect_flux() {
        let detector = DialectDetector::new();
        
        assert_eq!(
            detector.detect(r#"from(bucket: "my-bucket") |> range(start: -1h) |> filter(fn: (r) => r._measurement == "cpu")"#),
            Dialect::Flux
        );
    }
    
    #[test]
    fn test_detect_promql() {
        let detector = DialectDetector::new();
        
        assert_eq!(
            detector.detect(r#"rate(http_requests_total{job="api"}[5m])"#),
            Dialect::PromQL
        );
        assert_eq!(
            detector.detect(r#"sum by (instance) (rate(node_cpu_seconds_total[5m]))"#),
            Dialect::PromQL
        );
    }
    
    #[test]
    fn test_detect_tdengine() {
        let detector = DialectDetector::new();
        
        assert_eq!(
            detector.detect("SELECT _wstart, avg(current) FROM power.meters WHERE ts > NOW() - 1h INTERVAL(5m) FILL(PREV)"),
            Dialect::TDengine
        );
        assert_eq!(
            detector.detect("CREATE STABLE meters (ts TIMESTAMP, current FLOAT) TAGS (location BINARY(64))"),
            Dialect::TDengine
        );
    }
    
    #[test]
    fn test_detect_timescaledb() {
        let detector = DialectDetector::new();
        
        assert_eq!(
            detector.detect("SELECT time_bucket('5 minutes', time) AS bucket, avg(temperature) FROM conditions GROUP BY bucket"),
            Dialect::TimescaleDB
        );
    }
    
    #[test]
    fn test_detect_questdb() {
        let detector = DialectDetector::new();
        
        assert_eq!(
            detector.detect("SELECT ts, avg(value) FROM sensors SAMPLE BY 5m"),
            Dialect::QuestDB
        );
        assert_eq!(
            detector.detect("SELECT * FROM trades LATEST ON timestamp PARTITION BY symbol"),
            Dialect::QuestDB
        );
    }
    
    #[test]
    fn test_detect_clickhouse() {
        let detector = DialectDetector::new();
        
        assert_eq!(
            detector.detect("SELECT toStartOfHour(timestamp) AS hour, count() FROM events GROUP BY hour WITH TOTALS"),
            Dialect::ClickHouse
        );
    }
    
    #[test]
    fn test_detect_druid_sql() {
        let detector = DialectDetector::new();
        
        assert_eq!(
            detector.detect("SELECT FLOOR(__time TO HOUR), COUNT(*) FROM datasource GROUP BY 1"),
            Dialect::DruidSQL
        );
    }
    
    #[test]
    fn test_detect_graphite() {
        let detector = DialectDetector::new();
        
        assert_eq!(
            detector.detect(r#"summarize(servers.*.cpu.user, "1h", "avg")"#),
            Dialect::Graphite
        );
    }
    
    #[test]
    fn test_confidence_scoring() {
        let detector = DialectDetector::new();
        
        let (dialect, confidence) = detector.detect_with_confidence(
            r#"rate(http_requests_total{job="api"}[5m])"#
        );
        
        assert_eq!(dialect, Dialect::PromQL);
        assert!(confidence > 0.5);
    }
}
