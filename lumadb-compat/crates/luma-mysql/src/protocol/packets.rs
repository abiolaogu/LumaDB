use bytes::{BufMut, BytesMut, Buf};
use luma_protocol_core::{ProtocolError, Result};
use byteorder::{ByteOrder, LittleEndian};

// Helper: Length Encoded Integer
pub fn read_lenenc_int(src: &mut BytesMut) -> Result<Option<u64>> {
    if src.is_empty() { return Ok(None); }
    let first = src[0];
    if first < 251 {
        src.advance(1);
        Ok(Some(first as u64))
    } else if first == 0xfc {
        if src.len() < 3 { return Ok(None); }
        src.advance(1);
        let val = src.get_u16_le();
        Ok(Some(val as u64))
    } else if first == 0xfd {
        if src.len() < 4 { return Ok(None); }
        src.advance(1);
        let v1 = src.get_u8() as u32;
        let v2 = src.get_u8() as u32;
        let v3 = src.get_u8() as u32;
        let val = (v3 << 16) | (v2 << 8) | v1;
        Ok(Some(val as u64))
    } else if first == 0xfe {
         if src.len() < 9 { return Ok(None); }
         src.advance(1);
         let val = src.get_u64_le();
         Ok(Some(val))
    } else {
        Err(ProtocolError::Protocol(format!("Invalid length encoded int: {}", first)))
    }
}

pub fn write_lenenc_int(dst: &mut BytesMut, val: u64) {
    if val < 251 {
        dst.put_u8(val as u8);
    } else if val < 0x10000 {
        dst.put_u8(0xfc);
        dst.put_u16_le(val as u16);
    } else if val < 0x1000000 {
            dst.put_u8(0xfd);
            dst.put_u8((val & 0xff) as u8);
            dst.put_u8(( (val >> 8) & 0xff ) as u8);
            dst.put_u8(( (val >> 16) & 0xff ) as u8);
    } else {
        dst.put_u8(0xfe);
        dst.put_u64_le(val);
    }
}

pub fn read_lenenc_str(src: &mut BytesMut) -> Result<Option<String>> {
    let len = match read_lenenc_int(src)? {
        Some(l) => l as usize,
        None => return Ok(None),
    };
    if src.len() < len {
        // Need to backtrack? The implementation of read_lenenc_int consumed bytes.
        // This suggests we need peek or atomic read. 
        // For simplified assumption (full packet in buffer):
        return Err(ProtocolError::Protocol("Incomplete packet for string".into()));
    }
    let bytes = src.split_to(len);
    Ok(Some(String::from_utf8_lossy(&bytes).to_string()))
}

pub fn write_lenenc_str(dst: &mut BytesMut, s: &str) {
    write_lenenc_int(dst, s.len() as u64);
    dst.put_slice(s.as_bytes());
}

pub enum Command {
    Sleep,
    Quit,
    InitDb { schema: String },
    Query { query: String },
    FieldList,
    CreateDb,
    DropDb,
    Refresh,
    Shutdown,
    Statistics,
    ProcessInfo,
    Connect,
    ProcessKill,
    Debug,
    Ping,
    Time,
    DelayedInsert,
    ChangeUser,
    BinlogDump,
    TableDump,
    ConnectOut,
    RegisterSlave,
    StmtPrepare,
    StmtExecute,
    StmtSendLongData,
    StmtClose,
    StmtReset,
    SetOption,
    StmtFetch,
    Daemon,
    BinlogDumpGtids,
    ResetConnection,
}

impl Command {
    pub fn parse(payload: &[u8]) -> Result<Option<Self>> {
        if payload.is_empty() {
             return Ok(None);
        }
        let cmd = payload[0];
        let data = &payload[1..];
        
        match cmd {
             0x00 => Ok(Some(Command::Sleep)),
             0x01 => Ok(Some(Command::Quit)),
             0x02 => Ok(Some(Command::InitDb { schema: String::from_utf8_lossy(data).to_string() })),
             0x03 => Ok(Some(Command::Query { query: String::from_utf8_lossy(data).to_string() })),
             0x0e => Ok(Some(Command::Ping)),
             // ... others
             _ => Ok(Some(Command::Query { query: format!("UNKNOWN CMD: {}", cmd) })), // Fallback or Error
        }
    }
}

pub struct OKPacket {
    pub affected_rows: u64,
    pub last_insert_id: u64,
    pub status_flags: u16,
    pub warnings: u16,
    pub info: String,
}

