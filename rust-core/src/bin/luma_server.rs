use luma_core::{Database, Config, server};
use std::sync::Arc;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting LumaDB Multi-Protocol Server...");

    // 1. Initialize Database
    let mut config = Config::default();
    config.data_dir = "./data".into();
    // Ensure data directory exists
    tokio::fs::create_dir_all(&config.data_dir).await?;
    
    let db = Arc::new(Database::open(config).await?);
    println!("Database initialized at ./data");

    // 2. Start Servers
    let server_config = server::ServerConfig {
        pg_port: 5432,
        mysql_port: 3306,
        mongo_port: 27017,
        cql_port: 9042,
    };
    
    server::start_server(db.clone(), server_config).await?;

    // 3. Wait for shutdown
    match signal::ctrl_c().await {
        Ok(()) => {
            println!("Shutting down...");
            db.close().await?;
        },
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        },
    }

    Ok(())
}
