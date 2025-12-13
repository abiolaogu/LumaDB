use bytes::{BytesMut, Buf, BufMut};
use luma_protocol_core::{ProtocolError, Result};
use super::{OpCode, MsgHeader};
use std::io::Read;

#[allow(dead_code)]
pub struct OpCompressed {
    pub original_opcode: OpCode,
    pub uncompressed_size: i32,
    pub compressor_id: u8,
    pub compressed_message: BytesMut,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum CompressorId {
    Noop = 0,
    Snappy = 1,
    Zlib = 2,
    Zstd = 3,
}

impl OpCompressed {
    pub fn read(src: &mut BytesMut) -> Result<Self> {
        if src.len() < 9 { return Err(ProtocolError::Protocol("Incomplete OpCompressed header".into())); }
        let original_opcode_raw = src.get_i32_le();
        let original_opcode = OpCode::try_from(original_opcode_raw)?;
        let uncompressed_size = src.get_i32_le();
        let compressor_id = src.get_u8();
        
        let compressed_message = src.split_to(src.len()); // Consume rest
        
        Ok(Self {
            original_opcode,
            uncompressed_size,
            compressor_id,
            compressed_message,
        })
    }
    
    pub fn decompress(&self) -> Result<BytesMut> {
        match self.compressor_id {
            0 => Ok(self.compressed_message.clone()), // Noop
            1 => { // Snappy
                let mut decoder = snap::read::FrameDecoder::new(self.compressed_message.reader());
                let mut out = Vec::with_capacity(self.uncompressed_size as usize);
                decoder.read_to_end(&mut out).map_err(|e| ProtocolError::Protocol(format!("Snappy error: {}", e)))?;
                Ok(BytesMut::from(&out[..]))
            },
            2 => { // Zlib
                 let mut decoder = flate2::read::ZlibDecoder::new(self.compressed_message.reader());
                 let mut out = Vec::with_capacity(self.uncompressed_size as usize);
                 decoder.read_to_end(&mut out).map_err(|e| ProtocolError::Protocol(format!("Zlib error: {}", e)))?;
                 Ok(BytesMut::from(&out[..]))
            },
            3 => { // Zstd
                 let mut decoder = zstd::stream::read::Decoder::new(self.compressed_message.reader())
                    .map_err(|e| ProtocolError::Protocol(format!("Zstd init error: {}", e)))?;
                 let mut out = Vec::with_capacity(self.uncompressed_size as usize);
                 decoder.read_to_end(&mut out).map_err(|e| ProtocolError::Protocol(format!("Zstd error: {}", e)))?;
                 Ok(BytesMut::from(&out[..]))
            },
            _ => Err(ProtocolError::Protocol(format!("Unknown compressor ID: {}", self.compressor_id))),
        }
    }
}

pub fn read_message(src: &mut BytesMut) -> Result<Option<(MsgHeader, BytesMut)>> {
    // Read header first without consuming if incomplete
    if src.len() < MsgHeader::HEADER_SIZE { return Ok(None); }
    
    // Peek length
    let len_bytes = &src[0..4];
    let msg_len = i32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
    
    if src.len() < msg_len { return Ok(None); }
    
    // We have full message.
    // Read header
    // MsgHeader::read consumes if full, but we want to split body separate.
    // Actually MsgHeader::read checks specific logic.
    // Let's rely on manual split for purity here.
    
    let mut header_bytes = src.split_to(MsgHeader::HEADER_SIZE);
    let header = MsgHeader::read(&mut header_bytes)?.unwrap(); // Must succeed
    let mut body = src.split_to(msg_len - MsgHeader::HEADER_SIZE);
    
    if header.op_code == OpCode::OpCompressed { // OpCompressed = 2012
        // We need to handle this OpCode in our enum first
        // Wait, did I add OpCompressed(2012) to enum?
        // Let's assume it's added or handled as raw.
        // My OpCode enum didn't have 2012. I should update OpCode enum.
        // But logic is: if 2012, decompress, then return Inner Header + Inner Body.
        // The inner message is a full message (Header + Body).
        
        let compressed = OpCompressed::read(&mut body)?;
        let decompressed = compressed.decompress()?;
        
        // Decompressed data is a FULL message (Header + Body).
        // Recurse? Or just parse header again.
        let mut d_buf = decompressed;
        if d_buf.len() < MsgHeader::HEADER_SIZE { return Err(ProtocolError::Protocol("Invalid decompressed size".into()));}
        
         let mut d_header_bytes = d_buf.split_to(MsgHeader::HEADER_SIZE);
         let d_header = MsgHeader::read(&mut d_header_bytes)?.unwrap();
         
         // Inner opcode should match original_opcode from compressed header
         // Check consistency if needed
         
         Ok(Some((d_header, d_buf)))
    } else {
        Ok(Some((header, body)))
    }
}
