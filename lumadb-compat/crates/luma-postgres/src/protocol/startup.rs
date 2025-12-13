use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::BytesMut;
use crate::protocol::messages::{StartupMessage, BackendMessage, FrontendMessage};
use crate::protocol::auth::{Authenticator, AuthMethod};
use luma_protocol_core::{ProtocolError, Result};
use std::collections::HashMap;

pub struct ConnectionStartup;

impl ConnectionStartup {
    pub async fn handle_handshake<S>(socket: &mut S) -> Result<HashMap<String, String>>
    where
        S: AsyncReadExt + AsyncWriteExt + Unpin,
    {
        let mut buffer = BytesMut::with_capacity(1024);

        // 1. Read StartupMessage
        // We need to read initial bytes to get length
        let mut len_bytes = [0u8; 4];
        socket.read_exact(&mut len_bytes).await?;
        let len = i32::from_be_bytes(len_bytes) as usize;
        
        buffer.extend_from_slice(&len_bytes);
        buffer.resize(len, 0);
        socket.read_exact(&mut buffer[4..]).await?;

        let startup = StartupMessage::parse(&mut buffer)?.ok_or(ProtocolError::Protocol("Incomplete startup message".into()))?;

        match startup {
            StartupMessage::SslRequest => {
                // Deny SSL for now -> 'N'
                socket.write_u8(b'N').await?;
                // Client usually retries with normal startup immediately
                // Recursively call handle? Or strict loop?
                // Let's recurse once.
                return Box::pin(Self::handle_handshake(socket)).await;
            }
            StartupMessage::CancelRequest { .. } => {
                // Return generic error or handle cancellation logic
                return Err(ProtocolError::Protocol("Cancel request handling not implemented yet".into()));
            }
            StartupMessage::Normal { version, parameters } => {
                if version != 196608 { // 3.0
                     return Err(ProtocolError::Protocol(format!("Unsupported protocol version: {}", version)));
                }

                // 2. Authentication
                // For now, let's pick MD5 as default if not specified
                // Real implementation would look at config/hba.conf
                let auth = Authenticator::new(AuthMethod::MD5); 
                let msg = auth.begin_auth();
                
                let mut resp = BytesMut::new();
                msg.write(&mut resp);
                socket.write_all(&resp).await?;

                if let BackendMessage::AuthenticationMD5Password { .. } = msg {
                    // Expect PasswordMessage
                    // Read response
                    let mut type_byte = [0u8; 1];
                    socket.read_exact(&mut type_byte).await?;
                    if type_byte[0] != b'p' {
                         return Err(ProtocolError::Protocol("Expected PasswordMessage".into()));
                    }
                    
                    let mut len_bytes = [0u8; 4];
                    socket.read_exact(&mut len_bytes).await?;
                    let len = i32::from_be_bytes(len_bytes) as usize;
                    
                    let mut payload = vec![0u8; len - 4];
                    socket.read_exact(&mut payload).await?;
                    
                    // FrontendMessage::parse expects full frame including type and len?
                    // We already consumed type and len.
                    // Let's reconstruct or just use payload.
                    // Payload is the string/bytes.
                    
                    // MD5 password is usually c-string in the payload
                    // Find null terminator
                    if let Some(pos) = payload.iter().position(|&b| b == 0) {
                         let pass_hash = String::from_utf8_lossy(&payload[..pos]);
                         let user = parameters.get("user").map(|s| s.as_str()).unwrap_or("");
                         
                         if !auth.verify_md5(user, &pass_hash)? {
                              let mut err = BytesMut::new();
                              BackendMessage::ErrorResponse { 
                                  code: "28P01".into(), // invalid_password
                                  message: "Password authentication failed".into() 
                              }.write(&mut err);
                              socket.write_all(&err).await?;
                              return Err(ProtocolError::Protocol("Authentication failed".into()));
                         }
                    } else {
                        return Err(ProtocolError::Protocol("Invalid password format".into()));
                    }
                }
                
                // Auth Success
                let mut ok = BytesMut::new();
                BackendMessage::AuthenticationOk.write(&mut ok);
                socket.write_all(&ok).await?;

                // 3. ParameterStatus
                Self::send_parameter_status(socket).await?;

                // 4. BackendKeyData
                Self::send_backend_key_data(socket).await?;

                // 5. ReadyForQuery
                let mut ready = BytesMut::new();
                BackendMessage::ReadyForQuery.write(&mut ready);
                socket.write_all(&ready).await?;

                Ok(parameters)
            }
            _ => Err(ProtocolError::Protocol("Unexpected message during startup".into())),
        }
    }

    async fn send_parameter_status<S>(socket: &mut S) -> Result<()> 
    where S: AsyncWriteExt + Unpin {
        let mut buf = BytesMut::new();
        BackendMessage::ParameterStatus { name: "server_version".into(), value: "14.0".into() }.write(&mut buf);
        BackendMessage::ParameterStatus { name: "client_encoding".into(), value: "UTF8".into() }.write(&mut buf);
        BackendMessage::ParameterStatus { name: "DateStyle".into(), value: "ISO, MDY".into() }.write(&mut buf);
        socket.write_all(&buf).await?;
        Ok(())
    }

    async fn send_backend_key_data<S>(socket: &mut S) -> Result<()> 
    where S: AsyncWriteExt + Unpin {
        let mut buf = BytesMut::new();
        // TODO: Generate real keys and store mapping
        BackendMessage::BackendKeyData { process_id: 1234, secret_key: 5678 }.write(&mut buf);
        socket.write_all(&buf).await?;
        Ok(())
    }
}
