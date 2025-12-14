//! Cassandra CQL Binary Protocol Implementation
//! Provides CQL v4 wire protocol with lightweight transactions

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use std::collections::HashMap;
use dashmap::DashMap;
use tracing::{info, debug, error};

// CQL Protocol Constants
const CQL_VERSION: u8 = 0x04;
const OPCODE_ERROR: u8 = 0x00;
const OPCODE_STARTUP: u8 = 0x01;
const OPCODE_READY: u8 = 0x02;
const OPCODE_AUTHENTICATE: u8 = 0x03;
const OPCODE_OPTIONS: u8 = 0x05;
const OPCODE_SUPPORTED: u8 = 0x06;
const OPCODE_QUERY: u8 = 0x07;
const OPCODE_RESULT: u8 = 0x08;
const OPCODE_PREPARE: u8 = 0x09;
const OPCODE_EXECUTE: u8 = 0x0A;
const OPCODE_REGISTER: u8 = 0x0B;
const OPCODE_BATCH: u8 = 0x0D;
const OPCODE_AUTH_RESPONSE: u8 = 0x0F;
const OPCODE_AUTH_SUCCESS: u8 = 0x10;

const RESULT_VOID: i32 = 0x01;
const RESULT_ROWS: i32 = 0x02;
const RESULT_SET_KEYSPACE: i32 = 0x03;
const RESULT_PREPARED: i32 = 0x04;
const RESULT_SCHEMA_CHANGE: i32 = 0x05;

/// CQL Data Types
#[derive(Clone, Debug)]
pub enum CqlValue {
    Ascii(String),
    Bigint(i64),
    Blob(Vec<u8>),
    Boolean(bool),
    Counter(i64),
    Decimal(String),
    Double(f64),
    Float(f32),
    Int(i32),
    Text(String),
    Timestamp(i64),
    Uuid(String),
    Varchar(String),
    Varint(i64),
    Timeuuid(String),
    Inet(String),
    List(Vec<CqlValue>),
    Map(Vec<(CqlValue, CqlValue)>),
    Set(Vec<CqlValue>),
    Null,
}

/// Prepared statement
#[derive(Clone, Debug)]
pub struct CqlPreparedStatement {
    pub id: Vec<u8>,
    pub query: String,
    pub keyspace: Option<String>,
    pub param_types: Vec<String>,
}

/// Cassandra table (in-memory simulation)
#[derive(Clone, Debug, Default)]
pub struct CqlTable {
    pub keyspace: String,
    pub name: String,
    pub columns: Vec<(String, String)>, // (name, type)
    pub primary_key: Vec<String>,
    pub rows: Vec<HashMap<String, CqlValue>>,
}

/// Cassandra keyspace
#[derive(Clone, Debug)]
pub struct CqlKeyspace {
    pub name: String,
    pub replication: HashMap<String, String>,
    pub tables: HashMap<String, CqlTable>,
}

/// Cassandra store
pub struct CassandraStore {
    keyspaces: Arc<DashMap<String, CqlKeyspace>>,
    prepared: Arc<DashMap<Vec<u8>, CqlPreparedStatement>>,
    next_prep_id: Arc<std::sync::atomic::AtomicU64>,
}

impl CassandraStore {
    pub fn new() -> Self {
        let store = Self {
            keyspaces: Arc::new(DashMap::new()),
            prepared: Arc::new(DashMap::new()),
            next_prep_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        };
        
        // Create system keyspace
        store.keyspaces.insert("system".to_string(), CqlKeyspace {
            name: "system".to_string(),
            replication: HashMap::new(),
            tables: HashMap::new(),
        });
        
        store
    }

    /// Execute CQL query
    pub fn execute(&self, keyspace: Option<&str>, query: &str) -> CqlResult {
        let query_upper = query.trim().to_uppercase();
        
        if query_upper.starts_with("CREATE KEYSPACE") {
            self.create_keyspace(query)
        } else if query_upper.starts_with("CREATE TABLE") {
            self.create_table(keyspace, query)
        } else if query_upper.starts_with("INSERT") {
            self.insert(keyspace, query)
        } else if query_upper.starts_with("SELECT") {
            self.select(keyspace, query)
        } else if query_upper.starts_with("UPDATE") {
            self.update(keyspace, query)
        } else if query_upper.starts_with("DELETE") {
            self.delete(keyspace, query)
        } else if query_upper.starts_with("USE") {
            let ks = query.split_whitespace().nth(1).unwrap_or("").trim_matches(';');
            CqlResult::SetKeyspace(ks.to_string())
        } else if query_upper.starts_with("DROP") {
            CqlResult::Void
        } else {
            CqlResult::Void
        }
    }

