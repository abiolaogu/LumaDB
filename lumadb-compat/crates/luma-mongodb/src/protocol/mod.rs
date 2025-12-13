use bytes::{Buf, BufMut, BytesMut};
use luma_protocol_core::{ProtocolError, Result};
use std::convert::TryFrom;

pub mod op_msg;
pub mod op_query;
pub mod op_reply;
pub mod legacy;
pub mod wire;

pub use op_msg::OpMsg;
pub use op_query::OpQuery;
pub use op_reply::OpReply;
pub use legacy::{OpInsert, OpUpdate, OpDelete};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(i32)]
pub enum OpCode {
    OpReply = 1,     // Deprecated
    OpMsg = 2013,
    OpPing = 2010,
    OpCompressed = 2012,
    OpQuery = 2004,
    OpGetMore = 2005, // Legacy
    OpDelete = 2006, // Legacy
    OpKillCursors = 2007, // Legacy
    OpInsert = 2002, // Legacy
    OpUpdate = 2001, // Legacy
}

impl TryFrom<i32> for OpCode {
    type Error = ProtocolError;
    fn try_from(v: i32) -> Result<Self> {
        match v {
            1 => Ok(OpCode::OpReply),
            2013 => Ok(OpCode::OpMsg),
            2010 => Ok(OpCode::OpPing),
            2012 => Ok(OpCode::OpCompressed),
            2004 => Ok(OpCode::OpQuery),
            2005 => Ok(OpCode::OpGetMore),
            2006 => Ok(OpCode::OpDelete),
            2007 => Ok(OpCode::OpKillCursors),
            2002 => Ok(OpCode::OpInsert),
            2001 => Ok(OpCode::OpUpdate),
            _ => Err(ProtocolError::Protocol(format!("Unknown Mongo OpCode: {}", v))),
        }
    }
}

pub struct MsgHeader {
    pub message_length: i32,
    pub request_id: i32,
    pub response_to: i32,
    pub op_code: OpCode,
}

impl MsgHeader {
    pub const HEADER_SIZE: usize = 16;

    pub fn read(src: &mut BytesMut) -> Result<Option<Self>> {
        if src.len() < Self::HEADER_SIZE {
            return Ok(None);
        }
        
        // Peek length
        let len_bytes = &src[0..4];
        let message_length = i32::from_le_bytes(len_bytes.try_into().unwrap());
        
        if src.len() < message_length as usize {
            return Ok(None);
        }
        
        src.advance(4); // Consume length
        let request_id = src.get_i32_le();
        let response_to = src.get_i32_le();
        let op_code_raw = src.get_i32_le();
        let op_code = OpCode::try_from(op_code_raw)?;

        Ok(Some(Self {
            message_length,
            request_id,
            response_to,
            op_code,
        }))
    }

    pub fn write(&self, dst: &mut BytesMut) {
        dst.put_i32_le(self.message_length);
        dst.put_i32_le(self.request_id);
        dst.put_i32_le(self.response_to);
        dst.put_i32_le(self.op_code as i32);
    }
}

pub struct MongoProtocol;

impl MongoProtocol {
    // Protocol handler methods will go here
}
