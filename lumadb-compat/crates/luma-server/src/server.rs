
use crate::config::Config;
use crate::metrics;
use tokio::signal;
use std::sync::Arc;
use warp::Filter;

pub async fn run(config: Config) -> Result<(), anyhow::Error> {
    // Start Metrics
    if config.metrics.enabled {
        let conf = config.metrics.clone();
        tokio::spawn(async move {
            metrics::start_metrics_server(conf.host, conf.port).await;
        });
    }

    // Initialize Core Engine
    println!("Initializing LumaDB v3.0 Core Engine...");
    let storage_path = std::path::PathBuf::from("./data");
    let storage = std::sync::Arc::new(luma_protocol_core::storage::tiering::MultiTierStorage::new(storage_path).await);
    let query_executor = std::sync::Arc::new(luma_protocol_core::query::executor::QueryExecutor::new(storage.clone()));

    // Define HTTP Routes
    let health_route = warp::path("health").map(|| "OK");
    let routes = health_route.with(warp::trace::request());

    // Start PostgreSQL Protocol Server (Port 5432)
    let pg_executor = query_executor.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::protocols::postgres::run(5432, pg_executor).await {
            eprintln!("PostgreSQL Server failed: {}", e);
        }
    });

    // Start Prometheus Protocol Server (Port 9090)
    let prom_executor = query_executor.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::protocols::prometheus::run(9090, prom_executor).await {
            eprintln!("Prometheus Server failed: {}", e);
        }
    });

    // Start OTLP Protocol Server (Port 4317)
    let otlp_executor = query_executor.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::protocols::otlp::run(4317, otlp_executor).await {
            eprintln!("OTLP Server failed: {}", e);
        }
    });

    // Start Built-in Prometheus Scraper (Pull Mode)
    // TODO: Load config from file
    use luma_protocol_core::ingestion::prometheus::{PrometheusScraper, ScraperConfig};
    use std::time::Duration;
    let scraper_config = ScraperConfig {
        global_interval: Duration::from_secs(15),
        global_timeout: Duration::from_secs(10),
        jobs: vec![], // Add jobs here via config
    };
    let scraper = PrometheusScraper::new(scraper_config, storage.metrics.clone());
    tokio::spawn(async move {
        scraper.start().await;
    });

    // Start gRPC Service (Internal)
    let grpc_addr = "[::1]:50051".parse()?;
    
    // Use correct type from luma-core
    use luma_protocol_core::luma::v3::query_service_server::QueryServiceServer;
    
    let grpc_service = QueryServiceServer::new(crate::grpc::GrpcQueryService::new(query_executor.clone()));
    
    println!("LumaDB Server starting...");
    println!("  - HTTP: http://127.0.0.1:{}", config.server.port);
    println!("  - gRPC: {}", grpc_addr);
    println!("  - PostgreSQL: 0.0.0.0:5432");
    println!("  - Prometheus: 0.0.0.0:9090");
    println!("  - OTLP: 0.0.0.0:4317");

    let grpc_server = tonic::transport::Server::builder()
        .add_service(grpc_service)
        .serve(grpc_addr);

    // Start HTTP Server
    let http_server = warp::serve(routes).run(([127, 0, 0, 1], config.server.port));

    // Run concurrently
    let (_, grpc_res) = tokio::join!(http_server, grpc_server);

    if let Err(e) = grpc_res {
        eprintln!("gRPC exited with error: {}", e);
    }

    // Wait for shutdown
    match signal::ctrl_c().await {
        Ok(()) => println!("Shutdown signal received"),
        Err(err) => eprintln!("Signal error: {}", err),
    }

    Ok(())
}
