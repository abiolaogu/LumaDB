pub mod postgres;
pub mod mysql;
pub mod mongo;
pub mod cassandra;
pub mod translator;
pub mod query;

use crate::Database;
use std::sync::Arc;

/// Protocol server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub pg_port: u16,
    pub mysql_port: u16,
    pub mongo_port: u16,
    pub cql_port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            pg_port: 5432,
            mysql_port: 3306,
            mongo_port: 27017,
            cql_port: 9042,
        }
    }
}

/// Start all protocol listeners
pub async fn start_server(db: Arc<Database>, config: ServerConfig) -> crate::Result<()> {
    let db_pg = db.clone();
    let db_mysql = db.clone();
    let db_mongo = db.clone();
    let db_cql = db.clone();

    // Spawn independent tasks for each protocol
    
    // Postgres
    tokio::spawn(async move {
        if let Err(e) = postgres::start(db_pg, config.pg_port).await {
            eprintln!("Postgres server error: {}", e);
        }
    });

    // MySQL
    tokio::spawn(async move {
        if let Err(e) = mysql::start(db_mysql, config.mysql_port).await {
            eprintln!("MySQL server error: {}", e);
        }
    });

    // MongoDB
    tokio::spawn(async move {
        if let Err(e) = mongo::start(db_mongo, config.mongo_port).await {
            eprintln!("MongoDB server error: {}", e);
        }
    });

    // Cassandra
    tokio::spawn(async move {
        if let Err(e) = cassandra::start(db_cql, config.cql_port).await {
            eprintln!("Cassandra server error: {}", e);
        }
    });

    Ok(())
}
