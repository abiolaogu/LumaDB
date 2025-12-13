use bytes::{Buf, BufMut, BytesMut};
use super::frame::{read_long_string, read_string, write_string};
use crate::cql::types::CQLType;
use luma_protocol_core::{ProtocolError, Result};
use std::collections::HashMap;

pub struct ReadyBody;

impl ReadyBody {
    pub fn write(dst: &mut BytesMut) {
        // Empty body
    }
}

// Error Codes
pub const ERROR_SERVER: i32 = 0x0000;
pub const ERROR_PROTOCOL: i32 = 0x000A;
pub const ERROR_BAD_CREDENTIALS: i32 = 0x0100;
pub const ERROR_UNAVAILABLE: i32 = 0x1000;
pub const ERROR_OVERLOADED: i32 = 0x1001;
pub const ERROR_IS_BOOTSTRAPPING: i32 = 0x1002;
pub const ERROR_TRUNCATE: i32 = 0x1003;
pub const ERROR_WRITE_TIMEOUT: i32 = 0x1100;
pub const ERROR_READ_TIMEOUT: i32 = 0x1200;
pub const ERROR_SYNTAX: i32 = 0x2000;
pub const ERROR_UNAUTHORIZED: i32 = 0x2100;
pub const ERROR_INVALID: i32 = 0x2200;
pub const ERROR_CONFIG: i32 = 0x2300;
pub const ERROR_ALREADY_EXISTS: i32 = 0x2400;
pub const ERROR_UNPREPARED: i32 = 0x2500;

pub struct ErrorBody {
    pub code: i32,
    pub message: String,
    pub additional: Option<Vec<u8>>, // For specific errors like Unavailable
}

impl ErrorBody {
    pub fn write(&self, dst: &mut BytesMut) {
        dst.put_i32(self.code);
        write_string(dst, &self.message);
        // Specialized writing for errors not fully implemented yet, just generic
        if let Some(add) = &self.additional {
            dst.put_slice(add);
        }
    }
}

pub struct SupportedBody {
    pub options: HashMap<String, Vec<String>>,
}

impl SupportedBody {
    pub fn write(dst: &mut BytesMut, options: &HashMap<String, Vec<String>>) {
        dst.put_u16(options.len() as u16);
        for (k, v) in options {
            write_string(dst, k);
            dst.put_u16(v.len() as u16);
            for val in v {
                write_string(dst, val);
            }
        }
    }
}

pub struct AuthenticateBody {
    pub authenticator: String,
}

impl AuthenticateBody {
    pub fn write(&self, dst: &mut BytesMut) {
        write_string(dst, &self.authenticator);
    }
}

// Query Flags
pub const QUERY_FLAG_VALUES: u8 = 0x01;
pub const QUERY_FLAG_SKIP_METADATA: u8 = 0x02;
pub const QUERY_FLAG_PAGE_SIZE: u8 = 0x04;
pub const QUERY_FLAG_PAGING_STATE: u8 = 0x08;
pub const QUERY_FLAG_SERIAL_CONSISTENCY: u8 = 0x10;
pub const QUERY_FLAG_DEFAULT_TIMESTAMP: u8 = 0x20;
pub const QUERY_FLAG_VALUE_NAMES: u8 = 0x40; // v5+

#[derive(Debug)]
pub struct QueryOptions {
    pub consistency: u16,
    pub flags: u8,
    pub values: Option<Vec<Result<Option<Vec<u8>>>>>, // Bound values
    pub page_size: Option<i32>,
    pub paging_state: Option<Vec<u8>>,
    pub serial_consistency: Option<u16>,
    pub timestamp: Option<i64>,
}

