//! TDengine Compatibility Module
//!
//! Provides TDengine-compatible features for LumaDB:
//! - Window functions (INTERVAL, SESSION, STATE_WINDOW, EVENT_WINDOW, COUNT_WINDOW)
//! - SQL parser structures
//! - Aggregation functions

pub mod window;
pub mod parser;
pub mod aggregation;

pub use window::{
    WindowProcessor, WindowResult, TimeSeriesRow, Value, AggExpr, AggFunction,
    IntervalWindow, SessionWindow, StateWindow, CountWindow,
};
pub use parser::{WindowClause, FillClause, Expr};
pub use aggregation::compute_aggregation;
