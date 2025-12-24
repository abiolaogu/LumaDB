//! LumaDB Vector Database Compatibility Layer
//!
//! This crate provides drop-in replacement compatibility for popular vector databases:
//! - **Qdrant**: Full REST API compatibility
//! - **Pinecone**: Full REST API compatibility
//! - **MongoDB Atlas Vector Search**: Wire protocol with $vectorSearch support
//!
//! # Usage
//!
//! ```rust,ignore
//! use lumadb_compat::{QdrantServer, PineconeServer, MongoDBServer};
//!
//! // Start Qdrant-compatible server on port 6333
//! let qdrant = QdrantServer::new(storage.clone()).bind("0.0.0.0:6333");
//!
//! // Start Pinecone-compatible server on port 8081
//! let pinecone = PineconeServer::new(storage.clone()).bind("0.0.0.0:8081");
//!
//! // Start MongoDB-compatible server on port 27017
//! let mongodb = MongoDBServer::new(storage.clone()).bind("0.0.0.0:27017");
//! ```

#![warn(clippy::all)]
#![allow(clippy::module_name_repetitions)]

pub mod qdrant;
pub mod pinecone;
pub mod mongodb;
pub mod migration;

pub use qdrant::QdrantServer;
pub use pinecone::PineconeServer;
pub use mongodb::MongoDBServer;
pub use migration::{MigrationTool, MigrationSource};

use thiserror::Error;

/// Compatibility layer errors
#[derive(Error, Debug)]
pub enum CompatError {
    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Vector dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Network error: {0}")]
    Network(String),
}

impl From<lumadb_common::error::Error> for CompatError {
    fn from(e: lumadb_common::error::Error) -> Self {
        CompatError::Storage(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CompatError>;
