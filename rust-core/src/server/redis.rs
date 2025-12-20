use crate::{Database, Document, Value};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;

pub async fn start(db: Arc<Database>, port: u16) -> crate::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.map_err(|e| crate::LumaError::Io(e))?;
    println!("LumaDB Redis Adapter (RESP) listening on {}", addr);

    loop {
        let (mut socket, _) = match listener.accept().await {
            Ok(s) => s,
            Err(_) => continue,
        };
        let db = db.clone();
        
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                // Read from socket
                let n = match socket.read(&mut buf).await {
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(_) => return,
                };
                
                // Parse RESP (Simplified)
                // In production, use a proper streaming parser/buffer management.
                // Here we assume command fits in buffer for demo.
                let input = String::from_utf8_lossy(&buf[..n]);
                let parts: Vec<&str> = input.split("\r\n").collect();
                
                // Minimal Parser: Look for "SET", "GET", "DEL", "PING"
                // RESP Arrays start with *
                // Loop through tokens to find Command
                let mut cmd_idx = 0;
                let mut found = false;
                for (i, p) in parts.iter().enumerate() {
                    let p_upper = p.to_uppercase();
                    if p_upper == "SET" || p_upper == "GET" || p_upper == "DEL" || p_upper == "PING" {
                        cmd_idx = i;
                        found = true;
                        break;
                    }
                }
                
                if !found {
                    // Try simple inline protocol
                    if input.trim().eq_ignore_ascii_case("PING") {
                        let _ = socket.write_all(b"+PONG\r\n").await;
                        continue;
                    }
                    // Ignore garbage
                    continue;
                }
                
                let cmd = parts[cmd_idx].to_uppercase();
                
                match cmd.as_str() {
                    "PING" => {
                        let _ = socket.write_all(b"+PONG\r\n").await;
                    }
                    "SET" => {
                        if cmd_idx + 4 < parts.len() {
                            let key = parts[cmd_idx+2];
                            let val = parts[cmd_idx+4];
                            let mut data = HashMap::new();
                            data.insert("value".to_string(), Value::Bytes(val.as_bytes().to_vec()));
                            let doc = Document::with_id(key, data);
                            match db.insert("redis_default", doc).await {
                                Ok(_) => { let _ = socket.write_all(b"+OK\r\n").await; },
                                Err(e) => { let _ = socket.write_all(format!("-ERR {}\r\n", e).as_bytes()).await; }
                            }
                        }
                    }
                    "GET" => {
                        if cmd_idx + 2 < parts.len() {
                            let key = parts[cmd_idx+2];
                            match db.get("redis_default", &key.to_string()).await {
                                Ok(Some(doc)) => {
                                    if let Some(val) = doc.get("value") {
                                        match val {
                                            Value::Bytes(b) => {
                                                let _ = socket.write_all(format!("${}\r\n", b.len()).as_bytes()).await;
                                                let _ = socket.write_all(b).await;
                                                let _ = socket.write_all(b"\r\n").await;
                                            },
                                            Value::String(s) => {
                                                let b = s.as_bytes();
                                                let _ = socket.write_all(format!("${}\r\n", b.len()).as_bytes()).await;
                                                let _ = socket.write_all(b).await;
                                                let _ = socket.write_all(b"\r\n").await;
                                            }
                                            Value::Int(i) => {
                                                let s = i.to_string();
                                                let b = s.as_bytes();
                                                let _ = socket.write_all(format!("${}\r\n", b.len()).as_bytes()).await;
                                                let _ = socket.write_all(b).await;
                                                let _ = socket.write_all(b"\r\n").await;
                                            }
                                            _ => { let _ = socket.write_all(b"$-1\r\n").await; }
                                        }
                                    } else { let _ = socket.write_all(b"$-1\r\n").await; }
                                }
                                _ => { let _ = socket.write_all(b"$-1\r\n").await; }
                            }
                        }
                    }
                    "DEL" => {
                         if cmd_idx + 2 < parts.len() {
                             let key = parts[cmd_idx+2];
                             match db.delete("redis_default", &key.to_string()).await {
                                 Ok(true) => { let _ = socket.write_all(b":1\r\n").await; },
                                 _ => { let _ = socket.write_all(b":0\r\n").await; },
                             }
                        }
                    }
                    "HSET" => {
                        // HSET key field value
                        if cmd_idx + 6 < parts.len() {
                            let key = parts[cmd_idx+2];
                            let field = parts[cmd_idx+4];
                            let val = parts[cmd_idx+6];
                            
                            // Get existing or create new
                            let mut doc = match db.get("redis_default", &key.to_string()).await.unwrap_or(None) {
                                Some(d) => d,
                                None => Document::with_id(key, HashMap::new())
                            };
                            
                            doc.set(field, Value::Bytes(val.as_bytes().to_vec()));
                            db.insert("redis_default", doc).await.ok(); // Update
                            
                            let _ = socket.write_all(b":1\r\n").await;
                        }
                    }
                    "HGET" => {
                        // HGET key field
                        if cmd_idx + 4 < parts.len() {
                            let key = parts[cmd_idx+2];
                            let field = parts[cmd_idx+4];
                             match db.get("redis_default", &key.to_string()).await {
                                Ok(Some(doc)) => {
                                    if let Some(val) = doc.get(field) {
                                         // Reuse logic for returning value
                                         match val {
                                            Value::Bytes(b) => {
                                                let _ = socket.write_all(format!("${}\r\n", b.len()).as_bytes()).await;
                                                let _ = socket.write_all(b).await;
                                                let _ = socket.write_all(b"\r\n").await;
                                            },
                                            _ => { let _ = socket.write_all(b"$-1\r\n").await; }
                                         }
                                    } else { let _ = socket.write_all(b"$-1\r\n").await; }
                                }
                                _ => { let _ = socket.write_all(b"$-1\r\n").await; }
                             }
                        }
                    }
                    "HGETALL" => {
                         if cmd_idx + 2 < parts.len() {
                            let key = parts[cmd_idx+2];
                             match db.get("redis_default", &key.to_string()).await {
                                Ok(Some(doc)) => {
                                    // Return all fields as *N array
                                    let len = doc.data.len() * 2;
                                    let _ = socket.write_all(format!("*{}\r\n", len).as_bytes()).await;
                                    for (k, v) in doc.data {
                                        let _ = socket.write_all(format!("${}\r\n{}\r\n", k.len(), k).as_bytes()).await;
                                        match v {
                                            Value::Bytes(b) => {
                                                 let _ = socket.write_all(format!("${}\r\n", b.len()).as_bytes()).await;
                                                 let _ = socket.write_all(&b).await;
                                                 let _ = socket.write_all(b"\r\n").await;
                                            },
                                            _ => { let _ = socket.write_all(b"$0\r\n\r\n").await; }
                                        }
                                    }
                                }
                                _ => { let _ = socket.write_all(b"*0\r\n").await; }
                             }
                         }
                    }
                    "INCR" => {
                         if cmd_idx + 2 < parts.len() {
                            let key = parts[cmd_idx+2];
                             match db.get("redis_default", &key.to_string()).await {
                                Ok(Some(mut doc)) => {
                                    let new_val = match doc.get("value") {
                                        Some(Value::Int(i)) => i + 1,
                                        Some(Value::String(s)) => s.parse::<i64>().unwrap_or(0) + 1,
                                        _ => 1
                                    };
                                    doc.set("value", Value::Int(new_val));
                                    db.insert("redis_default", doc).await.ok();
                                    let _ = socket.write_all(format!(":{}\r\n", new_val).as_bytes()).await;
                                }
                                None => {
                                    let mut data = HashMap::new();
                                    data.insert("value".to_string(), Value::Int(1));
                                    db.insert("redis_default", Document::with_id(key, data)).await.ok();
                                    let _ = socket.write_all(b":1\r\n").await;
                                }
                                _ => { let _ = socket.write_all(b"-ERR error\r\n").await; }
                             }
                         }
                    }
                    "LPUSH" => {
                         if cmd_idx + 4 < parts.len() {
                            let key = parts[cmd_idx+2];
                            let val = parts[cmd_idx+4];
                             match db.get("redis_default", &key.to_string()).await {
                                Ok(Some(mut doc)) => {
                                    let mut list = match doc.get("value") {
                                        Some(Value::Array(arr)) => arr.clone(),
                                        _ => Vec::new()
                                    };
                                    list.insert(0, Value::String(val.to_string()));
                                    let len = list.len();
                                    doc.set("value", Value::Array(list));
                                    db.insert("redis_default", doc).await.ok();
                                    let _ = socket.write_all(format!(":{}\r\n", len).as_bytes()).await;
                                }
                                None => {
                                    let mut data = HashMap::new();
                                    data.insert("value".to_string(), Value::Array(vec![Value::String(val.to_string())]));
                                    db.insert("redis_default", Document::with_id(key, data)).await.ok();
                                    let _ = socket.write_all(b":1\r\n").await;
                                }
                                _ => { let _ = socket.write_all(b"-ERR error\r\n").await; }
                             }
                         }
                    }
                    _ => {
                        let _ = socket.write_all(b"-ERR unknown command\r\n").await;
                    }
                }
            }
        });
    }
}
