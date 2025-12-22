//! Universal Translator
//!
//! Translates between query dialects using the unified IR.

use crate::dialects::ir::*;

/// Universal translator that can convert IR to any target dialect
pub struct UniversalTranslator;

impl UniversalTranslator {
    pub fn new() -> Self {
        Self
    }
    
    /// Translate a query plan to the specified target dialect
    pub fn translate(&self, plan: &QueryPlan, target: Dialect) -> Result<String, TranslateError> {
        match target {
            Dialect::InfluxQL => self.to_influxql(plan),
            Dialect::Flux => self.to_flux(plan),
            Dialect::PromQL => self.to_promql(plan),
            Dialect::SQL | Dialect::TimescaleDB => self.to_sql(plan),
            Dialect::TDengine => self.to_tdengine(plan),
            Dialect::QuestDB => self.to_questdb(plan),
            Dialect::ClickHouse => self.to_clickhouse(plan),
            _ => Err(TranslateError {
                message: format!("Translation to {:?} not yet supported", target),
                unsupported_feature: None,
            }),
        }
    }
    
    fn to_influxql(&self, plan: &QueryPlan) -> Result<String, TranslateError> {
        let mut sql = String::from("SELECT ");
        
        // Aggregations or *
        if plan.aggregations.is_empty() {
            sql.push('*');
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
                    AggFunction::Stddev => "stddev",
                    AggFunction::Spread => "spread",
                    AggFunction::Rate => "derivative",
                    _ => "mean",
                };
                let col = agg.column.as_deref().unwrap_or("value");
                format!("{}(\"{}\")", func, col)
            }).collect();
            sql.push_str(&aggs.join(", "));
        }
        
        // FROM
        if let Some(source) = plan.sources.first() {
            sql.push_str(&format!(" FROM \"{}\"", source.name));
        }
        
        // WHERE with time range
        if let Some(ref time_range) = plan.time_range {
            match time_range {
                TimeRange::Relative { duration, .. } => {
                    let dur = format_influx_duration(*duration);
                    sql.push_str(&format!(" WHERE time > now() - {}", dur));
                }
                TimeRange::Absolute { start, end } => {
                    sql.push_str(&format!(" WHERE time >= {}ms AND time < {}ms", start, end));
                }
                _ => {}
            }
        }
        
        // GROUP BY time
        if let Some(window) = plan.windows.first() {
            if let WindowType::Interval { duration, .. } = &window.window_type {
                let dur = format_influx_duration(*duration);
                sql.push_str(&format!(" GROUP BY time({})", dur));
                
                if let Some(ref fill) = window.fill {
                    let fill_str = match fill {
                        FillStrategy::None => "none",
                        FillStrategy::Null => "null",
                        FillStrategy::Previous => "previous",
                        FillStrategy::Linear => "linear",
                        _ => "none",
                    };
                    sql.push_str(&format!(" FILL({})", fill_str));
                }
            }
        }
        
        // LIMIT
        if let Some(limit) = plan.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        Ok(sql)
    }
    
    fn to_flux(&self, plan: &QueryPlan) -> Result<String, TranslateError> {
        let mut parts = Vec::new();
        
        // from(bucket: ...)
        if let Some(source) = plan.sources.first() {
            let bucket = source.database.as_deref().unwrap_or(&source.name);
            parts.push(format!("from(bucket: \"{}\")", bucket));
        } else {
            parts.push("from(bucket: \"default\")".to_string());
        }
        
        // range
        if let Some(ref time_range) = plan.time_range {
            match time_range {
                TimeRange::Relative { duration, .. } => {
                    let dur = format_flux_duration(*duration);
                    parts.push(format!("|> range(start: -{})", dur));
                }
                TimeRange::Absolute { start, end } => {
                    parts.push(format!("|> range(start: {}, stop: {})", start / 1000, end / 1000));
                }
                _ => {}
            }
        } else {
            parts.push("|> range(start: -1h)".to_string());
        }
        
        // filter for measurement
        if let Some(source) = plan.sources.first() {
            parts.push(format!("|> filter(fn: (r) => r._measurement == \"{}\")", source.name));
        }
        
        // filters
        for filter in &plan.filters {
            if let FilterCondition::Comparison { column, op, value } = &filter.condition {
                if let Value::String(v) = value {
                    let op_str = match op {
                        ComparisonOp::Eq => "==",
                        ComparisonOp::NotEq => "!=",
                        _ => "==",
                    };
                    parts.push(format!("|> filter(fn: (r) => r.{} {} \"{}\")", column, op_str, v));
                }
            }
        }
        
        // aggregateWindow
        if let Some(window) = plan.windows.first() {
            if let WindowType::Interval { duration, .. } = &window.window_type {
                let dur = format_flux_duration(*duration);
                let func = if let Some(agg) = plan.aggregations.first() {
                    match &agg.function {
                        AggFunction::Avg => "mean",
                        AggFunction::Sum => "sum",
                        AggFunction::Count => "count",
                        AggFunction::Min => "min",
                        AggFunction::Max => "max",
                        AggFunction::First => "first",
                        AggFunction::Last => "last",
                        _ => "mean",
                    }
                } else {
                    "mean"
                };
                parts.push(format!("|> aggregateWindow(every: {}, fn: {})", dur, func));
            }
        }
        
        // limit
        if let Some(limit) = plan.limit {
            parts.push(format!("|> limit(n: {})", limit));
        }
        
        Ok(parts.join("\n    "))
    }
    
    fn to_promql(&self, plan: &QueryPlan) -> Result<String, TranslateError> {
        let mut query = String::new();
        
        // Metric selector
        if let Some(source) = plan.sources.first() {
            query.push_str(&source.name);
        } else {
            query.push_str("metric");
        }
        
        // Label matchers
        let mut matchers = Vec::new();
        for filter in &plan.filters {
            match &filter.condition {
                FilterCondition::Comparison { column, op, value } => {
                    if let Value::String(v) = value {
                        let op_str = match op {
                            ComparisonOp::Eq => "=",
                            ComparisonOp::NotEq => "!=",
                            _ => "=",
                        };
                        matchers.push(format!("{}{}\"{}\"", column, op_str, v));
                    }
                }
                FilterCondition::Regex { column, pattern, negated } => {
                    let op_str = if *negated { "!~" } else { "=~" };
                    matchers.push(format!("{}{}\"{}\"", column, op_str, pattern));
                }
                _ => {}
            }
        }
        
        if !matchers.is_empty() {
            query.push('{');
            query.push_str(&matchers.join(", "));
            query.push('}');
        }
        
        // Range vector
        if let Some(window) = plan.windows.first() {
            if let WindowType::Range { duration } | WindowType::Interval { duration, .. } = &window.window_type {
                let dur = format_promql_duration(*duration);
                query.push('[');
                query.push_str(&dur);
                query.push(']');
            }
        }
        
        // Wrap with functions
        for agg in &plan.aggregations {
            let func_name = match &agg.function {
                AggFunction::Rate => "rate",
                AggFunction::Irate => "irate",
                AggFunction::Increase => "increase",
                AggFunction::Delta => "delta",
                AggFunction::Avg => "avg_over_time",
                AggFunction::Sum => "sum_over_time",
                AggFunction::Min => "min_over_time",
                AggFunction::Max => "max_over_time",
                AggFunction::Count => "count_over_time",
                AggFunction::HistogramQuantile(q) => {
                    query = format!("histogram_quantile({}, {})", q, query);
                    continue;
                }
                _ => continue,
            };
            query = format!("{}({})", func_name, query);
        }
        
        // Aggregation operators
        let agg_ops: Vec<String> = plan.aggregations.iter()
            .filter_map(|agg| {
                match &agg.function {
                    AggFunction::Sum => Some("sum".to_string()),
                    AggFunction::Avg => Some("avg".to_string()),
                    AggFunction::Min => Some("min".to_string()),
                    AggFunction::Max => Some("max".to_string()),
                    AggFunction::Count => Some("count".to_string()),
                    AggFunction::TopK(k) => Some(format!("topk({}, ", k)),
                    AggFunction::BottomK(k) => Some(format!("bottomk({}, ", k)),
                    _ => None,
                }
            })
            .collect();
        
        for op in agg_ops {
            let labels: Vec<_> = plan.group_by.iter()
                .filter_map(|g| match &g.expr {
                    GroupByExpr::Tag(t) => Some(t.as_str()),
                    _ => None,
                })
                .collect();
            
            if labels.is_empty() {
                query = format!("{}({})", op, query);
            } else {
                query = format!("{} by ({}) ({})", op, labels.join(", "), query);
            }
        }
        
        Ok(query)
    }
    
    fn to_sql(&self, plan: &QueryPlan) -> Result<String, TranslateError> {
        let mut sql = String::from("SELECT ");
        
        // Time bucket for GROUP BY
        let mut select_parts = Vec::new();
        
        if let Some(window) = plan.windows.first() {
            if let WindowType::Interval { duration, .. } = &window.window_type {
                let interval = format_postgres_interval(*duration);
                select_parts.push(format!("time_bucket('{}', time) AS bucket", interval));
            }
        }
        
        // Aggregations
        for agg in &plan.aggregations {
            let func = match &agg.function {
                AggFunction::Avg => "AVG",
                AggFunction::Sum => "SUM",
                AggFunction::Count => "COUNT",
                AggFunction::Min => "MIN",
                AggFunction::Max => "MAX",
                AggFunction::First => "FIRST",
                AggFunction::Last => "LAST",
                AggFunction::Stddev => "STDDEV",
                _ => continue,
            };
            let col = agg.column.as_deref().unwrap_or("value");
            let alias = agg.alias.as_ref().map(|a| format!(" AS {}", a)).unwrap_or_default();
            select_parts.push(format!("{}({}){}", func, col, alias));
        }
        
        if select_parts.is_empty() {
            sql.push('*');
        } else {
            sql.push_str(&select_parts.join(", "));
        }
        
        // FROM
        if let Some(source) = plan.sources.first() {
            sql.push_str(&format!(" FROM {}", source.name));
        }
        
        // WHERE
        let mut where_parts = Vec::new();
        
        if let Some(ref time_range) = plan.time_range {
            match time_range {
                TimeRange::Relative { duration, .. } => {
                    let interval = format_postgres_interval(*duration);
                    where_parts.push(format!("time > NOW() - INTERVAL '{}'", interval));
                }
                TimeRange::Absolute { start, end } => {
                    where_parts.push(format!("time >= to_timestamp({}) AND time < to_timestamp({})", start / 1000, end / 1000));
                }
                _ => {}
            }
        }
        
        for filter in &plan.filters {
            if let FilterCondition::Comparison { column, op, value } = &filter.condition {
                let op_str = match op {
                    ComparisonOp::Eq => "=",
                    ComparisonOp::NotEq => "!=",
                    ComparisonOp::Lt => "<",
                    ComparisonOp::LtEq => "<=",
                    ComparisonOp::Gt => ">",
                    ComparisonOp::GtEq => ">=",
                    _ => "=",
                };
                let val_str = match value {
                    Value::String(s) => format!("'{}'", s),
                    Value::Int(i) => i.to_string(),
                    Value::Float(f) => f.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                where_parts.push(format!("{} {} {}", column, op_str, val_str));
            }
        }
        
        if !where_parts.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_parts.join(" AND "));
        }
        
        // GROUP BY
        let mut group_parts = Vec::new();
        if plan.windows.iter().any(|w| matches!(w.window_type, WindowType::Interval { .. })) {
            group_parts.push("bucket".to_string());
        }
        for group in &plan.group_by {
            match &group.expr {
                GroupByExpr::Column(c) => group_parts.push(c.clone()),
                GroupByExpr::Tag(t) => group_parts.push(t.clone()),
                _ => {}
            }
        }
        if !group_parts.is_empty() {
            sql.push_str(" GROUP BY ");
            sql.push_str(&group_parts.join(", "));
        }
        
        // ORDER BY
        if !plan.order_by.is_empty() {
            let orders: Vec<String> = plan.order_by.iter().map(|o| {
                let dir = if o.ascending { "ASC" } else { "DESC" };
                format!("{} {}", o.column, dir)
            }).collect();
            sql.push_str(" ORDER BY ");
            sql.push_str(&orders.join(", "));
        }
        
        // LIMIT
        if let Some(limit) = plan.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        Ok(sql)
    }
    
    fn to_tdengine(&self, plan: &QueryPlan) -> Result<String, TranslateError> {
        let mut sql = String::from("SELECT ");
        
        // Window columns
        let mut select_parts = Vec::new();
        
        if plan.windows.iter().any(|w| matches!(w.window_type, WindowType::Interval { .. })) {
            select_parts.push("_wstart".to_string());
        }
        
        // Aggregations
        for agg in &plan.aggregations {
            let func = match &agg.function {
                AggFunction::Avg => "AVG",
                AggFunction::Sum => "SUM",
                AggFunction::Count => "COUNT",
                AggFunction::Min => "MIN",
                AggFunction::Max => "MAX",
                AggFunction::First => "FIRST",
                AggFunction::Last => "LAST",
                AggFunction::LastRow => "LAST_ROW",
                AggFunction::Twa => "TWA",
                AggFunction::Spread => "SPREAD",
                AggFunction::Stddev => "STDDEV",
                _ => continue,
            };
            let col = agg.column.as_deref().unwrap_or("*");
            select_parts.push(format!("{}({})", func, col));
        }
        
        if select_parts.is_empty() {
            sql.push('*');
        } else {
            sql.push_str(&select_parts.join(", "));
        }
        
        // FROM
        if let Some(source) = plan.sources.first() {
            sql.push_str(&format!(" FROM {}", source.name));
        }
        
        // WHERE with time
        let mut where_added = false;
        if let Some(ref time_range) = plan.time_range {
            match time_range {
                TimeRange::Relative { duration, .. } => {
                    let dur = format_tdengine_duration(*duration);
                    sql.push_str(&format!(" WHERE ts > NOW() - {}", dur));
                    where_added = true;
                }
                _ => {}
            }
        }
        
        // Other filters
        for filter in &plan.filters {
            if let FilterCondition::Comparison { column, op, value } = &filter.condition {
                let op_str = match op {
                    ComparisonOp::Eq => "=",
                    ComparisonOp::NotEq => "!=",
                    _ => "=",
                };
                if let Value::String(v) = value {
                    let prefix = if where_added { " AND" } else { " WHERE" };
                    sql.push_str(&format!("{} {} {} '{}'", prefix, column, op_str, v));
                    where_added = true;
                }
            }
        }
        
        // PARTITION BY
        for group in &plan.group_by {
            match &group.expr {
                GroupByExpr::AllTags => {
                    sql.push_str(" PARTITION BY TBNAME");
                }
                GroupByExpr::Tag(t) => {
                    sql.push_str(&format!(" PARTITION BY {}", t));
                }
                _ => {}
            }
        }
        
        // INTERVAL
        if let Some(window) = plan.windows.first() {
            if let WindowType::Interval { duration, sliding, .. } = &window.window_type {
                let dur = format_tdengine_duration(*duration);
                sql.push_str(&format!(" INTERVAL({})", dur));
                
                if let Some(slide) = sliding {
                    let slide_dur = format_tdengine_duration(*slide);
                    sql.push_str(&format!(" SLIDING({})", slide_dur));
                }
                
                if let Some(ref fill) = window.fill {
                    let fill_str = match fill {
                        FillStrategy::None => "NONE",
                        FillStrategy::Null => "NULL",
                        FillStrategy::Previous => "PREV",
                        FillStrategy::Next => "NEXT",
                        FillStrategy::Linear => "LINEAR",
                        FillStrategy::Value(Value::Float(v)) => return Ok(format!("{} FILL(VALUE, {})", sql, v)),
                        _ => "NONE",
                    };
                    sql.push_str(&format!(" FILL({})", fill_str));
                }
            }
        }
        
        // LIMIT
        if let Some(limit) = plan.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        Ok(sql)
    }
    
    fn to_questdb(&self, plan: &QueryPlan) -> Result<String, TranslateError> {
        let mut sql = String::from("SELECT ");
        
        // SELECT
        let mut select_parts = Vec::new();
        
        for agg in &plan.aggregations {
            let func = match &agg.function {
                AggFunction::Avg => "avg",
                AggFunction::Sum => "sum",
                AggFunction::Count => "count",
                AggFunction::Min => "min",
                AggFunction::Max => "max",
                AggFunction::First => "first",
                AggFunction::Last => "last",
                _ => continue,
            };
            let col = agg.column.as_deref().unwrap_or("*");
            select_parts.push(format!("{}({})", func, col));
        }
        
        if select_parts.is_empty() {
            sql.push('*');
        } else {
            sql.push_str(&select_parts.join(", "));
        }
        
        // FROM
        if let Some(source) = plan.sources.first() {
            sql.push_str(&format!(" FROM {}", source.name));
        }
        
        // WHERE
        if let Some(ref time_range) = plan.time_range {
            if let TimeRange::Relative { duration, .. } = time_range {
                let unit = if *duration >= 86400000 {
                    format!("'d', -{}", duration / 86400000)
                } else if *duration >= 3600000 {
                    format!("'h', -{}", duration / 3600000)
                } else if *duration >= 60000 {
                    format!("'m', -{}", duration / 60000)
                } else {
                    format!("'s', -{}", duration / 1000)
                };
                sql.push_str(&format!(" WHERE timestamp > dateadd({}, now())", unit));
            }
        }
        
        // SAMPLE BY
        if let Some(window) = plan.windows.first() {
            if let WindowType::Interval { duration, .. } = &window.window_type {
                let dur = format_questdb_duration(*duration);
                sql.push_str(&format!(" SAMPLE BY {}", dur));
            } else if let WindowType::SampleBy { interval, .. } = &window.window_type {
                sql.push_str(&format!(" SAMPLE BY {}", interval));
            }
        }
        
        // LIMIT
        if let Some(limit) = plan.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        Ok(sql)
    }
    
    fn to_clickhouse(&self, plan: &QueryPlan) -> Result<String, TranslateError> {
        let mut sql = String::from("SELECT ");
        
        // SELECT
        let mut select_parts = Vec::new();
        
        // Time buckets
        for group in &plan.group_by {
            if let GroupByExpr::TimeBucket { interval, column } = &group.expr {
                let col = column.as_deref().unwrap_or("timestamp");
                let func = if *interval >= 86400000 {
                    "toStartOfDay"
                } else if *interval >= 3600000 {
                    "toStartOfHour"
                } else if *interval >= 60000 {
                    "toStartOfMinute"
                } else {
                    "toStartOfSecond"
                };
                select_parts.push(format!("{}({}) AS time_bucket", func, col));
            }
        }
        
        // Aggregations
        for agg in &plan.aggregations {
            let func = match &agg.function {
                AggFunction::Avg => "avg",
                AggFunction::Sum => "sum",
                AggFunction::Count => "count",
                AggFunction::Min => "min",
                AggFunction::Max => "max",
                AggFunction::CountDistinct => "uniq",
                AggFunction::Median => "median",
                _ => continue,
            };
            let col = agg.column.as_deref().unwrap_or("*");
            select_parts.push(format!("{}({})", func, col));
        }
        
        if select_parts.is_empty() {
            sql.push('*');
        } else {
            sql.push_str(&select_parts.join(", "));
        }
        
        // FROM
        if let Some(source) = plan.sources.first() {
            sql.push_str(&format!(" FROM {}", source.name));
        }
        
        // WHERE
        if let Some(ref time_range) = plan.time_range {
            if let TimeRange::Relative { duration, .. } = time_range {
                let unit = if *duration >= 86400000 {
                    format!("toIntervalDay({})", duration / 86400000)
                } else if *duration >= 3600000 {
                    format!("toIntervalHour({})", duration / 3600000)
                } else if *duration >= 60000 {
                    format!("toIntervalMinute({})", duration / 60000)
                } else {
                    format!("toIntervalSecond({})", duration / 1000)
                };
                sql.push_str(&format!(" WHERE timestamp > now() - {}", unit));
            }
        }
        
        // GROUP BY
        if plan.group_by.iter().any(|g| matches!(g.expr, GroupByExpr::TimeBucket { .. })) {
            sql.push_str(" GROUP BY time_bucket");
        }
        
        // WITH TOTALS
        if plan.hints.custom.get("with_totals").map(|v| v == "true").unwrap_or(false) {
            sql.push_str(" WITH TOTALS");
        }
        
        // ORDER BY
        if !plan.order_by.is_empty() {
            let orders: Vec<String> = plan.order_by.iter().map(|o| {
                let dir = if o.ascending { "" } else { " DESC" };
                format!("{}{}", o.column, dir)
            }).collect();
            sql.push_str(" ORDER BY ");
            sql.push_str(&orders.join(", "));
        }
        
        // LIMIT
        if let Some(limit) = plan.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        Ok(sql)
    }
}

