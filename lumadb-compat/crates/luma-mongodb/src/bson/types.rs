use bson::{Bson, Binary, Regex, Timestamp, DateTime, oid::ObjectId, Decimal128};
use luma_protocol_core::{ProtocolError, Result};

// Re-export specific BSON types or wrappers if needed for strict wire control.
// For now, we rely on the `bson` crate but provide helpers to validate types against wire specs.

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum BsonType {
    Double = 0x01,
    String = 0x02,
    Document = 0x03,
    Array = 0x04,
    Binary = 0x05,
    Undefined = 0x06,
    ObjectId = 0x07,
    Boolean = 0x08,
    DateTime = 0x09,
    Null = 0x0A,
    Regex = 0x0B,
    DbPointer = 0x0C,
    JavaScript = 0x0D,
    Symbol = 0x0E,
    JavaScriptWithScope = 0x0F,
    Int32 = 0x10,
    Timestamp = 0x11,
    Int64 = 0x12,
    Decimal128 = 0x13,
    MinKey = 0xFF,
    MaxKey = 0x7F,
}

pub fn get_bson_type(v: &Bson) -> BsonType {
    match v {
        Bson::Double(_) => BsonType::Double,
        Bson::String(_) => BsonType::String,
        Bson::Document(_) => BsonType::Document,
        Bson::Array(_) => BsonType::Array,
        Bson::Binary(_) => BsonType::Binary,
        Bson::ObjectId(_) => BsonType::ObjectId,
        Bson::Boolean(_) => BsonType::Boolean,
        Bson::DateTime(_) => BsonType::DateTime,
        Bson::Null => BsonType::Null,
        Bson::RegularExpression(_) => BsonType::Regex,
        Bson::JavaScriptCode(_) => BsonType::JavaScript,
        Bson::JavaScriptCodeWithScope(_) => BsonType::JavaScriptWithScope,
        Bson::Int32(_) => BsonType::Int32,
        Bson::Timestamp(_) => BsonType::Timestamp,
        Bson::Int64(_) => BsonType::Int64,
        Bson::Decimal128(_) => BsonType::Decimal128,
        _ => BsonType::Null // Fallback or handle Min/Max key if supported by crate
    }
}
