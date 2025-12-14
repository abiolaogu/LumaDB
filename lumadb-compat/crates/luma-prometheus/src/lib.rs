
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use prost::Message;
use snap::raw::Decoder;

use luma_protocol_core::{
    ProtocolAdapter, ProtocolError, QueryProcessor, Result,
};

// Include generated protos
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/prometheus.rs"));
}

use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::net::TcpListener;

pub struct PrometheusAdapter;

impl PrometheusAdapter {
    pub fn new() -> Self {
        Self
    }
}

pub struct PrometheusConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

pub async fn run(config: PrometheusConfig, _semaphore: Arc<Semaphore>) -> Result<()> {
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await.map_err(ProtocolError::Io)?;
    
    println!("Prometheus Remote Write Adapter listening on {}", addr);

    loop {
        let (socket, addr) = listener.accept().await.map_err(ProtocolError::Io)?;
        
        let adapter = PrometheusAdapter::new();
        let processor = Box::new(luma_protocol_core::MockQueryProcessor::default()); 
        
        tokio::spawn(async move {
            if let Err(e) = adapter.handle_connection(socket, addr, processor).await {
                eprintln!("Prometheus connection error: {}", e);
            }
        });
    }
}

#[async_trait]
impl ProtocolAdapter for PrometheusAdapter {
    fn default_port(&self) -> u16 {
        9090
    }

    async fn handle_connection(
        &self,
        mut socket: TcpStream,
        _addr: SocketAddr,
        _processor: Box<dyn QueryProcessor>, // In real impl, we'd use this
    ) -> Result<()> {
        let mut buf = vec![0; 8192];
        
        // Simple HTTP parser (Partial)
        // We expect POST /api/v1/write
        let n = match socket.read(&mut buf).await {
            Ok(n) if n == 0 => return Ok(()),
            Ok(n) => n,
            Err(e) => return Err(ProtocolError::Io(e)),
        };
        
        let req_str = String::from_utf8_lossy(&buf[..n]);
        if req_str.starts_with("POST /api/v1/write") {
             // Find body
            if let Some(body_start) = req_str.find("\r\n\r\n") {
                 let body = &buf[body_start+4..n];
                 
                 // Decompress Snappy
                 let mut decoder = Decoder::new();
                 match decoder.decompress_vec(body) {
                     Ok(decompressed) => {
                         // Decode Protobuf
                         match proto::WriteRequest::decode(&*decompressed) {
                             Ok(req) => {
                                 // Process TimeSeries
                                 println!("Received {} timeseries from Prometheus", req.timeseries.len());
                                 
                                 // Respond 204
                                 let response = "HTTP/1.1 204 No Content\r\n\r\n";
                                 socket.write_all(response.as_bytes()).await?;
                             }
                             Err(e) => {
                                 eprintln!("Protobuf decode error: {}", e);
                                 let response = "HTTP/1.1 400 Bad Request\r\n\r\nProtobuf Error";
                                 socket.write_all(response.as_bytes()).await?;
                             }
                         }
                     }
                     Err(e) => {
                         eprintln!("Snappy decompress error: {}", e);
                         let response = "HTTP/1.1 400 Bad Request\r\n\r\nSnappy Error";
                         socket.write_all(response.as_bytes()).await?;
                     }
                 }
            }
        } else {
             let response = "HTTP/1.1 404 Not Found\r\n\r\n";
             socket.write_all(response.as_bytes()).await?;
        }

        Ok(())
    }
}