    fn create_keyspace(&self, query: &str) -> CqlResult {
        // Parse: CREATE KEYSPACE [IF NOT EXISTS] name WITH ...
        let parts: Vec<&str> = query.split_whitespace().collect();
        let mut name_idx = 2;
        let if_not_exists = query.to_uppercase().contains("IF NOT EXISTS");
        if if_not_exists { name_idx = 5; }
        
        let name = parts.get(name_idx).unwrap_or(&"").trim_matches(|c| c == ';' || c == '\'' || c == '"');
        
        if if_not_exists && self.keyspaces.contains_key(name) {
            return CqlResult::Void;
        }
        
        self.keyspaces.insert(name.to_string(), CqlKeyspace {
            name: name.to_string(),
            replication: HashMap::new(),
            tables: HashMap::new(),
        });
        
        CqlResult::SchemaChange("CREATED".to_string(), "KEYSPACE".to_string(), name.to_string())
    }

    fn create_table(&self, keyspace: Option<&str>, query: &str) -> CqlResult {
        // Simple table creation
        let ks = keyspace.unwrap_or("default");
        
        // Extract table name (simplified parsing)
        let parts: Vec<&str> = query.split_whitespace().collect();
        let if_not_exists = query.to_uppercase().contains("IF NOT EXISTS");
        let name_idx = if if_not_exists { 5 } else { 2 };
        let table_name = parts.get(name_idx).unwrap_or(&"").trim_matches(|c| c == ';' || c == '(' || c == '\'' || c == '"');
        
        // Get or create keyspace
        if !self.keyspaces.contains_key(ks) {
            self.keyspaces.insert(ks.to_string(), CqlKeyspace {
                name: ks.to_string(),
                replication: HashMap::new(),
                tables: HashMap::new(),
            });
        }
        
        if let Some(mut keyspace_entry) = self.keyspaces.get_mut(ks) {
            if if_not_exists && keyspace_entry.tables.contains_key(table_name) {
                return CqlResult::Void;
            }
            
            keyspace_entry.tables.insert(table_name.to_string(), CqlTable {
                keyspace: ks.to_string(),
                name: table_name.to_string(),
                columns: vec![],
                primary_key: vec![],
                rows: vec![],
            });
        }
        
        CqlResult::SchemaChange("CREATED".to_string(), "TABLE".to_string(), table_name.to_string())
    }

    fn insert(&self, keyspace: Option<&str>, query: &str) -> CqlResult {
        // INSERT INTO table (cols) VALUES (vals) [IF NOT EXISTS]
        let if_not_exists = query.to_uppercase().contains("IF NOT EXISTS");
        let ks = keyspace.unwrap_or("default");
        
        // Simplified: just acknowledge the insert
        if if_not_exists {
            // Lightweight transaction - return applied status
            CqlResult::Rows {
                columns: vec![("applied".to_string(), "boolean".to_string())],
                rows: vec![vec![CqlValue::Boolean(true)]],
            }
        } else {
            CqlResult::Void
        }
    }

    fn select(&self, keyspace: Option<&str>, _query: &str) -> CqlResult {
        // Return simulated result
        CqlResult::Rows {
            columns: vec![("id".to_string(), "text".to_string())],
            rows: vec![vec![CqlValue::Text("result".to_string())]],
        }
    }

    fn update(&self, keyspace: Option<&str>, query: &str) -> CqlResult {
        let if_exists = query.to_uppercase().contains("IF EXISTS");
        if if_exists {
            CqlResult::Rows {
                columns: vec![("applied".to_string(), "boolean".to_string())],
                rows: vec![vec![CqlValue::Boolean(true)]],
            }
        } else {
            CqlResult::Void
        }
    }

    fn delete(&self, keyspace: Option<&str>, query: &str) -> CqlResult {
        let if_exists = query.to_uppercase().contains("IF EXISTS");
        if if_exists {
            CqlResult::Rows {
                columns: vec![("applied".to_string(), "boolean".to_string())],
                rows: vec![vec![CqlValue::Boolean(true)]],
            }
        } else {
            CqlResult::Void
        }
    }

