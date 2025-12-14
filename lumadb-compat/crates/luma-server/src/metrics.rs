use prometheus::{Registry, Counter, Histogram, register_counter, register_histogram};
use warp::Filter;
use std::net::SocketAddr;

lazy_static::lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    
    pub static ref CONNECTIONS_TOTAL: Counter = register_counter!(
        "lumadb_connections_total",
        "Total number of connections opened"
    ).unwrap();
    
    pub static ref QUERIES_TOTAL: Counter = register_counter!(
        "lumadb_queries_total",
        "Total number of queries executed"
    ).unwrap();

    pub static ref ACTIVE_CONNECTIONS: prometheus::GaugeVec = prometheus::register_gauge_vec!(
        "lumadb_active_connections",
        "Current number of active connections per protocol",
        &["protocol"]
    ).unwrap();

    pub static ref QUERY_DURATION_SECONDS: prometheus::HistogramVec = prometheus::register_histogram_vec!(
        "lumadb_query_duration_seconds",
        "Query execution latency distribution",
        &["protocol", "query_type"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    ).unwrap();
}

pub async fn start_metrics_server(host: String, port: u16) {
    let metrics_route = warp::path("metrics").map(|| {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    });
    
    let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
    println!("Starting metrics server on http://{}/metrics", addr);
    warp::serve(metrics_route).run(addr).await;
}
