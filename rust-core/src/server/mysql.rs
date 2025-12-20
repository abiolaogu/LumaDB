use crate::{Database, Document, Value};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn start(db: Arc<Database>, port: u16) -> crate::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.map_err(|e| crate::LumaError::Io(e))?;
    println!("LumaDB MySQL Adapter listening on {}", addr);

    loop {
        let (mut socket, _) = match listener.accept().await {
            Ok(s) => s,
            Err(_) => continue,
        };
        let db = db.clone();
        
        tokio::spawn(async move {
            // 1. Send Initial Handshake Packet (Protocol 10)
            // https://dev.mysql.com/doc/internals/en/connection-phase-packets.html#packet-Protocol::Handshake
            
            // Payload
            let mut handshake = Vec::new();
            handshake.push(10); // Protocol version
            handshake.extend_from_slice(b"5.7.0-LumaDB\0"); // Server version
            handshake.extend_from_slice(&[0, 0, 0, 0]); // Thread ID
            handshake.extend_from_slice(b"12345678\0"); // Salt part 1
            handshake.extend_from_slice(&[0, 0]); // Capabilities (lower)
            handshake.push(0); // Charset
            handshake.extend_from_slice(&[0, 0]); // Status
            handshake.extend_from_slice(&[0, 0]); // Caps (upper)
            handshake.push(0); // Auth plugin data len
            handshake.extend_from_slice(&[0u8; 10]); // Reserved
            handshake.extend_from_slice(b"123456789012\0"); // Salt part 2
            handshake.extend_from_slice(b"mysql_native_password\0"); // Auth plugin name

            // Write Packet Header (Length + SeqId)
            let len = handshake.len() as u32;
            let mut packet = Vec::new();
            packet.push(len as u8);
            packet.push((len >> 8) as u8);
            packet.push((len >> 16) as u8);
            packet.push(0); // SeqId 0
            packet.extend(handshake);

            if let Err(_) = socket.write_all(&packet).await { return; }

            // 2. Read Login Request
            let mut buf = [0u8; 4096]; // Buffer for login packet
            let n = match socket.read(&mut buf).await {
                Ok(n) if n > 0 => n,
                _ => return,
            };
            
            // Assume success for demo (Bypass Auth)
            // 3. Send OK Packet
            // Header
            let ok_len = 7;
            let mut ok_packet = Vec::new();
            ok_packet.push(ok_len);
            ok_packet.push(0);
            ok_packet.push(0);
            ok_packet.push(2); // SeqId 2 (Client sent 1)
            
            // Payload: Header(0x00), AffectedRows(0), LastInsertId(0), Status(0), Warnings(0)
            ok_packet.push(0x00); 
            ok_packet.push(0x00);
            ok_packet.push(0x00);
            ok_packet.extend_from_slice(&[2, 0]); // Server Status (AutoCommit)
            ok_packet.extend_from_slice(&[0, 0]); // Warnings

            if let Err(_) = socket.write_all(&ok_packet).await { return; }

            // 4. Command Loop
            loop {
                // Read Packet Header
                let mut header = [0u8; 4];
                if socket.read_exact(&mut header).await.is_err() { break; }
                
                let len = (header[0] as usize) | ((header[1] as usize) << 8) | ((header[2] as usize) << 16);
                let seq = header[3];

                let mut body = vec![0u8; len];
                if socket.read_exact(&mut body).await.is_err() { break; }

                if body.is_empty() { continue; }
                
                let cmd = body[0];
                match cmd {
                    0x03 => { // COM_QUERY
                        let query = String::from_utf8_lossy(&body[1..]);
                        // Execute query via DB (mock)
                        // Send Text Resultset
                        // 1. Column Count (1)
                        // 2. Column Def
                        // 3. EOF
                        // 4. Row
                        // 5. EOF
                        
                        // For simplicity, just return OK packet for everything (INSERT/UPDATE simulation)
                        // Or if SELECT, empty set.
                        
                        if query.to_uppercase().starts_with("SELECT") {
                             // Mock Column Count: 1
                             write_packet(&mut socket, &[1], seq + 1).await;
                             // Column Def (Dummy)
                             // Catalog(def), Schema(test), Table(t), OrgTable(t), Name(v), OrgName(v), 0x0c(utf8), ...
                             // Simplified packet for "version"
                             // Length-encoded string helpers needed. 
                             // Hardcoding a dummy column def for "version"
                             let col_def = b"\x03def\x04test\x01t\x01t\x07version\x07version\x0c\x3f\x00\xff\xff\xff\x00\xfd\x1f\x00\x00\x00\x00\x00"; 
                             write_packet(&mut socket, col_def, seq + 2).await;
                             
                             // 4. Row: "LumaDB 3.0"
                             // 0x0A (len 10) + "LumaDB 3.0"
                             let row = b"\x0ALumaDB 3.0";
                             write_packet(&mut socket, row, seq + 3).await;
                             
                             // 5. EOF / OK Packet per 5.7+ 
                             // OK packet with EOF flag
                             write_packet(&mut socket, &[0xfe, 0, 0, 0x02, 0], seq + 4).await;

                        } else {
                             // OK Packet for INSERT/UPDATE
                             write_packet(&mut socket, &[0x00, 0, 0, 0x02, 0, 0, 0], seq + 1).await;
                        }
                    },
                    0x01 => { // COM_QUIT
                        break;
                    },
                    _ => {
                        // Error Packet
                        write_packet(&mut socket, &[0xff, 0x19, 0x04, b'#', b'4', b'2', b'0', b'0', b'0', b'U', b'n', b'k', b'n', b'o', b'w', b'n'], seq + 1).await;
                    }
                }
            }
        });
    }
}

async fn write_packet(socket: &mut tokio::net::TcpStream, payload: &[u8], seq: u8) {
    let len = payload.len();
    let mut header = Vec::with_capacity(4 + len);
    header.push(len as u8);
    header.push((len >> 8) as u8);
    header.push((len >> 16) as u8);
    header.push(seq);
    header.extend_from_slice(payload);
    let _ = socket.write_all(&header).await;
}
