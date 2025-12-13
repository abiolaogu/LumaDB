use luma_protocol_core::Value;

// Helper to encode Value to MySQL Text Protocol string
pub fn encode_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(v) => Some((if *v { "1" } else { "0" }).to_string()),
        Value::Int8(v) => Some(v.to_string()),
        Value::Int16(v) => Some(v.to_string()),
        Value::Int32(v) => Some(v.to_string()),
        Value::Int64(v) => Some(v.to_string()),
        Value::Float32(v) => Some(v.to_string()),
        Value::Float64(v) => Some(v.to_string()),
        Value::Decimal(v) => Some(v.to_string()),
        Value::Text(v) => Some(v.clone()),
        Value::Bytes(v) => Some(String::from_utf8_lossy(v).to_string()),
        Value::Date(v) => Some(v.to_string()),
        Value::Time(v) => Some(v.to_string()),
        Value::Timestamp(v) => Some(v.to_string()),
        Value::Json(v) => Some(v.to_string()),
        _ => Some(format!("{:?}", value)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ColumnType {
    Decimal = 0x00,
    Tiny = 0x01,
    Short = 0x02,
    Long = 0x03,
    Float = 0x04,
    Double = 0x05,
    Null = 0x06,
    Timestamp = 0x07,
    LongLong = 0x08,
    Int24 = 0x09,
    Date = 0x0a,
    Time = 0x0b,
    DateTime = 0x0c,
    Year = 0x0d,
    VarChar = 0x0f,
    Bit = 0x10, 
    Json = 0xf5,
    NewDecimal = 0xf6,
    Enum = 0xf7,
    Set = 0xf8,
    TinyBlob = 0xf9,
    MediumBlob = 0xfa,
    LongBlob = 0xfb,
    Blob = 0xfc,
    VarString = 0xfd,
    String = 0xfe,
    Geometry = 0xff,
}

impl ColumnType {
    pub fn infer_from_value(v: &Value) -> Self {
        match v {
            Value::Null => ColumnType::Null,
            Value::Bool(_) => ColumnType::Tiny,
            Value::Int8(_) => ColumnType::Tiny,
            Value::Int16(_) => ColumnType::Short,
            Value::Int32(_) => ColumnType::Long,
            Value::Int64(_) => ColumnType::LongLong,
            Value::Float32(_) => ColumnType::Float,
            Value::Float64(_) => ColumnType::Double,
            Value::Text(_) => ColumnType::VarString,
            Value::Bytes(_) => ColumnType::Blob,
            Value::Date(_) => ColumnType::Date,
            Value::Time(_) => ColumnType::Time,
            Value::Timestamp(_) => ColumnType::Timestamp,
            Value::Decimal(_) => ColumnType::NewDecimal,
            Value::Json(_) => ColumnType::Json,
            _ => ColumnType::VarString,
        }
    }
}
