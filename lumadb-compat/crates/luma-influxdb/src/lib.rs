
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use luma_protocol_core::{
    ProtocolAdapter, ProtocolError, QueryProcessor, Result,
};

pub mod parser;
use parser::LineProtocolParser;

use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::net::TcpListener;

pub struct InfluxDBAdapter;

impl InfluxDBAdapter {
    pub fn new() -> Self {
        Self
    }
}

pub struct InfluxDbConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

pub async fn run(config: InfluxDbConfig, _semaphore: Arc<Semaphore>) -> Result<()> {
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await.map_err(ProtocolError::Io)?;
    
    // Check tracing here if available, or println
    println!("InfluxDB Adapter listening on {}", addr);

    loop {
        let (socket, addr) = listener.accept().await.map_err(ProtocolError::Io)?;
        
        // In a real impl, acquire semaphore permit here
        // let _permit = semaphore.acquire().await...
        
        let adapter = InfluxDBAdapter::new();
        let processor = Box::new(luma_protocol_core::MockQueryProcessor::default()); // Placeholder
        
        tokio::spawn(async move {
            if let Err(e) = adapter.handle_connection(socket, addr, processor).await {
                // error logging
                eprintln!("InfluxDB connection error: {}", e);
            }
        });
    }
}

#[async_trait]
impl ProtocolAdapter for InfluxDBAdapter {
    fn default_port(&self) -> u16 {
        8086 // InfluxDB default port
    }

    async fn handle_connection(
        &self,
        mut socket: TcpStream,
        _addr: SocketAddr,
        _processor: Box<dyn QueryProcessor>,
    ) -> Result<()> {
        let mut buf = [0; 4096]; // Buffer for small writes
        // In reality, this is an HTTP endpoint /write
        // For simplicity, we assume we might read raw Line Protocol or HTTP-wrapped.
        // Let's implement a very dumb HTTP parser that looks for "POST /write"
        
        loop {
            let n = match socket.read(&mut buf).await {
                Ok(n) if n == 0 => return Ok(()),
                Ok(n) => n,
                Err(e) => return Err(ProtocolError::Io(e)),
            };

            let req_str = String::from_utf8_lossy(&buf[..n]);
            
            // Basic HTTP check
            if req_str.starts_with("POST /write") || req_str.starts_with("POST /api/v2/write") {
                // Find body (after double newline)
                if let Some(body_start) = req_str.find("\r\n\r\n") {
                    let body = &req_str[body_start+4..];
                    
                    match LineProtocolParser::parse(body) {
                        Ok(_points) => {
                            // Convert points to Insert queries
                            // This is where mapping to LumaIR happens
                            // processor.process(ir).await...
                            
                            // Send HTTP 204 No Content
                            let response = "HTTP/1.1 204 No Content\r\n\r\n";
                            socket.write_all(response.as_bytes()).await?;
                        }
                        Err(e) => {
                             let response = format!("HTTP/1.1 400 Bad Request\r\n\r\nError: {}", e);
                             socket.write_all(response.as_bytes()).await?;
                        }
                    }
                }
            } else {
                 // Unknown request
                 let response = "HTTP/1.1 404 Not Found\r\n\r\n";
                 socket.write_all(response.as_bytes()).await?;
            }
        }
    }
}
