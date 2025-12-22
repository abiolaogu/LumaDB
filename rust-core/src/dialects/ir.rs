//! Unified Intermediate Representation (IR) for Multi-Dialect Queries
//!
//! All query dialects (InfluxQL, Flux, PromQL, TDengine, TimescaleDB, QuestDB,
//! ClickHouse, Druid, OpenTSDB, Graphite, MetricsQL) compile down to this
//! common representation for execution by LumaDB's query engine.

use std::collections::HashMap;
use std::time::Duration;

/// The unified query plan that all dialects compile to
#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// Data sources (tables, metrics, measurements)
    pub sources: Vec<DataSource>,
    
    /// Time range for the query
    pub time_range: Option<TimeRange>,
    
    /// Filter conditions (WHERE clauses, label matchers)
    pub filters: Vec<Filter>,
    
    /// Aggregation operations
    pub aggregations: Vec<Aggregation>,
    
    /// Window specifications (INTERVAL, range vectors, etc.)
    pub windows: Vec<Window>,
    
    /// Grouping specifications
    pub group_by: Vec<GroupBy>,
    
    /// Transformations (derivative, rate, fill, etc.)
    pub transformations: Vec<Transformation>,
    
    /// Ordering specifications
    pub order_by: Vec<OrderBy>,
    
    /// Limit on result rows
    pub limit: Option<i64>,
    
    /// Offset for pagination
    pub offset: Option<i64>,
    
    /// Output format specification
    pub output_format: OutputFormat,
    
    /// Query hints for optimization
    pub hints: QueryHints,
    
    /// Database context
    pub database: Option<String>,
    
    /// Original query text (for debugging)
    pub original_query: String,
    
    /// Source dialect
    pub source_dialect: Dialect,
}

impl QueryPlan {
    pub fn new(dialect: Dialect) -> Self {
        Self {
            sources: Vec::new(),
            time_range: None,
            filters: Vec::new(),
            aggregations: Vec::new(),
            windows: Vec::new(),
            group_by: Vec::new(),
            transformations: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            output_format: OutputFormat::default(),
            hints: QueryHints::default(),
            database: None,
            original_query: String::new(),
            source_dialect: dialect,
        }
    }
    
    pub fn with_source(mut self, source: DataSource) -> Self {
        self.sources.push(source);
        self
    }
    
    pub fn with_time_range(mut self, range: TimeRange) -> Self {
        self.time_range = Some(range);
        self
    }
    
    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filters.push(filter);
        self
    }
    
    pub fn with_aggregation(mut self, agg: Aggregation) -> Self {
        self.aggregations.push(agg);
        self
    }
    
    pub fn with_window(mut self, window: Window) -> Self {
        self.windows.push(window);
        self
    }
    
    pub fn with_group_by(mut self, group: GroupBy) -> Self {
        self.group_by.push(group);
        self
    }
    
    pub fn set_database(&mut self, db: &str) {
        self.database = Some(db.to_string());
    }
    
    pub fn set_time_range(&mut self, start: i64, end: i64) {
        self.time_range = Some(TimeRange::Absolute { start, end });
    }
    
    pub fn set_evaluation_time(&mut self, ts: i64) {
        if let Some(TimeRange::Relative { duration, .. }) = &self.time_range {
            self.time_range = Some(TimeRange::Relative {
                duration: *duration,
                anchor: Some(ts),
            });
        }
    }
    
    pub fn set_resolution(&mut self, step_ms: i64) {
        self.hints.step_ms = Some(step_ms);
    }
    
    pub fn set_timeout(&mut self, timeout_ms: i64) {
        self.hints.timeout_ms = Some(timeout_ms);
    }
    
    pub fn set_limit(&mut self, limit: i64) {
        self.limit = Some(limit);
    }
}

/// Query dialect enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dialect {
    /// LumaDB native queries
    LQL,
    NQL,
    JQL,
    
    /// InfluxDB dialects
    InfluxQL,
    Flux,
    
    /// Prometheus
    PromQL,
    MetricsQL,
    
    /// SQL-based time-series
    TDengine,
    TimescaleDB,
    QuestDB,
    ClickHouse,
    
    /// Druid
    DruidSQL,
    DruidNative,
    
    /// Other
    OpenTSDB,
    Graphite,
    
    /// Generic SQL
    SQL,
}

