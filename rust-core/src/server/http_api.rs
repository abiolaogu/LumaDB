use crate::Database;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn start(db: Arc<Database>, port_ch: u16, port_es: u16) -> crate::Result<()> {
    let db_ch = db.clone();
    let db_es = db.clone();

    // Spawn ClickHouse Listener
    tokio::spawn(async move {
        let addr = format!("0.0.0.0:{}", port_ch);
        if let Ok(listener) = TcpListener::bind(&addr).await {
             println!("LumaDB ClickHouse Adapter listening on {}", addr);
             loop {
                 if let Ok((mut socket, _)) = listener.accept().await {
                     tokio::spawn(async move {
                         let _ = socket.write_all(b"HTTP/1.1 200 OK\r\n\r\nClickHouse Ready").await;
                     });
                 }
             }
        }
    });

    // Spawn Elasticsearch Listener
    tokio::spawn(async move {
         let addr = format!("0.0.0.0:{}", port_es);
         if let Ok(listener) = TcpListener::bind(&addr).await {
              println!("LumaDB Elasticsearch Adapter listening on {}", addr);
              loop {
                  if let Ok((mut socket, _)) = listener.accept().await {
                      tokio::spawn(async move {
                          let mut buf = vec![0; 4096]; // Buffer for incoming request
                          let n = socket.read(&mut buf).await.unwrap_or(0);

                          if n == 0 {
                              return; // No data read, close connection
                          }

                          let request_str = String::from_utf8_lossy(&buf[..n]);
                          let mut lines = request_str.lines();
                          let request_line = lines.next().unwrap_or("");
                          let parts: Vec<&str> = request_line.split_whitespace().collect();

                          let method = parts.get(0).unwrap_or(&"");
                          let path = parts.get(1).unwrap_or(&"");

                          let headers_end = request_str.find("\r\n\r\n").map(|i| i + 4).unwrap_or(n);

                      // Check for ElasticSearch _search
                     if (path.contains("/_search") || path.contains("sql")) && *method == "POST" {
                         // Parse Body as JSON (Very simple mock parser)
                         let body_str = String::from_utf8_lossy(&buf[headers_end..n]);
                         
                         // Look for "query" and "match"
                         // This is a stub parser. In real generic impl we'd use serde_json
                         let mut response_hits: Vec<String> = Vec::new();
                         
                         // For now, return all docs in "default" if no query, or filter if simple scan
                         // Mock response
                         let response = r#"{
                            "took": 1,
                            "timed_out": false,
                            "_shards": { "total": 1, "successful": 1, "skipped": 0, "failed": 0 },
                            "hits": {
                                "total": { "value": 1, "relation": "eq" },
                                "max_score": 1.0,
                                "hits": [
                                    {
                                        "_index": "default",
                                        "_type": "_doc",
                                        "_id": "1",
                                        "_score": 1.0,
                                        "_source": { "message": "LumaDB Elastic Clone" }
                                    }
                                ]
                            }
                         }"#;
                         let http_response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", response.len(), response);
                         let _ = socket.write_all(http_response.as_bytes()).await;
                         return;
                     }

                     // Fallback for Druid/ClickHouse/Other
                     let _ = socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"status\":\"ok\",\"engine\":\"LumaDB\"}").await;
                      });
                  }
              }
         }
    });

    // Spawn Druid Listener (8082)
    let _db_druid = db_ch.clone(); // Re-use db ref
    tokio::spawn(async move {
         // Harcoded 8082 for Druid
         let addr = "0.0.0.0:8082";
         if let Ok(listener) = TcpListener::bind(&addr).await {
              println!("LumaDB Druid Adapter listening on {}", addr);
              loop {
                  if let Ok((mut socket, _)) = listener.accept().await {
                      tokio::spawn(async move {
                          // Druid Health Check or SQL response
                          let _ = socket.write_all(b"HTTP/1.1 200 OK\r\n\r\n[]").await;
                      });
                  }
              }
         }
    });

    Ok(())
}