impl Default for UniversalTranslator {
    fn default() -> Self {
        Self::new()
    }
}

/// SQL Translator for generic SQL output
pub struct SQLTranslator;

impl SQLTranslator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SQLTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectTranslator for SQLTranslator {
    fn translate(&self, plan: &QueryPlan) -> Result<String, TranslateError> {
        UniversalTranslator::new().translate(plan, Dialect::SQL)
    }
    
    fn target_dialect(&self) -> Dialect {
        Dialect::SQL
    }
}

// Helper functions

fn format_influx_duration(ms: i64) -> String {
    if ms >= 86400000 && ms % 86400000 == 0 {
        format!("{}d", ms / 86400000)
    } else if ms >= 3600000 && ms % 3600000 == 0 {
        format!("{}h", ms / 3600000)
    } else if ms >= 60000 && ms % 60000 == 0 {
        format!("{}m", ms / 60000)
    } else if ms >= 1000 {
        format!("{}s", ms / 1000)
    } else {
        format!("{}ms", ms)
    }
}

fn format_flux_duration(ms: i64) -> String {
    format_influx_duration(ms)
}

fn format_promql_duration(ms: i64) -> String {
    format_influx_duration(ms)
}

fn format_postgres_interval(ms: i64) -> String {
    if ms >= 86400000 && ms % 86400000 == 0 {
        format!("{} day", ms / 86400000)
    } else if ms >= 3600000 && ms % 3600000 == 0 {
        format!("{} hour", ms / 3600000)
    } else if ms >= 60000 && ms % 60000 == 0 {
        format!("{} minute", ms / 60000)
    } else if ms >= 1000 {
        format!("{} second", ms / 1000)
    } else {
        format!("{} millisecond", ms)
    }
}

