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
pub mod cql;
pub mod cluster;
pub mod translator;

pub use protocol::{CassandraProtocol, Frame, Opcode, Compression};

#[derive(Debug, Deserialize, Clone)]
pub struct CassandraConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub native_transport_max_threads: u32,
    pub cluster_name: String,
}

pub async fn run(config: CassandraConfig, sem: Arc<Semaphore>) -> Result<(), anyhow::Error> {
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("Cassandra listener started on {}", addr);

    loop {
        let permit = sem.clone().acquire_owned().await?;
        let (socket, _) = listener.accept().await?;
        
        tokio::spawn(async move {
            let _permit = permit;
            let protocol = CassandraProtocol; // Logic is static methods mostly, but struct exists
            // Mock processor
            let processor = Box::new(luma_protocol_core::MockQueryProcessor);
             // Stub address
            let peer_addr = socket.peer_addr().unwrap_or("0.0.0.0:0".parse().unwrap());

            if let Err(e) = protocol.handle_connection(socket, peer_addr, processor).await {
                error!("Cassandra Connection error: {}", e);
            }
        });
    }
}

#[async_trait]
impl ProtocolAdapter for CassandraProtocol {
    fn default_port(&self) -> u16 { 9042 }

    async fn handle_connection(
        &self,
        mut socket: TcpStream,
        _addr: SocketAddr,
        processor: Box<dyn QueryProcessor>, 
    ) -> Result<()> {
        let mut buffer = BytesMut::with_capacity(4096);
        let mut compression = Compression::None; // Negotiated? Assume None for now

        loop {
            // Read loop
            if buffer.len() == 0 {
                let mut tmp = [0u8; 4096];
                let n = socket.read(&mut tmp).await?;
                if n == 0 { return Ok(()); }
                buffer.extend_from_slice(&tmp[..n]);
            }
            
            while let Some(frame) = CassandraProtocol::read_frame(&mut buffer, compression)? {
                let stream_id = frame.stream_id;
                match frame.opcode {
                    Opcode::Startup => {
                         // Send READY
                         let ready_frame = Frame {
                             version: 0x84, // Response v4
                             flags: 0,
                             stream_id,
                             opcode: Opcode::Ready,
                             body: BytesMut::new(),
                         };
                         let mut resp_buf = BytesMut::new();
                         CassandraProtocol::write_frame(&ready_frame, &mut resp_buf, compression)?;
                         socket.write_all(&resp_buf).await?;
                    },
                    Opcode::Query => {
                         // Parse Body: <query><consistency> (Simplified)
                         // need a cursor on frame.body
                         use bytes::Buf;
                         let mut cursor = frame.body.clone(); // Clone for reading
                         // Long string
                         let len = cursor.get_i32() as usize; 
                         let query_bytes = &cursor[..len];
                         let query_str = String::from_utf8_lossy(query_bytes).to_string();
                         cursor.advance(len);
                         
                         info!("Received CQL: {}", query_str);
                         
                         // Translate
                         match crate::translator::CassandraTranslator::translate(&query_str) {
                             Ok(ir_op) => {
                                 info!("Generated LumaIR: {:?}", ir_op);
                                 // Execute (Mock)
                                 // Send RESULT Void
                                 let mut body = BytesMut::new();
                                 use bytes::BufMut;
                                 body.put_i32(1); // Kind: Void
                                 
                                 let res_frame = Frame {
                                     version: 0x84, 
                                     flags: 0,
                                     stream_id,
                                     opcode: Opcode::Result,
                                     body,
                                 };
                                 let mut resp_buf = BytesMut::new();
                                 CassandraProtocol::write_frame(&res_frame, &mut resp_buf, compression)?;
                                 socket.write_all(&resp_buf).await?;
                             },
                             Err(e) => {
                                  // Send ERROR
                                  let mut body = BytesMut::new();
                                  use bytes::BufMut;
                                  body.put_i32(0x2000); // Syntax Error
                                  let msg = e.to_string();
                                  body.put_u16(msg.len() as u16);
                                  body.put_slice(msg.as_bytes());
                                  
                                  let err_frame = Frame {
                                     version: 0x84,
                                     flags: 0,
                                     stream_id,
                                     opcode: Opcode::Error,
                                     body,
                                  };
                                  let mut resp_buf = BytesMut::new();
                                 CassandraProtocol::write_frame(&err_frame, &mut resp_buf, compression)?;
                                 socket.write_all(&resp_buf).await?;
                             }
                         }
                    },
                    Opcode::Options => {
                         let supported = Frame {
                             version: 0x84,
                             flags: 0,
                             stream_id,
                             opcode: Opcode::Supported,
                             body: BytesMut::new(), // Empty supported list for now
                         };
                         let mut resp_buf = BytesMut::new();
                         CassandraProtocol::write_frame(&supported, &mut resp_buf, compression)?;
                         socket.write_all(&resp_buf).await?;
                    },
                    _ => {
                         // Ignore or Error
                    }
                }
            }
        }
    }
}
