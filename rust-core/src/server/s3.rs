use crate::{Database, Document, Value};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;

pub async fn start(db: Arc<Database>, port: u16) -> crate::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    if let Ok(listener) = TcpListener::bind(&addr).await {
         println!("LumaDB MinIO (S3) Adapter listening on {}", addr);
         loop {
             if let Ok((mut socket, _)) = listener.accept().await {
                 let db = db.clone();
                 tokio::spawn(async move {
                     let mut buf = [0u8; 65536]; // Larger buffer for S3 uploads
                     let n = match socket.read(&mut buf).await {
                        Ok(n) if n > 0 => n,
                        _ => return,
                     };
                     
                     // Parse HTTP Header
                     // PUT /bucket/key HTTP/1.1
                     // Host: localhost:9000
                     // ...
                     // \r\n\r\nBody
                     
                     // Inefficient manual parse for demo:
                     // Find double CRLF
                     let mut headers_end = 0;
                     for i in 0..n-3 {
                         if &buf[i..i+4] == b"\r\n\r\n" {
                             headers_end = i+4;
                             break;
                         }
                     }
                     
                     if headers_end == 0 { return; } // Invalid
                     
                     let header_str = String::from_utf8_lossy(&buf[..headers_end]);
                     let mut lines = header_str.lines();
                     let request_line = lines.next().unwrap_or("");
                     let parts: Vec<&str> = request_line.split_whitespace().collect();
                     
                     if parts.len() < 2 { return; }
                     let method = parts[0];
                     let path = parts[1]; // /bucket/key
                     
                     // Extract bucket and key from path
                     let path_parts: Vec<&str> = path.trim_start_matches('/').splitn(2, '/').collect();
                     if path_parts.len() < 2 {
                          // List Buckets (Root)
                           if path == "/" && method == "GET" {
                               let response_body = r#"<?xml version="1.0" encoding="UTF-8"?><ListAllMyBucketsResult><Buckets><Bucket><Name>luma-data</Name><CreationDate>2024-01-01T00:00:00.000Z</CreationDate></Bucket></Buckets></ListAllMyBucketsResult>"#;
                               let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/xml\r\nContent-Length: {}\r\n\r\n{}", response_body.len(), response_body);
                               let _ = socket.write_all(response.as_bytes()).await;
                           }
                           return;
                     }
                     
                     let bucket = path_parts[0];
                     let key = path_parts[1];
                     
                     match method {
                         "PUT" => {
                             let body_len = n - headers_end;
                             let body = buf[headers_end..n].to_vec();
                             let mut data = HashMap::new();
                             data.insert("content".to_string(), Value::Bytes(body));
                             data.insert("content_type".to_string(), Value::String("application/octet-stream".into()));
                             data.insert("size".to_string(), Value::Int(body_len as i64));
                             let doc = Document::with_id(key, data);
                             match db.insert(bucket, doc).await {
                                 Ok(_) => { let _ = socket.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await; },
                                 Err(_) => { let _ = socket.write_all(b"HTTP/1.1 500 Internal Server Error\r\n\r\n").await; }
                             }
                         }
                         "GET" => {
                             // Check if List Objects (key empty or special params?) 
                             // Simplified: If key is "list" or we used logic before?
                             // path_parts logic handles simple keys. 
                             // If client requests GET /bucket/, key will be empty or handle above.
                             // Assuming standard GET object here.
                             match db.get(bucket, &key.to_string()).await {
                                 Ok(Some(doc)) => {
                                     if let Some(content) = doc.get("content").and_then(|v| v.as_bytes()) {
                                          let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\n\r\n", content.len());
                                          let _ = socket.write_all(response.as_bytes()).await;
                                          let _ = socket.write_all(content).await;
                                     } else { let _ = socket.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").await; }
                                 },
                                 _ => { let _ = socket.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").await; }
                             }
                         }
                         "HEAD" => {
                             match db.get(bucket, &key.to_string()).await {
                                 Ok(Some(doc)) => {
                                     let size = doc.get("size").and_then(|v| v.as_i64()).unwrap_or(0);
                                     let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\n\r\n", size);
                                     let _ = socket.write_all(response.as_bytes()).await;
                                 },
                                 _ => { let _ = socket.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").await; }
                             }
                         }
                         "DELETE" => {
                             match db.delete(bucket, &key.to_string()).await {
                                 Ok(_) => { let _ = socket.write_all(b"HTTP/1.1 204 No Content\r\n\r\n").await; },
                                 Err(_) => { let _ = socket.write_all(b"HTTP/1.1 500 Internal Server Error\r\n\r\n").await; }
                             }
                         }
                         _ => {
                              let _ = socket.write_all(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n").await;
                         }
                     }
                 });
             }
         }
    }
    Ok(())
}
