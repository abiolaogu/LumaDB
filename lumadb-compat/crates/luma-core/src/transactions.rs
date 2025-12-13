use async_trait::async_trait;
use crate::{ProtocolError, Result};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
    Snapshot, // For MVCC
}

#[derive(Debug, Clone)]
pub struct TransactionOptions {
    pub isolation_level: IsolationLevel,
    pub read_only: bool,
}

impl Default for TransactionOptions {
    fn default() -> Self {
        Self {
            isolation_level: IsolationLevel::ReadCommitted,
            read_only: false,
        }
    }
}

/// Handle to an active transaction
#[async_trait]
pub trait Transaction: Send + Sync {
    /// Get the transaction ID
    fn id(&self) -> Uuid;

    /// Commit the transaction
    async fn commit(self: Box<Self>) -> Result<()>;

    /// Rollback the transaction
    async fn rollback(self: Box<Self>) -> Result<()>;
}

/// Interface for managing transactions
#[async_trait]
pub trait TransactionManager: Send + Sync {
    /// Begin a new transaction
    async fn begin(&self, options: TransactionOptions) -> Result<Box<dyn Transaction>>;

    /// Execute a closure within a transaction
    async fn run_in_transaction<F, T>(
        &self,
        options: TransactionOptions,
        f: F,
    ) -> Result<T>
    where
        F: FnOnce(Arc<dyn Transaction>) -> futures::future::BoxFuture<'static, Result<T>> + Send + 'static,
        T: Send + 'static;
}
