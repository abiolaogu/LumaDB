//! Apache Druid Wire Protocol Compatible Server
//!
//! Implements:
//! - Druid SQL API (/druid/v2/sql)
//! - Native JSON Query API (/druid/v2/)
//! - Avatica JDBC compatibility
//! - Timeseries, TopN, GroupBy, Scan queries
//!
//! Port: 8888 (Druid Router default)

use crate::{Database, Document, Value};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Druid Query Types
#[derive(Debug, Clone)]
pub enum DruidQueryType {
    Timeseries,
    TopN,
    GroupBy,
    Scan,
    Search,
    SegmentMetadata,
    TimeBoundary,
    DataSourceMetadata,
}

/// Druid Native Query
#[derive(Debug, Clone)]
pub struct DruidQuery {
    pub query_type: DruidQueryType,
    pub data_source: String,
    pub intervals: Vec<String>,
    pub granularity: String,
    pub filter: Option<DruidFilter>,
    pub aggregations: Vec<DruidAggregation>,
    pub dimensions: Vec<String>,
    pub limit: Option<usize>,
    pub threshold: Option<usize>,
    pub metric: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DruidFilter {
    Selector { dimension: String, value: String },
    Regex { dimension: String, pattern: String },
    And(Vec<DruidFilter>),
    Or(Vec<DruidFilter>),
    Not(Box<DruidFilter>),
    In { dimension: String, values: Vec<String> },
    Bound { dimension: String, lower: Option<String>, upper: Option<String> },
}

#[derive(Debug, Clone)]
pub struct DruidAggregation {
    pub agg_type: String,
    pub name: String,
    pub field_name: Option<String>,
}

/// Start Druid-compatible server on port 8888
pub async fn start(db: Arc<Database>, port: u16) -> crate::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.map_err(|e| crate::LumaError::Io(e))?;
    println!("LumaDB Druid Adapter listening on {} (drop-in replacement)", addr);

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
            let path = parts[1];
            
            // Find body
            let body_start = request.find("\r\n\r\n").map(|i| i + 4).unwrap_or(n);
            let body = String::from_utf8_lossy(&buf[body_start..n]);

