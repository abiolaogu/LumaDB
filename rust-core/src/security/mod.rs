use crate::{Result, LumaError, Document};
use std::sync::Arc;
use dashmap::DashMap;

pub mod rbac;
pub mod auth;
pub mod rate_limit;

pub use rbac::{Role, Permission, AccessControl};
pub use auth::{Authenticator, User};
pub use rate_limit::RateLimiter;

/// Security Manager for LumaDB
pub struct SecurityManager {
    users: DashMap<String, User>,
    roles: DashMap<String, Role>,
}

impl SecurityManager {
    pub fn new() -> Self {
        let sm = Self {
            users: DashMap::new(),
            roles: DashMap::new(),
        };
        
        // Default Admin User
        // In production this would come from storage or config
        let admin_role = Role::new("admin")
            .with_permission(Permission::All);
            
        let admin_user = User::new("admin", "admin") // Default password
            .with_role("admin");
            
        sm.roles.insert("admin".to_string(), admin_role);
        sm.users.insert("admin".to_string(), admin_user);
        
        sm
    }

    pub fn get_user(&self, username: &str) -> Option<User> {
        self.users.get(username).map(|u| u.clone())
    }
    
    pub fn verify_password(&self, username: &str, hash: &str) -> bool {
        // Validation logic (e.g. check hash match)
        // Stub: Check plaintext logic or MD5 check
        // pgwire sends MD5(password + salt) usually.
        // For simplicity in this module, we just expose user retrieval.
        true
    }
}
