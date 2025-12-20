// pub mod postgres;  // Disabled - requires pgwire crate
pub mod mysql;
pub mod mongo;
pub mod cassandra;
pub mod redis;
pub mod s3;
pub mod aerospike;
pub mod kdb;
pub mod http_api;
pub mod translator;
pub mod query;
pub mod prometheus;  // NEW: Full Prometheus drop-in replacement
pub mod influxdb;    // NEW: Full InfluxDB drop-in replacement
pub mod druid;       // NEW: Full Druid drop-in replacement

use std::sync::Arc;
use crate::Database;

/// Server configuration
#[derive(Clone)]
pub struct ServerConfig {
    pub pg_port: u16,
    pub mysql_port: u16,
    pub mongo_port: u16,
    pub cql_port: u16,
    pub prometheus_port: u16,
    pub influxdb_port: u16,
    pub druid_port: u16,  // NEW: Druid port
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            pg_port: 5432,
            mysql_port: 3306,
            mongo_port: 27017,
            cql_port: 9042,
            prometheus_port: 9090,
            influxdb_port: 8086,
            druid_port: 8888,  // NEW: Druid default
        }
    }
}

/// Start all protocol listeners
pub async fn start_server(db: Arc<Database>, config: ServerConfig) -> crate::Result<()> {
    // let db_pg = db.clone();  // Commented - pgwire disabled
    let db_mysql = db.clone();
    let db_mongo = db.clone();
    let db_cql = db.clone();
    let db_redis = db.clone();
    let db_s3 = db.clone();
    let db_aero = db.clone();
    let db_kdb = db.clone();
    let db_http = db.clone();
    let db_prometheus = db.clone();
    let db_influxdb = db.clone();
    let db_druid = db.clone();  // NEW: Druid

    // Postgres - Disabled (pgwire dependency removed)
    // tokio::spawn(async move {
    //     if let Err(e) = postgres::start(db_pg, config.pg_port).await {
    //         eprintln!("Postgres server error: {}", e);
    //     }
    // });

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
    
    // Redis
    tokio::spawn(async move {
        if let Err(e) = redis::start(db_redis, 6379).await {
             eprintln!("Redis server error: {}", e);
        }
    });

    // S3 (MinIO)
    tokio::spawn(async move {
        if let Err(e) = s3::start(db_s3, 9000).await {
             eprintln!("S3 server error: {}", e);
        }
    });

    // Aerospike
    tokio::spawn(async move {
        if let Err(e) = aerospike::start(db_aero, 3000).await {
             eprintln!("Aerospike server error: {}", e);
        }
    });

    // Kdb+
    tokio::spawn(async move {
        if let Err(e) = kdb::start(db_kdb, 5001).await {
             eprintln!("Kdb+ server error: {}", e);
        }
    });
    
    // HTTP API (ClickHouse/Elastic/Druid)
    tokio::spawn(async move {
         // ClickHouse 8123, Elastic 9200
         if let Err(e) = http_api::start(db_http, 8123, 9200).await {
              eprintln!("HTTP API error: {}", e);
         }
    });

    // NEW: Prometheus (drop-in replacement) - Port 9090
    let prom_port = config.prometheus_port;
    tokio::spawn(async move {
        if let Err(e) = prometheus::start(db_prometheus, prom_port).await {
            eprintln!("Prometheus server error: {}", e);
        }
    });

    // NEW: InfluxDB (drop-in replacement) - Port 8086
    let influx_port = config.influxdb_port;
    tokio::spawn(async move {
        if let Err(e) = influxdb::start(db_influxdb, influx_port).await {
            eprintln!("InfluxDB server error: {}", e);
        }
    });

    // NEW: Druid (drop-in replacement) - Port 8888
    let druid_port = config.druid_port;
    tokio::spawn(async move {
        if let Err(e) = druid::start(db_druid, druid_port).await {
            eprintln!("Druid server error: {}", e);
        }
    });

    Ok(())
}