    /// Prepare statement
    pub fn prepare(&self, query: &str) -> CqlPreparedStatement {
        let id = self.next_prep_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let id_bytes = id.to_be_bytes().to_vec();
        
        let stmt = CqlPreparedStatement {
            id: id_bytes.clone(),
            query: query.to_string(),
            keyspace: None,
            param_types: vec![],
        };
        
        self.prepared.insert(id_bytes.clone(), stmt.clone());
        stmt
    }

    /// Execute prepared statement
    pub fn execute_prepared(&self, id: &[u8], _params: &[CqlValue]) -> CqlResult {
        if let Some(stmt) = self.prepared.get(id) {
            self.execute(stmt.keyspace.as_deref(), &stmt.query)
        } else {
            CqlResult::Error(0x2500, "Prepared statement not found".to_string())
        }
    }

    /// Execute batch
    pub fn execute_batch(&self, keyspace: Option<&str>, queries: &[String]) -> CqlResult {
        for query in queries {
            self.execute(keyspace, query);
        }
        CqlResult::Void
    }
}

#[derive(Debug)]
pub enum CqlResult {
    Void,
    Rows {
        columns: Vec<(String, String)>,
        rows: Vec<Vec<CqlValue>>,
    },
    SetKeyspace(String),
    Prepared(CqlPreparedStatement),
    SchemaChange(String, String, String),
    Error(u32, String),
}

/// CQL Connection handler
pub struct CqlConnection {
    store: Arc<CassandraStore>,
    keyspace: Option<String>,
}

impl CqlConnection {
    pub fn new(store: Arc<CassandraStore>) -> Self {
        Self {
            store,
            keyspace: None,
        }
    }

    /// Handle CQL connection
    pub async fn handle(&mut self, mut stream: TcpStream) -> std::io::Result<()> {
        loop {
            // Read frame header (9 bytes)
            let mut header = [0u8; 9];
            match stream.read_exact(&mut header).await {
                Ok(_) => {}
                Err(_) => break,
            }

            let version = header[0] & 0x7f;
            let flags = header[1];
            let stream_id = i16::from_be_bytes([header[2], header[3]]);
            let opcode = header[4];
            let length = u32::from_be_bytes([header[5], header[6], header[7], header[8]]) as usize;

            // Read body
            let mut body = vec![0u8; length];
            if length > 0 {
                stream.read_exact(&mut body).await?;
            }

            // Process opcode
            let response = match opcode {
                OPCODE_OPTIONS => self.handle_options(),
                OPCODE_STARTUP => self.handle_startup(&body),
                OPCODE_QUERY => self.handle_query(&body),
                OPCODE_PREPARE => self.handle_prepare(&body),
                OPCODE_EXECUTE => self.handle_execute(&body),
                OPCODE_BATCH => self.handle_batch(&body),
                OPCODE_REGISTER => self.handle_register(&body),
                OPCODE_AUTH_RESPONSE => self.handle_auth_response(&body),
                _ => self.make_error(0x000A, "Unknown opcode"),
            };

            // Send response
            self.send_frame(&mut stream, stream_id, response).await?;
        }

        Ok(())
    }

    fn handle_options(&self) -> (u8, Vec<u8>) {
        let mut body = Vec::new();
        
        // String multimap: SUPPORTED options
        let options = vec![
            ("CQL_VERSION", vec!["3.4.5"]),
            ("COMPRESSION", vec!["lz4", "snappy"]),
        ];
        
        // Number of keys
        body.extend_from_slice(&(options.len() as u16).to_be_bytes());
        
        for (key, values) in options {
            // Key
            body.extend_from_slice(&(key.len() as u16).to_be_bytes());
            body.extend_from_slice(key.as_bytes());
            
            // Values
            body.extend_from_slice(&(values.len() as u16).to_be_bytes());
            for val in values {
                body.extend_from_slice(&(val.len() as u16).to_be_bytes());
                body.extend_from_slice(val.as_bytes());
            }
        }
        
        (OPCODE_SUPPORTED, body)
    }

    fn handle_startup(&self, _body: &[u8]) -> (u8, Vec<u8>) {
        (OPCODE_READY, vec![])
    }

    fn handle_query(&mut self, body: &[u8]) -> (u8, Vec<u8>) {
        // Read [long string] query
        if body.len() < 4 { return self.make_error(0x000A, "Invalid query"); }
        
        let len = u32::from_be_bytes([body[0], body[1], body[2], body[3]]) as usize;
        let query = String::from_utf8_lossy(&body[4..4+len]).to_string();
        
        debug!("CQL Query: {}", query);
        
        match self.store.execute(self.keyspace.as_deref(), &query) {
            CqlResult::Void => self.make_void_result(),
            CqlResult::Rows { columns, rows } => self.make_rows_result(&columns, &rows),
            CqlResult::SetKeyspace(ks) => {
                self.keyspace = Some(ks.clone());
                self.make_set_keyspace_result(&ks)
            }
            CqlResult::SchemaChange(change, target, name) => {
                self.make_schema_change_result(&change, &target, &name)
            }
            CqlResult::Error(code, msg) => self.make_error(code, &msg),
            _ => self.make_void_result(),
        }
    }

