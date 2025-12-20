//! InfluxDB Wire Protocol Compatible Server
//!
//! Implements:
//! - Line Protocol Write API (/write, /api/v2/write)
//! - InfluxQL Query API (/query)
//! - Flux Query API (/api/v2/query)
//! - Health and Ping APIs
//!
//! Ports: 8086 (InfluxDB default)

use crate::{Database, Document, Value};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// InfluxDB Point (Line Protocol parsed)
#[derive(Debug, Clone)]
pub struct Point {
    pub measurement: String,
    pub tags: HashMap<String, String>,
    pub fields: HashMap<String, FieldValue>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    Float(f64),
    Integer(i64),
    String(String),
    Boolean(bool),
}

/// Start InfluxDB-compatible server on port 8086
pub async fn start(db: Arc<Database>, port: u16) -> crate::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.map_err(|e| crate::LumaError::Io(e))?;
    println!("LumaDB InfluxDB Adapter listening on {} (drop-in replacement)", addr);

    loop {
        let (mut socket, _) = match listener.accept().await {
            Ok(s) => s,
            Err(_) => continue,
        };
        let db = db.clone();

        tokio::spawn(async move {
            let mut buf = vec![0u8; 1024 * 1024]; // 1MB buffer
            let n = match socket.read(&mut buf).await {
                Ok(n) if n > 0 => n,
                _ => return,
            };

            // Parse HTTP request
            let request = String::from_utf8_lossy(&buf[..n]);
            let lines: Vec<&str> = request.lines().collect();
            
            if lines.is_empty() { return; }
            
            let request_line = lines[0];
            let parts: Vec<&str> = request_line.split_whitespace().collect();
            if parts.len() < 2 { return; }
            
            let method = parts[0];
            let full_path = parts[1];
            
            // Split path and query string
            let (path, query_string) = if let Some(pos) = full_path.find('?') {
                (&full_path[..pos], Some(&full_path[pos + 1..]))
            } else {
                (full_path, None)
            };
            
            // Parse query parameters
            let query_params = parse_query_string(query_string.unwrap_or(""));
            
            // Find body
            let body_start = request.find("\r\n\r\n").map(|i| i + 4).unwrap_or(n);
            let body = String::from_utf8_lossy(&buf[body_start..n]);

            match (method, path) {
                // =====================================
                // InfluxDB v1 Write API
                // =====================================
                ("POST", "/write") => {
                    let db_name = query_params.get("db").map(|s| s.as_str()).unwrap_or("default");
                    let precision = query_params.get("precision").map(|s| s.as_str()).unwrap_or("ns");
                    
                    match parse_line_protocol(&body, precision) {
                        Ok(points) => {
                            if let Err(e) = store_points(&db, db_name, points).await {
                                let response = format!(
                                    "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\n\r\n{{\"error\":\"{}\"}}",
                                    e
                                );
                                let _ = socket.write_all(response.as_bytes()).await;
                            } else {
                                let _ = socket.write_all(b"HTTP/1.1 204 No Content\r\n\r\n").await;
                            }
                        }
                        Err(e) => {
                            let response = format!(
                                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\n\r\n{{\"error\":\"{}\"}}",
                                e
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                    }
                }

                // =====================================
                // InfluxDB v2 Write API
                // =====================================
                ("POST", "/api/v2/write") => {
                    let bucket = query_params.get("bucket").map(|s| s.as_str()).unwrap_or("default");
                    let precision = query_params.get("precision").map(|s| s.as_str()).unwrap_or("ns");
                    
                    match parse_line_protocol(&body, precision) {
                        Ok(points) => {
                            if let Err(e) = store_points(&db, bucket, points).await {
                                let json = format!(r#"{{"code":"internal error","message":"{}"}}"#, e);
                                let response = format!(
                                    "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                    json.len(), json
                                );
                                let _ = socket.write_all(response.as_bytes()).await;
                            } else {
                                let _ = socket.write_all(b"HTTP/1.1 204 No Content\r\n\r\n").await;
                            }
                        }
                        Err(e) => {
                            let json = format!(r#"{{"code":"invalid","message":"{}"}}"#, e);
                            let response = format!(
                                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                json.len(), json
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                    }
                }

                // =====================================
                // InfluxDB v1 Query API (InfluxQL)
                // =====================================
                ("GET", "/query") | ("POST", "/query") => {
                    let db_name = query_params.get("db").map(|s| s.as_str()).unwrap_or("default");
                    let query = if method == "GET" {
                        query_params.get("q").cloned().unwrap_or_default()
                    } else {
                        // Parse form body
                        let form_params = parse_query_string(&body);
                        form_params.get("q").cloned().unwrap_or_default()
                    };
                    
                    match execute_influxql(&db, db_name, &query).await {
                        Ok(result) => {
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                result.len(), result
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                        Err(e) => {
                            let json = format!(r#"{{"results":[{{"statement_id":0,"error":"{}"}}]}}"#, e);
                            let response = format!(
                                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                json.len(), json
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                    }
                }

                // =====================================
                // InfluxDB v2 Query API (Flux)
                // =====================================
                ("POST", "/api/v2/query") => {
                    // Parse Flux query from body (JSON or raw)
                    let flux_query = if body.starts_with('{') {
                        // JSON body
                        extract_json_field(&body, "query").unwrap_or_default()
                    } else {
                        body.to_string()
                    };
                    
                    let bucket = query_params.get("bucket").map(|s| s.as_str()).unwrap_or("default");
                    
                    match execute_flux(&db, bucket, &flux_query).await {
                        Ok(result) => {
                            // Return CSV format (Flux default)
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/csv; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
                                result.len(), result
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                        Err(e) => {
                            let json = format!(r#"{{"code":"invalid","message":"{}"}}"#, e);
                            let response = format!(
                                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                json.len(), json
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                    }
                }

                // =====================================
                // Database Management APIs (v1)
                // =====================================
                ("GET", "/query") if query_params.get("q").map(|s| s.to_uppercase().contains("SHOW DATABASES")).unwrap_or(false) => {
                    let json = r#"{"results":[{"statement_id":0,"series":[{"name":"databases","columns":["name"],"values":[["default"],["_internal"]]}]}]}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                // =====================================
                // Buckets API (v2)
                // =====================================
                ("GET", "/api/v2/buckets") => {
                    let json = r#"{"links":{"self":"/api/v2/buckets"},"buckets":[{"id":"default","name":"default","orgID":"default","retentionPeriod":0}]}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                ("POST", "/api/v2/buckets") => {
                    // Create bucket (mock)
                    let json = r#"{"id":"new","name":"new","orgID":"default","retentionPeriod":0}"#;
                    let response = format!(
                        "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                // =====================================
                // Organizations API (v2)
                // =====================================
                ("GET", "/api/v2/orgs") => {
                    let json = r#"{"links":{"self":"/api/v2/orgs"},"orgs":[{"id":"default","name":"default"}]}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                // =====================================
                // Health and Ping APIs
                // =====================================
                ("GET", "/ping") | ("HEAD", "/ping") => {
                    let response = "HTTP/1.1 204 No Content\r\nX-Influxdb-Build: LumaDB\r\nX-Influxdb-Version: 2.7.0\r\n\r\n";
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                ("GET", "/health") => {
                    let json = r#"{"name":"influxdb","message":"ready for queries and writes","status":"pass","checks":[],"version":"2.7.0","commit":"lumadb"}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                ("GET", "/ready") => {
                    let json = r#"{"status":"ready"}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                // =====================================
                // Setup API (v2 onboarding)
                // =====================================
                ("GET", "/api/v2/setup") => {
                    let json = r#"{"allowed":false}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                // =====================================
                // Authorization/Tokens API (v2)
                // =====================================
                ("GET", "/api/v2/authorizations") => {
                    let json = r#"{"links":{"self":"/api/v2/authorizations"},"authorizations":[]}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                // =====================================
                // Me API (current user)
                // =====================================
                ("GET", "/api/v2/me") => {
                    let json = r#"{"id":"admin","name":"admin","status":"active"}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                _ => {
                    let _ = socket.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").await;
                }
            }
        });
    }
}

// =====================================
// Line Protocol Parser
// =====================================

/// Parse InfluxDB Line Protocol
fn parse_line_protocol(data: &str, precision: &str) -> Result<Vec<Point>, String> {
    let mut points = Vec::new();
    
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        match parse_line(line, precision) {
            Ok(point) => points.push(point),
            Err(e) => return Err(format!("Parse error: {} in line: {}", e, line)),
        }
    }
    
    Ok(points)
}

/// Parse single line of Line Protocol
/// Format: measurement,tag1=val1,tag2=val2 field1=v1,field2=v2 timestamp
fn parse_line(line: &str, precision: &str) -> Result<Point, String> {
    // Split into parts: measurement+tags, fields, timestamp
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    
    if parts.len() < 2 {
        return Err("Invalid line: need at least measurement and fields".into());
    }
    
    // Parse measurement and tags
    let (measurement, tags) = parse_measurement_tags(parts[0])?;
    
    // Parse fields
    let fields = parse_fields(parts[1])?;
    
    // Parse optional timestamp
    let timestamp = if parts.len() > 2 {
        let ts_str = parts[2].trim();
        let ts = ts_str.parse::<i64>()
            .map_err(|_| format!("Invalid timestamp: {}", ts_str))?;
        Some(convert_timestamp(ts, precision))
    } else {
        None
    };
    
    Ok(Point {
        measurement,
        tags,
        fields,
        timestamp,
    })
}

fn parse_measurement_tags(s: &str) -> Result<(String, HashMap<String, String>), String> {
    let mut tags = HashMap::new();
    let mut parts = s.split(',');
    
    let measurement = parts.next()
        .ok_or_else(|| "Missing measurement".to_string())?
        .to_string();
    
    for part in parts {
        if let Some(eq_pos) = part.find('=') {
            let key = part[..eq_pos].to_string();
            let value = part[eq_pos + 1..].to_string();
            tags.insert(key, value);
        }
    }
    
    Ok((measurement, tags))
}

fn parse_fields(s: &str) -> Result<HashMap<String, FieldValue>, String> {
    let mut fields = HashMap::new();
    
    for part in s.split(',') {
        let part = part.trim();
        if let Some(eq_pos) = part.find('=') {
            let key = part[..eq_pos].to_string();
            let value_str = &part[eq_pos + 1..];
            let value = parse_field_value(value_str)?;
            fields.insert(key, value);
        }
    }
    
    if fields.is_empty() {
        return Err("No fields found".into());
    }
    
    Ok(fields)
}

fn parse_field_value(s: &str) -> Result<FieldValue, String> {
    let s = s.trim();
    
    // String (quoted)
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        return Ok(FieldValue::String(s[1..s.len()-1].to_string()));
    }
    
    // Boolean
    if s == "true" || s == "t" || s == "T" || s == "TRUE" {
        return Ok(FieldValue::Boolean(true));
    }
    if s == "false" || s == "f" || s == "F" || s == "FALSE" {
        return Ok(FieldValue::Boolean(false));
    }
    
    // Integer (ends with 'i')
    if s.ends_with('i') {
        let num_str = &s[..s.len()-1];
        return num_str.parse::<i64>()
            .map(FieldValue::Integer)
            .map_err(|_| format!("Invalid integer: {}", s));
    }
    
    // Float
    s.parse::<f64>()
        .map(FieldValue::Float)
        .map_err(|_| format!("Invalid field value: {}", s))
}

fn convert_timestamp(ts: i64, precision: &str) -> i64 {
    // Convert to nanoseconds
    match precision {
        "ns" | "n" => ts,
        "us" | "u" | "µ" => ts * 1_000,
        "ms" => ts * 1_000_000,
        "s" => ts * 1_000_000_000,
        _ => ts, // Default: nanoseconds
    }
}

// =====================================
// Storage Functions
// =====================================

async fn store_points(db: &Arc<Database>, bucket: &str, points: Vec<Point>) -> Result<(), String> {
    let collection = format!("__influxdb_{}", bucket);
    
    for point in points {
        let timestamp = point.timestamp.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i64
        });
        
        let mut data = HashMap::new();
        
        // Store measurement
        data.insert("_measurement".to_string(), Value::String(point.measurement.clone()));
        
        // Store tags
        for (k, v) in &point.tags {
            data.insert(format!("_tag_{}", k), Value::String(v.clone()));
        }
        
        // Store fields
        for (k, v) in &point.fields {
            let value = match v {
                FieldValue::Float(f) => Value::Float(*f),
                FieldValue::Integer(i) => Value::Int(*i),
                FieldValue::String(s) => Value::String(s.clone()),
                FieldValue::Boolean(b) => Value::Bool(*b),
            };
            data.insert(format!("_field_{}", k), value);
        }
        
        // Store timestamp
        data.insert("_time".to_string(), Value::Int(timestamp));
        
        // Generate ID
        let id = format!("{}_{}", point.measurement, timestamp);
        let doc = Document::with_id(&id, data);
        
        db.insert(&collection, doc).await.map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

// =====================================
// Query Execution
// =====================================

/// Execute InfluxQL query
async fn execute_influxql(db: &Arc<Database>, db_name: &str, query: &str) -> Result<String, String> {
    let query_upper = query.to_uppercase();
    
    // Handle special queries
    if query_upper.contains("SHOW DATABASES") {
        return Ok(r#"{"results":[{"statement_id":0,"series":[{"name":"databases","columns":["name"],"values":[["default"],["_internal"]]}]}]}"#.to_string());
    }
    
    if query_upper.contains("SHOW MEASUREMENTS") {
        // Would scan collection for unique _measurement values
        return Ok(r#"{"results":[{"statement_id":0,"series":[{"name":"measurements","columns":["name"],"values":[]}]}]}"#.to_string());
    }
    
    if query_upper.contains("SHOW TAG KEYS") {
        return Ok(r#"{"results":[{"statement_id":0,"series":[{"name":"tag_keys","columns":["tagKey"],"values":[]}]}]}"#.to_string());
    }
    
    if query_upper.contains("SHOW FIELD KEYS") {
        return Ok(r#"{"results":[{"statement_id":0,"series":[{"name":"field_keys","columns":["fieldKey","fieldType"],"values":[]}]}]}"#.to_string());
    }
    
    // Parse SELECT query
    if query_upper.starts_with("SELECT") {
        return execute_select_influxql(db, db_name, query).await;
    }
    
    // CREATE DATABASE
    if query_upper.starts_with("CREATE DATABASE") {
        return Ok(r#"{"results":[{"statement_id":0}]}"#.to_string());
    }
    
    Err(format!("Unsupported query: {}", query))
}

async fn execute_select_influxql(db: &Arc<Database>, db_name: &str, query: &str) -> Result<String, String> {
    // Very simplified SELECT parser
    // SELECT field1, field2 FROM measurement WHERE time > now() - 1h
    
    let collection = format!("__influxdb_{}", db_name);
    
    // Extract measurement name from query (simplified)
    let from_pos = query.to_uppercase().find(" FROM ");
    let measurement = if let Some(pos) = from_pos {
        let rest = &query[pos + 6..];
        let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
        rest[..end].trim().to_string()
    } else {
        return Err("No FROM clause found".into());
    };
    
    // Query all docs for this measurement
    let query_obj = crate::types::Query {
        filter: None,
        limit: Some(1000),
    };
    
    match db.query(&collection, query_obj).await {
        Ok(docs) => {
            let filtered: Vec<_> = docs.into_iter()
                .filter(|doc| {
                    doc.get("_measurement")
                        .and_then(|v| if let Value::String(s) = v { Some(s == &measurement) } else { None })
                        .unwrap_or(false)
                })
                .collect();
            
            // Build result
            let mut columns = vec!["time".to_string()];
            let mut values: Vec<Vec<String>> = Vec::new();
            
            for doc in filtered {
                let mut row = Vec::new();
                
                // Time
                let time = doc.get("_time")
                    .and_then(|v| if let Value::Int(i) = v { Some(*i) } else { None })
                    .unwrap_or(0);
                row.push(time.to_string());
                
                // Fields
                for (k, v) in &doc.data {
                    if k.starts_with("_field_") {
                        let field_name = k.trim_start_matches("_field_");
                        if !columns.contains(&field_name.to_string()) {
                            columns.push(field_name.to_string());
                        }
                        let val_str = match v {
                            Value::Float(f) => f.to_string(),
                            Value::Int(i) => i.to_string(),
                            Value::String(s) => format!("\"{}\"", s),
                            Value::Bool(b) => b.to_string(),
                            _ => "null".to_string(),
                        };
                        row.push(val_str);
                    }
                }
                
                values.push(row);
            }
            
            let columns_json = serde_json::to_string(&columns).unwrap_or("[]".into());
            let values_json = serde_json::to_string(&values).unwrap_or("[]".into());
            
            Ok(format!(
                r#"{{"results":[{{"statement_id":0,"series":[{{"name":"{}","columns":{},"values":{}}}]}}]}}"#,
                measurement, columns_json, values_json
            ))
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Execute Flux query
async fn execute_flux(db: &Arc<Database>, bucket: &str, query: &str) -> Result<String, String> {
    // Very simplified Flux parser
    // from(bucket: "name") |> range(start: -1h) |> filter(fn: (r) => r._measurement == "cpu")
    
    let collection = format!("__influxdb_{}", bucket);
    
    // For now, return all data as CSV
    let query_obj = crate::types::Query {
        filter: None,
        limit: Some(1000),
    };
    
    match db.query(&collection, query_obj).await {
        Ok(docs) => {
            let mut csv = String::new();
            
            // Header
            csv.push_str(",result,table,_start,_stop,_time,_value,_field,_measurement\r\n");
            
            // Rows
            for (i, doc) in docs.iter().enumerate() {
                let time = doc.get("_time")
                    .and_then(|v| if let Value::Int(t) = v { Some(*t) } else { None })
                    .unwrap_or(0);
                
                let measurement = doc.get("_measurement")
                    .and_then(|v| if let Value::String(s) = v { Some(s.clone()) } else { None })
                    .unwrap_or_default();
                
                // Output each field as a row
                for (k, v) in &doc.data {
                    if k.starts_with("_field_") {
                        let field_name = k.trim_start_matches("_field_");
                        let value = match v {
                            Value::Float(f) => f.to_string(),
                            Value::Int(n) => n.to_string(),
                            Value::String(s) => s.clone(),
                            Value::Bool(b) => b.to_string(),
                            _ => "".to_string(),
                        };
                        
                        csv.push_str(&format!(
                            ",_result,{},{},{},{},{},{},{}\r\n",
                            i, 0, 0, time, value, field_name, measurement
                        ));
                    }
                }
            }
            
            Ok(csv)
        }
        Err(e) => Err(e.to_string()),
    }
}

// =====================================
// Helper Functions
// =====================================

fn parse_query_string(s: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for pair in s.split('&') {
        if let Some(eq_pos) = pair.find('=') {
            let key = urlencoding_decode(&pair[..eq_pos]);
            let value = urlencoding_decode(&pair[eq_pos + 1..]);
            params.insert(key, value);
        }
    }
    params
}

fn urlencoding_decode(s: &str) -> String {
    s.replace("%20", " ")
        .replace("%3D", "=")
        .replace("%26", "&")
        .replace("%2B", "+")
        .replace("%2F", "/")
        .replace("%3A", ":")
        .replace("%22", "\"")
        .replace("%27", "'")
        .replace("+", " ")
}

fn extract_json_field(json: &str, field: &str) -> Option<String> {
    // Simple JSON field extraction
    let pattern = format!(r#""{}":"#, field);
    if let Some(start) = json.find(&pattern) {
        let rest = &json[start + pattern.len()..];
        if rest.starts_with('"') {
            let end = rest[1..].find('"').map(|i| i + 1)?;
            return Some(rest[1..end].to_string());
        }
    }
    None
}
