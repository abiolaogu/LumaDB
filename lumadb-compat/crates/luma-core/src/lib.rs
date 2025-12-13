pub mod types;
pub mod transactions;

use async_trait::async_trait;
pub use types::Value;
pub use transactions::{Transaction, TransactionManager, IsolationLevel, TransactionOptions};
use std::net::SocketAddr;
use thiserror::Error;

/// Core error type for protocol operations
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Protocol Error: {0}")]
    Protocol(String),
    #[error("Authentication Failed: {0}")]
    Auth(String),
    #[error("Internal Error: {0}")]
    Internal(String),
    #[error("Type Conversion Error: {0}")]
    TypeConversion(String),
}


pub type Result<T> = std::result::Result<T, ProtocolError>;

pub mod ir;
pub use ir::*;
pub mod processor;
pub use processor::{QueryProcessor, MockQueryProcessor, QueryRequest, QueryResult};
pub mod remote;
pub use remote::RemoteQueryProcessor;

/// Trait for a protocol adapter (e.g., Postgres, MySQL)
#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    /// The port this protocol listens on by default
    fn default_port(&self) -> u16;

    /// Handle a new connection
    async fn handle_connection(
        &self,
        socket: tokio::net::TcpStream,
        addr: SocketAddr,
        processor: Box<dyn QueryProcessor>,
    ) -> Result<()>;
}