impl QueryOptions {
    pub fn read(src: &mut BytesMut) -> Result<Self> {
        if src.len() < 2 { return Err(ProtocolError::Protocol("Incomplete consistency".into())); }
        let consistency = src.get_u16();
        
        if !src.has_remaining() { return Err(ProtocolError::Protocol("Incomplete flags".into())); }
        let flags = src.get_u8();
        
        let mut values = None;
        if (flags & QUERY_FLAG_VALUES) != 0 {
            // Read values count
            if src.len() < 2 { return Err(ProtocolError::Protocol("Incomplete values count".into())); }
            let count = src.get_u16() as usize;
            let mut vals = Vec::with_capacity(count);
            for _ in 0..count {
                 // Each value is [int length] + [bytes]
                 // Length can be -1 (null), -2 (not set) for protocols v5? v4 uses -1 for null.
                 if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete value len".into())); }
                 let len = src.get_i32();
                 if len < 0 {
                     vals.push(Ok(None));
                 } else {
                     let ulen = len as usize;
                     if src.len() < ulen { return Err(ProtocolError::Protocol("Incomplete value body".into())); }
                     let data = src.split_to(ulen);
                     vals.push(Ok(Some(data.to_vec())));
                 }
            }
            values = Some(vals);
        }
        
        let mut page_size = None;
        if (flags & QUERY_FLAG_PAGE_SIZE) != 0 {
            if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete page size".into())); }
            page_size = Some(src.get_i32());
        }
        
        let mut paging_state = None;
        if (flags & QUERY_FLAG_PAGING_STATE) != 0 {
             if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete paging state len".into())); }
             let len = src.get_i32();
             if len < 0 {
                 paging_state = None; // Should not happen for state
             } else {
                 let ulen = len as usize;
                 if src.len() < ulen { return Err(ProtocolError::Protocol("Incomplete paging state".into())); }
                 let data = src.split_to(ulen);
                 paging_state = Some(data.to_vec());
             }
        }
        
        let mut serial_consistency = None;
        if (flags & QUERY_FLAG_SERIAL_CONSISTENCY) != 0 {
             if src.len() < 2 { return Err(ProtocolError::Protocol("Incomplete serial consistency".into())); }
             serial_consistency = Some(src.get_u16());
        }
        
        let mut timestamp = None;
        if (flags & QUERY_FLAG_DEFAULT_TIMESTAMP) != 0 {
             if src.len() < 8 { return Err(ProtocolError::Protocol("Incomplete timestamp".into())); }
             timestamp = Some(src.get_i64());
        }

        Ok(Self {
            consistency,
            flags,
            values,
            page_size,
            paging_state,
            serial_consistency,
            timestamp,
        })
    }
}

pub struct QueryBody {
    pub query: String,
    pub options: QueryOptions,
}

impl QueryBody {
    pub fn read(src: &mut BytesMut) -> Result<Self> {
        let query = read_long_string(src)?;
        let options = QueryOptions::read(src)?;
        Ok(Self { query, options })
    }
}

pub struct PrepareBody {
    pub query: String,
}

impl PrepareBody {
    pub fn read(src: &mut BytesMut) -> Result<Self> {
        let query = read_long_string(src)?;
        Ok(Self { query })
    }
}

pub struct ExecuteBody {
    pub id: Vec<u8>,
    pub options: QueryOptions,
}

pub struct BatchBody {
    pub type_: u8,
    pub queries: Vec<BatchQuery>,
    pub consistency: u16,
    pub serial_consistency: Option<u16>,
    pub timestamp: Option<i64>,
}

#[derive(Debug)]
pub struct BatchQuery {
    pub kind: u8, // 0 = string, 1 = id
    pub query_or_id: StringOrId,
    pub values: Option<Vec<Result<Option<Vec<u8>>>>>,
}

#[derive(Debug)]
pub enum StringOrId {
    String(String),
    Id(Vec<u8>),
}

