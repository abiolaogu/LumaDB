use bytes::{Buf, BufMut, BytesMut};
use luma_protocol_core::{ProtocolError, Result};
use byteorder::{LittleEndian, ByteOrder};

pub mod packets;
pub mod handshake;
pub mod auth;

// Re-export key packet types
pub use packets::{Command, OKPacket, ERRPacket, ColumnDefinition, EOFPacket, TextRow};
pub use handshake::HandshakeV10;

// MySQL Packet Header
// 3 bytes length
// 1 byte sequence id
pub struct PacketHeader {
    pub length: u32,
    pub seq_id: u8,
}

pub struct Packet {
    pub header: PacketHeader,
    pub payload: BytesMut,
}

pub struct MySQLProtocol {
    pub seq_id: u8,
}

impl MySQLProtocol {
    pub fn new() -> Self {
        Self { seq_id: 0 }
    }

    /// Reads a full packet from the buffer.
    pub fn read_packet(src: &mut BytesMut) -> Result<Option<Packet>> {
        if src.len() < 4 {
            return Ok(None);
        }

        let len = (src[0] as u32) | ((src[1] as u32) << 8) | ((src[2] as u32) << 16);
        let seq_id = src[3];

        if src.len() < 4 + len as usize {
             return Ok(None);
        }

        // Advance buffer
        let _header = src.split_to(4);
        let payload = src.split_to(len as usize);

        Ok(Some(Packet {
            header: PacketHeader { length: len, seq_id },
            payload,
        }))
    }

    pub fn write_packet(payload: &[u8], seq_id: u8, dst: &mut BytesMut) {
        let len = payload.len();
        // MySQL packet max size is 16MB-1. If larger, needs splitting (TODO)
        
        dst.reserve(4 + len);
        dst.put_u8(len as u8);
        dst.put_u8((len >> 8) as u8);
        dst.put_u8((len >> 16) as u8);
        dst.put_u8(seq_id);
        dst.put_slice(payload);
    }
}
