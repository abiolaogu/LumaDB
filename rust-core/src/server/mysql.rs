use crate::{Database, Result};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::server::translator::Translator;

trait U24Write {
    fn write_u24(&mut self, n: u32);
}

impl U24Write for Vec<u8> {
    fn write_u24(&mut self, n: u32) {
        self.push((n & 0xFF) as u8);
        self.push(((n >> 8) & 0xFF) as u8);
        self.push(((n >> 16) & 0xFF) as u8);
    }
}

pub async fn start(db: Arc<Database>, port: u16) -> Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await.map_err(|e| crate::LumaError::Io(e))?;
    println!("MySQL adapter listening on port {}", port);
    let translator = Arc::new(Translator::new(db));

    loop {
        let (socket, _) = listener.accept().await.map_err(|e| crate::LumaError::Io(e))?;
        let translator = translator.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, translator).await {
                eprintln!("MySQL Connection error: {}", e);
            }
        });
    }
}

async fn handle_connection(mut socket: TcpStream, translator: Arc<Translator>) -> Result<()> {
    // 1. Initial Handshake Packet
    let mut handshake = Vec::new();
    handshake.push(10); // Protocol version
    handshake.extend_from_slice(b"5.7.0-LumaDB\0"); // Server version
    handshake.extend_from_slice(&1u32.to_le_bytes()); // Thread ID
    handshake.extend_from_slice(b"12345678"); // Auth plugin data part 1
    handshake.push(0); // Filter
    
    // Capabilities (ClientLongPassword | ConnectWithDB | Protocol41 | SecureConnection)
    let caps = 0xF7_FFu16; 
    handshake.extend_from_slice(&caps.to_le_bytes());
    handshake.push(33); // Charset (utf8)
    handshake.extend_from_slice(&2u16.to_le_bytes()); // Status flags
    
    handshake.extend_from_slice(&[0u8; 13]); // Filler
    handshake.extend_from_slice(b"123456789012"); // Auth plugin data part 2
    handshake.push(0);
    
    write_packet(&mut socket, 0, &handshake).await?;
    
    // 2. Handshake Response (Client Login)
    let _ = read_packet(&mut socket).await?;
    
    // 3. OK Packet (Auth Success)
    let ok_packet = vec![0, 0, 0, 2, 0, 0, 0];
    write_packet(&mut socket, 2, &ok_packet).await?;
    
    // 4. Command Phase
    let mut seq = 0;
    loop {
        let (seq_id, packet) = match read_packet(&mut socket).await {
            Ok(res) => res,
            Err(_) => break, // Connection closed
        };
        seq = seq_id + 1;
        
        if packet.is_empty() { continue; }
        
        let cmd = packet[0];
        match cmd {
            3 => { // COM_QUERY
                let sql = String::from_utf8_lossy(&packet[1..]);
                println!("MySQL Query: {}", sql);
                
                match translator.execute_sql(&sql).await {
                    Ok(_) => {
                        let ok_packet = vec![0, 0, 0, 2, 0, 0, 0];
                        write_packet(&mut socket, seq, &ok_packet).await?;
                    }
                    Err(e) => {
                         let mut err = vec![0xFF];
                         err.extend_from_slice(&1000u16.to_le_bytes());
                         err.extend_from_slice(b"#HY000");
                         err.extend_from_slice(e.to_string().as_bytes());
                         write_packet(&mut socket, seq, &err).await?;
                    }
                }
            },
            1 => { // COM_QUIT
                break;
            },
            14 => { // COM_PING
                let ok_packet = vec![0, 0, 0, 2, 0, 0, 0];
                 write_packet(&mut socket, seq, &ok_packet).await?;
            }
            _ => {
                println!("Unknown MySQL command: {}", cmd);
                 let mut err = vec![0xFF];
                 err.extend_from_slice(&1000u16.to_le_bytes());
                 err.extend_from_slice(b"#HY000Unsupported command");
                 write_packet(&mut socket, seq, &err).await?;
            }
        }
    }
    Ok(())
}

async fn write_packet(socket: &mut TcpStream, seq: u8, data: &[u8]) -> Result<()> {
    let len = data.len() as u32;
    let mut header = Vec::new();
    header.write_u24(len);
    header.push(seq);
    
    socket.write_all(&header).await.map_err(|e| crate::LumaError::Io(e))?;
    socket.write_all(data).await.map_err(|e| crate::LumaError::Io(e))?;
    Ok(())
}

async fn read_packet(socket: &mut TcpStream) -> Result<(u8, Vec<u8>)> {
    let mut header = [0u8; 4];
    socket.read_exact(&mut header).await.map_err(|e| crate::LumaError::Io(e))?;
    
    let len = (header[0] as u32) | ((header[1] as u32) << 8) | ((header[2] as u32) << 16);
    let seq = header[3];
    
    let mut body = vec![0u8; len as usize];
    socket.read_exact(&mut body).await.map_err(|e| crate::LumaError::Io(e))?;
    
    Ok((seq, body))
}