impl BatchBody {
    pub fn read(src: &mut BytesMut) -> Result<Self> {
        if src.len() < 1 { return Err(ProtocolError::Protocol("Incomplete batch header".into())); }
        let type_ = src.get_u8();
        if src.len() < 2 { return Err(ProtocolError::Protocol("Incomplete batch query count".into())); }
        let count = src.get_u16() as usize;
        let mut queries = Vec::with_capacity(count);
        
        for _ in 0..count {
             if src.len() < 1 { return Err(ProtocolError::Protocol("Incomplete batch query kind".into())); }
             let kind = src.get_u8();
             let query_or_id = if kind == 0 {
                 StringOrId::String(read_long_string(src)?)
             } else {
                 if src.len() < 2 { return Err(ProtocolError::Protocol("Incomplete ID len".into())); }
                 let len = src.get_u16() as usize;
                 if src.len() < len { return Err(ProtocolError::Protocol("Incomplete ID".into())); }
                 StringOrId::Id(src.split_to(len).to_vec())
             };
             
             // In BATCH, values are for each query, separate from consistency flags
             // Values: [short count] + [value]...
             if src.len() < 2 { return Err(ProtocolError::Protocol("Incomplete values count".into())); }
             let v_count = src.get_u16() as usize;
             let mut vals = Vec::with_capacity(v_count);
             for _ in 0..v_count {
                  if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete value len".into())); }
                  let len = src.get_i32();
                  if len < 0 {
                      vals.push(Ok(None));
                  } else {
                      let ulen = len as usize;
                      if src.len() < ulen { return Err(ProtocolError::Protocol("Incomplete value body".into())); }
                      let data = src.split_to(ulen);
                      vals.push(Ok(Some(data.to_vec())));
                  }
             }
             
             queries.push(BatchQuery {
                 kind,
                 query_or_id,
                 values: Some(vals),
             });
        }
        
        if src.len() < 2 { return Err(ProtocolError::Protocol("Incomplete consistency".into())); }
        let consistency = src.get_u16();
        
        let mut serial_consistency = None;
        let mut timestamp = None;
        
        if src.has_remaining() {
            let flags = src.get_u8();
             if (flags & 0x10) != 0 { // SERIAL_CONSISTENCY
                 if src.len() < 2 { return Err(ProtocolError::Protocol("Incomplete serial consistency".into())); }
                 serial_consistency = Some(src.get_u16());
            }
            if (flags & 0x20) != 0 { // DEFAULT_TIMESTAMP
                 if src.len() < 8 { return Err(ProtocolError::Protocol("Incomplete timestamp".into())); }
                 timestamp = Some(src.get_i64());
            }
        }
        
        Ok(Self { type_, queries, consistency, serial_consistency, timestamp })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResultKind {
    Void = 0x0001,
    Rows = 0x0002,
    SetKeyspace = 0x0003,
    Prepared = 0x0004,
    SchemaChange = 0x0005,
}

pub struct ResultBody {
    pub kind: ResultKind,
    pub rows: Option<RowsBody>, // Only if kind == Rows
}

pub struct RowsBody {
    pub metadata: RowsMetadata,
    pub rows_count: i32,
    pub rows_content: Vec<Vec<Option<Vec<u8>>>>, // List of rows, each row is list of col values
}

pub struct RowsMetadata {
    pub flags: i32,
    pub columns_count: i32,
    pub paging_state: Option<Vec<u8>>,
    pub col_specs: Vec<ColSpec>,
}

pub struct ColSpec {
    pub ks_name: String,
    pub table_name: String,
    pub name: String,
    pub type_: crate::cql::types::CQLType,
}

impl ResultBody {
    pub fn write(&self, dst: &mut BytesMut) {
        dst.put_i32(self.kind as i32);
        if let ResultKind::Rows = self.kind {
            if let Some(ref rows) = self.rows {
                // Write Metadata
                let flags = rows.metadata.flags;
                dst.put_i32(flags);
                dst.put_i32(rows.metadata.columns_count);
                // .. write global table spec if needed ..
                // For now assuming No Global Tables, so we write spec for each col
                for col in &rows.metadata.col_specs {
                     // TODO: Check if Global_Tables_Spec flag is set
                     // If NOT set, we write ks_name, table_name for each col
                     write_string(dst, &col.ks_name);
                    write_string(dst, &col.table_name);
                    write_string(dst, &col.name);
                    write_type(dst, &col.type_);
                }
                
                // Write Rows Count
                dst.put_i32(rows.rows_count);
                
                // Write Content
                for row in &rows.rows_content {
                    for val in row {
                        match val {
                            Some(v) => {
                                dst.put_i32(v.len() as i32);
                                dst.put_slice(v);
                            }
                            None => {
                                dst.put_i32(-1); // Null
                            }
                        }
                    }
                }
            }
        }
    }
}
