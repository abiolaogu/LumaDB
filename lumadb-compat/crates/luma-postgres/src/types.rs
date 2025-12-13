use bytes::{BufMut, Bytes, BytesMut};
use luma_protocol_core::{ProtocolError, Result, Value};
use byteorder::{BigEndian, ByteOrder};

// Basic Postgres OIDs
pub const INT8OID: i32 = 20;
pub const INT2OID: i32 = 21;
pub const INT4OID: i32 = 23;
pub const TEXTOID: i32 = 25;
pub const BOOLOID: i32 = 16;
pub const FLOAT4OID: i32 = 700;
pub const FLOAT8OID: i32 = 701;
pub const VARCHAROID: i32 = 1043;
pub const DATEOID: i32 = 1082;
pub const TIMESTAMPOID: i32 = 1114;
pub const TIMESTAMPTZOID: i32 = 1184;
pub const NUMERICOID: i32 = 1700;
pub const BYTEAOID: i32 = 17;
pub const JSONOID: i32 = 114;
pub const JSONBOID: i32 = 3802;
pub const UUIDOID: i32 = 2950;
pub const INT4ARRAYOID: i32 = 1007; // _int4
pub const TEXTARRAYOID: i32 = 1009; // _text
pub const POINT: i32 = 600;
pub const BOX: i32 = 603;
pub const VECTOROID: i32 = 3000; // pgvector standard

// Format codes
pub const FORMAT_TEXT: i16 = 0;
pub const FORMAT_BINARY: i16 = 1;

pub struct Oid(pub i32);

pub fn infer_oid(value: &Value) -> i32 {
    match value {
        Value::Null => INT4OID, // Default
        Value::Bool(_) => BOOLOID,
        Value::Int8(_) => INT2OID,
        Value::Int16(_) => INT2OID,
        Value::Int32(_) => INT4OID,
        Value::Int64(_) => INT8OID,
        Value::Float32(_) => FLOAT4OID,
        Value::Float64(_) => FLOAT8OID,
        Value::Decimal(_) => NUMERICOID,
        Value::Text(_) => TEXTOID,
        Value::Bytes(_) => BYTEAOID,
        Value::Timestamp(_) => TIMESTAMPTZOID,
        Value::Date(_) => DATEOID,
        Value::Time(_) => 1083, // TIME
        Value::Uuid(_) => UUIDOID,
        Value::Json(_) => JSONBOID,
        Value::Bson(_) => JSONBOID, // Map BSON to JSONB
        Value::Vector(_) => VECTOROID,
        Value::List(vals) => {
            // Simple inference for arrays: check first element
            if let Some(first) = vals.first() {
                match first {
                    Value::Int32(_) => INT4ARRAYOID,
                    _ => TEXTARRAYOID, // Fallback
                }
            } else {
                TEXTARRAYOID // Empty array
            }
        },
        _ => TEXTOID,
    }
}

pub fn encode_value(value: &Value, format: i16) -> Result<Option<Bytes>> {
    if let Value::Null = value {
        return Ok(None);
    }

    if format == FORMAT_TEXT {
        let s = match value {
            Value::Bool(v) => if *v { "t" } else { "f" }.to_string(),
            Value::Int8(v) => v.to_string(),
            Value::Int16(v) => v.to_string(),
            Value::Int32(v) => v.to_string(),
            Value::Int64(v) => v.to_string(),
            Value::Float32(v) => v.to_string(),
            Value::Float64(v) => v.to_string(),
            Value::Text(v) => v.clone(),
            Value::Uuid(v) => v.to_string(),
            Value::Json(v) => v.to_string(),
            Value::Vector(v) => format!("{:?}", v), // [1.0, 2.0] style
            Value::List(v) => {
                 // Array serialization: {1,2,3}
                 // Naive implementation
                 let parts: Vec<String> = v.iter().map(|item| {
                     // TODO: Recursively encode?
                     match item {
                         Value::Int32(i) => i.to_string(),
                         Value::Text(t) => format!("\"{}\"", t.replace("\"", "\\\"")), // Quote and escape
                         _ => "?".to_string(),
                     }
                 }).collect();
                 format!("{{{}}}", parts.join(","))
            },
            _ => format!("{:?}", value),
        };
        return Ok(Some(Bytes::from(s)));
    }

    // Binary Encoding
    let mut buf = BytesMut::new();
    match value {
        Value::Bool(v) => buf.put_u8(if *v { 1 } else { 0 }),
        Value::Int8(v) => buf.put_i16(*v as i16), // usually int2
        Value::Int16(v) => buf.put_i16(*v),
        Value::Int32(v) => buf.put_i32(*v),
        Value::Int64(v) => buf.put_i64(*v),
        Value::Float32(v) => buf.put_f32(*v),
        Value::Float64(v) => buf.put_f64(*v),
        Value::Text(v) => buf.put(v.as_bytes()),
        Value::Bytes(v) => buf.put(v.clone()),
        _ => return Err(ProtocolError::TypeConversion("Unsupported binary type".into())),
    }

    Ok(Some(buf.freeze()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_protocol_core::Value;

    #[test]
    fn test_infer_oid() {
        assert_eq!(infer_oid(&Value::Int32(10)), INT4OID);
        assert_eq!(infer_oid(&Value::Text("hello".to_string())), TEXTOID);
        assert_eq!(infer_oid(&Value::Null), INT4OID);
    }

    #[test]
    fn test_encode_value_text() {
        let val = Value::Int32(123);
        let encoded = encode_value(&val, FORMAT_TEXT).unwrap().unwrap();
        assert_eq!(encoded, Bytes::from("123"));

        let val = Value::Bool(true);
        let encoded = encode_value(&val, FORMAT_TEXT).unwrap().unwrap();
        assert_eq!(encoded, Bytes::from("t"));
    }

    #[test]
    fn test_encode_value_binary() {
         // Not strictly implemented for all types yet, but let's test what we have
         let val = Value::Int32(123);
         let encoded = encode_value(&val, FORMAT_BINARY).unwrap().unwrap();
         // 123 in BigEndian 4 bytes = 0000007B
         assert_eq!(encoded, Bytes::from_static(&[0, 0, 0, 0x7B]));
    }
}