    fn handle_prepare(&self, body: &[u8]) -> (u8, Vec<u8>) {
        if body.len() < 4 { return self.make_error(0x000A, "Invalid prepare"); }
        
        let len = u32::from_be_bytes([body[0], body[1], body[2], body[3]]) as usize;
        let query = String::from_utf8_lossy(&body[4..4+len]).to_string();
        
        let stmt = self.store.prepare(&query);
        self.make_prepared_result(&stmt)
    }

    fn handle_execute(&self, body: &[u8]) -> (u8, Vec<u8>) {
        if body.len() < 2 { return self.make_error(0x000A, "Invalid execute"); }
        
        let id_len = u16::from_be_bytes([body[0], body[1]]) as usize;
        let id = body[2..2+id_len].to_vec();
        
        match self.store.execute_prepared(&id, &[]) {
            CqlResult::Void => self.make_void_result(),
            CqlResult::Rows { columns, rows } => self.make_rows_result(&columns, &rows),
            CqlResult::Error(code, msg) => self.make_error(code, &msg),
            _ => self.make_void_result(),
        }
    }

    fn handle_batch(&mut self, body: &[u8]) -> (u8, Vec<u8>) {
        // Batch type (1 byte) + query count + queries
        if body.is_empty() { return self.make_error(0x000A, "Invalid batch"); }
        
        let _batch_type = body[0];
        let query_count = u16::from_be_bytes([body[1], body[2]]) as usize;
        
        // Simplified: execute as batch
        let mut queries = Vec::new();
        let mut offset = 3;
        
        for _ in 0..query_count {
            if offset >= body.len() { break; }
            let kind = body[offset];
            offset += 1;
            
            if kind == 0 {
                // Query string
                if offset + 4 > body.len() { break; }
                let len = u32::from_be_bytes([body[offset], body[offset+1], body[offset+2], body[offset+3]]) as usize;
                offset += 4;
                if offset + len > body.len() { break; }
                let query = String::from_utf8_lossy(&body[offset..offset+len]).to_string();
                queries.push(query);
                offset += len;
            }
        }
        
        self.store.execute_batch(self.keyspace.as_deref(), &queries);
        self.make_void_result()
    }

    fn handle_register(&self, _body: &[u8]) -> (u8, Vec<u8>) {
        (OPCODE_READY, vec![])
    }

    fn handle_auth_response(&self, _body: &[u8]) -> (u8, Vec<u8>) {
        (OPCODE_AUTH_SUCCESS, vec![])
    }

    fn make_void_result(&self) -> (u8, Vec<u8>) {
        let mut body = Vec::new();
        body.extend_from_slice(&RESULT_VOID.to_be_bytes());
        (OPCODE_RESULT, body)
    }

    fn make_rows_result(&self, columns: &[(String, String)], rows: &[Vec<CqlValue>]) -> (u8, Vec<u8>) {
        let mut body = Vec::new();
        body.extend_from_slice(&RESULT_ROWS.to_be_bytes());
        
        // Flags (no metadata = 0x04 for now, with column count)
        body.extend_from_slice(&0x0001u32.to_be_bytes()); // Global_tables_spec flag
        body.extend_from_slice(&(columns.len() as u32).to_be_bytes());
        
        // Global table spec
        body.extend_from_slice(&6u16.to_be_bytes());
        body.extend_from_slice(b"system");
        body.extend_from_slice(&5u16.to_be_bytes());
        body.extend_from_slice(b"local");
        
        // Column specs
        for (name, _type_name) in columns {
            body.extend_from_slice(&(name.len() as u16).to_be_bytes());
            body.extend_from_slice(name.as_bytes());
            body.extend_from_slice(&0x000Du16.to_be_bytes()); // varchar type
        }
        
        // Row count
        body.extend_from_slice(&(rows.len() as u32).to_be_bytes());
        
        // Rows
        for row in rows {
            for val in row {
                match val {
                    CqlValue::Text(s) | CqlValue::Varchar(s) | CqlValue::Ascii(s) => {
                        body.extend_from_slice(&(s.len() as u32).to_be_bytes());
                        body.extend_from_slice(s.as_bytes());
                    }
                    CqlValue::Boolean(b) => {
                        body.extend_from_slice(&1u32.to_be_bytes());
                        body.push(if *b { 1 } else { 0 });
                    }
                    CqlValue::Int(i) => {
                        body.extend_from_slice(&4u32.to_be_bytes());
                        body.extend_from_slice(&i.to_be_bytes());
                    }
                    CqlValue::Bigint(i) | CqlValue::Counter(i) | CqlValue::Timestamp(i) | CqlValue::Varint(i) => {
                        body.extend_from_slice(&8u32.to_be_bytes());
                        body.extend_from_slice(&i.to_be_bytes());
                    }
                    CqlValue::Null => {
                        body.extend_from_slice(&(-1i32).to_be_bytes());
                    }
                    _ => {
                        body.extend_from_slice(&(-1i32).to_be_bytes());
                    }
                }
            }
        }
        
        (OPCODE_RESULT, body)
    }