impl Dialect {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "lql" => Some(Dialect::LQL),
            "nql" => Some(Dialect::NQL),
            "jql" => Some(Dialect::JQL),
            "influxql" => Some(Dialect::InfluxQL),
            "flux" => Some(Dialect::Flux),
            "promql" => Some(Dialect::PromQL),
            "metricsql" => Some(Dialect::MetricsQL),
            "tdengine" => Some(Dialect::TDengine),
            "timescale" | "timescaledb" => Some(Dialect::TimescaleDB),
            "questdb" => Some(Dialect::QuestDB),
            "clickhouse" => Some(Dialect::ClickHouse),
            "druidsql" => Some(Dialect::DruidSQL),
            "druidnative" => Some(Dialect::DruidNative),
            "opentsdb" => Some(Dialect::OpenTSDB),
            "graphite" => Some(Dialect::Graphite),
            "sql" => Some(Dialect::SQL),
            _ => None,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            Dialect::LQL => "lql",
            Dialect::NQL => "nql",
            Dialect::JQL => "jql",
            Dialect::InfluxQL => "influxql",
            Dialect::Flux => "flux",
            Dialect::PromQL => "promql",
            Dialect::MetricsQL => "metricsql",
            Dialect::TDengine => "tdengine",
            Dialect::TimescaleDB => "timescaledb",
            Dialect::QuestDB => "questdb",
            Dialect::ClickHouse => "clickhouse",
            Dialect::DruidSQL => "druidsql",
            Dialect::DruidNative => "druidnative",
            Dialect::OpenTSDB => "opentsdb",
            Dialect::Graphite => "graphite",
            Dialect::SQL => "sql",
        }
    }
}

/// Data source (table, metric, measurement)
#[derive(Debug, Clone)]
pub struct DataSource {
    /// Source name (table, metric, measurement)
    pub name: String,
    
    /// Database/bucket/namespace
    pub database: Option<String>,
    
    /// Retention policy (InfluxDB)
    pub retention_policy: Option<String>,
    
    /// Alias for the source
    pub alias: Option<String>,
    
    /// Source type
    pub source_type: DataSourceType,
}

#[derive(Debug, Clone)]
pub enum DataSourceType {
    Table,
    Metric,
    Measurement,
    SuperTable,
    SubTable,
    Stream,
    Subquery(Box<QueryPlan>),
}

/// Time range specification
#[derive(Debug, Clone)]
pub enum TimeRange {
    /// Absolute time range with Unix timestamps (milliseconds)
    Absolute {
        start: i64,
        end: i64,
    },
    /// Relative time range (e.g., "last 1h")
    Relative {
        duration: i64,  // milliseconds
        anchor: Option<i64>,  // evaluation time, defaults to now
    },
    /// Open-ended range
    Since {
        start: i64,
    },
    Until {
        end: i64,
    },
}

/// Filter condition
#[derive(Debug, Clone)]
pub struct Filter {
    pub condition: FilterCondition,
}

#[derive(Debug, Clone)]
pub enum FilterCondition {
    /// Simple comparison: column op value
    Comparison {
        column: String,
        op: ComparisonOp,
        value: Value,
    },
    /// Regex match (PromQL =~, InfluxQL =~)
    Regex {
        column: String,
        pattern: String,
        negated: bool,
    },
    /// IN clause
    In {
        column: String,
        values: Vec<Value>,
        negated: bool,
    },
    /// BETWEEN clause
    Between {
        column: String,
        low: Value,
        high: Value,
        negated: bool,
    },
    /// IS NULL
    IsNull {
        column: String,
        negated: bool,
    },
    /// Logical AND
    And(Vec<FilterCondition>),
    /// Logical OR
    Or(Vec<FilterCondition>),
    /// Logical NOT
    Not(Box<FilterCondition>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComparisonOp {
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Like,
    NotLike,
}

/// Value types
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String),
    Timestamp(i64),
    Duration(i64),
    Binary(Vec<u8>),
    Array(Vec<Value>),
}

/// Aggregation specification
#[derive(Debug, Clone)]
pub struct Aggregation {
    /// Aggregation function
    pub function: AggFunction,
    
