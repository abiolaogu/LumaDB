use crate::config::Config;
use crate::metrics;
use tokio::signal;

use std::sync::Arc;
use tokio::sync::Semaphore;

pub async fn run(config: Config) -> Result<(), anyhow::Error> {
    // Start metrics
    if config.metrics.enabled {
        let conf = config.metrics.clone();
        tokio::spawn(async move {
            metrics::start_metrics_server(conf.host, conf.port).await;
        });
    let manager = crate::connection::ConnectionManager::new(&config);

    // Start Adapters
    #[cfg(feature = "postgres")]
    if let Some(conf) = &config.postgres {
        if conf.enabled {
            println!("Starting PostgreSQL adapter on {}:{}", conf.host, conf.port);
            
            let pg_conf = luma_postgres::PostgresConfig {
                enabled: conf.enabled,
                host: conf.host.clone(),
                port: conf.port,
                max_connections: conf.max_connections,
                ssl_mode: "prefer".to_string(), 
            };
            
            // We need to pass the raw semaphore to the existing adapter run function because it expects Arc<Semaphore>.
            // Since ConnectionManager wraps it, we might need to expose it or refactor the adapter to take a "PermitAcquirer".
            // For now, let's just grab the semaphore from the manager if possible, OR refactor connection manager.
            // Actually simplest is: ConnectionManager logic is nice, but `luma-postgres` `run` expects `Arc<Semaphore>`.
            // Let's expose inner semaphore from manager for now to satisfy the interface we just built.
            // Or better: Let's refactor luma-postgres to NOT take a semaphore, but just run.
            // Wait, the requirement was "Implement ConnectionManager in luma-server". 
            // So `luma-server` should handle the accept loop?
            // `luma_postgres::run` implementation I wrote *contains* the accept loop.
            // So `luma_postgres::run` NEEDS the semaphore.
            
            // Let's assume ConnectionManager can give us the Arc<Semaphore>.
            // I'll update ConnectionManager to have `get_semaphore`.
            if let Some(sem) = manager.get_semaphore("postgres") {
                 tokio::spawn(async move {
                    if let Err(e) = luma_postgres::run(pg_conf, sem).await {
                         eprintln!("Postgres server failed: {}", e);
                    }
                });
            }
        }
    }

    #[cfg(feature = "mysql")]
    if let Some(conf) = &config.mysql {
        if conf.enabled {
            println!("Starting MySQL adapter on {}:{}", conf.host, conf.port);
        }
    }
    
    #[cfg(feature = "cassandra")]
    if let Some(conf) = &config.cassandra {
         if conf.enabled {
             println!("Starting Cassandra adapter on {}:{}", conf.host, conf.port);
         }
    }
    
    #[cfg(feature = "mongodb")]
    if let Some(conf) = &config.mongodb {
         if conf.enabled {
             println!("Starting MongoDB adapter on {}:{}", conf.host, conf.port);
         }
    }

    match signal::ctrl_c().await {
        Ok(()) => {},
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        },
    }
    println!("Shutting down LumaDB server");
    Ok(())
}