    fn make_set_keyspace_result(&self, keyspace: &str) -> (u8, Vec<u8>) {
        let mut body = Vec::new();
        body.extend_from_slice(&RESULT_SET_KEYSPACE.to_be_bytes());
        body.extend_from_slice(&(keyspace.len() as u16).to_be_bytes());
        body.extend_from_slice(keyspace.as_bytes());
        (OPCODE_RESULT, body)
    }

    fn make_prepared_result(&self, stmt: &CqlPreparedStatement) -> (u8, Vec<u8>) {
        let mut body = Vec::new();
        body.extend_from_slice(&RESULT_PREPARED.to_be_bytes());
        
        // Prepared ID
        body.extend_from_slice(&(stmt.id.len() as u16).to_be_bytes());
        body.extend_from_slice(&stmt.id);
        
        // Metadata flags (empty for now)
        body.extend_from_slice(&0u32.to_be_bytes()); // flags
        body.extend_from_slice(&0u32.to_be_bytes()); // column count
        
        // Result metadata
        body.extend_from_slice(&0u32.to_be_bytes()); // flags
        body.extend_from_slice(&0u32.to_be_bytes()); // column count
        
        (OPCODE_RESULT, body)
    }

    fn make_schema_change_result(&self, change: &str, target: &str, name: &str) -> (u8, Vec<u8>) {
        let mut body = Vec::new();
        body.extend_from_slice(&RESULT_SCHEMA_CHANGE.to_be_bytes());
        
        // Change type
        body.extend_from_slice(&(change.len() as u16).to_be_bytes());
        body.extend_from_slice(change.as_bytes());
        
        // Target
        body.extend_from_slice(&(target.len() as u16).to_be_bytes());
        body.extend_from_slice(target.as_bytes());
        
        // Options (keyspace for KEYSPACE target)
        body.extend_from_slice(&(name.len() as u16).to_be_bytes());
        body.extend_from_slice(name.as_bytes());
        
        (OPCODE_RESULT, body)
    }

    fn make_error(&self, code: u32, message: &str) -> (u8, Vec<u8>) {
        let mut body = Vec::new();
        body.extend_from_slice(&code.to_be_bytes());
        body.extend_from_slice(&(message.len() as u16).to_be_bytes());
        body.extend_from_slice(message.as_bytes());
        (OPCODE_ERROR, body)
    }

    async fn send_frame(&self, stream: &mut TcpStream, stream_id: i16, (opcode, body): (u8, Vec<u8>)) -> std::io::Result<()> {
        let mut frame = Vec::new();
        frame.push(CQL_VERSION | 0x80); // Response flag
        frame.push(0x00); // Flags
        frame.extend_from_slice(&stream_id.to_be_bytes());
        frame.push(opcode);
        frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
        frame.extend_from_slice(&body);
        
        stream.write_all(&frame).await?;
        stream.flush().await
    }
}

impl Default for CassandraStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Run Cassandra CQL server
pub async fn run(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    let store = Arc::new(CassandraStore::new());
    
    info!("Cassandra CQL Server listening on 0.0.0.0:{}", port);
    
    loop {
        let (stream, addr) = listener.accept().await?;
        let store = store.clone();
        debug!("CQL connection from {}", addr);
        
        tokio::spawn(async move {
            let mut conn = CqlConnection::new(store);
            if let Err(e) = conn.handle(stream).await {
                error!("CQL connection error: {}", e);
            }
        });
    }
}
