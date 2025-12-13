use luma_protocol_core::{ProtocolAdapter, QueryProcessor, Result};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use async_trait::async_trait;
use bytes::BytesMut;

mod protocol;
mod types;
use protocol::{StartupMessage, BackendMessage, FrontendMessage};
use types::{encode_value, infer_oid, Oid};

pub struct PostgresProtocol;

impl PostgresProtocol {
    pub fn new() -> Self {
        Self
    }
}

use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::net::TcpListener;
use tracing::{info, error};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct PostgresConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub max_connections: u32,
    pub ssl_mode: String,
}

pub async fn run(config: PostgresConfig, sem: Arc<Semaphore>) -> Result<(), anyhow::Error> {
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("PostgreSQL listener started on {}", addr);

    loop {
        // Enforce limit
        let permit = sem.clone().acquire_owned().await?;
        
        let (socket, _) = listener.accept().await?;
        
            tokio::spawn(async move {
                let _permit = permit;
                // Instantiate the protocol handler
                let protocol = PostgresProtocol::new();
                let peer_addr = socket.peer_addr().unwrap_or("0.0.0.0:0".parse().unwrap());
                // Mock processor for now
                let processor = Box::new(luma_protocol_core::MockQueryProcessor);
                
                if let Err(e) = protocol.handle_connection(socket, peer_addr, processor).await {
                    error!("Connection error: {}", e);
                }
            });
    }
}

async fn handle_connection(_socket: tokio::net::TcpStream) -> Result<(), anyhow::Error> {
    // Placeholder logic
     Ok(())
}

#[async_trait]
impl ProtocolAdapter for PostgresProtocol {
    fn default_port(&self) -> u16 { 5432 }

    async fn handle_connection(
        &self,
        mut socket: TcpStream,
        _addr: SocketAddr,
        processor: Box<dyn QueryProcessor>, 
    ) -> Result<()> {
        let mut buffer = BytesMut::with_capacity(4096);

        // Handshake
        match protocol::startup::ConnectionStartup::handle_handshake(&mut socket).await {
            Ok(_) => {},
            Err(e) => {
                error!("Handshake failed: {}", e);
                return Err(e);
            }
        }

        // --- Command Phase ---
        loop {
            let n = socket.read_buf(&mut buffer).await?;
            if n == 0 {
                return Ok(());
            }

            while let Some(msg) = protocol::FrontendMessage::parse(&mut buffer)? {
                match msg {
                    protocol::FrontendMessage::Query { query } => {
                        let request = luma_protocol_core::QueryRequest {
                            query,
                            params: vec![],
                        };

                        match processor.process(request).await {
                            Ok(result) => {
                                // Send RowDescription 
                                if let Some(first_row) = result.rows.first() {
                                    let mut fields = Vec::new();
                                    for (i, val) in first_row.iter().enumerate() {
                                        fields.push((format!("col_{}", i), infer_oid(val)));
                                    }
                                    let mut resp = BytesMut::new();
                                    BackendMessage::RowDescription { fields }.write(&mut resp);
                                    socket.write_all(&resp).await?;
                                }

                                // Send DataRows
                                for row in result.rows {
                                    let mut encoded_row = Vec::new();
                                    for val in row {
                                        // Use Format Text (0) for now as we didn't negotiate Binary
                                        encoded_row.push(encode_value(&val, 0)?); 
                                    }
                                    let mut resp = BytesMut::new();
                                    BackendMessage::DataRow { values: encoded_row }.write(&mut resp);
                                    socket.write_all(&resp).await?;
                                }
                                
                                // Send CommandComplete
                                let mut resp = BytesMut::new();
                                BackendMessage::CommandComplete { tag: format!("SELECT {}", result.row_count) }.write(&mut resp);
                                BackendMessage::ReadyForQuery.write(&mut resp);
                                socket.write_all(&resp).await?;
                            }
                            Err(e) => {
                                let mut resp = BytesMut::new();
                                BackendMessage::ErrorResponse {
                                    message: e.to_string(),
                                    code: "XX000".to_string(),
                                }.write(&mut resp);
                                BackendMessage::ReadyForQuery.write(&mut resp);
                                socket.write_all(&resp).await?;
                            }
                        }
                    }
                    protocol::FrontendMessage::Terminate => {
                        return Ok(());
                    }
                    protocol::FrontendMessage::Parse { .. } => {
                        let mut resp = BytesMut::new();
                        BackendMessage::ParseComplete.write(&mut resp);
                        socket.write_all(&resp).await?;
                    }
                    protocol::FrontendMessage::Bind { .. } => {
                        let mut resp = BytesMut::new();
                        BackendMessage::BindComplete.write(&mut resp);
                        socket.write_all(&resp).await?;
                    }
                    protocol::FrontendMessage::Describe { .. } => {
                        // For now just say no data
                        let mut resp = BytesMut::new();
                        BackendMessage::NoData.write(&mut resp);
                        socket.write_all(&resp).await?;
                    }
                    protocol::FrontendMessage::Execute { .. } => {
                        // Mock execution: return standard Select 1 result
                        let mut resp = BytesMut::new();
                        // Values
                        let mut values = Vec::new();
                        values.push(encode_value(&luma_protocol_core::Value::Int32(1), 0)?);
                        
                        BackendMessage::DataRow { values }.write(&mut resp);
                        BackendMessage::CommandComplete { tag: "SELECT 1".to_string() }.write(&mut resp);
                        socket.write_all(&resp).await?;
                    }
                    protocol::FrontendMessage::Sync => {
                        let mut resp = BytesMut::new();
                        BackendMessage::ReadyForQuery.write(&mut resp);
                        socket.write_all(&resp).await?;
                    }
                    protocol::FrontendMessage::Close { .. } => {
                        let mut resp = BytesMut::new();
                        BackendMessage::CloseComplete.write(&mut resp);
                        socket.write_all(&resp).await?;
                    }
                    protocol::FrontendMessage::Flush => {
                         // No-op for now, just ensure data is sent (it is because we await write_all)
                    }
                    protocol::FrontendMessage::CopyData { .. } => {
                        // Ignore
                    }
                }
            }
        }
    }
}
