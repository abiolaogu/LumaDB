use bytes::{Buf, BufMut, BytesMut};
use luma_protocol_core::{ProtocolError, Result};
use std::collections::HashMap;
use std::io::Read;

// Compression types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Compression {
    None,
    Lz4,
    Snappy,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Opcode {
    Error = 0x00,
    Startup = 0x01,
    Ready = 0x02,
    Authenticate = 0x03,
    Options = 0x05,
    Supported = 0x06,
    Query = 0x07,
    Result = 0x08,
    Prepare = 0x09,
    Execute = 0x0A,
    Register = 0x0B,
    Event = 0x0C,
    Batch = 0x0D,
    AuthChallenge = 0x0E,
    AuthResponse = 0x0F,
    AuthSuccess = 0x10,
}

impl TryFrom<u8> for Opcode {
    type Error = ProtocolError;
    fn try_from(v: u8) -> Result<Self> {
        match v {
            0x00 => Ok(Opcode::Error),
            0x01 => Ok(Opcode::Startup),
            0x02 => Ok(Opcode::Ready),
            0x03 => Ok(Opcode::Authenticate),
            0x05 => Ok(Opcode::Options),
            0x06 => Ok(Opcode::Supported),
            0x07 => Ok(Opcode::Query),
            0x08 => Ok(Opcode::Result),
            0x09 => Ok(Opcode::Prepare),
            0x0A => Ok(Opcode::Execute),
            0x0B => Ok(Opcode::Register),
            0x0C => Ok(Opcode::Event),
            0x0D => Ok(Opcode::Batch),
            0x0E => Ok(Opcode::AuthChallenge),
            0x0F => Ok(Opcode::AuthResponse),
            0x10 => Ok(Opcode::AuthSuccess),
            _ => Err(ProtocolError::Protocol(format!("Unknown opcode: {}", v))),
        }
    }
}

pub struct StartupBody {
    pub options: HashMap<String, String>,
}

impl StartupBody {
    pub fn read(src: &mut BytesMut) -> Result<Self> {
        if src.len() < 2 { return Err(ProtocolError::Protocol("Incomplete startup map count".into())); }
        let count = src.get_u16();
        let mut options = HashMap::new();
        for _ in 0..count {
            let key = read_string(src)?;
            let val = read_string(src)?;
            options.insert(key, val);
        }
        Ok(Self { options })
    }
    
    pub fn write(dst: &mut BytesMut, options: &HashMap<String, String>) {
        dst.put_u16(options.len() as u16);
        for (k, v) in options {
            write_string(dst, k);
            write_string(dst, v);
        }
    }
}

pub fn compress(data: &[u8], algo: Compression) -> Result<Vec<u8>> {
    match algo {
        Compression::None => Ok(data.to_vec()),
        Compression::Lz4 => {
             // LZ4 framing format or block? Cassandra usually uses block compression for body
             // but specific framing might be needed. The spec says "LZ4 compressed body".
             // We'll use simple block compression for now. 
             let mut encoder = lz4::EncoderBuilder::new().level(4).build(Vec::new())
                .map_err(|e| ProtocolError::Protocol(format!("LZ4 init error: {}", e)))?;
             std::io::Write::write_all(&mut encoder, data)
                .map_err(|e| ProtocolError::Protocol(format!("LZ4 write error: {}", e)))?;
             let (result, result_result) = encoder.finish();
             result_result.map_err(|e| ProtocolError::Protocol(format!("LZ4 finish error: {}", e)))?;
             Ok(result)
        },
        Compression::Snappy => {
            let mut encoder = snap::write::FrameEncoder::new(Vec::new());
             std::io::Write::write_all(&mut encoder, data)
                .map_err(|e| ProtocolError::Protocol(format!("Snappy write error: {}", e)))?;
             encoder.into_inner().map_err(|e| ProtocolError::Protocol(format!("Snappy finish error: {}", e)))
        }
    }
}

pub fn decompress(data: &[u8], algo: Compression) -> Result<Vec<u8>> {
    match algo {
        Compression::None => Ok(data.to_vec()),
        Compression::Lz4 => {
            let mut decoder = lz4::Decoder::new(data)
               .map_err(|e| ProtocolError::Protocol(format!("LZ4 decode init error: {}", e)))?;
            let mut decoded = Vec::new();
            decoder.read_to_end(&mut decoded)
               .map_err(|e| ProtocolError::Protocol(format!("LZ4 decode error: {}", e)))?;
            Ok(decoded)
        },
        Compression::Snappy => {
            let mut decoder = snap::read::FrameDecoder::new(data);
            let mut decoded = Vec::new();
            decoder.read_to_end(&mut decoded)
               .map_err(|e| ProtocolError::Protocol(format!("Snappy decode error: {}", e)))?;
            Ok(decoded)
        }
    }
}

// Helpers
pub fn read_string(src: &mut BytesMut) -> Result<String> {
    if src.len() < 2 { return Err(ProtocolError::Protocol("Incomplete string len".into())); }
    let len = src.get_u16() as usize;
    if src.len() < len { return Err(ProtocolError::Protocol("Incomplete string body".into())); }
    let bytes = src.split_to(len);
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

pub fn write_string(dst: &mut BytesMut, s: &str) {
    dst.put_u16(s.len() as u16);
    dst.put_slice(s.as_bytes());
}

pub fn read_long_string(src: &mut BytesMut) -> Result<String> {
    if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete long string len".into())); }
    let len = src.get_i32() as usize;
    if len < 0 { return Ok("".to_string()); } // Valid for empty
    if len as usize > src.len() { return Err(ProtocolError::Protocol("Incomplete long string body".into())); }
    let bytes = src.split_to(len as usize);
    Ok(String::from_utf8_lossy(&bytes).to_string())
}
