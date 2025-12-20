//! Authentication module
//!
//! Provides user authentication without pgwire dependency.

use std::sync::Arc;
use crate::security::SecurityManager;
use crate::crypto::{Md5, Rng};

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

pub trait Authenticator: Send + Sync {
    fn authenticate(&self, user: &str, pass: &str) -> bool;
}

/// Simple password authenticator
pub struct SimpleAuthenticator {
    security: Arc<SecurityManager>,
}

impl SimpleAuthenticator {
    pub fn new(security: Arc<SecurityManager>) -> Self {
        Self { security }
    }
}

impl Authenticator for SimpleAuthenticator {
    fn authenticate(&self, user: &str, pass: &str) -> bool {
        match self.security.get_user(user) {
            Some(u) => {
                // Simple hash comparison
                let hash = format!("{:x}", md5::compute(pass.as_bytes()));
                u.password_hash == hash
            }
            None => false,
        }
    }
}

// Note: PostgreSQL wire-protocol authentication (pgwire integration)
// is disabled until pgwire crate is available.
// When re-enabled, implement StartupHandler trait here.
