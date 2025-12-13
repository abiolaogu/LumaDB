use bytes::{Buf, BufMut, BytesMut};
use luma_protocol_core::{ProtocolError, Result};
use std::collections::HashMap;

/// Postgres Protocol Version 3
const SSL_REQUEST: i32 = 80877103;

#[derive(Debug)]
pub enum StartupMessage {
    Normal {
        version: i32,
        parameters: HashMap<String, String>,
    },
    SslRequest,
    CancelRequest {
        process_id: i32,
        secret_key: i32,
    },
    PasswordMessage {
        password: String,
    },
    SASLInitialResponse {
        mechanism: String,
        data: Vec<u8>,
    },
    SASLResponse {
        data: Vec<u8>,
    },
}

impl StartupMessage {
    pub fn parse(src: &mut BytesMut) -> Result<Option<Self>> {
        if src.len() < 4 {
            return Ok(None);
        }

        // Peek length
        let len = i32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

        if src.len() < len {
            return Ok(None);
        }

        // We have the full frame
        let mut data = src.split_to(len);
        let _len = data.get_i32(); // consume length

        let protocol_version = data.get_i32();

        if protocol_version == SSL_REQUEST {
            return Ok(Some(StartupMessage::SslRequest));
        }

        if protocol_version == 80877102 { // Cancel
             let process_id = data.get_i32();
             let secret_key = data.get_i32();
             return Ok(Some(StartupMessage::CancelRequest { process_id, secret_key }));
        }

        // Normal startup
        let mut parameters = HashMap::new();
        
        while data.has_remaining() {
            let key = read_cstring(&mut data)?;
            if key.is_empty() {
                break;
            }
            let value = read_cstring(&mut data)?;
            parameters.insert(key, value);
        }

        Ok(Some(StartupMessage::Normal {
            version: protocol_version,
            parameters,
        }))
    }
}

fn read_cstring(buf: &mut BytesMut) -> Result<String> {
    let mut bytes = Vec::new();
    while buf.has_remaining() {
        let b = buf.get_u8();
        if b == 0 {
            break;
        }
        bytes.push(b);
    }
    String::from_utf8(bytes).map_err(|_| ProtocolError::Protocol("Invalid UTF-8 in startup message".into()))
}

#[derive(Debug)]
pub enum BackendMessage {
    AuthenticationOk,
    AuthenticationMD5Password {
        salt: [u8; 4],
    },
    AuthenticationSASL {
        mechanisms: Vec<String>,
    },
    AuthenticationSASLContinue {
        data: Vec<u8>,
    },
    AuthenticationSASLFinal {
        data: Vec<u8>,
    },
    ReadyForQuery,
    ErrorResponse {
        message: String,
        code: String,
    },
    CommandComplete {
        tag: String,
    },
    BackendKeyData {
        process_id: i32,
        secret_key: i32,
    },
    ParameterStatus {
        name: String,
        value: String,
    },
    DataRow {
        values: Vec<Option<bytes::Bytes>>,
    },
    RowDescription {
        fields: Vec<(String, i32)>, // Name, Type OID
    },
    ParseComplete,
    BindComplete,
    NoData,
    PortalSuspended,
    CloseComplete,
}

