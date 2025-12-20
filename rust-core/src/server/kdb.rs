use crate::Database;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn start(db: Arc<Database>, port: u16) -> crate::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    if let Ok(listener) = TcpListener::bind(&addr).await {
         println!("LumaDB Kdb+ Adapter listening on {}", addr);
         loop {
             if let Ok((mut socket, _)) = listener.accept().await {
                 tokio::spawn(async move {
                     // Q IPC Handshake: 1 byte capability
                     let mut buf = [0; 1];
                     if let Ok(_) = socket.read(&mut buf).await {
                         let _ = socket.write_all(&[0x06]).await; // Reply V3 protocol
                     }
                 });
             }
         }
    }
    Ok(())
}