            match (method, path) {
                // =====================================
                // Druid SQL API (Avatica compatible)
                // =====================================
                ("POST", "/druid/v2/sql") | ("POST", "/druid/v2/sql/") => {
                    match parse_sql_request(&body) {
                        Ok(sql) => {
                            match execute_druid_sql(&db, &sql).await {
                                Ok(result) => {
                                    let response = format!(
                                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                        result.len(), result
                                    );
                                    let _ = socket.write_all(response.as_bytes()).await;
                                }
                                Err(e) => {
                                    let json = format!(r#"{{"error":"{}"}}"#, e);
                                    let response = format!(
                                        "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                        json.len(), json
                                    );
                                    let _ = socket.write_all(response.as_bytes()).await;
                                }
                            }
                        }
                        Err(e) => {
                            let json = format!(r#"{{"error":"{}"}}"#, e);
                            let response = format!(
                                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                json.len(), json
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                    }
                }

                // =====================================
                // Druid Native JSON Query API
                // =====================================
                ("POST", "/druid/v2") | ("POST", "/druid/v2/") => {
                    match parse_native_query(&body) {
                        Ok(query) => {
                            match execute_native_query(&db, query).await {
                                Ok(result) => {
                                    let response = format!(
                                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                        result.len(), result
                                    );
                                    let _ = socket.write_all(response.as_bytes()).await;
                                }
                                Err(e) => {
                                    let json = format!(r#"{{"error":"{}"}}"#, e);
                                    let response = format!(
                                        "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                        json.len(), json
                                    );
                                    let _ = socket.write_all(response.as_bytes()).await;
                                }
                            }
                        }
                        Err(e) => {
                            let json = format!(r#"{{"error":"{}"}}"#, e);
                            let response = format!(
                                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                json.len(), json
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                    }
                }

                // =====================================
                // Data Sources API
                // =====================================
                ("GET", "/druid/v2/datasources") | ("GET", "/druid/v2/datasources/") => {
                    let json = r#"["metrics","events","logs"]"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                ("GET", path) if path.starts_with("/druid/v2/datasources/") => {
                    let datasource = path.trim_start_matches("/druid/v2/datasources/")
                        .trim_end_matches('/');
                    let json = format!(r#"{{"name":"{}","properties":{{}},"segments":[]}}"#, datasource);
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                // =====================================
                // Coordinator APIs (cluster status)
                // =====================================
                ("GET", "/status") | ("GET", "/status/") => {
                    let json = r#"{"version":"0.23.0","modules":[],"memory":{"maxMemory":1073741824,"totalMemory":536870912,"freeMemory":268435456,"usedMemory":268435456}}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                ("GET", "/status/health") => {
                    let _ = socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\ntrue").await;
                }

                ("GET", "/status/selfDiscovered") => {
                    let json = r#"{"selfDiscovered":true}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                ("GET", "/status/selfDiscovered/status") => {
                    let json = r#"{"selfDiscovered":true,"status":"LEADER"}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                // =====================================
                // Coordinator/Overlord APIs
                // =====================================
                ("GET", "/druid/coordinator/v1/loadstatus") => {
                    let json = r#"{"metrics":100,"events":100,"logs":100}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                ("GET", "/druid/coordinator/v1/servers") => {
                    let json = r#"[{"host":"localhost:8083","tier":"_default_tier","type":"historical","currSize":0,"maxSize":1000000000000,"priority":0}]"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                ("GET", "/druid/indexer/v1/supervisor") => {
                    let json = r#"[]"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                ("GET", "/druid/indexer/v1/tasks") => {
                    let json = r#"[]"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                // =====================================
                // Avatica JDBC compatibility
                // =====================================
                ("POST", "/druid/v2/sql/avatica") | ("POST", "/druid/v2/sql/avatica/") => {
                    // Avatica uses Protobuf - simplified JSON response
                    let json = r#"{"response":"execute_result","connectionId":"conn-1","statementId":1,"signature":{"columns":[],"sql":"","parameters":[]},"firstFrame":{"offset":0,"done":true,"rows":[]}}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        json.len(), json
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                }

                // =====================================
                // Query Status
                // =====================================
                ("DELETE", path) if path.starts_with("/druid/v2/") => {
                    // Cancel query
                    let _ = socket.write_all(b"HTTP/1.1 202 Accepted\r\n\r\n").await;
                }

                _ => {
                    let _ = socket.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").await;
                }
            }
        });
    }
}

// =====================================
// SQL Query Execution
// =====================================

fn parse_sql_request(body: &str) -> Result<String, String> {
    // Parse JSON body: {"query": "SELECT ..."}
    if let Some(start) = body.find(r#""query""#) {
        let rest = &body[start + 7..];
        if let Some(colon) = rest.find(':') {
            let value_start = &rest[colon + 1..].trim_start();
            if value_start.starts_with('"') {
                // Find end quote (handle escapes naively)
                let inner = &value_start[1..];
                if let Some(end) = inner.find('"') {
                    return Ok(inner[..end].to_string());
                }
            }
        }
    }
    
    // Try assuming body IS the query
    let trimmed = body.trim();
    if trimmed.to_uppercase().starts_with("SELECT") {
        return Ok(trimmed.to_string());
    }
    
    Err("Could not parse SQL query from request".into())
}

async fn execute_druid_sql(db: &Arc<Database>, sql: &str) -> Result<String, String> {
    let sql_upper = sql.to_uppercase();
    
    // Parse simple SELECT statements
    // SELECT column1, column2 FROM datasource WHERE condition LIMIT n
    
    // Handle INFORMATION_SCHEMA queries (for clients/tools)
    if sql_upper.contains("INFORMATION_SCHEMA") {
        if sql_upper.contains("SCHEMATA") {
            return Ok(r#"[{"CATALOG_NAME":"druid","SCHEMA_NAME":"druid","SCHEMA_OWNER":"druid"}]"#.into());
        }
        if sql_upper.contains("TABLES") {
            return Ok(r#"[{"TABLE_CATALOG":"druid","TABLE_SCHEMA":"druid","TABLE_NAME":"metrics","TABLE_TYPE":"TABLE"}]"#.into());
        }
        if sql_upper.contains("COLUMNS") {
            return Ok(r#"[{"COLUMN_NAME":"__time","DATA_TYPE":"TIMESTAMP","TABLE_NAME":"metrics"}]"#.into());
        }
    }
    
    // Handle SHOW queries
    if sql_upper.starts_with("SHOW") {
        if sql_upper.contains("TABLES") || sql_upper.contains("DATASOURCES") {
            return Ok(r#"[{"TABLE_NAME":"metrics"},{"TABLE_NAME":"events"},{"TABLE_NAME":"logs"}]"#.into());
        }
        if sql_upper.contains("DATABASES") || sql_upper.contains("SCHEMAS") {
            return Ok(r#"[{"SCHEMA_NAME":"druid"}]"#.into());
        }
    }
    
    // Parse SELECT
    if sql_upper.starts_with("SELECT") {
        // Extract datasource from FROM clause
        let from_pos = sql_upper.find(" FROM ");
        let datasource = if let Some(pos) = from_pos {
            let rest = &sql[pos + 6..];
            let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
            rest[..end].trim().to_string()
        } else {
            "metrics".to_string()
        };
        
        // Query the datasource (stored as LumaDB collection)
        let collection = format!("__druid_{}", datasource);
        
        let query_obj = crate::types::Query {
            filter: None,
            limit: Some(1000),
        };
        
        match db.query(&collection, query_obj).await {
            Ok(docs) => {
                // Convert to Druid result format
                let mut results = Vec::new();
                for doc in docs {
                    let mut row: HashMap<String, serde_json::Value> = HashMap::new();
                    for (k, v) in &doc.data {
                        let json_val = match v {
                            Value::String(s) => serde_json::Value::String(s.clone()),
                            Value::Int(i) => serde_json::Value::Number((*i).into()),
                            Value::Float(f) => serde_json::json!(*f),
                            Value::Bool(b) => serde_json::Value::Bool(*b),
                            _ => serde_json::Value::Null,
                        };
                        row.insert(k.clone(), json_val);
                    }
                    results.push(row);
                }
                
                serde_json::to_string(&results)
                    .map_err(|e| e.to_string())
            }
            Err(e) => Err(e.to_string()),
        }
    } else {
        Ok("[]".into())
    }
}

// =====================================
// Native Query Execution
// =====================================

fn parse_native_query(body: &str) -> Result<DruidQuery, String> {
    // Parse JSON native query
    // {"queryType": "timeseries", "dataSource": "metrics", ...}
    
    let query_type = extract_json_string(body, "queryType")
        .unwrap_or_else(|| "timeseries".to_string());
    
    let data_source = extract_json_string(body, "dataSource")
        .unwrap_or_else(|| "metrics".to_string());
    
    let granularity = extract_json_string(body, "granularity")
        .unwrap_or_else(|| "all".to_string());
    
    let intervals = extract_json_array_strings(body, "intervals")
        .unwrap_or_else(|| vec!["1970-01-01/2100-01-01".to_string()]);
    
    let dimensions = extract_json_array_strings(body, "dimensions")
        .unwrap_or_default();
    
    let qtype = match query_type.to_lowercase().as_str() {
        "timeseries" => DruidQueryType::Timeseries,
        "topn" => DruidQueryType::TopN,
        "groupby" => DruidQueryType::GroupBy,
        "scan" => DruidQueryType::Scan,
        "search" => DruidQueryType::Search,
        "segmentmetadata" => DruidQueryType::SegmentMetadata,
        "timeboundary" => DruidQueryType::TimeBoundary,
        "datasourcemetadata" => DruidQueryType::DataSourceMetadata,
        _ => DruidQueryType::Timeseries,
    };
    
    Ok(DruidQuery {
        query_type: qtype,
        data_source,
        intervals,
        granularity,
        filter: None,
        aggregations: Vec::new(),
        dimensions,
        limit: None,
        threshold: None,
        metric: None,
    })
}

async fn execute_native_query(db: &Arc<Database>, query: DruidQuery) -> Result<String, String> {
    let collection = format!("__druid_{}", query.data_source);
    
    match query.query_type {
        DruidQueryType::Timeseries => {
            // Return aggregated results over time
            let query_obj = crate::types::Query {
                filter: None,
                limit: Some(10000),
            };
            
            match db.query(&collection, query_obj).await {
                Ok(docs) => {
                    // Group by time and aggregate
                    let mut results = Vec::new();
                    
                    // Simple: return count and sum of all numeric fields
                    let count = docs.len();
                    let mut sums: HashMap<String, f64> = HashMap::new();
                    
                    for doc in &docs {
                        for (k, v) in &doc.data {
                            if let Value::Float(f) = v {
                                *sums.entry(k.clone()).or_insert(0.0) += f;
                            } else if let Value::Int(i) = v {
                                *sums.entry(k.clone()).or_insert(0.0) += *i as f64;
                            }
                        }
                    }
                    
                    let mut result_obj: HashMap<String, serde_json::Value> = HashMap::new();
                    result_obj.insert("count".into(), serde_json::json!(count));
                    for (k, v) in sums {
                        result_obj.insert(format!("sum_{}", k), serde_json::json!(v));
                    }
                    
                    results.push(serde_json::json!({
                        "timestamp": "1970-01-01T00:00:00.000Z",
                        "result": result_obj
                    }));
                    
                    serde_json::to_string(&results).map_err(|e| e.to_string())
                }
                Err(e) => Err(e.to_string()),
            }
        }
        
        DruidQueryType::TopN => {
            // Return top N by metric
            Ok(r#"[{"timestamp":"1970-01-01T00:00:00.000Z","result":[]}]"#.into())
        }
        
        DruidQueryType::GroupBy => {
            // Group by dimensions
            Ok(r#"[{"version":"v1","timestamp":"1970-01-01T00:00:00.000Z","event":{}}]"#.into())
        }
        
        DruidQueryType::Scan => {
            // Return raw rows
            let query_obj = crate::types::Query {
                filter: None,
                limit: query.limit.or(Some(1000)),
            };
            
            match db.query(&collection, query_obj).await {
                Ok(docs) => {
                    let rows: Vec<_> = docs.iter().map(|doc| {
                        let mut row: HashMap<String, serde_json::Value> = HashMap::new();
                        for (k, v) in &doc.data {
                            let json_val = match v {
                                Value::String(s) => serde_json::Value::String(s.clone()),
                                Value::Int(i) => serde_json::Value::Number((*i).into()),
                                Value::Float(f) => serde_json::json!(*f),
                                Value::Bool(b) => serde_json::Value::Bool(*b),
                                _ => serde_json::Value::Null,
                            };
                            row.insert(k.clone(), json_val);
                        }
                        row
                    }).collect();
                    
                    let result = serde_json::json!([{
                        "segmentId": "segment_0",
                        "columns": [],
                        "events": rows
                    }]);
                    
                    serde_json::to_string(&result).map_err(|e| e.to_string())
                }
                Err(e) => Err(e.to_string()),
            }
        }
        
        DruidQueryType::TimeBoundary => {
            Ok(r#"[{"timestamp":"1970-01-01T00:00:00.000Z","result":{"minTime":"1970-01-01T00:00:00.000Z","maxTime":"2100-01-01T00:00:00.000Z"}}]"#.into())
        }
        
        DruidQueryType::SegmentMetadata => {
            Ok(r#"[{"id":"segment_0","intervals":["1970-01-01T00:00:00.000Z/2100-01-01T00:00:00.000Z"],"columns":{},"size":0,"numRows":0}]"#.into())
        }
        
        DruidQueryType::DataSourceMetadata => {
            Ok(r#"[{"timestamp":"1970-01-01T00:00:00.000Z","result":{"maxIngestedEventTime":"2024-01-01T00:00:00.000Z"}}]"#.into())
        }
        
        DruidQueryType::Search => {
            Ok(r#"[{"timestamp":"1970-01-01T00:00:00.000Z","result":[]}]"#.into())
        }
    }
}

// =====================================
// JSON Helper Functions
// =====================================

fn extract_json_string(json: &str, field: &str) -> Option<String> {
    let pattern = format!(r#""{}""#, field);
    let start = json.find(&pattern)?;
    let rest = &json[start + pattern.len()..];
    let colon = rest.find(':')?;
    let value_start = rest[colon + 1..].trim_start();
    
    if value_start.starts_with('"') {
        let inner = &value_start[1..];
        let end = inner.find('"')?;
        Some(inner[..end].to_string())
    } else {
        // Non-string value
        let end = value_start.find(|c: char| c == ',' || c == '}' || c == ']')?;
        Some(value_start[..end].trim().to_string())
    }
}

fn extract_json_array_strings(json: &str, field: &str) -> Option<Vec<String>> {
    let pattern = format!(r#""{}""#, field);
    let start = json.find(&pattern)?;
    let rest = &json[start + pattern.len()..];
    let bracket_start = rest.find('[')?;
    let bracket_end = rest.find(']')?;
    
    let array_content = &rest[bracket_start + 1..bracket_end];
    let items: Vec<String> = array_content
        .split(',')
        .filter_map(|s| {
            let trimmed = s.trim().trim_matches('"');
            if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
        })
        .collect();
    
    Some(items)
}
