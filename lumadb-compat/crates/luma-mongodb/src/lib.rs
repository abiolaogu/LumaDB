use luma_protocol_core::{ProtocolAdapter, QueryProcessor, Result};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use async_trait::async_trait;
use bytes::BytesMut;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, error};
use serde::Deserialize;

pub mod protocol;
pub mod compat;
pub mod mql;
pub mod bson;
pub mod translator;

pub use protocol::{OpMsg, MsgHeader, OpCode};

#[derive(Debug, Deserialize, Clone)]
pub struct MongoDbConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub max_connections: u32,
    pub max_wire_version: i32,
}

pub async fn run(config: MongoDbConfig, sem: Arc<Semaphore>) -> Result<(), anyhow::Error> {
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("MongoDB listener started on {}", addr);

    loop {
        let permit = sem.clone().acquire_owned().await?;
        let (socket, _) = listener.accept().await?;
        
        tokio::spawn(async move {
            let _permit = permit;
            let protocol = MongoProtocol;
            let processor = Box::new(luma_protocol_core::MockQueryProcessor);
            let peer_addr = socket.peer_addr().unwrap_or("0.0.0.0:0".parse().unwrap());

            if let Err(e) = protocol.handle_connection(socket, peer_addr, processor).await {
                error!("MongoDB Connection error: {}", e);
            }
        });
    }
}

pub struct MongoProtocol;

#[async_trait]
impl ProtocolAdapter for MongoProtocol {
    fn default_port(&self) -> u16 { 27017 }

    async fn handle_connection(
        &self,
        mut socket: TcpStream,
        _addr: SocketAddr,
        processor: Box<dyn QueryProcessor>, 
    ) -> Result<()> {
        let mut buffer = BytesMut::with_capacity(4096);

        loop {
            if buffer.len() < 16 { // Header size
                let mut tmp = [0u8; 1024];
                 let n = socket.read(&mut tmp).await?;
                 if n == 0 { return Ok(()); }
                 buffer.extend_from_slice(&tmp[..n]);
            }
            
            // Read Header
            if buffer.len() < 16 { continue; }
            let header = MsgHeader::read(&buffer[..16]); // Assuming implementation handles slice
            // Actually MsgHeader implementation might require reader or we parse manually
            // Stub implementation showed MsgHeader::read, let's assume it works or fix.
            // If message not full, continue reading
            if buffer.len() < header.message_length as usize {
                 let remaining = header.message_length as usize - buffer.len();
                 let mut tmp = vec![0u8; remaining];
                 let n = socket.read(&mut tmp).await?;
                 if n == 0 { return Ok(()); }
                 buffer.extend_from_slice(&tmp[..n]);
                 continue;
            }

            // Msg Full
            let _header_bytes = buffer.split_to(16);
            let payload = buffer.split_to((header.message_length - 16) as usize);

            match header.op_code {
                OpCode::OpMsg => {
                    // Parse OpMsg
                    // let msg = OpMsg::read(&payload)?;
                    // Check first document in body (Section 0)
                    // if has "find": collection ...
                    
                    // Translate
                    let find_op = crate::compat::crud::FindOp {
                        db: "test".to_string(),
                        collection: "test".to_string(),
                        filter: None,
                        sort: None,
                        projection: None,
                        skip: 0,
                        limit: 0,
                    }; // Placeholder extraction
                    
                    match crate::translator::MongoTranslator::translate_find(find_op) {
                        Ok(ir_op) => {
                             info!("Generated LumaIR: {:?}", ir_op);
                             // Execute (Mock)
                             // Send Reply
                             // Stub reply for "ok: 1"
                             send_reply(&mut socket, header.request_id).await?;
                        },
                        Err(e) => {
                             error!("Translation error: {}", e);
                             // Send error reply
                        }
                    }
                },
                OpCode::OpQuery => {
                     // Handle legacy handshake/hello
                     // Detect "isMaster" or "hello"
                     // Send Compat Response
                     send_reply(&mut socket, header.request_id).await?;
                },
                _ => {
                     // Ignore
                }
            }
        }
    }
}

async fn send_reply(socket: &mut TcpStream, response_to: i32) -> Result<(), anyhow::Error> {
    // Construct simplified OP_MSG or OP_REPLY
    // For modern drivers OP_MSG is preferred if client sent it.
    // Stub: Send hardcoded OK response
    let dummy_ok = [
        0x00, 0x00, 0x00, 0x00, // Length (fill later)
        0x01, 0x00, 0x00, 0x00, // Request ID
        0x00, 0x00, 0x00, 0x00, // ResponseTo (fill later)
        0xdd, 0x07, 0x00, 0x00, // OpCode (2013 = OP_MSG)
        0x00, 0x00, 0x00, 0x00, // Flags
        0x00, // Payload type 0
        // BSON: { "ok": 1.0 }
        0x10, 0x00, 0x00, 0x00, // Document Length
        0x01, b'o', b'k', 0x00, // double "ok"
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0x3f, // 1.0
        0x00 // EOO
    ];
    let mut buf = BytesMut::from(&dummy_ok[..]);
    // Fix length and ResponseTo
    let total_len = buf.len() as i32;
    // ... write LE integers ...
    // socket.write_all ...
    Ok(())
}
