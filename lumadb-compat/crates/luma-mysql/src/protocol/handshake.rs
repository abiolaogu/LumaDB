use bytes::{BufMut, BytesMut, Buf};
use luma_protocol_core::{ProtocolError, Result};
use byteorder::{LittleEndian, ByteOrder};

// Capabilities (Full List)
pub const CLIENT_LONG_PASSWORD: u32 = 1;
pub const CLIENT_FOUND_ROWS: u32 = 2;
pub const CLIENT_LONG_FLAG: u32 = 4;
pub const CLIENT_CONNECT_WITH_DB: u32 = 8;
pub const CLIENT_NO_SCHEMA: u32 = 16;
pub const CLIENT_COMPRESS: u32 = 32;
pub const CLIENT_ODBC: u32 = 64;
pub const CLIENT_LOCAL_FILES: u32 = 128;
pub const CLIENT_IGNORE_SPACE: u32 = 256;
pub const CLIENT_PROTOCOL_41: u32 = 512;
pub const CLIENT_INTERACTIVE: u32 = 1024;
pub const CLIENT_SSL: u32 = 2048;
pub const CLIENT_IGNORE_SIGPIPE: u32 = 4096;
pub const CLIENT_TRANSACTIONS: u32 = 8192; // 4.1+
pub const CLIENT_RESERVED: u32 = 16384;
pub const CLIENT_SECURE_CONNECTION: u32 = 32768;
pub const CLIENT_MULTI_STATEMENTS: u32 = 65536;
pub const CLIENT_MULTI_RESULTS: u32 = 131072;
pub const CLIENT_PS_MULTI_RESULTS: u32 = 262144;
pub const CLIENT_PLUGIN_AUTH: u32 = 524288;
pub const CLIENT_CONNECT_ATTRS: u32 = 1048576;
pub const CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA: u32 = 2097152;
pub const CLIENT_CAN_HANDLE_EXPIRED_PASSWORDS: u32 = 4194304;
pub const CLIENT_SESSION_TRACK: u32 = 8388608;
pub const CLIENT_DEPRECATE_EOF: u32 = 16777216;

pub struct HandshakeV10 {
    pub protocol_version: u8, // 10
    pub server_version: String,
    pub connection_id: u32,
    pub auth_plugin_data_part_1: [u8; 8],
    pub capability_flags_1: u16, // Lower 16 bits
    pub character_set: u8,
    pub status_flags: u16,
    pub capability_flags_2: u16, // Upper 16 bits
    pub auth_plugin_data_len: u8, 
    pub auth_plugin_data_part_2: [u8; 12], 
    pub auth_plugin_name: String,
}

impl HandshakeV10 {
    pub fn new(connection_id: u32, salt: &[u8]) -> Self {
        // Need 20 bytes salt total. Split into 8 + 12.
        let mut part1 = [0u8; 8];
        let mut part2 = [0u8; 12];
        part1.copy_from_slice(&salt[0..8]);
        part2.copy_from_slice(&salt[8..20]);

        Self {
            protocol_version: 10,
            server_version: "8.0.35-LumaDB".to_string(), // As requested
            connection_id,
            auth_plugin_data_part_1: part1,
            capability_flags_1: (CLIENT_PROTOCOL_41 | CLIENT_SECURE_CONNECTION | CLIENT_PLUGIN_AUTH) as u16,
            character_set: 45, // utf8mb4_general_ci usually
            status_flags: 0x0002, // SERVER_STATUS_AUTOCOMMIT
            capability_flags_2: ((CLIENT_PROTOCOL_41 | CLIENT_SECURE_CONNECTION | CLIENT_PLUGIN_AUTH) >> 16) as u16,
            auth_plugin_data_len: 21, // 8 + 12 + 1 (autocommit logic varies) usually 21 for 20 bytes salt + null terminator logic
            auth_plugin_data_part_2: part2,
            auth_plugin_name: "mysql_native_password".to_string(),
        }
    }

    pub fn write(&self, dst: &mut BytesMut) {
        dst.put_u8(self.protocol_version);
        dst.put_slice(self.server_version.as_bytes());
        dst.put_u8(0);
        dst.put_u32_le(self.connection_id);
        dst.put_slice(&self.auth_plugin_data_part_1);
        dst.put_u8(0); // filler
        dst.put_u16_le(self.capability_flags_1);
        dst.put_u8(self.character_set);
        dst.put_u16_le(self.status_flags);
        dst.put_u16_le(self.capability_flags_2);
        dst.put_u8(self.auth_plugin_data_len); // length of auth plugin data
        dst.put_slice(&[0u8; 10]); // reserved
        dst.put_slice(&self.auth_plugin_data_part_2);
        // spec: auth_plugin_data_part_2 then terminator?
        // if auth_plugin_data_len > 0
        // actually standard is 12 bytes + 0 terminator.
        dst.put_u8(0); // terminator for part 2
        dst.put_slice(self.auth_plugin_name.as_bytes());
        dst.put_u8(0);
    }
}

pub struct HandshakeResponse41 {
    pub capabilities: u32,
    pub max_packet_size: u32,
    pub character_set: u8,
    pub username: String,
    pub auth_response: Vec<u8>,
    pub database: Option<String>,
    pub auth_plugin_name: Option<String>,
}

impl HandshakeResponse41 {
    pub fn parse(mut src: BytesMut) -> Result<Self> {
        // Assume packet header already stripped
        if src.len() < 32 {
            return Err(ProtocolError::Protocol("Handshake response too short".into()));
        }

        let capabilities = src.get_u32_le();
        let max_packet_size = src.get_u32_le();
        let character_set = src.get_u8();
        let _reserved = src.split_to(23); // 23 bytes reserved

        // Username: null terminated string
        let mut username = String::new();
        if let Some(pos) = src.iter().position(|&b| b == 0) {
            let bytes = src.split_to(pos);
            src.advance(1); // skip null
            username = String::from_utf8_lossy(&bytes).to_string();
        } else {
            return Err(ProtocolError::Protocol("Invalid username format".into()));
        }

        let mut auth_response = Vec::new();
        if capabilities & CLIENT_PLUGIN_AUTH != 0 {
             if capabilities & CLIENT_SECURE_CONNECTION != 0 {
                 let len = src.get_u8() as usize;
                 auth_response = src.split_to(len).to_vec();
             } else {
                 if let Some(pos) = src.iter().position(|&b| b == 0) {
                      auth_response = src.split_to(pos).to_vec();
                      src.advance(1);
                 }
             }
        }

        let mut database = None;
        if capabilities & CLIENT_CONNECT_WITH_DB != 0 {
             if let Some(pos) = src.iter().position(|&b| b == 0) {
                let bytes = src.split_to(pos);
                src.advance(1);
                database = Some(String::from_utf8_lossy(&bytes).to_string());
            }
        }

        let mut auth_plugin_name = None;
        if capabilities & CLIENT_PLUGIN_AUTH != 0 {
             if let Some(pos) = src.iter().position(|&b| b == 0) {
                let bytes = src.split_to(pos);
                src.advance(1);
                auth_plugin_name = Some(String::from_utf8_lossy(&bytes).to_string());
            }
        }

        Ok(Self {
            capabilities,
            max_packet_size,
            character_set,
            username,
            auth_response,
            database,
            auth_plugin_name,
        })
    }
}
