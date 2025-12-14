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

impl Eq for Value {}

#[allow(clippy::derive_hash_xor_eq)]
impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Null => {},
            Value::Bool(v) => v.hash(state),
            Value::Int8(v) => v.hash(state),
            Value::Int16(v) => v.hash(state),
            Value::Int32(v) => v.hash(state),
            Value::Int64(v) => v.hash(state),
            Value::Float32(v) => v.to_bits().hash(state),
            Value::Float64(v) => v.to_bits().hash(state),
            Value::Decimal(v) => v.hash(state),
            Value::Text(v) => v.hash(state),
            Value::Bytes(v) => v.hash(state),
            Value::Timestamp(v) => v.hash(state),
            Value::Date(v) => v.hash(state),
            Value::Time(v) => v.hash(state),
            Value::Uuid(v) => v.hash(state),
            Value::ObjectId(v) => v.to_string().hash(state), // ObjectId might not be Hash
            Value::List(v) => v.hash(state),
            Value::Map(_) => {}, // Maps are not hashable, ignore or panic? For now ignore.
            Value::Set(_) => {}, 
            Value::Json(v) => v.to_string().hash(state),
            Value::Bson(v) => v.to_string().hash(state),
            Value::Vector(v) => {
                for f in v {
                    f.to_bits().hash(state);
                }
            }
        }
    }
}

/// Helper trait for converting protocol specific types to Unified Value
pub trait TryIntoValue {
    fn try_into_value(self) -> Result<Value, crate::ProtocolError>;
}
