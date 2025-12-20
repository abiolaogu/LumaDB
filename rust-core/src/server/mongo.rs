use crate::{Database, Document, Value};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn start(db: Arc<Database>, port: u16) -> crate::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.map_err(|e| crate::LumaError::Io(e))?;
    println!("LumaDB MongoDB Adapter listening on {}", addr);

    loop {
        let (mut socket, _) = match listener.accept().await {
            Ok(s) => s,
            Err(_) => continue,
        };
        let db = db.clone();
        
        tokio::spawn(async move {
            let mut buf = [0u8; 16384];
            loop {
                // Read MsgHeader
                if socket.read_exact(&mut buf[0..16]).await.is_err() { return; }
                
                // Parse Header
                // int32 messageLength
                // int32 requestID
                // int32 responseTo
                // int32 opCode
                
                let msg_len = i32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
                let request_id = i32::from_le_bytes(buf[4..8].try_into().unwrap());
                // let response_to = i32::from_le_bytes(buf[8..12].try_into().unwrap());
                let op_code = i32::from_le_bytes(buf[12..16].try_into().unwrap());
                
                // Read Body
                if msg_len > 16 {
                    let body_len = msg_len - 16;
                    if socket.read_exact(&mut buf[16..16+body_len]).await.is_err() { return; }
                }

                // Handle OP_MSG (2013) or OP_QUERY (2004 - Legacy)
                if op_code == 2013 { // OP_MSG
                    // Handle "isMaster" / "hello" handshake
                    // BSON parsing is complex without `bson` crate.
                    // Accessing raw bytes to detect command name.
                    // Assuming "isMaster" or "hello" is present.
                    // We'll just construct a generic "ok" response.
                    
                    // Construct Reply (OP_MSG payload)
                    // Section 0: Body (BSON)
                    // { "ok": 1.0, "ismaster": true, ... }
                    
                    // Minimal BSON for { "ok": 1.0, "isMaster": true, "maxWireVersion": 13, "minWireVersion": 0 }
                    // 12 bytes header (len+0+0) + elements + 0
                    // Type 1 (double) "ok" \0 00 00 00 00 00 00 F0 3F (1.0)
                    // Type 8 (bool) "isMaster" \0 01
                    // Type 16 (int32) "maxWireVersion" \0 0D 00 00 00
                    // ...
                    
                    // BSON Stub bytes
                    let bson_data = b"\x3C\x00\x00\x00\x01ok\x00\x00\x00\x00\x00\x00\x00\xf0\x3f\x08isMaster\x00\x01\x10maxWireVersion\x00\x0d\x00\x00\x00\x00";
                    
                    // Response Header
                    let resp_len = 16 + 4 + bson_data.len(); // Header + Body flags + Body
                    let mut resp = Vec::with_capacity(resp_len);
                    resp.extend_from_slice(&(resp_len as i32).to_le_bytes()); // MsgLen
                    resp.extend_from_slice(&request_id.to_le_bytes()); // RequestID (new)
                    resp.extend_from_slice(&request_id.to_le_bytes()); // ResponseTo (matches req)
                    resp.extend_from_slice(&2013i32.to_le_bytes()); // OP_MSG
                    
                    // OP_MSG Body
                    resp.extend_from_slice(&0u32.to_le_bytes()); // Flag bits
                    resp.push(0); // Payload Type 0
                    resp.extend_from_slice(bson_data);
                    
                    let _ = socket.write_all(&resp).await;
                } else if op_code == 2004 { // OP_QUERY (Legacy)
                     // Legacy handshake support...
                     // Send OP_REPLY
                }
            }
        });
    }
}
