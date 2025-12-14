
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::{BytesMut, Buf, BufMut};
use std::sync::Arc;
use luma_protocol_core::{QueryProcessor, QueryRequest, QueryResult, Value};
use tracing::{info, error, debug, warn};

/// PostgreSQL authentication configuration
#[derive(Clone)]
pub struct AuthConfig {
    pub username: String,
    pub password: String,
    pub require_auth: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            username: "lumadb".to_string(),
            password: "lumadb".to_string(),
            require_auth: true,
        }
    }
}

pub async fn run(
    port: u16, 
    processor: Arc<dyn QueryProcessor + Send + Sync>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_with_auth(port, processor, AuthConfig::default()).await
}

pub async fn run_with_auth(
    port: u16, 
    processor: Arc<dyn QueryProcessor + Send + Sync>,
    auth_config: AuthConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    info!("PostgreSQL Protocol Server listening on {}", addr);

    let auth_config = Arc::new(auth_config);
    
    loop {
        let (socket, peer_addr) = listener.accept().await?;
        let processor = processor.clone();
        let auth = auth_config.clone();
        tokio::spawn(async move {
            debug!("New PostgreSQL connection from {}", peer_addr);
            if let Err(e) = handle_connection(socket, processor, auth).await {
                error!("Postgres connection error from {}: {}", peer_addr, e);
            }
        });
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    processor: Arc<dyn QueryProcessor + Send + Sync>,
    auth_config: Arc<AuthConfig>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buf = BytesMut::with_capacity(4096);

    // 1. Read StartupMessage length
    let mut len_buf = [0u8; 4];
    if socket.read_exact(&mut len_buf).await.is_err() { return Ok(()); }
    let startup_len = u32::from_be_bytes(len_buf) as usize;
    
    // Read startup message body
    let mut startup_body = vec![0u8; startup_len - 4];
    socket.read_exact(&mut startup_body).await?;
    
    // Parse username from startup message (simplified)
    let client_user = parse_startup_user(&startup_body).unwrap_or_default();
    
    if auth_config.require_auth {
        // 2. Send AuthenticationMD5Password (R with auth type 5)
        let salt: [u8; 4] = rand::random();
        let mut auth_req = BytesMut::new();
        auth_req.put_u8(b'R');
        auth_req.put_u32(12); // Length: 4 (len) + 4 (type) + 4 (salt)  
        auth_req.put_u32(5);  // AuthenticationMD5Password
        auth_req.put_slice(&salt);
        socket.write_all(&auth_req).await?;
        
        // 3. Read PasswordMessage
        let mut type_buf = [0u8; 1];
        socket.read_exact(&mut type_buf).await?;
        if type_buf[0] != b'p' {
            warn!("Expected PasswordMessage, got {:?}", type_buf[0]);
            return Ok(());
        }
        
        let mut pw_len = [0u8; 4];
        socket.read_exact(&mut pw_len).await?;
        let pw_body_len = u32::from_be_bytes(pw_len) as usize - 4;
        let mut pw_body = vec![0u8; pw_body_len];
        socket.read_exact(&mut pw_body).await?;
        
        // Client sends: md5(md5(password + username) + salt)
        let expected = compute_md5_password(&auth_config.password, &client_user, &salt);
        let received = String::from_utf8_lossy(&pw_body).trim_end_matches('\0').to_string();
        
        if received != expected {
            warn!("Authentication failed for user '{}'", client_user);
            send_error(&mut socket, "password authentication failed").await?;
            return Ok(());
        }
        
        debug!("User '{}' authenticated successfully", client_user);
    }
    
    // 4. Send AuthenticationOk
    socket.write_all(&[0x52, 0, 0, 0, 8, 0, 0, 0, 0]).await?;
    
    // 5. Ready For Query 'Z'
    socket.write_all(&[0x5a, 0, 0, 0, 5, 0x49]).await?;

    // 6. Query Loop
    loop {
        let mut type_buf = [0u8; 1];
        if socket.read_exact(&mut type_buf).await.is_err() { break; }
        let msg_type = type_buf[0];

        // Read length
        let mut len_buf = [0u8; 4];
        if socket.read_exact(&mut len_buf).await.is_err() { break; }
        let len = u32::from_be_bytes(len_buf) as usize;
        let body_len = len - 4;

        // Read body
        buf.resize(body_len, 0);
        socket.read_exact(&mut buf).await?;
        
        match msg_type {
            b'Q' => { // Simple Query
                let query_string = std::str::from_utf8(&buf)
                    .unwrap_or("")
                    .trim_end_matches('\0');
                
                debug!("Received SQL: {}", query_string);

                // Execute
                let start = std::time::Instant::now();
                let result = processor.process(QueryRequest {
                    query: query_string.to_string(),
                    params: vec![], // No params in simple query
                }).await;

                match result {
                    Ok(res) => send_row_description_and_data(&mut socket, res).await?,
                    Err(e) => send_error(&mut socket, &e.to_string()).await?,
                }
                
                // Command Complete
                let tag = "SELECT 1"; // Dynamic based on op
                let mut tag_bytes = BytesMut::new();
                tag_bytes.put_u8(b'C');
                tag_bytes.put_u32(4 + tag.len() as u32 + 1);
                tag_bytes.put_slice(tag.as_bytes());
                tag_bytes.put_u8(0);
                socket.write_all(&tag_bytes).await?;

                // Ready For Query
                socket.write_all(&[0x5a, 0, 0, 0, 5, 0x49]).await?;
            },
            b'X' => break, // Terminate
            _ => {
                // Ignore unknown
            }
        }
    }

    Ok(())
}

async fn send_row_description_and_data(socket: &mut TcpStream, res: QueryResult) -> Result<(), std::io::Error> {
    // RowDescription 'T'
    // Infer schema from first row or use default
    let num_fields = if res.rows.is_empty() { 
        0 
    } else { 
        res.rows[0].len() 
    };

    let mut buf = BytesMut::new();
    buf.put_u8(b'T');
    
    // Placeholder length
    let start_idx = buf.len();
    buf.put_u32(0); 
    
    buf.put_u16(num_fields as u16);
    
    for i in 0..num_fields {
        let name = format!("col_{}", i);
        buf.put_slice(name.as_bytes());
        buf.put_u8(0); 
        buf.put_u32(0); 
        buf.put_u16(0); 
        
        let type_oid = 25; // TEXT
        buf.put_u32(type_oid); 
        
        buf.put_u16(0); 
        buf.put_i32(-1); 
        buf.put_u16(0); 
    }
    
    let len = (buf.len() - start_idx) as u32;
    let bytes = &mut buf[start_idx..start_idx+4]; 
    bytes.copy_from_slice(&len.to_be_bytes());
    
    socket.write_all(&buf).await?;

    // DataRow 'D'
    for row in res.rows {
        let mut row_buf = BytesMut::new();
        row_buf.put_u8(b'D');
        let r_start = row_buf.len();
        row_buf.put_u32(0);
        
        row_buf.put_u16(num_fields as u16);
        
        for val in row {
            let s = match val {
                Value::Text(t) => t,
                Value::Int64(i) => i.to_string(),
                Value::Float64(f) => f.to_string(),
                _ => "null".to_string() // Fallback
            };
            
            row_buf.put_u32(s.len() as u32);
            row_buf.put_slice(s.as_bytes());
        }
        
        let r_len = (row_buf.len() - r_start) as u32;
        let r_bytes = &mut row_buf[r_start..r_start+4];
        r_bytes.copy_from_slice(&r_len.to_be_bytes());
        
        socket.write_all(&row_buf).await?;
    }

    Ok(())
}

async fn send_error(socket: &mut TcpStream, msg: &str) -> Result<(), std::io::Error> {
    let mut buf = BytesMut::new();
    buf.put_u8(b'E');
    let start = buf.len();
    buf.put_u32(0);
    
    buf.put_u8(b'S'); // Severity
    buf.put_slice(b"ERROR\0");
    buf.put_u8(b'M'); // Message
    buf.put_slice(msg.as_bytes());
    buf.put_u8(0);
    buf.put_u8(0); // End
    
    let len = (buf.len() - start) as u32;
    let bytes = &mut buf[start..start+4];
    bytes.copy_from_slice(&len.to_be_bytes());
    
    socket.write_all(&buf).await
}

/// Parse username from PostgreSQL StartupMessage
/// Format: protocol_version (4 bytes) + null-terminated key-value pairs
fn parse_startup_user(body: &[u8]) -> Option<String> {
    // Skip protocol version (4 bytes)
    if body.len() < 4 {
        return None;
    }
    let params = &body[4..];
    
    // Parse null-terminated key-value pairs
    let mut iter = params.split(|&b| b == 0);
    while let (Some(key), Some(value)) = (iter.next(), iter.next()) {
        if key == b"user" {
            return String::from_utf8(value.to_vec()).ok();
        }
    }
    None
}

/// Compute PostgreSQL MD5 password hash
/// Format: "md5" + md5(md5(password + username) + salt)
fn compute_md5_password(password: &str, username: &str, salt: &[u8; 4]) -> String {
    // Step 1: md5(password + username)
    let mut input1 = password.as_bytes().to_vec();
    input1.extend_from_slice(username.as_bytes());
    let inner_hash = format!("{:x}", md5::compute(&input1));
    
    // Step 2: md5(inner_hash + salt)
    let mut input2 = inner_hash.as_bytes().to_vec();
    input2.extend_from_slice(salt);
    let outer_hash = format!("{:x}", md5::compute(&input2));
    
    format!("md5{}", outer_hash)
}
