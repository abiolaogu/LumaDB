//! Multi-Dialect Query Language Support
//!
//! This module provides native query language parsers for all major
//! time-series and analytics databases, enabling users to query LumaDB
//! using their familiar syntax.
//!
//! Supported dialects:
//! - InfluxQL (InfluxDB 1.x)
//! - Flux (InfluxDB 2.x/3.x)
//! - PromQL (Prometheus)
//! - MetricsQL (VictoriaMetrics)
//! - TDengine SQL
//! - TimescaleDB SQL
//! - QuestDB SQL
//! - ClickHouse SQL
//! - Druid SQL/Native
//! - OpenTSDB Query
//! - Graphite Functions

pub mod ir;
pub mod detector;
pub mod translator;

pub mod influxql;
pub mod flux;
pub mod promql;
pub mod timescale;
pub mod questdb;
pub mod clickhouse;
pub mod druid;
pub mod opentsdb;
pub mod graphite;
pub mod metricsql;

// Re-exports
pub use ir::{
    Dialect, QueryPlan, DataSource, DataSourceType,
    TimeRange, Filter, FilterCondition, ComparisonOp, Value,
    Aggregation, AggFunction, Window, WindowType, FillStrategy,
    GroupBy, GroupByExpr, Transformation, TransformType,
    OrderBy, OutputFormat, QueryHints, QueryResult,
    DialectParser, DialectTranslator, ParseError, TranslateError,
};
pub use detector::DialectDetector;
pub use translator::UniversalTranslator;

use std::collections::HashMap;
use std::sync::Arc;

/// Registry of all dialect parsers
pub struct DialectRegistry {
    parsers: HashMap<Dialect, Arc<dyn DialectParser>>,
    translators: HashMap<Dialect, Arc<dyn DialectTranslator>>,
    detector: DialectDetector,
}

impl Default for DialectRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DialectRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            parsers: HashMap::new(),
            translators: HashMap::new(),
            detector: DialectDetector::new(),
        };
        
        // Register all parsers
        registry.register_parser(Dialect::InfluxQL, Arc::new(influxql::InfluxQLParser::new()));
        registry.register_parser(Dialect::Flux, Arc::new(flux::FluxParser::new()));
        registry.register_parser(Dialect::PromQL, Arc::new(promql::PromQLParser::new()));
        registry.register_parser(Dialect::MetricsQL, Arc::new(metricsql::MetricsQLParser::new()));
        registry.register_parser(Dialect::TimescaleDB, Arc::new(timescale::TimescaleParser::new()));
        registry.register_parser(Dialect::QuestDB, Arc::new(questdb::QuestDBParser::new()));
        registry.register_parser(Dialect::ClickHouse, Arc::new(clickhouse::ClickHouseParser::new()));
        registry.register_parser(Dialect::DruidSQL, Arc::new(druid::DruidSQLParser::new()));
        registry.register_parser(Dialect::DruidNative, Arc::new(druid::DruidNativeParser::new()));
        registry.register_parser(Dialect::OpenTSDB, Arc::new(opentsdb::OpenTSDBParser::new()));
        registry.register_parser(Dialect::Graphite, Arc::new(graphite::GraphiteParser::new()));
        
        // Register translators
        registry.register_translator(Dialect::InfluxQL, Arc::new(influxql::InfluxQLTranslator::new()));
        registry.register_translator(Dialect::PromQL, Arc::new(promql::PromQLTranslator::new()));
        registry.register_translator(Dialect::SQL, Arc::new(translator::SQLTranslator::new()));
        
        registry
    }
    
    pub fn register_parser(&mut self, dialect: Dialect, parser: Arc<dyn DialectParser>) {
        self.parsers.insert(dialect, parser);
    }
    
    pub fn register_translator(&mut self, dialect: Dialect, translator: Arc<dyn DialectTranslator>) {
        self.translators.insert(dialect, translator);
    }
    
    /// Parse a query with explicit dialect
    pub fn parse(&self, dialect: Dialect, query: &str) -> Result<QueryPlan, ParseError> {
        let parser = self.parsers.get(&dialect)
            .ok_or_else(|| ParseError {
                message: format!("Unsupported dialect: {:?}", dialect),
                position: None,
                line: None,
                column: None,
            })?;
        
        parser.parse(query)
    }
    
    /// Parse a query with auto-detected dialect
    pub fn parse_auto(&self, query: &str) -> Result<QueryPlan, ParseError> {
        let dialect = self.detector.detect(query);
        self.parse(dialect, query)
    }
    
    /// Parse with confidence score
    pub fn parse_auto_with_confidence(&self, query: &str) -> Result<(QueryPlan, Dialect, f64), ParseError> {
        let (dialect, confidence) = self.detector.detect_with_confidence(query);
        let plan = self.parse(dialect, query)?;
        Ok((plan, dialect, confidence))
    }
    
    /// Translate a query plan to target dialect
    pub fn translate(&self, plan: &QueryPlan, target: Dialect) -> Result<String, TranslateError> {
        let translator = self.translators.get(&target)
            .ok_or_else(|| TranslateError {
                message: format!("No translator for dialect: {:?}", target),
                unsupported_feature: None,
            })?;
        
        translator.translate(plan)
    }
    
    /// Translate between dialects
    pub fn translate_query(
        &self, 
        query: &str, 
        source: Option<Dialect>, 
        target: Dialect
    ) -> Result<String, TranslateError> {
        // Parse source
        let plan = if let Some(src_dialect) = source {
            self.parse(src_dialect, query)
        } else {
            self.parse_auto(query)
        }.map_err(|e| TranslateError {
            message: format!("Parse error: {}", e),
            unsupported_feature: None,
        })?;
        
        // Translate to target
        self.translate(&plan, target)
    }
    
    /// Get the detector for external use
    pub fn detector(&self) -> &DialectDetector {
        &self.detector
    }
    
    /// List all supported dialects
    pub fn supported_dialects(&self) -> Vec<Dialect> {
        self.parsers.keys().copied().collect()
    }
}

/// Global registry instance
static REGISTRY: std::sync::OnceLock<DialectRegistry> = std::sync::OnceLock::new();

pub fn registry() -> &'static DialectRegistry {
    REGISTRY.get_or_init(DialectRegistry::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_registry_creation() {
        let registry = DialectRegistry::new();
        assert!(!registry.supported_dialects().is_empty());
    }
    
    #[test]
    fn test_auto_detection_and_parse() {
        let registry = DialectRegistry::new();
        
        // PromQL query
        let result = registry.parse_auto("rate(http_requests_total[5m])");
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.source_dialect, Dialect::PromQL);
    }
}
