use crate::{Database, Result};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use byteorder::{BigEndian, ByteOrder};
use crate::server::translator::Translator;

pub async fn start(db: Arc<Database>, port: u16) -> Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await.map_err(|e| crate::LumaError::Io(e))?;
    println!("Postgres adapter listening on port {}", port);

    let translator = Arc::new(Translator::new(db));

    loop {
        let (socket, _) = listener.accept().await.map_err(|e| crate::LumaError::Io(e))?;
        let translator = translator.clone();
        
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, translator).await {
                eprintln!("PG Connection error: {}", e);
            }
        });
    }
}

async fn handle_connection(mut socket: TcpStream, translator: Arc<Translator>) -> Result<()> {
    // 1. Startup Message
    let len = socket.read_u32().await.map_err(|e| crate::LumaError::Io(e))?;
    let mut buf = vec![0u8; (len - 4) as usize];
    socket.read_exact(&mut buf).await.map_err(|e| crate::LumaError::Io(e))?;

    let version = BigEndian::read_i32(&buf[0..4]);
    
    if version == 80877103 {
        // SSL Request (not supported yet)
        socket.write_u8(b'N').await.map_err(|e| crate::LumaError::Io(e))?; // SSL No
        // Expect Startup packet again? No, SSL handshake is separate.
        // Actually for simplicity, if SSL req, clients usually restart without SSL or continue.
        // Let's assume user connects with `psql "sslmode=disable"`
        // But if psql sends SSLRequest, it waits for response.
        // We just sent 'N', so functionality continues to startup.
        
        // Wait for real startup
        let len2 = socket.read_u32().await.map_err(|e| crate::LumaError::Io(e))?;
        let mut buf2 = vec![0u8; (len2 - 4) as usize];
        socket.read_exact(&mut buf2).await.map_err(|e| crate::LumaError::Io(e))?;
         // Process startup params (user, database, etc) - ignoring for now
    } else {
        // Already read startup packet
    }

    // 2. AuthenticationOk
    socket.write_u8(b'R').await.map_err(|e| crate::LumaError::Io(e))?; // Auth info
    socket.write_u32(8).await.map_err(|e| crate::LumaError::Io(e))?; // Length
    socket.write_u32(0).await.map_err(|e| crate::LumaError::Io(e))?; // AuthOk

    // 3. ReadyForQuery
    send_ready_for_query(&mut socket).await?;

    // 4. Command Loop
    loop {
        let tag = socket.read_u8().await;
        if tag.is_err() { break; } // Connection closed
        let tag = tag.unwrap();

        match tag {
            b'Q' => { // Simple Query
                let query_len = socket.read_u32().await.map_err(|e| crate::LumaError::Io(e))?;
                let mut query_buf = vec![0u8; (query_len - 4) as usize]; // -4 includes query string + null
                socket.read_exact(&mut query_buf).await.map_err(|e| crate::LumaError::Io(e))?;
                
                let sql = String::from_utf8_lossy(&query_buf);
                let sql = sql.trim_matches(char::from(0));
                
                println!("Executing SQL: {}", sql);

                match translator.execute_sql(sql).await {
                    Ok(docs) => {
                        // TODO: Send RowDescription
                        // For now just sending CommandComplete
                        let tag = format!("SELECT {}", docs.len());
                        send_command_complete(&mut socket, &tag).await?;
                    },
                    Err(e) => {
                        send_error(&mut socket, &e.to_string()).await?;
                    }
                }
                
                 send_ready_for_query(&mut socket).await?;
            }
            b'X' => { // Terminate
                break;
            }
            _ => {
                // consume remainder
                // let _ = socket.read_u32().await;
                println!("Unknown PG tag: {}", tag as char);
                // break to avoid desync
                break; 
            }
        }
    }

    Ok(())
}

async fn send_ready_for_query(socket: &mut TcpStream) -> Result<()> {
    socket.write_u8(b'Z').await.map_err(|e| crate::LumaError::Io(e))?;
    socket.write_u32(5).await.map_err(|e| crate::LumaError::Io(e))?;
    socket.write_u8(b'I').await.map_err(|e| crate::LumaError::Io(e))?; // Idle
    Ok(())
}

async fn send_command_complete(socket: &mut TcpStream, tag: &str) -> Result<()> {
    let mut buf = Vec::new();
    buf.push(b'C');
    let len = 4 + tag.len() + 1;
    buf.extend_from_slice(&(len as u32).to_be_bytes());
    buf.extend_from_slice(tag.as_bytes());
    buf.push(0);
    socket.write_all(&buf).await.map_err(|e| crate::LumaError::Io(e))?;
    Ok(())
}

async fn send_error(socket: &mut TcpStream, msg: &str) -> Result<()> {
    socket.write_u8(b'E').await.map_err(|e| crate::LumaError::Io(e))?;
    // Length... calculation is annoying. simplified.
    // ErrorResponse is structured: len(u32), then fields (Type(u8), Str(nullterm)).
    
    let mut payload = Vec::new();
    payload.push(b'S'); // Severity
    payload.extend_from_slice(b"ERROR\0");
    payload.push(b'M'); // Message
    payload.extend_from_slice(msg.as_bytes());
    payload.push(0); // Null term
    payload.push(0); // End of fields

    socket.write_u32(4 + payload.len() as u32).await.map_err(|e| crate::LumaError::Io(e))?;
    socket.write_all(&payload).await.map_err(|e| crate::LumaError::Io(e))?;
    Ok(())
}