impl OKPacket {
    pub fn write(&self, dst: &mut BytesMut) {
        dst.put_u8(0x00); // OK header
        // encoding length-encoded integers is tricky.
        // For now, simpler implementation:
        // if val < 251, 1 byte.
        // if < 2^16, 0xfc + 2 bytes
        // if < 2^24, 0xfd + 3 bytes
        // else 0xfe + 8 bytes
        write_lenenc_int(dst, self.affected_rows);
        write_lenenc_int(dst, self.last_insert_id);
        
        dst.put_u16_le(self.status_flags); 
        dst.put_u16_le(self.warnings);
        // info string?
        // if Capabilities & CLIENT_SESSION_TRACK, then lenenc string.
        // else just string EOF.
        dst.put_slice(self.info.as_bytes());
    }
}
// Column Definition Packet
pub struct ERRPacket {
    pub error_code: u16,
    pub sql_state_marker: u8, // '#'
    pub sql_state: String, // 5 chars
    pub error_message: String,
}

impl ERRPacket {
     pub fn write(&self, dst: &mut BytesMut) {
         dst.put_u8(0xff); // ERR header
         dst.put_u16_le(self.error_code);
         dst.put_u8(b'#');
         dst.put_slice(self.sql_state.as_bytes());
         dst.put_slice(self.error_message.as_bytes());
     }
}

pub struct ColumnDefinition {
    pub catalog: String,
    pub schema: String,
    pub table: String,
    pub org_table: String,
    pub name: String,
    pub org_name: String,
    pub charset: u16,
    pub column_length: u32,
    pub column_type: u8,
    pub flags: u16,
    pub decimals: u8,
}

impl ColumnDefinition {
    pub fn write(&self, dst: &mut BytesMut) {
        write_lenenc_str(dst, &self.catalog);
        write_lenenc_str(dst, &self.schema);
        write_lenenc_str(dst, &self.table);
        write_lenenc_str(dst, &self.org_table);
        write_lenenc_str(dst, &self.name);
        write_lenenc_str(dst, &self.org_name);
        write_lenenc_int(dst, 0x0c); // length of fixed fields
        dst.put_u16_le(self.charset);
        dst.put_u32_le(self.column_length);
        dst.put_u8(self.column_type);
        dst.put_u16_le(self.flags);
        dst.put_u8(self.decimals);
        dst.put_slice(&[0u8; 2]); // filler
    }
}

pub struct EOFPacket {
    pub warnings: u16,
    pub status_flags: u16,
}

impl EOFPacket {
    pub fn write(&self, dst: &mut BytesMut) {
        dst.put_u8(0xfe);
        dst.put_u16_le(self.warnings);
        dst.put_u16_le(self.status_flags);
    }
}

// Text Resultset Row
// All values are length-encoded strings. NULL is 0xFB.
pub struct TextRow<'a> {
    pub values: Vec<Option<&'a str>>,
}

impl<'a> TextRow<'a> {
    pub fn write(&self, dst: &mut BytesMut) {
        for val in &self.values {
            match val {
                Some(s) => write_lenenc_str(dst, s),
                None => dst.put_u8(0xfb),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_lenenc_int() {
        let mut buf = BytesMut::new();
        write_lenenc_int(&mut buf, 250);
        write_lenenc_int(&mut buf, 65535);
        
        // Read back
        assert_eq!(read_lenenc_int(&mut buf).unwrap().unwrap(), 250);
        assert_eq!(read_lenenc_int(&mut buf).unwrap().unwrap(), 65535);
    }

    #[test]
    fn test_parse_command_query() {
        let payload = b"\x03SELECT 1";
        let cmd = Command::parse(payload).unwrap().unwrap();
        match cmd {
            Command::Query { query } => assert_eq!(query, "SELECT 1"),
            _ => panic!("Expected Query"),
        }
    }

    #[test]
    fn test_ok_packet_serialization() {
       let mut buf = BytesMut::new();
       let ok = OKPacket {
           affected_rows: 1,
           last_insert_id: 12345,
           status_flags: 2,
           warnings: 0,
           info: "".to_string(),
       };
       ok.write(&mut buf);
       
       assert_eq!(buf[0], 0x00); // Header
       assert_eq!(buf[1], 1); // Affected
       assert_eq!(buf[2], 0xfc); // Last insert ID > 250 -> 2 bytes
       assert_eq!(buf[3], 0x39); // 12345 & 0xFF
       assert_eq!(buf[4], 0x30); // 12345 >> 8
    }
}