fn format_tdengine_duration(ms: i64) -> String {
    if ms >= 86400000 && ms % 86400000 == 0 {
        format!("{}d", ms / 86400000)
    } else if ms >= 3600000 && ms % 3600000 == 0 {
        format!("{}h", ms / 3600000)
    } else if ms >= 60000 && ms % 60000 == 0 {
        format!("{}m", ms / 60000)
    } else if ms >= 1000 {
        format!("{}s", ms / 1000)
    } else {
        format!("{}a", ms) // TDengine uses 'a' for milliseconds
    }
}

fn format_questdb_duration(ms: i64) -> String {
    if ms >= 86400000 && ms % 86400000 == 0 {
        format!("{}d", ms / 86400000)
    } else if ms >= 3600000 && ms % 3600000 == 0 {
        format!("{}h", ms / 3600000)
    } else if ms >= 60000 && ms % 60000 == 0 {
        format!("{}m", ms / 60000)
    } else if ms >= 1000 {
        format!("{}s", ms / 1000)
    } else {
        format!("{}T", ms) // QuestDB uses 'T' for milliseconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_translate_to_influxql() {
        let translator = UniversalTranslator::new();
        
        let mut plan = QueryPlan::new(Dialect::PromQL);
        plan.sources.push(DataSource {
            name: "cpu".to_string(),
            database: None,
            retention_policy: None,
            alias: None,
            source_type: DataSourceType::Metric,
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
        
        let result = translator.translate(&plan, Dialect::InfluxQL);
        assert!(result.is_ok());
        
        let sql = result.unwrap();
        assert!(sql.contains("SELECT mean"));
        assert!(sql.contains("FROM \"cpu\""));
    }
    
    #[test]
    fn test_translate_to_promql() {
        let translator = UniversalTranslator::new();
        
        let mut plan = QueryPlan::new(Dialect::InfluxQL);
        plan.sources.push(DataSource {
            name: "http_requests_total".to_string(),
            database: None,
            retention_policy: None,
            alias: None,
            source_type: DataSourceType::Metric,
        });
        plan.aggregations.push(Aggregation {
            function: AggFunction::Rate,
            column: None,
            args: vec![],
            alias: None,
            distinct: false,
        });
        plan.windows.push(Window {
            window_type: WindowType::Range { duration: 300000 },
            fill: None,
        });
        
        let result = translator.translate(&plan, Dialect::PromQL);
        assert!(result.is_ok());
        
        let query = result.unwrap();
        assert!(query.contains("rate"));
        assert!(query.contains("http_requests_total"));
    }
}
