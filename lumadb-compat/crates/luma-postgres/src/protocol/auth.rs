use luma_protocol_core::{ProtocolError, Result};
use super::messages::{BackendMessage, FrontendMessage};
use rand::{Rng, thread_rng};
use md5::{Md5, Digest};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthMethod {
    Trust,
    MD5,
    SCRAM,
}

pub struct Authenticator {
    method: AuthMethod,
    salt: [u8; 4],
}

impl Authenticator {
    pub fn new(method: AuthMethod) -> Self {
        let mut salt = [0u8; 4];
        thread_rng().fill(&mut salt);
        Self { method, salt }
    }

    pub fn begin_auth(&self) -> BackendMessage {
        match self.method {
            AuthMethod::Trust => BackendMessage::AuthenticationOk,
            AuthMethod::MD5 => BackendMessage::AuthenticationMD5Password { salt: self.salt },
            AuthMethod::SCRAM => BackendMessage::AuthenticationSASL { 
                mechanisms: vec!["SCRAM-SHA-256".to_string()] 
            },
        }
    }

     // Simple MD5 verification for now. SCRAM is complex and will be added stepwise.
    pub fn verify_md5(&self, username: &str, password_hash: &str) -> Result<bool> {
         // Client sends: md5(md5(password + user) + salt)
         // We need to verify this against stored credential.
         // Assumption: We store plain text password or the md5(pass+user) hash.
         // For this stage, let's assume we have the plain password for "postgres" user = "postgres" (or whatever)
         // In real DB, we store the hash.
         
         // Let's implement verification against a known password "postgres" for user "postgres"
         if username != "postgres" {
             return Ok(false);
         }
         let password = "postgres"; // Hardcoded for prototype

         // Expected = md5(md5(password + username) + salt)
         // 1. inner = md5(password + username)
         let mut hasher = Md5::new();
         hasher.update(password.as_bytes());
         hasher.update(username.as_bytes());
         let inner_hash = hex::encode(hasher.finalize()); // hex string

         // 2. outer = md5(inner_hash + salt)
         let mut hasher = Md5::new();
         hasher.update(inner_hash.as_bytes());
         hasher.update(self.salt);
         let expected_hash = format!("md5{}", hex::encode(hasher.finalize()));

         Ok(password_hash == expected_hash)
    }
}
