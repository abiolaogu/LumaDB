use crate::{Database, Result};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use byteorder::{LittleEndian, ByteOrder};
use crate::server::translator::Translator;
use bson::Document;

pub async fn start(db: Arc<Database>, port: u16) -> Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await.map_err(|e| crate::LumaError::Io(e))?;
    println!("MongoDB adapter listening on port {}", port);
    let translator = Arc::new(Translator::new(db));

    loop {
        let (socket, _) = listener.accept().await.map_err(|e| crate::LumaError::Io(e))?;
        let translator = translator.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, translator).await {
                eprintln!("Mongo Connection error: {}", e);
            }
        });
    }
}

use crate::types::Value;
use bson::Bson;

async fn handle_connection(mut socket: TcpStream, translator: Arc<Translator>) -> Result<()> {
    loop {
        // Read Msg Header (16 bytes)
        let mut header_buf = [0u8; 16];
        if socket.read_exact(&mut header_buf).await.is_err() {
            break;
        }
        
        let msg_len = LittleEndian::read_i32(&header_buf[0..4]);
        let request_id = LittleEndian::read_i32(&header_buf[4..8]);
        let _response_to = LittleEndian::read_i32(&header_buf[8..12]);
        let opcode = LittleEndian::read_i32(&header_buf[12..16]);
        
        let body_len = msg_len - 16;
        let mut body = vec![0u8; body_len as usize];
        socket.read_exact(&mut body).await.map_err(|e| crate::LumaError::Io(e))?;
        
        if opcode == 2013 { // OP_MSG
            // Parse sections
            let _flags = LittleEndian::read_u32(&body[0..4]);
            
            // Section kind (byte)
            let section_kind = body[4];
            if section_kind == 0 {
                let mut reader = &body[5..]; 
                if let Ok(doc) = Document::from_reader(&mut reader) {
                     println!("Mongo Command: {:?}", doc);
                     
                     let reply_doc = if doc.contains_key("hello") || doc.contains_key("isMaster") {
                         bson::doc! {
                             "helloOk": true,
                             "isWritablePrimary": true,
                             "maxBsonObjectSize": 16777216,
                             "maxMessageSizeBytes": 48000000,
                             "maxWriteBatchSize": 100000,
                             "localTime": bson::DateTime::now(),
                             "minWireVersion": 0,
                             "maxWireVersion": 13,
                             "ok": 1.0
                         }
                     } else {
                         // Execute command
                         match translator.execute_mongo(doc).await {
                             Ok(res) => res,
                             Err(e) => bson::doc! { "ok": 0.0, "errmsg": e.to_string() }
                         }
                     };
                     
                     send_op_msg(&mut socket, request_id, reply_doc).await?;
                }
            }
        } else if opcode == 2004 { // OP_QUERY
             let reply = bson::doc! { "ok": 1.0 };
             send_op_reply(&mut socket, request_id, reply).await?;
        }
    }
    Ok(())
}


async fn send_op_msg(socket: &mut TcpStream, response_to: i32, doc: Document) -> Result<()> {
    let mut buf = Vec::new();
    // Placeholder header
    buf.extend_from_slice(&[0; 16]);
    
    // Body flags (0)
    buf.extend_from_slice(&0u32.to_le_bytes());
    
    // Section 0 (Body)
    buf.push(0);
    doc.to_writer(&mut buf).unwrap();
    
    let total_len = buf.len() as i32;
    // Fill header
    let mut header = &mut buf[0..16];
    LittleEndian::write_i32(&mut header[0..4], total_len);
    LittleEndian::write_i32(&mut header[4..8], 0); // RequestID
    LittleEndian::write_i32(&mut header[8..12], response_to);
    LittleEndian::write_i32(&mut header[12..16], 2013); // OP_MSG
    
    socket.write_all(&buf).await.map_err(|e| crate::LumaError::Io(e))?;
    Ok(())
}

async fn send_op_reply(socket: &mut TcpStream, response_to: i32, doc: Document) -> Result<()> {
    let mut buf = Vec::new();
    // Header
    buf.extend_from_slice(&[0; 16]);
    
    // OP_REPLY fields
    buf.extend_from_slice(&0u32.to_le_bytes()); // flags
    buf.extend_from_slice(&0u64.to_le_bytes()); // cursorID
    buf.extend_from_slice(&0u32.to_le_bytes()); // startingFrom
    buf.extend_from_slice(&1u32.to_le_bytes()); // numberReturned
    
    doc.to_writer(&mut buf).unwrap();
    
    let total_len = buf.len() as i32;
    let mut header = &mut buf[0..16];
    LittleEndian::write_i32(&mut header[0..4], total_len);
    LittleEndian::write_i32(&mut header[4..8], 0);
    LittleEndian::write_i32(&mut header[8..12], response_to);
    LittleEndian::write_i32(&mut header[12..16], 1); // OP_REPLY
    
    socket.write_all(&buf).await.map_err(|e| crate::LumaError::Io(e))?;
    Ok(())
}
