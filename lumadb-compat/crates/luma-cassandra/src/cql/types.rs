use bytes::{Buf, BufMut, BytesMut};
use luma_protocol_core::{ProtocolError, Result, Value};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum CQLType {
    Custom(String),
    Ascii,
    BigInt,
    Blob,
    Boolean,
    Counter,
    Decimal,
    Double,
    Float,
    Int,
    Timestamp,
    Uuid,
    Varchar,
    Varint,
    Timeuuid,
    Inet,
    List(Box<CQLType>),
    Map(Box<CQLType>, Box<CQLType>),
    Set(Box<CQLType>),
    Udt(String, Vec<(String, CQLType)>), // Name, Fields
    Tuple(Vec<CQLType>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CQLValue {
    Native(Value),
    List(Vec<CQLValue>),
    Set(Vec<CQLValue>),
    Map(Vec<(CQLValue, CQLValue)>),
    Tuple(Vec<Option<CQLValue>>),
    Udt(HashMap<String, Option<CQLValue>>),
}

pub fn decode_value(type_: &CQLType, bytes: &[u8]) -> Result<CQLValue> {
    if bytes.is_empty() { return Ok(CQLValue::Native(Value::Null)); }
    let mut buf = bytes;
    match type_ {
         CQLType::List(elem_type) | CQLType::Set(elem_type) => {
             if buf.len() < 4 { return Err(ProtocolError::Protocol("Invalid List/Set len".into())); }
             let count = buf.get_i32();
             if count < 0 { return Ok(CQLValue::List(vec![])); }
             let mut list = Vec::with_capacity(count as usize);
             for _ in 0..count {
                 if buf.len() < 4 { return Err(ProtocolError::Protocol("Invalid element len".into())); }
                 let len = buf.get_i32();
                 if len < 0 {
                      // Nulls in lists are generally not allowed but we handle gracefully
                 } else {
                      let ulen = len as usize;
                      if buf.len() < ulen { return Err(ProtocolError::Protocol("Invalid List element body".into())); }
                      let val_bytes = &buf[..ulen];
                      buf.advance(ulen);
                      list.push(decode_value(elem_type, val_bytes)?);
                 }
             }
             if matches!(type_, CQLType::Set(_)) {
                 Ok(CQLValue::Set(list))
             } else {
                 Ok(CQLValue::List(list))
             }
         },
         CQLType::Map(k_type, v_type) => {
             if buf.len() < 4 { return Err(ProtocolError::Protocol("Invalid Map len".into())); }
             let count = buf.get_i32();
             if count < 0 { return Ok(CQLValue::Map(vec![])); }
             let mut map = Vec::with_capacity(count as usize);
             for _ in 0..count {
                 // Key
                 if buf.len() < 4 { return Err(ProtocolError::Protocol("Invalid Map key len".into())); }
                 let k_len = buf.get_i32();
                 if k_len < 0 { return Err(ProtocolError::Protocol("Null Map key not allowed".into())); }
                 let uk_len = k_len as usize;
                 if buf.len() < uk_len { return Err(ProtocolError::Protocol("Invalid Map key body".into())); }
                 let k_bytes = &buf[..uk_len];
                 buf.advance(uk_len);
                 let key = decode_value(k_type, k_bytes)?;
                 
                 // Value
                 if buf.len() < 4 { return Err(ProtocolError::Protocol("Invalid Map value len".into())); }
                 let v_len = buf.get_i32();
                 if v_len < 0 {
                      // Null value?
                 } else {
                      let uv_len = v_len as usize;
                      if buf.len() < uv_len { return Err(ProtocolError::Protocol("Invalid Map value body".into())); }
                      let v_bytes = &buf[..uv_len];
                      buf.advance(uv_len);
                      let val = decode_value(v_type, v_bytes)?;
                      map.push((key, val));
                 }
             }
             Ok(CQLValue::Map(map))
         },
         _ => {
             let val = decode_scalar(type_, bytes)?;
             Ok(CQLValue::Native(val))
         }
    }
}

pub fn decode_scalar(type_: &CQLType, bytes: &[u8]) -> Result<Value> {
    if bytes.is_empty() { return Ok(Value::Null); }
    let mut buf = bytes;
    match type_ {
        CQLType::Int => {
            if buf.len() < 4 { return Err(ProtocolError::Protocol("Invalid Int".into())); }
            Ok(Value::Int32(buf.get_i32()))
        },
        CQLType::BigInt | CQLType::Counter | CQLType::Timestamp => {
             if buf.len() < 8 { return Err(ProtocolError::Protocol("Invalid BigInt".into())); }
             Ok(Value::Int64(buf.get_i64()))
        },
        CQLType::Varchar | CQLType::Ascii => {
            let s = String::from_utf8_lossy(bytes).to_string();
            Ok(Value::Text(s))
        },
        CQLType::Boolean => {
             if buf.len() < 1 { return Err(ProtocolError::Protocol("Invalid Boolean".into())); }
             Ok(Value::Bool(buf.get_u8() != 0))
        },
        CQLType::Float => {
            if buf.len() < 4 { return Err(ProtocolError::Protocol("Invalid Float".into())); }
            Ok(Value::Float32(buf.get_f32()))
        },
        CQLType::Double => {
            if buf.len() < 8 { return Err(ProtocolError::Protocol("Invalid Double".into())); }
            Ok(Value::Float64(buf.get_f64()))
        },
        CQLType::Uuid | CQLType::Timeuuid => {
             if buf.len() < 16 { return Err(ProtocolError::Protocol("Invalid UUID".into())); }
             let uuid_bytes: [u8; 16] = bytes[0..16].try_into().unwrap();
             Ok(Value::Uuid(Uuid::from_bytes(uuid_bytes)))
        },
        _ => Err(ProtocolError::Protocol(format!("Unsupported type decode: {:?}", type_))),
    }
}

pub fn encode_value(value: &Value, dst: &mut BytesMut) {
     // Basic encoding for native values
    match value {
        Value::Null => {
            dst.put_i32(-1);
        },
        Value::Int32(v) => {
            dst.put_i32(4);
            dst.put_i32(*v);
        },
        Value::Int64(v) => {
            dst.put_i32(8);
            dst.put_i64(*v);
        },
        Value::Text(v) => {
            dst.put_i32(v.len() as i32);
            dst.put_slice(v.as_bytes());
        },
        Value::Bool(v) => {
             dst.put_i32(1);
             dst.put_u8(if *v { 1 } else { 0 });
        },
        Value::Float32(v) => {
            dst.put_i32(4);
            dst.put_f32(*v);
        },
        Value::Float64(v) => {
            dst.put_i32(8);
            dst.put_f64(*v);
        },
        Value::Uuid(v) => {
             dst.put_i32(16);
             dst.put_slice(v.as_bytes());
        },
        _ => {
            let s = format!("{:?}", value);
            dst.put_i32(s.len() as i32);
            dst.put_slice(s.as_bytes());
        }
    }
}