impl BackendMessage {
    pub fn write(&self, dst: &mut BytesMut) {
        match self {
            BackendMessage::AuthenticationOk => {
                dst.put_u8(b'R');
                dst.put_i32(8); // len
                dst.put_i32(0); // Auth OK
            }
            BackendMessage::AuthenticationMD5Password { salt } => {
                dst.put_u8(b'R');
                dst.put_i32(12); // len
                dst.put_i32(5); // MD5
                dst.put_slice(salt);
            }
            BackendMessage::AuthenticationSASL { mechanisms } => {
                dst.put_u8(b'R');
                let mut mech_len = 0;
                for m in mechanisms {
                    mech_len += m.len() + 1;
                }
                dst.put_i32(4 + 4 + mech_len as i32 + 1); // len: 4 + 4(code) + mechs + 1(null)
                dst.put_i32(10); // SASL
                for m in mechanisms {
                    dst.put_slice(m.as_bytes());
                    dst.put_u8(0);
                }
                dst.put_u8(0); // Terminator
            }
            BackendMessage::AuthenticationSASLContinue { data } => {
                dst.put_u8(b'R');
                dst.put_i32(4 + 4 + data.len() as i32);
                dst.put_i32(11); // SASL Continue
                dst.put_slice(data);
            }
            BackendMessage::AuthenticationSASLFinal { data } => {
                dst.put_u8(b'R');
                dst.put_i32(4 + 4 + data.len() as i32);
                dst.put_i32(12); // SASL Final
                dst.put_slice(data);
            }
            BackendMessage::ReadyForQuery => {
                dst.put_u8(b'Z');
                dst.put_i32(5);
                dst.put_u8(b'I'); // Idle
            }
            BackendMessage::ErrorResponse { message, code } => {
                dst.put_u8(b'E');
                // Calculate len: 4 (len) + 1 (S) + 1 (severity) + 0 + 1 (C) + 5 (code) + 0 + 1 (M) + msg + 0 + 0 (end)
                let len = 4 + 1 + "ERROR".len() + 1 + 1 + code.len() + 1 + 1 + message.len() + 1 + 1;
                dst.put_i32(len as i32);
                
                dst.put_u8(b'S');
                dst.put_slice(b"ERROR\0");
                dst.put_u8(b'C');
                dst.put_slice(code.as_bytes());
                dst.put_u8(0);
                dst.put_u8(b'M');
                dst.put_slice(message.as_bytes());
                dst.put_u8(0);
                dst.put_u8(0);
            }
            BackendMessage::CommandComplete { tag } => {
                dst.put_u8(b'C');
                dst.put_i32((4 + tag.len() + 1) as i32);
                dst.put_slice(tag.as_bytes());
                dst.put_u8(0);
            }
            BackendMessage::BackendKeyData { process_id, secret_key } => {
                dst.put_u8(b'K');
                dst.put_i32(12); // 4 + 4 + 4
                dst.put_i32(*process_id);
                dst.put_i32(*secret_key);
            }
            BackendMessage::ParameterStatus { name, value } => {
                dst.put_u8(b'S');
                let len = 4 + name.len() + 1 + value.len() + 1;
                dst.put_i32(len as i32);
                dst.put_slice(name.as_bytes());
                dst.put_u8(0);
                dst.put_slice(value.as_bytes());
                dst.put_u8(0);
            }
            BackendMessage::DataRow { values } => {
                dst.put_u8(b'D');
                // Length calculation: 4 (len) + 2 (col count) + for each col: 4 (len) + val len
                let mut len = 4 + 2;
                for val in values {
                    len += 4;
                    if let Some(v) = val {
                        len += v.len();
                    }
                }
                dst.put_i32(len as i32);
                dst.put_i16(values.len() as i16);
                for val in values {
                    match val {
                        Some(v) => {
                            dst.put_i32(v.len() as i32);
                            dst.put_slice(v);
                        }
                        None => {
                            dst.put_i32(-1); // NULL
                        }
                    }
                }
            }
            BackendMessage::RowDescription { fields } => {
                dst.put_u8(b'T');
                // Len: 4 + 2 (count) + for each field: name\0 + 4(table oid) + 2(col attr) + 4(type oid) + 2(type len) + 4(type mod) + 2(format)
                let mut len = 4 + 2;
                for (name, _) in fields {
                    len += name.len() + 1 + 18;
                }
                dst.put_i32(len as i32);
                dst.put_i16(fields.len() as i16);
                for (name, oid) in fields {
                    dst.put_slice(name.as_bytes());
                    dst.put_u8(0);
                    dst.put_i32(0); // Table OID
                    dst.put_i16(0); // Col Attr
                    dst.put_i32(*oid); // Type OID
                    dst.put_i16(-1); // Type Len (var)
                    dst.put_i32(-1); // Type Mod
                    dst.put_i16(0); // Format (Text) - TODO make configurable
                }
            }
            BackendMessage::ParseComplete => {
                dst.put_u8(b'1');
                dst.put_i32(4);
            }
            BackendMessage::BindComplete => {
                dst.put_u8(b'2');
                dst.put_i32(4);
            }
            BackendMessage::NoData => {
                dst.put_u8(b'n');
                dst.put_i32(4);
            }
            BackendMessage::PortalSuspended => {
                dst.put_u8(b's');
                dst.put_i32(4);
            }
            BackendMessage::CloseComplete => {
                dst.put_u8(b'3');
                dst.put_i32(4);
            }
        }
    }
}

