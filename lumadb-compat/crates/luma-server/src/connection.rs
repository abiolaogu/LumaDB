use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Semaphore, SemaphorePermit};
use crate::config::Config;
use crate::metrics::ACTIVE_CONNECTIONS;

#[derive(Clone)]
pub struct ConnectionManager {
    semaphores: HashMap<String, Arc<Semaphore>>,
}

impl ConnectionManager {
    pub fn new(config: &Config) -> Self {
        let mut semaphores = HashMap::new();

        // Helper to add semaphore for a protocol
        let mut add_sem = |protocol: &str, limit: u32| {
            semaphores.insert(
                protocol.to_string(),
                Arc::new(Semaphore::new(limit as usize)),
            );
        };

        #[cfg(feature = "postgres")]
        if let Some(c) = &config.postgres { add_sem("postgres", c.max_connections); }

        #[cfg(feature = "mysql")]
        if let Some(c) = &config.mysql { add_sem("mysql", c.max_connections); // Assuming MySqlConfig has max_connections
        }

        #[cfg(feature = "cassandra")]
        if let Some(c) = &config.cassandra { add_sem("cassandra", 1000); // Stub: add field to CassandraConfig
        }

        #[cfg(feature = "mongodb")]
        if let Some(c) = &config.mongodb { add_sem("mongodb", c.max_connections); }

        Self { semaphores }
    }

    pub fn get_semaphore(&self, protocol: &str) -> Option<Arc<Semaphore>> {
        self.semaphores.get(protocol).cloned()
    }

    pub async fn acquire(&self, protocol: &str) -> Option<ConnectionPermit> {
        if let Some(sem) = self.semaphores.get(protocol) {
            match sem.acquire().await {
                Ok(permit) => {
                    ACTIVE_CONNECTIONS.with_label_values(&[protocol]).inc();
                    Some(ConnectionPermit {
                        _permit: permit,
                        protocol: protocol.to_string(),
                    })
                }
                Err(_) => None,
            }
        } else {
            // No limit configured or protocol unknown, usually implies unlimited or error.
            // For safety, let's treat unknown as allowed but unmetriced? Or deny?
            // Safer to deny if we want strict enforcement, but for now allow (return None effectively means we need an OptionWrapper)
            // Actually, if we return Option<Permit>, None implies failure to acquire.
            // Let's assume unlimited if not in map? No, strict.
            None
        }
    }
}

pub struct ConnectionPermit<'a> {
    _permit: SemaphorePermit<'a>,
    protocol: String,
}

impl<'a> Drop for ConnectionPermit<'a> {
    fn drop(&mut self) {
        ACTIVE_CONNECTIONS.with_label_values(&[&self.protocol]).dec();
    }
}
