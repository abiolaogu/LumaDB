pub mod postgres;
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

// ... (omitted shared imports) ...

/// Start all protocol listeners
pub async fn start_server(db: Arc<Database>, config: ServerConfig) -> crate::Result<()> {
    let db_pg = db.clone();
    let db_mysql = db.clone();
    let db_mongo = db.clone();
    let db_cql = db.clone();
    let db_redis = db.clone();
    let db_s3 = db.clone();
    let db_aero = db.clone();
    let db_kdb = db.clone();
    let db_http = db.clone();

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

    Ok(())
}
