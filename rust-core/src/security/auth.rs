use pgwire::api::auth::StartupHandler;
use pgwire::messages::startup::StartupMessage;
use pgwire::messages::frontend::ClientMessage;
use pgwire::messages::backend::{AuthenticationRequest, BackendMessage};
use pgwire::error::{PgWireError, PgWireResult};
use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, FramedParts};
use pgwire::messages::PgWireMessageServerCodec;
use std::sync::Arc;
use crate::security::SecurityManager;
use crate::crypto::{Md5, Rng}; // Internal

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub password_hash: String,
    pub roles: Vec<String>,
}

impl User {
    pub fn new(username: &str, password_hash: &str) -> Self {
        Self {
            username: username.to_string(),
            password_hash: password_hash.to_string(),
            roles: Vec::new(),
        }
    }

    pub fn with_role(mut self, role: &str) -> Self {
        self.roles.push(role.to_string());
        self
    }
}

pub trait Authenticator {
    fn authenticate(&self, user: &str, pass: &str) -> bool;
}

#[derive(Clone)]
pub struct LumaStartupHandler {
    security: Arc<SecurityManager>,
}

impl LumaStartupHandler {
    pub fn new(security: Arc<SecurityManager>) -> Self {
        Self { security }
    }
}

#[async_trait::async_trait]
impl StartupHandler for LumaStartupHandler {
    async fn on_startup<S>(
        &self,
        stream: &mut S,
        message: &StartupMessage
    ) -> PgWireResult<()>
    where
        S: Unpin + Send + Sync + AsyncRead + AsyncWrite,
    {
        let params = match message {
            StartupMessage::Startup { params, .. } => params,
            _ => return Ok(()),
        };

        let username = params.get("user").ok_or(PgWireError::UserError("Missing user".into()))?;
        
        // 1. Fetch User (Stub: Accept any user for demo if not in DB)
        let _user_record = self.security.get_user(username).ok_or(
            PgWireError::UserError("Invalid username".into())
        )?;

        // 2. Generate Salt
        let mut salt = [0u8; 4];
        let mut rng = Rng::new();
        rng.fill(&mut salt);

        // 3. Send Auth Request
        let mut framed = Framed::new(stream, PgWireMessageServerCodec::new());
        framed.send(BackendMessage::Authentication(AuthenticationRequest::MD5Password(salt))).await?;

        // 4. Read Response
        // Note: In demo with stubbed crypto, we cannot verify properly.
        // We will accept any valid PasswordMessage response.
        let response = framed.next().await.ok_or(PgWireError::IoError(std::io::Error::from(std::io::ErrorKind::ConnectionAborted)))??;
        
        match response {
            ClientMessage::PasswordMessage(_) => {
                 // BYPASS VERIFICATION due to missing real MD5
                 eprintln!("Auth: Skipping signature verification (Demo Mode)");
                 framed.send(BackendMessage::Authentication(AuthenticationRequest::Ok)).await?;
                 Ok(())
            },
            _ => Err(PgWireError::UserError("Expected password message".into())),
        }
    }
}
