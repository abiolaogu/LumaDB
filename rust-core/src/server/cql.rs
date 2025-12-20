use crate::{Database, Document, Value};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn start(db: Arc<Database>, port: u16) -> crate::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.map_err(|e| crate::LumaError::Io(e))?;
    println!("LumaDB Cassandra/Scylla Adapter listening on {}", addr);

    loop {
        let (mut socket, _) = match listener.accept().await {
            Ok(s) => s,
            Err(_) => continue,
        };
        let db = db.clone();
        
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                // Read Frame Header (v4)
                // 1 byte version (Request = 0x04)
                // 1 byte flags
                // 2 bytes stream id
                // 1 byte opcode
                // 4 bytes length
                
                if socket.read_exact(&mut buf[0..9]).await.is_err() { return; }
                
                let version = buf[0];
                let flags = buf[1];
                let stream_id = [buf[2], buf[3]];
                let opcode = buf[4];
                let length = u32::from_be_bytes(buf[5..9].try_into().unwrap()) as usize;
                
                // Read body
                if length > 0 {
                    if socket.read_exact(&mut buf[9..9+length]).await.is_err() { return; }
                }
                
                match opcode {
                    0x01 => { // STARTUP
                        // Respond with READY (0x02)
                        let mut resp = Vec::new();
                        resp.push(0x84); // Response v4
                        resp.push(0); // Flags
                        resp.extend_from_slice(&stream_id);
                        resp.push(0x02); // Opcode READY
                        resp.extend_from_slice(&0u32.to_be_bytes()); // Length 0
                        
                        let _ = socket.write_all(&resp).await;
                    },
                    0x05 => { // OPTIONS
                        // RESPOND SUPPORTED (0x06)
                        // Map<String, List<String>>
                        // For stub: length 0 (empty map) ok?
                         let mut resp = Vec::new();
                        resp.push(0x84); // Response v4
                        resp.push(0); // Flags
                        resp.extend_from_slice(&stream_id);
                        resp.push(0x06); // Opcode SUPPORTED
                        // Body: StringMultimap
                        // [short] n maps
                        // ..
                        // Empty map: 00 00 
                        resp.extend_from_slice(&2u32.to_be_bytes()); // Length 2
                        resp.extend_from_slice(&[0, 0]);
                        
                        let _ = socket.write_all(&resp).await;
                    },
                     0x07 => { // QUERY
                         // Respond with RESULT (0x08) - VOID (0x0001) for now
                        let mut resp = Vec::new();
                        resp.push(0x84); // Response v4
                        resp.push(0); // Flags
                        resp.extend_from_slice(&stream_id);
                        resp.push(0x08); // Opcode RESULT
                        // Body: [int] kind
                        // Kind 1 = Void
                        resp.extend_from_slice(&4u32.to_be_bytes()); // Length 4
                        resp.extend_from_slice(&1u32.to_be_bytes()); // Kind Void
                        
                        let _ = socket.write_all(&resp).await;
                     },
                     _ => {
                         // Ignore or Error
                     }
                }
            }
        });
    }
}
