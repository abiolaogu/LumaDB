//! TSDB Module
//!
//! Unified time-series database engine for Prometheus, InfluxDB, and Druid compatibility.

pub mod core;
pub mod gorilla;

pub use core::{TsdbEngine, TsdbConfig, Sample, Series, LabelMatcher, MatchType};
pub use gorilla::{compress_samples, decompress_samples};
