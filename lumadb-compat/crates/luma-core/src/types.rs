use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::borrow::Cow;

/// Unified Value Enum supporting Postgres, MySQL, Cassandra, and MongoDB types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    Null,
    // Numeric
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Decimal(String), // Precision preserving string representation

    // String/Binary
    Text(String),
    Bytes(bytes::Bytes),

    // Date/Time
    Timestamp(DateTime<Utc>),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
    
    // Identifiers
    Uuid(Uuid),
    ObjectId(bson::oid::ObjectId),
    
    // Collections
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    Set(Vec<Value>),

    // JSON/BSON
    Json(serde_json::Value),
    Bson(bson::Document),

    // Custom
    Vector(Vec<f32>), // For AI/Embdedding support
}

impl Value {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }
    
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int64(v) => Some(*v),
            Value::Int32(v) => Some(*v as i64),
            Value::Int16(v) => Some(*v as i64),
            Value::Int8(v) => Some(*v as i64),
            _ => None,
        }
    }
}

/// Helper trait for converting protocol specific types to Unified Value
pub trait TryIntoValue {
    fn try_into_value(self) -> Result<Value, crate::ProtocolError>;
}
