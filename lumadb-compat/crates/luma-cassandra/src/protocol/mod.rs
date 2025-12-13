pub mod frame;
pub mod messages;
pub mod auth;
mod tests;

use bytes::{Buf, BufMut, BytesMut};
use luma_protocol_core::{ProtocolError, Result};
pub use frame::Opcode;
pub use frame::Compression;

pub struct Frame {
    pub version: u8,
    pub flags: u8,
    pub stream_id: i16,
    pub opcode: Opcode,
    pub body: BytesMut,
}

pub struct CassandraProtocol;

impl CassandraProtocol {
    pub fn read_frame(src: &mut BytesMut, compression: Compression) -> Result<Option<Frame>> {
        if src.len() < 9 {
             return Ok(None);
        }

        let version = src[0] & 0x7F; // Mask out direction bit
        let flags = src[1];
        let stream_id = i16::from_be_bytes([src[2], src[3]]);
        let opcode_byte = src[4];
        let opcode = Opcode::try_from(opcode_byte)?;
        
        let length = u32::from_be_bytes([src[5], src[6], src[7], src[8]]) as usize;
        
        if src.len() < 9 + length {
             return Ok(None);
        }

        let _header = src.split_to(9);
        let mut body_bytes = src.split_to(length);

        if (flags & 0x01) != 0 { // COMPRESSION flag
             let decompressed = frame::decompress(&body_bytes, compression)?;
             body_bytes = BytesMut::from(&decompressed[..]);
        }

        Ok(Some(Frame {
            version,
            flags,
            stream_id,
            opcode,
            body: body_bytes,
        }))
    }
    
    pub fn write_frame(frame: &Frame, dst: &mut BytesMut, compression: Compression) -> Result<()> {
        // Compress body if needed
        // Valid compression usually applied if body len > threshold, but for simplicity we rely on 'flags' checking or algo presence
        // However, usually the connection has negotiated compression. If 'compression' is set, we compress and set flag.
        
        let mut body = frame::compress(&frame.body, compression)?;
        let mut flags = frame.flags;
        if compression != Compression::None {
            flags |= 0x01; 
        } else {
             body = frame.body.to_vec();
        }

        let length = body.len() as u32;
        dst.reserve(9 + length as usize);
        
        dst.put_u8(frame.version);
        dst.put_u8(flags);
        dst.put_i16(frame.stream_id);
        dst.put_u8(frame.opcode as u8);
        dst.put_u32(length);
        dst.put_slice(&body);
        Ok(())
    }
}
