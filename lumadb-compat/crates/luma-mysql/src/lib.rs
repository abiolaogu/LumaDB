use luma_protocol_core::{ProtocolAdapter, QueryProcessor, Result};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use async_trait::async_trait;
use bytes::BytesMut;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, error};
use serde::Deserialize;

pub mod protocol;
pub mod types;
pub mod compat;
pub mod parser;
pub mod translator;

pub use protocol::{MySQLProtocol, Packet};

#[derive(Debug, Deserialize, Clone)]
pub struct MySqlConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub max_connections: u32,
    pub version_string: String,
}

pub async fn run(config: MySqlConfig, sem: Arc<Semaphore>) -> Result<(), anyhow::Error> {
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("MySQL listener started on {}", addr);

    loop {
        let permit = sem.clone().acquire_owned().await?;
        let (socket, _) = listener.accept().await?;
        
        tokio::spawn(async move {
            let _permit = permit;
            let protocol = MySQLProtocol::new();
             // Mock processor
            let processor = Box::new(luma_protocol_core::MockQueryProcessor);
             // Stub address
            let peer_addr = socket.peer_addr().unwrap_or("0.0.0.0:0".parse().unwrap());

            if let Err(e) = protocol.handle_connection(socket, peer_addr, processor).await {
                error!("MySQL Connection error: {}", e);
            }
        });
    }
}

#[async_trait]
impl ProtocolAdapter for MySQLProtocol {
    fn default_port(&self) -> u16 { 3306 }

    async fn handle_connection(
        &self,
        mut socket: TcpStream,
        _addr: SocketAddr,
        processor: Box<dyn QueryProcessor>, 
    ) -> Result<()> {
        let mut buffer = BytesMut::with_capacity(4096);
        let mut seq_id = 0;

        // --- Handshake ---
        // 1. Send Initial Handshake
        let mut resp = BytesMut::new();
        let handshake = protocol::handshake::HandshakeV10::new(0); // Thread ID 0
        handshake.write(&mut resp);
        MySQLProtocol::write_packet(&resp, seq_id, &mut buffer); // seq 0
        socket.write_all(&buffer).await?;
        buffer.clear();
        
        // 2. Read Handshake Response
        // Expecting HandshakeResponse41
        // Simplified: Just read packet and ignore auth for now (Trust auth)
        loop {
            // Read until we get a full packet
            if buffer.len() == 0 { // read more
                 let mut tmp = [0u8; 1024];
                 let n = socket.read(&mut tmp).await?;
                 if n == 0 { return Ok(()); }
                 buffer.extend_from_slice(&tmp[..n]);
            }
            
            if let Some(packet) = MySQLProtocol::read_packet(&mut buffer)? {
                seq_id = packet.header.seq_id + 1;
                // Assume Auth OK
                let mut ok_buf = BytesMut::new();
                let mut ok_pack_buf = BytesMut::new();
                protocol::packets::OKPacket::default().write(&mut ok_buf);
                MySQLProtocol::write_packet(&ok_buf, seq_id, &mut ok_pack_buf);
                socket.write_all(&ok_pack_buf).await?;
                break;
            }
        }
        
        seq_id = 0; // Reset or continue? Usually reset for Command Phase? No, continues incrementing or resets per command.
        // Command phase usually starts with seq_id 0 for each new command.
        
        // --- Command Loop ---
        loop {
             buffer.clear(); // We parsed previous packet
             let mut tmp = [0u8; 4096];
             let n = socket.read(&mut tmp).await?;
             if n == 0 { return Ok(()); }
             buffer.extend_from_slice(&tmp[..n]);
             
             while let Some(packet) = MySQLProtocol::read_packet(&mut buffer)? {
                 seq_id = packet.header.seq_id;
                 let payload = packet.payload;
                 if payload.is_empty() { continue; }
                 
                 let cmd_byte = payload[0];
                 match cmd_byte {
                     0x03 => { // COM_QUERY
                         let query_str = String::from_utf8_lossy(&payload[1..]).to_string();
                         info!("Received MySQL Query: {}", query_str);
                         
                         // 1. Parse
                         let dialect = sqlparser::dialect::MySqlDialect {};
                         let ast = match sqlparser::parser::Parser::parse_sql(&dialect, &query_str) {
                             Ok(ast) => ast,
                             Err(e) => {
                                 // Send ERR
                                 send_err(&mut socket, seq_id + 1, &e.to_string()).await?;
                                 continue;
                             }
                         };
                         
                         // 2. Translate
                          if let Some(stmt) = ast.into_iter().next() {
                               match crate::translator::MysqlTranslator::translate(stmt) {
                                   Ok(ir_op) => {
                                       // 3. Execute
                                       info!("Generated LumaIR: {:?}", ir_op);
                                       // Mock Resultset
                                       send_mock_resultset(&mut socket).await?;
                                   },
                                   Err(e) => send_err(&mut socket, seq_id + 1, &e.to_string()).await?,
                               }
                          } else {
                              send_ok(&mut socket, seq_id + 1).await?;
                          }
                     },
                     0x01 => return Ok(()), // COM_QUIT
                     _ => {
                         // Unsupported
                         send_err(&mut socket, seq_id + 1, "Unsupported Command").await?;
                     }
                 }
             }
        }
    }
}

async fn send_ok(socket: &mut TcpStream, seq_id: u8) -> Result<(), anyhow::Error> {
    let mut payload = BytesMut::new();
    protocol::packets::OKPacket::default().write(&mut payload);
    let mut packet = BytesMut::new();
    MySQLProtocol::write_packet(&payload, seq_id, &mut packet);
    socket.write_all(&packet).await?;
    Ok(())
}

async fn send_err(socket: &mut TcpStream, seq_id: u8, msg: &str) -> Result<(), anyhow::Error> {
     let mut payload = BytesMut::new();
    // protocol::packets::ERRPacket // Need to instantiate struct
    // Simplified ERR packet writing manually if struct not easy
    // Err packet: 0xFF, error_code (u16), marker '#', sql_state (5 bytes), msg
    use bytes::BufMut;
    payload.put_u8(0xFF);
    payload.put_u16_le(1000);
    payload.put_slice(b"#");
    payload.put_slice(b"HY000"); // Generic state
    payload.put_slice(msg.as_bytes());
    
    let mut packet = BytesMut::new();
    MySQLProtocol::write_packet(&payload, seq_id, &mut packet);
    socket.write_all(&packet).await?;
    Ok(())
}

async fn send_mock_resultset(socket: &mut TcpStream) -> Result<(), anyhow::Error> {
    // 1. Column Count (1)
    let mut packet = BytesMut::new();
    let cc_payload = vec![1]; // 1 column
    MySQLProtocol::write_packet(&cc_payload, 1, &mut packet);
    socket.write_all(&packet).await?;
    
    // 2. Column Def
    packet.clear();
    let col_def = protocol::packets::ColumnDefinition::dummy("col1");
    // write col def logic... assumes existing struct has write
    // implementation pending in packets.rs, assume generic bytes for now
    // ...
    // 3. EOF
    // 4. Rows
    // 5. EOF
    
    // Simplification: Just send OK for now to avoid hang
    send_ok(socket, 2).await
}