#[derive(Debug)]
pub enum FrontendMessage {
    Query { query: String },
    Terminate,
    Parse {
        name: String,
        query: String,
        param_types: Vec<i32>,
    },
    Bind {
        portal: String,
        statement: String,
        param_formats: Vec<i16>,
        params: Vec<Option<bytes::Bytes>>,
        result_formats: Vec<i16>,
    },
    Execute {
        portal: String,
        max_rows: i32,
    },
    Sync,
    Describe {
        kind: u8, // 'S' for statement, 'P' for portal
        name: String,
    },
    Close {
        kind: u8,
        name: String,
    },
    Flush, // Added Flush variant
    CopyData {
        data: bytes::Bytes,
    }, // Added CopyData variant
}

impl FrontendMessage {
    pub fn parse(src: &mut BytesMut) -> Result<Option<Self>> {
        if src.len() < 5 {
            return Ok(None);
        }

        let msg_type = src[0];
        let len = i32::from_be_bytes([src[1], src[2], src[3], src[4]]) as usize;

        if src.len() < 1 + len {
            return Ok(None);
        }

        let mut data = src.split_to(1 + len);
        let _type = data.get_u8();
        let _len = data.get_i32();

        match msg_type {
            b'Q' => {
                let query = read_cstring(&mut data)?;
                Ok(Some(FrontendMessage::Query { query }))
            }
            b'X' => Ok(Some(FrontendMessage::Terminate)),
            b'P' => { // Parse
                let name = read_cstring(&mut data)?;
                let query = read_cstring(&mut data)?;
                let num_params = data.get_i16() as usize;
                let mut param_types = Vec::with_capacity(num_params);
                for _ in 0..num_params {
                    param_types.push(data.get_i32());
                }
                Ok(Some(FrontendMessage::Parse { name, query, param_types }))
            }
            b'p' => { // PasswordMessage or SASLResponse (same tag 'p')
                // Wait, SASLResponse is also 'p'. How to distinguish?
                // Context matters. But raw parser sees only bytes.
                // Standard PasswordMessage is just string.
                // SASLResponse is: len, data(bytes).
                // Actually Password message is: 'p', len, string_password
                // SASLResponse is: 'p', len, data
                // In context of SASL, it's SASLResponse. In context of MD5/Cleartext, it's Password.
                // We'll return it as PasswordMessage or GenericResponse, and let proper handler decode?
                // Or maybe they are compatible?
                // Password is \0 terminated. SASL data is NOT.
                // Safe bet: Parse as GenericBytes and let auth handler decide?
                // No, let's look at spec. SASLResponse: data (Bytea).
                // PasswordMessage: password (String).
                // Let's implement PasswordMessage as Bytes to be safe.
                // Actually, let's just use `PasswordMessage` holding bytes.
                // But wait, the tool requires me to follow specific structure.
                // Let's assume PasswordMessage for now as a catch-all for 'p' messages content.
                // "p" tag is used for both.
                // Let's assume it's SASLResponse if doing SASL, Password if doing MD5.
                // For now, let's read as bytes.
                let data = data.to_vec();
                Ok(Some(FrontendMessage::SASLResponse { data }))
            }
            b'B' => { // Bind
                let portal = read_cstring(&mut data)?;
                let statement = read_cstring(&mut data)?;
                let num_param_formats = data.get_i16() as usize;
                let mut param_formats = Vec::with_capacity(num_param_formats);
                for _ in 0..num_param_formats {
                    param_formats.push(data.get_i16());
                }
                
                let num_params = data.get_i16() as usize;
                let mut params = Vec::with_capacity(num_params);
                for _ in 0..num_params {
                    let len = data.get_i32();
                    if len == -1 {
                        params.push(None);
                    } else {
                        let bytes = data.copy_to_bytes(len as usize);
                        params.push(Some(bytes));
                    }
                }

                let num_result_formats = data.get_i16() as usize;
                let mut result_formats = Vec::with_capacity(num_result_formats);
                for _ in 0..num_result_formats {
                    result_formats.push(data.get_i16());
                }

                Ok(Some(FrontendMessage::Bind { 
                    portal, statement, param_formats, params, result_formats 
                }))
            }
            b'E' => { // Execute
                let portal = read_cstring(&mut data)?;
                let max_rows = data.get_i32();
                Ok(Some(FrontendMessage::Execute { portal, max_rows }))
            }
            b'S' => { // Sync
                Ok(Some(FrontendMessage::Sync))
            }
            b'D' => { // Describe
                let kind = data.get_u8();
                let name = read_cstring(&mut data)?;
                Ok(Some(FrontendMessage::Describe { kind, name }))
            }
            b'C' => { // Close
                let kind = data.get_u8();
                let name = read_cstring(&mut data)?;
                Ok(Some(FrontendMessage::Close { kind, name }))
            }
            _ => {
                // Unknown message, skip or error
                 Ok(None) 
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_startup_message_parsing() {
        let mut buf = BytesMut::new();
        // Length 4 + 4(ver) + 5("user\0") + 9("postgres\0") + 1(\0) = 23
        let mut body = BytesMut::new();
        body.put_i32(196608); // Version 3.0
        body.put_slice(b"user\0postgres\0\0");
        
        let len = 4 + body.len() as i32;
        buf.put_i32(len);
        buf.put(body);

        let msg = StartupMessage::parse(&mut buf).unwrap().unwrap();
        match msg {
            StartupMessage::Normal { version, parameters } => {
                assert_eq!(version, 196608);
                assert_eq!(parameters.get("user").map(|s| s.as_str()), Some("postgres"));
            }
            _ => panic!("Expected Normal startup message"),
        }
    }

    #[test]
    fn test_frontend_query_parsing() {
        let mut buf = BytesMut::new();
        let query = "SELECT 1";
        let len = 4 + query.len() as i32 + 1; // +1 for null terminator

        buf.put_u8(b'Q');
        buf.put_i32(len);
        buf.put_slice(query.as_bytes());
        buf.put_u8(0);

        let msg = FrontendMessage::parse(&mut buf).unwrap().unwrap();
        match msg {
            FrontendMessage::Query { query: q } => {
                assert_eq!(q, query);
            }
            _ => panic!("Expected Query message"),
        }
    }

    #[test]
    fn test_backend_datarow_writing() {
        let values = vec![
            Some(Bytes::from_static(b"1")),
            None,
            Some(Bytes::from_static(b"test")),
        ];
        let msg = BackendMessage::DataRow { values };
        
        let mut buf = BytesMut::new();
        msg.write(&mut buf);
        
        assert_eq!(buf[0], b'D');
    }

    #[test]
    fn test_parse_message() {
        let mut buf = BytesMut::new();
        // 'P', len, "name\0", "query\0", num_params(i16), param_oids(i32...)
        // Simple case: name="", query="SELECT 1", 0 params
        let mut body = BytesMut::new();
        body.put_slice(b"\0");
        body.put_slice(b"SELECT 1\0");
        body.put_i16(0);
        
        buf.put_u8(b'P');
        buf.put_i32(4 + body.len() as i32);
        buf.put(body);
        
        let msg = FrontendMessage::parse(&mut buf).unwrap().unwrap();
        match msg {
            FrontendMessage::Parse { name, query, param_types } => {
                assert_eq!(name, "");
                assert_eq!(query, "SELECT 1");
                assert_eq!(param_types.len(), 0);
            },
            _ => panic!("Expected Parse message"),
        }
    }

    #[test]
    fn test_bind_message() {
        let mut buf = BytesMut::new();
        // 'B', len, portal\0, stmt\0, 0 fmt, 0 params, 0 res fmt
        let mut body = BytesMut::new();
        body.put_slice(b"\0"); // portal
        body.put_slice(b"\0"); // stmt
        body.put_i16(0); // param formats
        body.put_i16(0); // params
        body.put_i16(0); // result formats

        buf.put_u8(b'B');
        buf.put_i32(4 + body.len() as i32);
        buf.put(body);
        
        let msg = FrontendMessage::parse(&mut buf).unwrap().unwrap();
        match msg {
            FrontendMessage::Bind { portal, statement, .. } => {
                assert_eq!(portal, "");
                assert_eq!(statement, "");
            },
            _ => panic!("Expected Bind message"),
        }
    }
}
