use crate::{Database, Result};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use byteorder::{BigEndian, ByteOrder};
use crate::server::translator::Translator;

pub async fn start(db: Arc<Database>, port: u16) -> Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await.map_err(|e| crate::LumaError::Io(e))?;
    println!("Cassandra adapter listening on port {}", port);
    let translator = Arc::new(Translator::new(db));

    loop {
        let (socket, _) = listener.accept().await.map_err(|e| crate::LumaError::Io(e))?;
        let translator = translator.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, translator).await {
                eprintln!("Cassandra Connection error: {}", e);
            }
        });
    }
}

async fn handle_connection(mut socket: TcpStream, translator: Arc<Translator>) -> Result<()> {
    loop {
        // Frame Header: 9 bytes
        let mut header = [0u8; 9];
        if socket.read_exact(&mut header).await.is_err() {
            break;
        }
        
        let stream = BigEndian::read_i16(&header[2..4]);
        let opcode = header[4];
        let length = BigEndian::read_u32(&header[5..9]) as usize;
        
        let mut body = vec![0u8; length];
        socket.read_exact(&mut body).await.map_err(|e| crate::LumaError::Io(e))?;
        
        match opcode {
            0x01 => { // STARTUP
                // Just send READY
                send_frame(&mut socket, stream, 0x02, &[]).await?;
            },
            0x07 => { // QUERY
                let query_len = BigEndian::read_u32(&body[0..4]) as usize;
                let query_str = String::from_utf8_lossy(&body[4..4+query_len]);
                
                println!("CQL Query: {}", query_str);
                
                match translator.execute_sql(&query_str).await {
                     Ok(_) => {
                         // Send RESULT (Void - 0x0001)
                         let mut res_body = Vec::new();
                         res_body.extend_from_slice(&1i32.to_be_bytes());
                         send_frame(&mut socket, stream, 0x08, &res_body).await?;
                     }
                     Err(e) => {
                         // ERROR
                         let mut err_body = Vec::new();
                         err_body.extend_from_slice(&0x2000i32.to_be_bytes()); // Syntax Error
                         let msg = e.to_string();
                         err_body.extend_from_slice(&(msg.len() as u16).to_be_bytes());
                         err_body.extend_from_slice(msg.as_bytes());
                         send_frame(&mut socket, stream, 0x00, &err_body).await?;
                     }
                }
            },
            0x09 => { // PREPARE
                 let mut err_body = Vec::new();
                 err_body.extend_from_slice(&0x000Ai32.to_be_bytes()); // Protocol error
                 let msg = "Prepare not supported";
                 err_body.extend_from_slice(&(msg.len() as u16).to_be_bytes());
                 err_body.extend_from_slice(msg.as_bytes());
                 send_frame(&mut socket, stream, 0x00, &err_body).await?;
            }
            0x15 => { // OPTIONS
                let mut body = Vec::new();
                body.extend_from_slice(&0u16.to_be_bytes()); // 0 items
                send_frame(&mut socket, stream, 0x06, &body).await?;
            }
            _ => {
                println!("Unknown CQL Opcode: {:02X}", opcode);
            }
        }
    }
    Ok(())
}

async fn send_frame(socket: &mut TcpStream, stream: i16, opcode: u8, body: &[u8]) -> Result<()> {
    let mut header = Vec::new();
    header.push(0x84); // Response v4
    header.push(0); // Flags
    header.extend_from_slice(&stream.to_be_bytes());
    header.push(opcode);
    header.extend_from_slice(&(body.len() as u32).to_be_bytes());
    
    socket.write_all(&header).await.map_err(|e| crate::LumaError::Io(e))?;
    socket.write_all(body).await.map_err(|e| crate::LumaError::Io(e))?;
    Ok(())
}
