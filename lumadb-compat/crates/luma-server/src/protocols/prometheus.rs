
use std::sync::Arc;
use luma_protocol_core::QueryProcessor;
use warp::Filter;

pub async fn run(
    port: u16, 
    _processor: Arc<dyn QueryProcessor + Send + Sync>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let write_route = warp::post()
        .and(warp::path!("api" / "v1" / "write"))
        .and(warp::body::bytes())
        .map(|_body: bytes::Bytes| {
            // TODO: Decompress Snappy
            // TODO: Decode Protobuf (WriteRequest)
            // TODO: Ingest into storage
            tracing::info!("Received Prometheus Remote Write request");
            warp::reply::with_status("OK", warp::http::StatusCode::OK)
        });

    let addr = ([0, 0, 0, 0], port);
    println!("Prometheus Remote Write Server listening on 0.0.0.0:{}", port);
    
    warp::serve(write_route).run(addr).await;
    
    Ok(())
}