    /// Column to aggregate (None for COUNT(*))
    pub column: Option<String>,
    
    /// Additional arguments
    pub args: Vec<Value>,
    
    /// Alias for the result
    pub alias: Option<String>,
    
    /// DISTINCT modifier
    pub distinct: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggFunction {
    // Basic aggregations
    Count,
    Sum,
    Avg,
    Min,
    Max,
    
    // Statistical
    Stddev,
    StddevPop,
    StddevSamp,
    Variance,
    VarPop,
    VarSamp,
    
    // Time-series specific
    First,
    Last,
    FirstRow,
    LastRow,
    Spread,       // max - min
    Mode,
    Median,
    
    // Percentiles
    Percentile(f64),
    Apercentile(f64),
    
    // PromQL functions
    Rate,
    Irate,
    Increase,
    Delta,
    Idelta,
    Deriv,
    PredictLinear,
    Resets,
    Changes,
    
    // Time-weighted
    Twa,          // Time-weighted average
    Integral,
    
    // Sampling
    Sample(usize),
    TopK(usize),
    BottomK(usize),
    
    // Cardinality
    CountDistinct,
    HyperLogLog,
    
    // Histogram
    HistogramQuantile(f64),
    Histogram,
    
    // Custom/Other
    Custom(String),
}

/// Window specification
#[derive(Debug, Clone)]
pub struct Window {
    pub window_type: WindowType,
    pub fill: Option<FillStrategy>,
}

#[derive(Debug, Clone)]
pub enum WindowType {
    /// Time-based interval (InfluxQL GROUP BY time(), TDengine INTERVAL)
    Interval {
        duration: i64,      // milliseconds
        offset: Option<i64>,
        sliding: Option<i64>,
    },
    /// PromQL range vector
    Range {
        duration: i64,
    },
    /// Session window (TDengine SESSION)
    Session {
        gap: i64,
    },
    /// State window (TDengine STATE_WINDOW)
    State {
        column: String,
    },
    /// Event window (TDengine EVENT_WINDOW)
    Event {
        start_condition: Box<FilterCondition>,
        end_condition: Box<FilterCondition>,
    },
    /// Count window (TDengine COUNT_WINDOW)
    Count {
        count: i64,
        sliding: Option<i64>,
    },
    /// Sample by (QuestDB SAMPLE BY)
    SampleBy {
        interval: String,
        align: Option<String>,
    },
    /// Row-based window
    Rows {
        preceding: Option<i64>,
        following: Option<i64>,
    },
}

#[derive(Debug, Clone)]
pub enum FillStrategy {
    None,
    Null,
    Previous,
    Next,
    Linear,
    Value(Value),
}

/// Group by specification
#[derive(Debug, Clone)]
pub struct GroupBy {
    pub expr: GroupByExpr,
}

#[derive(Debug, Clone)]
pub enum GroupByExpr {
    /// Group by column
    Column(String),
    /// Group by time bucket
    TimeBucket {
        interval: i64,
        column: Option<String>,
    },
    /// Group by tag (TDengine PARTITION BY)
    Tag(String),
    /// Group by all tags (PARTITION BY TBNAME)
    AllTags,
    /// Group by expression
    Expression(String),
}

/// Transformation specification
#[derive(Debug, Clone)]
pub struct Transformation {
    pub transform_type: TransformType,
    pub column: Option<String>,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TransformType {
    // Math functions
    Abs,
    Ceil,
    Floor,
    Round(Option<i32>),
    Sqrt,
    Log(Option<f64>),
    Exp,
    Pow(f64),
    
    // Time functions
    Derivative { unit: Option<String> },
    NonNegativeDerivative { unit: Option<String> },
    Difference,
    NonNegativeDifference,
    MovingAverage { points: usize },
    CumulativeSum,
    Elapsed { unit: Option<String> },
    
    // Fill/Interpolation
    Fill(FillStrategy),
    Interpolate,
    
    // String functions
    Concat(Vec<String>),
    Substring { start: i32, length: Option<i32> },
    Lower,
    Upper,
    Trim,
    
    // Label manipulation (PromQL)
    LabelReplace {
        dst_label: String,
        replacement: String,
        src_label: String,
        regex: String,
    },
    LabelJoin {
        dst_label: String,
        separator: String,
        src_labels: Vec<String>,
    },
    
    // Cast
    Cast(DataType),
    
    // Custom
    Custom { name: String, args: Vec<Value> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    String,
    Binary,
    Timestamp,
    Duration,
    Json,
}

/// Order by specification
#[derive(Debug, Clone)]
pub struct OrderBy {
    pub column: String,
    pub ascending: bool,
    pub nulls_first: Option<bool>,
}

/// Output format specification
#[derive(Debug, Clone, Default)]
pub struct OutputFormat {
    /// Target dialect for response formatting
    pub dialect: Option<Dialect>,
    
    /// Timestamp format
    pub timestamp_format: TimestampFormat,
    
    /// Include column metadata
    pub include_meta: bool,
    
    /// Result type (for PromQL: vector, matrix, scalar)
    pub result_type: Option<String>,
    
    /// Custom format options
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub enum TimestampFormat {
    #[default]
    UnixMs,
    UnixNs,
    UnixUs,
    UnixS,
    Rfc3339,
    Iso8601,
    Custom(String),
}

/// Query optimization hints
#[derive(Debug, Clone, Default)]
pub struct QueryHints {
    /// Force index usage
    pub force_index: Option<String>,
    
    /// Parallel execution hint
    pub parallel: Option<usize>,
    
    /// Memory limit
    pub memory_limit: Option<usize>,
    
    /// Timeout in milliseconds
    pub timeout_ms: Option<i64>,
    
    /// Step/resolution for range queries
    pub step_ms: Option<i64>,
    
    /// Lookback delta for PromQL
    pub lookback_delta: Option<i64>,
    
    /// Cache hint
    pub use_cache: bool,
    
    /// Custom hints
    pub custom: HashMap<String, String>,
}

/// Query result
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Column metadata
    pub columns: Vec<ColumnMeta>,
    
    /// Data rows
    pub rows: Vec<Row>,
    
    /// Total row count (if known)
    pub total_rows: Option<i64>,
    
    /// Execution stats
    pub stats: ExecutionStats,
}

#[derive(Debug, Clone)]
pub struct ColumnMeta {
    pub name: String,
    pub data_type: DataType,
    pub is_tag: bool,
}

pub type Row = Vec<Value>;

#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    pub execution_time_ms: f64,
    pub rows_scanned: i64,
    pub bytes_scanned: i64,
}

/// Parser trait for all dialects
pub trait DialectParser: Send + Sync {
    fn parse(&self, query: &str) -> Result<QueryPlan, ParseError>;
    fn dialect(&self) -> Dialect;
}

/// Translator trait for converting IR to dialect-specific SQL
pub trait DialectTranslator: Send + Sync {
    fn translate(&self, plan: &QueryPlan) -> Result<String, TranslateError>;
    fn target_dialect(&self) -> Dialect;
}

/// Parse error
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub position: Option<usize>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

/// Translation error
#[derive(Debug, Clone)]
pub struct TranslateError {
    pub message: String,
    pub unsupported_feature: Option<String>,
}

impl std::fmt::Display for TranslateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TranslateError {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_query_plan_builder() {
        let plan = QueryPlan::new(Dialect::PromQL)
            .with_source(DataSource {
                name: "http_requests_total".to_string(),
                database: None,
                retention_policy: None,
                alias: None,
                source_type: DataSourceType::Metric,
            })
            .with_time_range(TimeRange::Relative {
                duration: 3600000,
                anchor: None,
            })
            .with_aggregation(Aggregation {
                function: AggFunction::Rate,
                column: Some("value".to_string()),
                args: vec![],
                alias: None,
                distinct: false,
            });
        
        assert_eq!(plan.source_dialect, Dialect::PromQL);
        assert_eq!(plan.sources.len(), 1);
        assert!(plan.time_range.is_some());
    }
    
    #[test]
    fn test_dialect_from_str() {
        assert_eq!(Dialect::from_str("promql"), Some(Dialect::PromQL));
        assert_eq!(Dialect::from_str("InfluxQL"), Some(Dialect::InfluxQL));
        assert_eq!(Dialect::from_str("tdengine"), Some(Dialect::TDengine));
        assert_eq!(Dialect::from_str("unknown"), None);
    }
}
